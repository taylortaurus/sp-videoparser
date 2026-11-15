use crate::models::{HttpResponse, VideoParseInfo, VideoSource};
use crate::parser::{parse_video_id, parse_video_share_url, get_supported_platforms};
use crate::utils::{apply_proxy, extract_url_from_string};
use axum::{
    body::Body,
    extract::{Query, ConnectInfo, State},
    http::{header, HeaderMap, StatusCode, Method, Uri},
    middleware::{self, Next},
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tower_http::cors::CorsLayer;
use reqwest::Client;
use rust_embed::RustEmbed;

/// 简单的按 IP 固定窗口限流器（每分钟）
struct RateLimiter {
    rpm: u32,
    windows: Mutex<HashMap<IpAddr, (Instant, u32)>>,
}

impl RateLimiter {
    fn new(rpm: u32) -> Self {
        Self { rpm, windows: Mutex::new(HashMap::new()) }
    }

    /// 返回 true 表示允许，false 表示超出限额
    fn check(&self, ip: IpAddr) -> bool {
        let mut map = self.windows.lock().unwrap();
        let now = Instant::now();
        let entry = map.entry(ip).or_insert((now, 0));
        if now.duration_since(entry.0) >= Duration::from_secs(60) {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= self.rpm
    }
}

/// 安全中间件共享状态：限流器 + 可选 Basic Auth 凭据
#[derive(Clone)]
struct SecurityState {
    rate_limiter: Arc<RateLimiter>,
    auth: Option<Arc<(String, String)>>,
}

fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(default)
}

/// 限流 + Basic Auth 中间件
async fn security_middleware(
    State(state): State<SecurityState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();

    // 限流豁免：健康检查、静态资源与媒体代理。
    // 代理接口服务图集/视频在线预览，单次展示可能触发数十张图片请求；
    // 限流仅用于保护解析接口（且代理已有 SSRF 防护），故此处豁免。
    let rate_exempt = path == "/api/health"
        || path.starts_with("/assets")
        || path.starts_with("/api/proxy")
        || path == "/title.jpg";
    if !rate_exempt && !state.rate_limiter.check(addr.ip()) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"code":429,"msg":"请求过于频繁，请稍后再试"}"#,
        )
            .into_response();
    }

    // Basic Auth：仅在配置了凭据时启用
    if let Some(cred) = &state.auth {
        let auth_exempt = path == "/"
            || path == "/api/health"
            || path == "/api/platforms"
            || path.starts_with("/assets")
            || path == "/title.jpg";
        if !auth_exempt && !check_basic_auth(&req, cred) {
            return (
                StatusCode::UNAUTHORIZED,
                [
                    (header::CONTENT_TYPE, "application/json"),
                    (header::WWW_AUTHENTICATE, "Basic realm=\"parse-video\""),
                ],
                r#"{"code":401,"msg":"需要认证"}"#,
            )
                .into_response();
        }
    }

    next.run(req).await
}

/// 校验 Authorization: Basic 头是否匹配配置凭据
fn check_basic_auth(req: &axum::extract::Request, cred: &(String, String)) -> bool {
    let header_val = match req.headers().get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        Some(v) => v,
        None => return false,
    };
    let encoded = match header_val.strip_prefix("Basic ") {
        Some(e) => e,
        None => return false,
    };
    let decoded = match STANDARD.decode(encoded) {
        Ok(d) => d,
        Err(_) => return false,
    };
    let decoded_str = String::from_utf8_lossy(&decoded);
    let (user, pass) = match decoded_str.split_once(':') {
        Some(p) => p,
        None => return false,
    };
    user == cred.0 && pass == cred.1
}

/// Ctrl+C 优雅关闭信号
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("接收到中断信号, 开始优雅关闭服务");
}

/// 校验代理目标，防止 SSRF（仅允许 http/https 且解析到公网地址）
async fn validate_proxy_target(raw: &str) -> Result<(), String> {
    let parsed = url::Url::parse(raw).map_err(|_| "无效的URL".to_string())?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err("仅支持 http/https 协议".to_string()),
    }
    let host = parsed.host_str().ok_or_else(|| "URL缺少host".to_string())?.to_string();
    if host.eq_ignore_ascii_case("localhost") {
        return Err("禁止访问内网地址".to_string());
    }
    let port = parsed.port_or_known_default().unwrap_or(80);

    // 解析所有 IP 并逐一校验是否为内网/保留地址
    let addrs = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|e| format!("DNS解析失败: {}", e))?;
    let mut any = false;
    for addr in addrs {
        any = true;
        if ip_is_blocked(addr.ip()) {
            return Err("禁止访问内网/保留地址".to_string());
        }
    }
    if !any {
        return Err("无法解析目标主机".to_string());
    }
    Ok(())
}

/// 判断 IP 是否为内网/保留/特殊地址
fn ip_is_blocked(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_multicast()
                || v4.is_broadcast()
                || v4.octets()[0] == 0
        }
        IpAddr::V6(v6) => {
            if v6.is_loopback() || v6.is_unspecified() || v6.is_multicast() {
                return true;
            }
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return ip_is_blocked(IpAddr::V4(mapped));
            }
            let seg = v6.segments();
            // fc00::/7 唯一本地地址 或 fe80::/10 链路本地地址
            (seg[0] & 0xfe00) == 0xfc00 || (seg[0] & 0xffc0) == 0xfe80
        }
    }
}

async fn logger_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method_str = method.to_string();
    let path = uri.path().to_string();
    let query = uri.query().unwrap_or("").to_string();

    // 处理请求
    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status().as_u16();
    let latency_ms = duration.as_secs_f64() * 1000.0;

    // 结构化访问日志
    tracing::info!(
        client_ip = %addr.ip(),
        method = %method_str,
        path = %path,
        query = %query,
        status = status,
        latency_ms = format!("{:.2}", latency_ms),
        "请求处理完成"
    );

    response
}

pub async fn start_server(port: u16) -> anyhow::Result<()> {
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "视频解析服务正在启动");

    // API 路由（带 /api 前缀）
    let api_routes = Router::new()
        .route("/health", get(health_handler))
        .route("/video/share/url/parse", get(parse_share_url_handler))
        .route("/video/id/parse", get(parse_video_id_handler))
        .route("/platforms", get(platforms_handler))
        .route("/proxy/video", get(proxy_video_handler))
        .route("/proxy/image", get(proxy_image_handler));

    // 安全中间件状态：限流（RATE_LIMIT_RPM，默认 60）+ 可选 Basic Auth
    let rate_limit_rpm = env_u32("RATE_LIMIT_RPM", 60);
    let username = std::env::var("PARSE_VIDEO_USERNAME").unwrap_or_default();
    let password = std::env::var("PARSE_VIDEO_PASSWORD").unwrap_or_default();
    let auth = if !username.is_empty() && !password.is_empty() {
        Some(Arc::new((username, password)))
    } else {
        None
    };
    tracing::info!(rate_limit_rpm, "限流策略已配置");
    tracing::info!(enabled = auth.is_some(), "Basic 认证策略已配置");
    if std::env::var("PARSE_VIDEO_PROXY").map(|v| !v.is_empty()).unwrap_or(false) {
        tracing::info!("已启用出站代理");
    }
    let security_state = SecurityState {
        rate_limiter: Arc::new(RateLimiter::new(rate_limit_rpm)),
        auth,
    };

    // 主应用路由
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/title.jpg", get(title_image_handler))
        .route("/assets/*path", get(assets_handler))
        .nest("/api", api_routes)
        .layer(middleware::from_fn_with_state(
            security_state,
            security_middleware,
        ))
        .layer(middleware::from_fn(logger_middleware))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(addr = %addr, "HTTP 服务已就绪, 开始监听");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("服务已停止");
    Ok(())
}

/// 前端静态资源（编译期由 templates/dist 内嵌进二进制，serve 无需外部文件）
#[derive(RustEmbed)]
#[folder = "templates/dist/"]
struct Assets;

/// 按扩展名推断 MIME 类型
fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}

/// 从内嵌资源返回指定文件
fn embedded_response(path: &str) -> Response {
    match Assets::get(path) {
        Some(file) => (
            [(header::CONTENT_TYPE, mime_for(path))],
            file.data.into_owned(),
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

async fn index_handler() -> impl IntoResponse {
    match Assets::get("index.html") {
        Some(file) => Html(String::from_utf8_lossy(&file.data).into_owned()).into_response(),
        None => {
            tracing::error!("内嵌资源中缺少 index.html, 前端可能未构建");
            (StatusCode::INTERNAL_SERVER_ERROR, "前端资源未构建").into_response()
        }
    }
}

async fn title_image_handler() -> impl IntoResponse {
    embedded_response("title.jpg")
}

async fn assets_handler(axum::extract::Path(path): axum::extract::Path<String>) -> impl IntoResponse {
    embedded_response(&format!("assets/{}", path))
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "time": chrono::Local::now().to_rfc3339(),
    }))
}

#[derive(Debug, Deserialize)]
struct ParseShareUrlQuery {
    url: String,
}

async fn parse_share_url_handler(
    Query(params): Query<ParseShareUrlQuery>,
) -> Json<HttpResponse<VideoParseInfo>> {
    let url = match extract_url_from_string(&params.url) {
        Ok(url) => url,
        Err(e) => {
            return Json(HttpResponse::error(format!("URL提取失败: {}", e)));
        }
    };
    
    match parse_video_share_url(&url).await {
        Ok(info) => Json(HttpResponse::success(info)),
        Err(e) => Json(HttpResponse::error(format!("视频解析失败: {}", e))),
    }
}

#[derive(Debug, Deserialize)]
struct ParseVideoIdQuery {
    source: String,
    video_id: String,
}

async fn parse_video_id_handler(
    Query(params): Query<ParseVideoIdQuery>,
) -> Json<HttpResponse<VideoParseInfo>> {
    // 解析平台来源
    let source = match VideoSource::from_str(&params.source) {
        Some(s) => s,
        None => {
            return Json(HttpResponse::error(format!(
                "不支持的平台: {}",
                params.source
            )));
        }
    };

    // 解析视频
    match parse_video_id(source, &params.video_id).await {
        Ok(info) => Json(HttpResponse::success(info)),
        Err(e) => Json(HttpResponse::error(format!("视频解析失败: {}", e))),
    }
}

#[derive(Debug, Serialize)]
struct PlatformInfo {
    source: String,
    name: String,
    domains: Vec<String>,
}

async fn platforms_handler() -> Json<HttpResponse<Vec<PlatformInfo>>> {
    let platforms = get_supported_platforms()
        .into_iter()
        .map(|(source, name, domains)| PlatformInfo {
            source: source.as_str().to_string(),
            name: name.to_string(),
            domains: domains.iter().map(|s| s.to_string()).collect(),
        })
        .collect();

    Json(HttpResponse::success(platforms))
}

#[derive(Debug, Deserialize)]
struct ProxyQuery {
    url: String,
}

fn video_proxy_request(client: &Client, url: &str) -> reqwest::RequestBuilder {
    let mut request = client.get(url).header(reqwest::header::ACCEPT, "*/*");
    let twitter_video = reqwest::Url::parse(url)
        .map(|url| url.host_str() == Some("video.twimg.com"))
        .unwrap_or(false);
    // Twitter CDN rejects Douyin's Origin; retain the existing profile for other platforms.
    if !twitter_video {
        request = request
            .header(reqwest::header::REFERER, "https://www.douyin.com/")
            .header(reqwest::header::ORIGIN, "https://www.douyin.com");
    }
    request
}

async fn proxy_video_handler(
    Query(params): Query<ProxyQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    tracing::info!(url = %params.url, "收到视频代理请求");

    // SSRF 防护：校验目标地址
    if let Err(e) = validate_proxy_target(&params.url).await {
        tracing::warn!(url = %params.url, reason = %e, "视频代理请求被拒绝");
        return (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            e,
        )
            .into_response();
    }

    let client = match apply_proxy(
        Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(60)),
    )
    .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "创建 HTTP 客户端失败");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain")],
                "创建HTTP客户端失败".to_string(),
            )
                .into_response();
        }
    };

    // 转发客户端的 Range 请求头，支持视频分段拖动播放
    let mut req = video_proxy_request(&client, &params.url);
    if let Some(range) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        req = req.header(reqwest::header::RANGE, range);
    }

    let response = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "拉取视频资源失败");
            return (
                StatusCode::BAD_GATEWAY,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                format!("获取视频失败: {}", e),
            )
                .into_response();
        }
    };

    // 透传上游状态码（200 或 206 Partial Content）
    let status = StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::OK);
    let mut builder = Response::builder().status(status);

    // 透传关键响应头（reqwest 与 axum 的 http 版本不同，统一按字符串转发）
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("video/mp4")
        .to_string();
    builder = builder.header(header::CONTENT_TYPE, content_type);

    if let Some(v) = response.headers().get(reqwest::header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()) {
        builder = builder.header(header::CONTENT_LENGTH, v);
    }
    if let Some(v) = response.headers().get(reqwest::header::CONTENT_RANGE).and_then(|v| v.to_str().ok()) {
        builder = builder.header(header::CONTENT_RANGE, v);
    }
    builder = builder
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "public, max-age=31536000");

    // 流式转发响应体，避免大视频全量进内存
    builder
        .body(Body::from_stream(response.bytes_stream()))
        .unwrap()
        .into_response()
}

async fn proxy_image_handler(Query(params): Query<ProxyQuery>) -> impl IntoResponse {
    tracing::debug!(url = %params.url, "收到图片代理请求");

    // SSRF 防护：校验目标地址
    if let Err(e) = validate_proxy_target(&params.url).await {
        tracing::warn!(url = %params.url, reason = %e, "图片代理请求被拒绝");
        return (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            e,
        )
            .into_response();
    }

    // 创建HTTP客户端
    let client = match apply_proxy(
        Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(15)),
    )
    .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "创建 HTTP 客户端失败");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain")],
                "创建HTTP客户端失败",
            )
                .into_response();
        }
    };
    
    let response = match client.get(&params.url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "拉取图片资源失败");
            return (
                StatusCode::BAD_GATEWAY,
                [(header::CONTENT_TYPE, "text/plain")],
                format!("获取图片失败: {}", e),
            )
                .into_response();
        }
    };
    
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    
    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "读取图片数据失败");
            return (
                StatusCode::BAD_GATEWAY,
                [(header::CONTENT_TYPE, "text/plain")],
                format!("读取图片数据失败: {}", e),
            )
                .into_response();
        }
    };
    
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "public, max-age=31536000")
        .body(Body::from(bytes))
        .unwrap()
        .into_response()
}


#[cfg(test)]
mod proxy_tests {
    use super::video_proxy_request;
    use reqwest::{header, Client};

    #[test]
    fn twitter_video_omits_douyin_origin() {
        let client = Client::new();
        for url in [
            "https://video.twimg.com/ext_tw_video/1/video.mp4",
            "https://VIDEO.TWIMG.COM/ext_tw_video/1/video.mp4",
        ] {
            let request = video_proxy_request(&client, url).build().unwrap();
            assert!(!request.headers().contains_key(header::REFERER), "{}", url);
            assert!(!request.headers().contains_key(header::ORIGIN), "{}", url);
            assert_eq!(request.headers()[header::ACCEPT], "*/*");
        }
    }

    #[test]
    fn other_video_hosts_keep_douyin_origin() {
        let client = Client::new();
        for url in [
            "https://v3.douyinvod.com/video.mp4",
            "https://video.twimg.com.example.org/video.mp4",
            "https://video.twimg.com@cdn.example.org/video.mp4",
            "https://cdn.example.org/video.mp4?source=video.twimg.com",
        ] {
            let request = video_proxy_request(&client, url).build().unwrap();
            assert_eq!(
                request.headers()[header::REFERER],
                "https://www.douyin.com/",
                "{}", url
            );
            assert_eq!(
                request.headers()[header::ORIGIN],
                "https://www.douyin.com",
                "{}", url
            );
        }
    }
}

#[cfg(test)]
mod ssrf_tests {
    use super::ip_is_blocked;
    use std::net::IpAddr;

    #[test]
    fn blocks_private_and_reserved() {
        for s in ["127.0.0.1", "10.0.0.1", "192.168.1.1", "172.16.0.1", "169.254.169.254", "0.0.0.0", "::1"] {
            assert!(ip_is_blocked(s.parse::<IpAddr>().unwrap()), "应拦截 {}", s);
        }
    }

    #[test]
    fn allows_public() {
        for s in ["8.8.8.8", "1.1.1.1", "114.114.114.114"] {
            assert!(!ip_is_blocked(s.parse::<IpAddr>().unwrap()), "应放行 {}", s);
        }
    }
}

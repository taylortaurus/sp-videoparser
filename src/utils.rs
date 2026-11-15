use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest::{redirect::Policy, Client};
use std::time::Duration;

pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (iPhone; CPU iPhone OS 26_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Mobile/15E148 Safari/604.1";

/// 抖音移动分享页使用的稳定请求画像。该页面会在无浏览器挑战的情况下
/// 将完整视频数据嵌入 `window._ROUTER_DATA`，适合作为协议级回退入口。
pub const DOUYIN_MOBILE_USER_AGENT: &str =
    "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Mobile/15E148 Safari/604.1";

pub fn extract_url_from_string(text: &str) -> Result<String> {
    let url_regex = Regex::new(r"https?://[\w.-]+[\w/-]*[\w.-:]*\??[\w=&:\-+%.]*/*")?;

    url_regex
        .find(text)
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| anyhow!("字符串中未找到URL"))
}

/// 若设置了 PARSE_VIDEO_PROXY 环境变量，则为客户端应用代理
pub(crate) fn apply_proxy(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    match std::env::var("PARSE_VIDEO_PROXY") {
        Ok(proxy) if !proxy.is_empty() => match reqwest::Proxy::all(&proxy) {
            Ok(p) => builder.proxy(p),
            Err(_) => builder,
        },
        _ => builder,
    }
}

pub fn create_http_client() -> Result<Client> {
    apply_proxy(
        Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(DEFAULT_USER_AGENT),
    )
    .build()
    .map_err(|e| anyhow!("创建HTTP客户端失败: {}", e))
}

pub fn create_cookie_http_client() -> Result<Client> {
    apply_proxy(
        Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(DEFAULT_USER_AGENT)
            .cookie_store(true),
    )
    .build()
    .map_err(|e| anyhow!("创建HTTP客户端失败: {}", e))
}

pub fn create_no_redirect_client() -> Result<Client> {
    apply_proxy(
        Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(DEFAULT_USER_AGENT)
            .redirect(Policy::none()),
    )
    .build()
    .map_err(|e| anyhow!("创建HTTP客户端失败: {}", e))
}

pub fn generate_numeric_id(length: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut result = String::with_capacity(length);

    for _ in 0..length {
        result.push(char::from_digit(rng.gen_range(0..10), 10).unwrap());
    }

    result
}

pub fn generate_random_string(length: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut rng = rand::thread_rng();

    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub fn extract_json_from_html(html: &str, pattern: &str) -> Result<String> {
    let re = Regex::new(pattern)?;

    re.captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
        .ok_or_else(|| anyhow!("无法从HTML中提取JSON数据"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_url() {
        let text = "分享视频 https://v.douyin.com/abc123/ 快来看";
        let url = extract_url_from_string(text).unwrap();
        assert_eq!(url, "https://v.douyin.com/abc123/");
    }

    #[test]
    fn test_generate_random_string() {
        let s = generate_random_string(10);
        assert_eq!(s.len(), 10);
    }
}

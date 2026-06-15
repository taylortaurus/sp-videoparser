mod download;
mod models;
mod parser;
mod server;
mod utils;

use clap::{Parser as ClapParser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// 本地时区时间戳格式化器（ISO 8601，带时区偏移），用于结构化日志
struct LocalTimer;

impl tracing_subscriber::fmt::time::FormatTime for LocalTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z"))
    }
}

#[derive(ClapParser)]
#[command(name = "rust_video_parser")]
#[command(author = "Rust Video Parser Team")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "短视频去水印解析工具 - 支持26个平台", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// HTTP服务器端口（可由环境变量 SERVER_PORT 指定）
    #[arg(short, long, env = "SERVER_PORT", default_value = "8080")]
    port: u16,
}

#[derive(Subcommand)]
enum Commands {
    /// 启动HTTP服务器
    Serve {
        #[arg(short, long, env = "SERVER_PORT", default_value = "8080")]
        port: u16,
    },
    /// 解析单个分享链接并输出结果
    Parse {
        url: String,
        /// 以 JSON 输出（结构同 HTTP API 响应，便于脚本处理）
        #[arg(long)]
        json: bool,
    },
    /// 根据平台标识与视频 ID 解析（对应 HTTP 接口 /api/video/id/parse）
    Id {
        /// 视频 ID
        video_id: String,
        /// 平台标识，如 douyin、bilibili（见 platforms 命令）
        #[arg(short, long)]
        source: String,
        /// 以 JSON 输出
        #[arg(long)]
        json: bool,
    },
    /// 解析并下载视频/图集到本地目录
    Download {
        url: String,
        /// 输出目录
        #[arg(short, long, default_value = "downloads")]
        output: String,
    },
    /// 列出所有受支持的平台
    Platforms,
    /// 显示版本信息
    Version,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载 .env 配置（若存在），便于通过环境变量集中管理运行参数
    let _ = dotenvy::dotenv();

    // 初始化工业级结构化日志：本地时区时间戳 + 日志级别 + 模块目标 + 键值字段
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_video_parser=info,tower_http=warn".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_timer(LocalTimer)
                .with_level(true)
                .with_target(true)
                .with_thread_ids(false)
                .with_ansi(true),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Serve { port }) => {
            server::start_server(port).await?;
        }
        Some(Commands::Parse { url, json }) => {
            parse_video(&url, json).await?;
        }
        Some(Commands::Id { video_id, source, json }) => {
            parse_by_id(&source, &video_id, json).await?;
        }
        Some(Commands::Download { url, output }) => {
            download_video(&url, &output).await?;
        }
        Some(Commands::Platforms) => {
            list_platforms();
        }
        Some(Commands::Version) => {
            println!("rust_video_parser {}", env!("CARGO_PKG_VERSION"));
        }
        None => {
            server::start_server(cli.port).await?;
        }
    }

    Ok(())
}

/// 解析分享链接并打印结果
async fn parse_video(url_text: &str, json: bool) -> anyhow::Result<()> {
    use crate::utils::extract_url_from_string;
    use crate::parser::parse_video_share_url;

    let url = extract_url_from_string(url_text)?;
    if !json {
        println!("正在解析视频...");
        println!("提取到链接: {}", url);
    }

    let result = parse_video_share_url(&url).await;
    print_parse_result(result, json);
    Ok(())
}

/// 根据平台标识与视频 ID 解析并打印结果
async fn parse_by_id(source: &str, video_id: &str, json: bool) -> anyhow::Result<()> {
    use crate::models::VideoSource;
    use crate::parser::parse_video_id;

    let src = match VideoSource::from_str(source) {
        Some(s) => s,
        None => {
            let msg = format!("不支持的平台: {}（可用 platforms 命令查看支持列表）", source);
            print_parse_result(Err(anyhow::anyhow!(msg)), json);
            return Ok(());
        }
    };

    if !json {
        println!("正在解析视频...");
    }
    let result = parse_video_id(src, video_id).await;
    print_parse_result(result, json);
    Ok(())
}

/// 统一输出解析结果：`--json` 时输出与 HTTP API 一致的响应结构，否则为人类可读文本
fn print_parse_result(result: anyhow::Result<crate::models::VideoParseInfo>, json: bool) {
    use crate::models::{HttpResponse, VideoParseInfo};

    if json {
        let resp = match result {
            Ok(info) => HttpResponse::success(info),
            Err(e) => HttpResponse::<VideoParseInfo>::error(e.to_string()),
        };
        match serde_json::to_string_pretty(&resp) {
            Ok(s) => println!("{}", s),
            Err(e) => eprintln!("结果序列化失败: {}", e),
        }
        return;
    }

    match result {
        Ok(info) => {
            println!("解析成功");
            println!("标题: {}", info.title);
            println!("作者: {} ({})", info.author.name, info.author.uid);

            if let Some(video_url) = &info.video_url {
                println!("视频地址: {}", video_url);
            }
            if let Some(music_url) = &info.music_url {
                println!("音乐地址: {}", music_url);
            }
            if let Some(cover_url) = &info.cover_url {
                println!("封面地址: {}", cover_url);
            }
            if !info.images.is_empty() {
                println!("图集（{} 张）:", info.images.len());
                for (i, img) in info.images.iter().enumerate() {
                    println!("  [{}] {}", i + 1, img.url);
                    if let Some(live_photo) = &img.live_photo_url {
                        println!("      LivePhoto: {}", live_photo);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("解析失败: {}", e);
        }
    }
}

/// 解析视频并下载媒体文件到本地
async fn download_video(url_text: &str, output: &str) -> anyhow::Result<()> {
    use crate::utils::extract_url_from_string;
    use crate::parser::parse_video_share_url;

    println!("正在解析视频...");
    let url = extract_url_from_string(url_text)?;

    match parse_video_share_url(&url).await {
        Ok(info) => {
            println!("解析成功: {}", info.title);
            crate::download::download_media(&info, output).await?;
        }
        Err(e) => {
            eprintln!("解析失败: {}", e);
        }
    }

    Ok(())
}

fn list_platforms() {
    use crate::parser::get_supported_platforms;

    println!("受支持的视频平台:\n");

    let platforms = get_supported_platforms();
    for (i, (source, name, domains)) in platforms.iter().enumerate() {
        println!("{:>2}. {} ({})", i + 1, name, source.as_str());
        println!("    域名: {}", domains.join(", "));
    }

    println!("\n合计 {} 个平台", platforms.len());
}

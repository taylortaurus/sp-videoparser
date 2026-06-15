use crate::models::VideoParseInfo;
use crate::utils::DEFAULT_USER_AGENT;
use anyhow::{anyhow, Result};
use futures::StreamExt;
use regex::Regex;
use std::path::Path;
use tokio::io::AsyncWriteExt;

/// 清理文件名中的非法字符
pub fn sanitize_filename(name: &str) -> String {
    let re = Regex::new(r#"[<>:"/\\|?*\x00-\x1f]"#).unwrap();
    let mut name = re.replace_all(name, "_").trim().to_string();
    if name.is_empty() {
        name = "download".to_string();
    }
    // 截断过长的文件名（按字符）
    let chars: Vec<char> = name.chars().collect();
    if chars.len() > 200 {
        name = chars[..200].iter().collect();
    }
    name
}

/// 从 URL 路径提取文件扩展名（含点，例如 ".mp4"）
pub fn ext_from_url(raw_url: &str) -> String {
    let parsed = match url::Url::parse(raw_url) {
        Ok(u) => u,
        Err(_) => return String::new(),
    };
    let path = parsed.path();
    if let Some(dot) = path.rfind('.') {
        let ext = path[dot..].to_lowercase();
        // 仅在 ext 不含路径分隔且长度合理时采用
        if ext.len() > 1 && ext.len() <= 10 && !ext.contains('/') {
            return ext;
        }
    }
    String::new()
}

/// 下载单个文件到指定路径（流式写入，避免大文件占满内存）
pub async fn download_file(file_url: &str, save_path: &Path) -> Result<()> {
    let client = crate::utils::create_http_client()?;
    let response = client
        .get(file_url)
        .header("User-Agent", DEFAULT_USER_AGENT)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!("HTTP {}", response.status().as_u16()));
    }

    let mut file = tokio::fs::File::create(save_path).await?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                let _ = tokio::fs::remove_file(save_path).await;
                return Err(anyhow!("下载中断: {}", e));
            }
        };
        if let Err(e) = file.write_all(&chunk).await {
            let _ = tokio::fs::remove_file(save_path).await;
            return Err(anyhow!("写入文件失败: {}", e));
        }
    }
    file.flush().await?;
    Ok(())
}

/// 下载解析结果中的所有媒体文件
pub async fn download_media(info: &VideoParseInfo, output_dir: &str) -> Result<()> {
    tokio::fs::create_dir_all(output_dir)
        .await
        .map_err(|e| anyhow!("创建输出目录失败: {}", e))?;

    let base_name = sanitize_filename(&info.title);
    let dir = Path::new(output_dir);
    let mut count = 0;

    // 下载视频
    if let Some(video_url) = &info.video_url {
        if !video_url.is_empty() {
            let ext = {
                let e = ext_from_url(video_url);
                if e.is_empty() { ".mp4".to_string() } else { e }
            };
            let filename = format!("{}{}", base_name, ext);
            tracing::info!(file = %filename, "开始下载视频");
            download_file(video_url, &dir.join(&filename))
                .await
                .map_err(|e| anyhow!("视频下载失败: {}", e))?;
            count += 1;
        }
    }

    // 下载图集
    for (i, img) in info.images.iter().enumerate() {
        let ext = {
            let e = ext_from_url(&img.url);
            if e.is_empty() { ".jpg".to_string() } else { e }
        };
        let filename = format!("{}_{:03}{}", base_name, i + 1, ext);
        tracing::info!(file = %filename, "开始下载图片");
        download_file(&img.url, &dir.join(&filename))
            .await
            .map_err(|e| anyhow!("图片 {} 下载失败: {}", i + 1, e))?;
        count += 1;

        // 下载 LivePhoto 视频
        if let Some(live_url) = &img.live_photo_url {
            if !live_url.is_empty() {
                let live_ext = {
                    let e = ext_from_url(live_url);
                    if e.is_empty() { ".mp4".to_string() } else { e }
                };
                let live_name = format!("{}_{:03}_live{}", base_name, i + 1, live_ext);
                tracing::info!(file = %live_name, "开始下载 LivePhoto");
                if let Err(e) = download_file(live_url, &dir.join(&live_name)).await {
                    tracing::warn!(file = %live_name, error = %e, "LivePhoto 下载失败");
                }
                count += 1;
            }
        }
    }

    // 下载封面
    if let Some(cover_url) = &info.cover_url {
        if !cover_url.is_empty() {
            let ext = {
                let e = ext_from_url(cover_url);
                if e.is_empty() { ".jpg".to_string() } else { e }
            };
            let filename = format!("{}_cover{}", base_name, ext);
            tracing::info!(file = %filename, "开始下载封面");
            if let Err(e) = download_file(cover_url, &dir.join(&filename)).await {
                tracing::warn!(file = %filename, error = %e, "封面下载失败");
            }
            count += 1;
        }
    }

    // 下载音乐
    if let Some(music_url) = &info.music_url {
        if !music_url.is_empty() {
            let ext = {
                let e = ext_from_url(music_url);
                if e.is_empty() { ".mp3".to_string() } else { e }
            };
            let filename = format!("{}_music{}", base_name, ext);
            tracing::info!(file = %filename, "开始下载背景音乐");
            if let Err(e) = download_file(music_url, &dir.join(&filename)).await {
                tracing::warn!(file = %filename, error = %e, "背景音乐下载失败");
            }
            count += 1;
        }
    }

    if count > 0 {
        tracing::info!(count, dir = %output_dir, "媒体下载完成");
    } else {
        tracing::warn!("无可下载的媒体文件");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("a/b:c*d"), "a_b_c_d");
        assert_eq!(sanitize_filename("   "), "download");
        assert_eq!(sanitize_filename("正常标题"), "正常标题");
    }

    #[test]
    fn test_ext_from_url() {
        assert_eq!(ext_from_url("https://cdn.x.com/a/b.mp4?token=1"), ".mp4");
        assert_eq!(ext_from_url("https://cdn.x.com/a/cover.JPG"), ".jpg");
        assert_eq!(ext_from_url("https://cdn.x.com/novod"), "");
    }
}

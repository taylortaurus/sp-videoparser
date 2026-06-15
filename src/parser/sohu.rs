use crate::models::VideoParseInfo;
use crate::parser::VideoParser;
use crate::utils::create_http_client;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use regex::Regex;
use serde_json::Value;

pub struct SohuParser;

#[async_trait]
impl VideoParser for SohuParser {
    async fn parse_share_url(&self, share_url: &str) -> Result<VideoParseInfo> {
        let vid = self.extract_vid(share_url)?;
        self.parse_video_id(&vid).await
    }

    async fn parse_video_id(&self, video_id: &str) -> Result<VideoParseInfo> {
        if video_id.is_empty() {
            return Err(anyhow!("视频ID不能为空"));
        }

        // 搜狐视频信息API
        let api_url = format!(
            "https://api.tv.sohu.com/v4/video/info/{}.json?site=2&api_key=9854b2afa779e1a6bcdd07b217417549&sver=6.2.0",
            video_id
        );

        let client = create_http_client()?;
        let response = client
            .get(&api_url)
            .header("User-Agent", crate::utils::DEFAULT_USER_AGENT)
            .send()
            .await?;

        let json: Value = response.json().await?;

        // 检查API状态
        let status = json.pointer("/status").and_then(|v| v.as_i64()).unwrap_or(0);
        if status != 200 {
            let msg = json.pointer("/statusText").and_then(|v| v.as_str()).unwrap_or("");
            return Err(anyhow!("搜狐视频API返回错误: {} (status: {})", msg, status));
        }

        let data = json
            .pointer("/data")
            .ok_or_else(|| anyhow!("API响应中未找到视频数据"))?;

        // 提取视频播放地址
        let video_url = data
            .pointer("/url_high_mp4")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| data.pointer("/download_url").and_then(|v| v.as_str()))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("未找到视频播放地址"))?
            .to_string();

        let mut info = VideoParseInfo::new();
        info.title = data.pointer("/video_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        info.video_url = Some(video_url);
        info.cover_url = data
            .pointer("/originalCutCover")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        info.author.uid = data.pointer("/user/user_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        info.author.name = data.pointer("/user/nickname").and_then(|v| v.as_str()).unwrap_or("").to_string();
        info.author.avatar = data.pointer("/user/small_pic").and_then(|v| v.as_str()).unwrap_or("").to_string();

        Ok(info)
    }
}

impl SohuParser {
    /// 从 URL 中提取搜狐视频 ID
    fn extract_vid(&self, raw_url: &str) -> Result<String> {
        // 匹配 tv.sohu.com/v/{base64}.html 格式
        // base64 编码部分解码后为 us/{uid}/{vid}.shtml
        let base64_re = Regex::new(r"/v/([A-Za-z0-9+/=]+)\.html")?;
        if let Some(caps) = base64_re.captures(raw_url) {
            let encoded = caps.get(1).unwrap().as_str();
            let decoded = STANDARD
                .decode(encoded)
                .map_err(|e| anyhow!("base64解码失败: {}", e))?;
            let decoded_str = String::from_utf8_lossy(&decoded);
            return self.extract_vid_from_path(&decoded_str);
        }

        // 匹配 my.tv.sohu.com/us/{uid}/{vid}.shtml 格式
        if raw_url.contains("my.tv.sohu.com") || raw_url.contains("tv.sohu.com/us/") {
            return self.extract_vid_from_path(raw_url);
        }

        Err(anyhow!("不是有效的搜狐视频链接"))
    }

    /// 从路径中提取视频 ID（适配 us/{uid}/{vid}.shtml 格式）
    fn extract_vid_from_path(&self, path: &str) -> Result<String> {
        let re = Regex::new(r"/?us/\d+/(\d+)\.shtml")?;
        re.captures(path)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| anyhow!("无法从路径 {} 中提取视频ID", path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_vid_from_path() {
        let p = SohuParser;
        assert_eq!(p.extract_vid_from_path("us/123456/789.shtml").unwrap(), "789");
        assert_eq!(p.extract_vid_from_path("https://my.tv.sohu.com/us/100/200.shtml").unwrap(), "200");
    }
}

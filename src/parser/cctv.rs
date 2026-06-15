use crate::models::VideoParseInfo;
use crate::parser::VideoParser;
use crate::utils::create_http_client;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

pub struct CctvParser;

#[async_trait]
impl VideoParser for CctvParser {
    async fn parse_share_url(&self, share_url: &str) -> Result<VideoParseInfo> {
        // 请求页面 HTML 提取视频 GUID
        let guid = self.extract_guid(share_url).await?;
        self.parse_video_id(&guid).await
    }

    async fn parse_video_id(&self, video_id: &str) -> Result<VideoParseInfo> {
        if video_id.is_empty() {
            return Err(anyhow!("视频GUID不能为空"));
        }

        // 央视网视频信息 API
        let api_url = format!(
            "https://vdn.apps.cntv.cn/api/getHttpVideoInfo.do?pid={}",
            video_id
        );

        let client = create_http_client()?;
        let response = client
            .get(&api_url)
            .header("User-Agent", crate::utils::DEFAULT_USER_AGENT)
            .send()
            .await?;

        let json: Value = response.json().await?;

        // 检查 API 状态
        let status = json.pointer("/status").and_then(|v| v.as_str()).unwrap_or("");
        if status != "001" {
            let msg = json.pointer("/title").and_then(|v| v.as_str()).unwrap_or("");
            return Err(anyhow!(
                "央视网视频API返回错误 (status: {}, title: {})",
                status,
                msg
            ));
        }

        // 提取 HLS 视频播放地址
        // 注：manifest 中的 h5e/enc/enc2 高码率流在 H.264 帧级加扰，播放花屏，仅 hls_url 可正常播放
        let video_url = json
            .pointer("/hls_url")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("未找到视频播放地址"))?
            .to_string();

        let mut info = VideoParseInfo::new();
        info.title = json
            .pointer("/title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        info.video_url = Some(video_url);
        info.cover_url = json
            .pointer("/image")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        info.author.name = json
            .pointer("/play_channel")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(info)
    }
}

impl CctvParser {
    /// 从页面 URL 请求并提取视频 GUID
    async fn extract_guid(&self, page_url: &str) -> Result<String> {
        let client = create_http_client()?;
        let response = client
            .get(page_url)
            .header("User-Agent", crate::utils::DEFAULT_USER_AGENT)
            .send()
            .await?;

        let html = response.text().await?;
        let re = Regex::new(r#"var\s+guid\s*=\s*"([^"]+)""#)?;

        re.captures(&html)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("页面中未找到视频GUID"))
    }
}

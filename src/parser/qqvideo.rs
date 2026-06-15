use crate::models::VideoParseInfo;
use crate::parser::VideoParser;
use crate::utils::create_http_client;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

pub struct QqvideoParser;

#[async_trait]
impl VideoParser for QqvideoParser {
    async fn parse_share_url(&self, share_url: &str) -> Result<VideoParseInfo> {
        let vid = self.extract_vid(share_url)?;
        self.parse_video_id(&vid).await
    }

    async fn parse_video_id(&self, video_id: &str) -> Result<VideoParseInfo> {
        if video_id.is_empty() {
            return Err(anyhow!("视频ID不能为空"));
        }

        let api_url = format!(
            "https://vv.video.qq.com/getinfo?vids={}&platform=101001&otype=json&defn=shd",
            video_id
        );

        let client = create_http_client()?;
        let response = client
            .get(&api_url)
            .header("User-Agent", crate::utils::DEFAULT_USER_AGENT)
            .send()
            .await?;

        let body = response.text().await?;

        // 去除 JSONP 前缀 QZOutputJson= 和尾部分号
        let json_str = body
            .trim()
            .trim_start_matches("QZOutputJson=")
            .trim_end_matches(';');
        let json: Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow!("解析腾讯视频API响应失败: {}", e))?;

        // 检查 API 级别错误
        let em = json.pointer("/em").and_then(|v| v.as_i64()).unwrap_or(0);
        if em != 0 {
            let msg = json.pointer("/msg").and_then(|v| v.as_str()).unwrap_or("");
            return Err(anyhow!("腾讯视频API返回错误: {} (em: {})", msg, em));
        }

        // 检查视频列表
        let vi = json
            .pointer("/vl/vi/0")
            .ok_or_else(|| anyhow!("未找到视频信息，视频可能已被删除或设为私密"))?;

        // 提取 CDN 地址
        let base_url = vi
            .pointer("/ul/ui/0/url")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fname = vi.pointer("/fn").and_then(|v| v.as_str()).unwrap_or("");
        let fvkey = vi.pointer("/fvkey").and_then(|v| v.as_str()).unwrap_or("");
        if base_url.is_empty() || fname.is_empty() || fvkey.is_empty() {
            return Err(anyhow!("视频地址信息不完整"));
        }

        // 构造视频播放地址
        let video_url = format!("{}{}?vkey={}", base_url, fname, fvkey);

        let vid = vi.pointer("/vid").and_then(|v| v.as_str()).unwrap_or("");
        let mut info = VideoParseInfo::new();
        info.title = vi.pointer("/ti").and_then(|v| v.as_str()).unwrap_or("").to_string();
        info.video_url = Some(video_url);
        info.cover_url = Some(format!(
            "https://puui.qpic.cn/vpic_cover/{}/{}_hz.jpg/496",
            vid, vid
        ));

        Ok(info)
    }
}

impl QqvideoParser {
    /// 从 URL 中提取腾讯视频 ID
    fn extract_vid(&self, raw_url: &str) -> Result<String> {
        let parsed = url::Url::parse(raw_url).map_err(|e| anyhow!("URL格式无效: {}", e))?;
        let host = parsed.host_str().unwrap_or("");

        // 移动端播放页: m.v.qq.com/x/m/play?vid={vid}
        if host.contains("m.v.qq.com") {
            return parsed
                .query_pairs()
                .find(|(k, _)| k == "vid")
                .map(|(_, v)| v.to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow!("移动端链接中未找到vid参数"));
        }

        // PC端页面: v.qq.com/x/page/{vid}.html 或 v.qq.com/x/cover/{cid}/{vid}.html
        if host.contains("v.qq.com") {
            let re = Regex::new(r"/x/(?:page|cover)/(?:[^/]+/)?(\w+)\.html")?;
            return re
                .captures(parsed.path())
                .and_then(|caps| caps.get(1))
                .map(|m| m.as_str().to_string())
                .ok_or_else(|| anyhow!("无法从路径 {} 中提取视频ID", parsed.path()));
        }

        Err(anyhow!("不支持的腾讯视频域名: {}", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_vid_pc() {
        let p = QqvideoParser;
        assert_eq!(p.extract_vid("https://v.qq.com/x/page/abc123.html").unwrap(), "abc123");
        assert_eq!(p.extract_vid("https://v.qq.com/x/cover/cid1/vid2.html").unwrap(), "vid2");
    }

    #[test]
    fn test_extract_vid_mobile() {
        let p = QqvideoParser;
        assert_eq!(p.extract_vid("https://m.v.qq.com/x/m/play?vid=xyz789").unwrap(), "xyz789");
    }
}

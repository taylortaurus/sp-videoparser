use crate::models::{ImgInfo, VideoParseInfo};
use crate::parser::VideoParser;
use crate::utils::{create_http_client, create_no_redirect_client};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

pub struct TwitterParser;

#[async_trait]
impl VideoParser for TwitterParser {
    async fn parse_share_url(&self, share_url: &str) -> Result<VideoParseInfo> {
        let mut real_url = share_url.to_string();

        // 处理 t.co 短链: 需要先跟随重定向获取真实URL
        if real_url.contains("t.co/") {
            let client = create_no_redirect_client()?;
            let response = client
                .get(&real_url)
                .header("User-Agent", crate::utils::DEFAULT_USER_AGENT)
                .send()
                .await?;
            real_url = response
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| anyhow!("获取 t.co 重定向地址失败"))?
                .to_string();
        }

        // 从 URL 中提取 tweet ID
        let tweet_id = self.extract_tweet_id(&real_url)?;
        self.parse_video_id(&tweet_id).await
    }

    async fn parse_video_id(&self, video_id: &str) -> Result<VideoParseInfo> {
        let token = self.get_token(video_id);
        let api_url = format!(
            "https://cdn.syndication.twimg.com/tweet-result?id={}&token={}",
            video_id, token
        );

        let client = create_http_client()?;
        let response = client
            .get(&api_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .header("Accept", "application/json")
            .header("Referer", "https://platform.twitter.com/")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Twitter API 返回错误状态码: {}", response.status().as_u16()));
        }

        let json: Value = response.json().await?;

        if let Ok(info) = Self::parse_syndication_response(&json) {
            return Ok(info);
        }

        // Some playable tweets are returned as empty tombstones by X's public APIs.
        tracing::info!(tweet_id = video_id, "Twitter 主接口未提供媒体，尝试 FxTwitter 备用解析");
        let response = client
            .get(format!("https://api.fxtwitter.com/status/{}", video_id))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(anyhow!("Twitter 备用接口返回错误状态码: {}", response.status().as_u16()));
        }
        let json: Value = response.json().await?;
        Self::parse_fxtwitter_response(&json, video_id)
    }
}

impl TwitterParser {
    fn parse_syndication_response(json: &Value) -> Result<VideoParseInfo> {
        // 提取作者信息
        let author_name = json.pointer("/user/name").and_then(|v| v.as_str()).unwrap_or("");
        let author_screen_name = json.pointer("/user/screen_name").and_then(|v| v.as_str()).unwrap_or("");
        let author_avatar = json.pointer("/user/profile_image_url_https").and_then(|v| v.as_str()).unwrap_or("");
        let author_id = json.pointer("/user/id_str").and_then(|v| v.as_str()).unwrap_or("");

        // 提取推文文本作为标题
        let title = json.pointer("/text").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let mut video_url = String::new();
        let mut cover_url = String::new();

        // 从 mediaDetails 中查找 video 类型
        if let Some(media_details) = json.pointer("/mediaDetails").and_then(|v| v.as_array()) {
            for media in media_details {
                let media_type = media.pointer("/type").and_then(|v| v.as_str()).unwrap_or("");
                if media_type == "video" || media_type == "animated_gif" {
                    cover_url = media.pointer("/media_url_https").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    video_url = Self::pick_best_mp4(media.pointer("/video_info/variants"));
                    break;
                }
            }
        }

        // 如果没有视频, 尝试图集
        let mut images: Vec<ImgInfo> = Vec::new();
        if video_url.is_empty() {
            // 尝试从顶层 video 字段获取
            if let Some(variants) = json.pointer("/video/variants") {
                cover_url = json.pointer("/video/poster").and_then(|v| v.as_str()).unwrap_or("").to_string();
                video_url = Self::pick_best_mp4(Some(variants));
            }

            // 如果还是没有视频, 提取图片
            if video_url.is_empty() {
                if let Some(media_details) = json.pointer("/mediaDetails").and_then(|v| v.as_array()) {
                    for media in media_details {
                        if media.pointer("/type").and_then(|v| v.as_str()) == Some("photo") {
                            if let Some(img) = media.pointer("/media_url_https").and_then(|v| v.as_str()) {
                                if !img.is_empty() {
                                    images.push(ImgInfo { url: img.to_string(), live_photo_url: None });
                                }
                            }
                        }
                    }
                    if let Some(first) = images.first() {
                        cover_url = first.url.clone();
                    }
                }
            }
        }

        if video_url.is_empty() && images.is_empty() {
            return Err(anyhow!("该推文中没有找到视频或图片"));
        }

        // 使用 screen_name 作为作者名称的补充
        let display_name = if author_name.is_empty() { author_screen_name } else { author_name };

        let mut info = VideoParseInfo::new();
        info.title = title;
        info.video_url = if video_url.is_empty() { None } else { Some(video_url) };
        info.cover_url = if cover_url.is_empty() { None } else { Some(cover_url) };
        info.images = images;
        info.author.uid = author_id.to_string();
        info.author.name = display_name.to_string();
        info.author.avatar = author_avatar.to_string();

        Ok(info)
    }

    fn parse_fxtwitter_response(json: &Value, video_id: &str) -> Result<VideoParseInfo> {
        if json.get("code").and_then(Value::as_u64) != Some(200) {
            return Err(anyhow!("Twitter 备用接口无法读取该推文，请确认推文公开且仍可访问"));
        }
        let tweet = &json["tweet"];
        if tweet.get("id").and_then(Value::as_str) != Some(video_id) {
            return Err(anyhow!("Twitter 备用接口未返回匹配的推文"));
        }
        let media_details: Vec<Value> = tweet
            .pointer("/media/all")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|media| {
                let kind = media.get("type").and_then(Value::as_str).unwrap_or("");
                let variants = media.get("variants").cloned().unwrap_or_else(|| {
                    if media.get("format").and_then(Value::as_str) == Some("video/mp4") {
                        serde_json::json!([{"content_type": "video/mp4", "url": media["url"]}])
                    } else {
                        Value::Null
                    }
                });
                serde_json::json!({
                    "type": if kind == "gif" { "animated_gif" } else { kind },
                    "media_url_https": media[if kind == "photo" { "url" } else { "thumbnail_url" }],
                    "video_info": {"variants": variants}
                })
            })
            .collect();
        Self::parse_syndication_response(&serde_json::json!({
            "text": tweet["text"],
            "user": {
                "id_str": tweet["author"]["id"],
                "name": tweet["author"]["name"],
                "screen_name": tweet["author"]["screen_name"],
                "profile_image_url_https": tweet["author"]["avatar_url"]
            },
            "mediaDetails": media_details
        }))
    }

    /// 从 variants 中选取最高码率的 mp4
    fn pick_best_mp4(variants: Option<&Value>) -> String {
        let mut video_url = String::new();
        let mut max_bitrate: i64 = -1;
        if let Some(arr) = variants.and_then(|v| v.as_array()) {
            for variant in arr {
                if variant.pointer("/content_type").and_then(|v| v.as_str()) != Some("video/mp4") {
                    continue;
                }
                let bitrate = variant.pointer("/bitrate").and_then(|v| v.as_i64()).unwrap_or(0);
                let url = variant.pointer("/url").and_then(|v| v.as_str()).unwrap_or("");
                if bitrate > max_bitrate || video_url.is_empty() {
                    max_bitrate = bitrate;
                    video_url = url.to_string();
                }
            }
        }
        video_url
    }

    /// 从推文 URL 中提取 tweet ID
    /// 支持 x.com / twitter.com / mobile.twitter.com 的 /status/ 与 /statuses/ 格式
    fn extract_tweet_id(&self, share_url: &str) -> Result<String> {
        let re = Regex::new(r"(?:twitter\.com|x\.com)/[^/]+/status(?:es)?/(\d+)")?;
        re.captures(share_url)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| anyhow!("无法从 URL 中提取推文 ID: {}", share_url))
    }

    /// 计算 syndication API 所需的 token
    /// 算法: (tweetId / 1e15 * π) 转换为字符串, 去除 "0" 和 "."
    fn get_token(&self, id: &str) -> String {
        let num: f64 = id.parse().unwrap_or(0.0);
        let token = (num / 1e15) * std::f64::consts::PI;
        format!("{}", token).replace('0', "").replace('.', "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fallback_video() -> Value {
        json!({
            "code": 200,
            "tweet": {
                "id": "123",
                "text": "Video fixture",
                "author": {"id": "42", "name": "Author", "screen_name": "author", "avatar_url": "https://example.org/avatar.jpg"},
                "media": {"all": [{
                    "type": "video", "format": "video/mp4",
                    "url": "https://video.twimg.com/high.mp4",
                    "thumbnail_url": "https://pbs.twimg.com/cover.jpg",
                    "variants": [
                        {"content_type": "application/x-mpegURL", "url": "https://video.twimg.com/stream.m3u8", "bitrate": 9999},
                        {"content_type": "video/mp4", "url": "https://video.twimg.com/low.mp4", "bitrate": 100},
                        {"content_type": "video/mp4", "url": "https://video.twimg.com/high.mp4", "bitrate": 200}
                    ]
                }]}
            }
        })
    }

    #[test]
    fn fallback_video_preserves_media_and_author() {
        let info = TwitterParser::parse_fxtwitter_response(&fallback_video(), "123").unwrap();
        assert_eq!(info.video_url.as_deref(), Some("https://video.twimg.com/high.mp4"));
        assert_eq!(info.cover_url.as_deref(), Some("https://pbs.twimg.com/cover.jpg"));
        assert_eq!(info.title, "Video fixture");
        assert_eq!(info.author.uid, "42");
        assert_eq!(info.author.name, "Author");
        assert_eq!(info.author.avatar, "https://example.org/avatar.jpg");
    }

    #[test]
    fn fallback_rejects_api_errors_and_wrong_tweet() {
        let mut response = fallback_video();
        response["code"] = json!(404);
        assert!(TwitterParser::parse_fxtwitter_response(&response, "123").is_err());
        response["code"] = json!(200);
        assert!(TwitterParser::parse_fxtwitter_response(&response, "999").is_err());
    }

    #[test]
    fn fallback_video_without_variants_uses_mp4_url() {
        let mut response = fallback_video();
        response["tweet"]["media"]["all"][0].as_object_mut().unwrap().remove("variants");
        let info = TwitterParser::parse_fxtwitter_response(&response, "123").unwrap();
        assert_eq!(info.video_url.as_deref(), Some("https://video.twimg.com/high.mp4"));
    }

    #[test]
    fn fallback_rejects_tweet_without_media() {
        let response = json!({"code": 200, "tweet": {"id": "123", "text": "Text only"}});
        assert!(TwitterParser::parse_fxtwitter_response(&response, "123").is_err());
    }

    #[test]
    fn fallback_photos_preserve_original_urls() {
        let response = json!({"code": 200, "tweet": {"id": "123", "media": {"all": [
            {"type": "photo", "url": "https://pbs.twimg.com/one.jpg"},
            {"type": "photo", "url": "https://pbs.twimg.com/two.jpg"}
        ]}}});
        let info = TwitterParser::parse_fxtwitter_response(&response, "123").unwrap();
        assert!(info.video_url.is_none());
        assert_eq!(info.images.len(), 2);
        assert_eq!(info.images[1].url, "https://pbs.twimg.com/two.jpg");
        assert_eq!(info.cover_url.as_deref(), Some("https://pbs.twimg.com/one.jpg"));
    }

    #[test]
    fn test_extract_tweet_id() {
        let p = TwitterParser;
        assert_eq!(p.extract_tweet_id("https://x.com/foo/status/1234567890").unwrap(), "1234567890");
        assert_eq!(p.extract_tweet_id("https://twitter.com/foo/statuses/999").unwrap(), "999");
    }

    #[test]
    fn test_get_token_deterministic() {
        let p = TwitterParser;
        let t = p.get_token("1234567890123456789");
        assert!(!t.contains('0'));
        assert!(!t.contains('.'));
        assert!(!t.is_empty());
    }
}

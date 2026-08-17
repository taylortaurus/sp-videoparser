use crate::models::{Author, ImgInfo, VideoParseInfo};
use crate::parser::VideoParser;
use crate::utils::{
    create_cookie_http_client, create_http_client, create_no_redirect_client,
    extract_json_from_html, generate_numeric_id, generate_random_string,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use regex::Regex;
use reqwest::header::{COOKIE, REFERER};
use scraper::{Html, Selector};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tracing;

pub struct DouyinParser;

#[async_trait]
impl VideoParser for DouyinParser {
    async fn parse_share_url(&self, share_url: &str) -> Result<VideoParseInfo> {
        let url = url::Url::parse(share_url)?;

        match url.host_str() {
            Some("v.douyin.com") => self.parse_app_share_url(share_url).await,
            Some("www.iesdouyin.com") | Some("www.douyin.com") => {
                self.parse_pc_share_url(share_url).await
            }
            _ => Err(anyhow!("不支持的抖音链接类型")),
        }
    }

    async fn parse_video_id(&self, video_id: &str) -> Result<VideoParseInfo> {
        let html = self.fetch_legacy_share_html(video_id).await?;

        // 检查是否为图集
        let is_note = self.check_is_note(&html);

        // 尝试图集API，失败则降级到普通API（与Go版本一致）
        let (data, is_note_final) = if is_note {
            match self.parse_note_data(video_id).await {
                Ok(data) => (data, true),
                Err(_) => {
                    // 图集API失败，降级到普通视频API
                    self.parse_video_data_with_fallback(&html, video_id).await?
                }
            }
        } else {
            self.parse_video_data_with_fallback(&html, video_id).await?
        };

        self.extract_video_info(&data, is_note_final).await
    }
}

impl DouyinParser {
    async fn fetch_legacy_share_html(&self, video_id: &str) -> Result<String> {
        let req_url = format!("https://www.iesdouyin.com/share/video/{}", video_id);
        let client = create_http_client()?;
        Ok(client
            .get(req_url)
            .header("User-Agent", crate::utils::DEFAULT_USER_AGENT)
            .send()
            .await?
            .text()
            .await?)
    }

    async fn parse_video_data_with_fallback(
        &self,
        initial_html: &str,
        video_id: &str,
    ) -> Result<(Value, bool)> {
        match self.parse_video_data_from_html(initial_html, video_id) {
            Ok(data) => Ok((data, false)),
            Err(initial_error) => {
                let fallback_url = format!(
                    "https://www.douyin.com/discover?modal_id={}",
                    url::form_urlencoded::byte_serialize(video_id.as_bytes()).collect::<String>()
                );
                let client = create_cookie_http_client()?;
                let html = self.fetch_mobile_share_html(&client, &fallback_url).await?;
                tracing::debug!(
                    html_len = html.len(),
                    has_aweme_id = html.contains("\"aweme_id\""),
                    has_item_id = html.contains("\"itemId\""),
                    "收到抖音移动分享页"
                );

                match self
                    .fetch_web_detail_data(&client, &fallback_url, video_id)
                    .await
                {
                    Ok(data) => {
                        let is_note = data
                            .pointer("/images")
                            .and_then(Value::as_array)
                            .is_some_and(|images| !images.is_empty());
                        return Ok((data, is_note));
                    }
                    Err(error) => {
                        tracing::debug!(%error, "抖音 Web 详情接口不可用，降级解析分享页");
                    }
                }

                let data =
                    self.parse_video_data_from_html(&html, video_id)
                        .map_err(|fallback_error| {
                            anyhow!(
                                "抖音视频数据解析失败（旧分享页: {}; 移动分享页: {}）",
                                initial_error,
                                fallback_error
                            )
                        })?;
                Ok((data, false))
            }
        }
    }

    async fn fetch_mobile_share_html(
        &self,
        client: &reqwest::Client,
        fallback_url: &str,
    ) -> Result<String> {
        let html = client
            .get(fallback_url)
            .header("User-Agent", crate::utils::DOUYIN_MOBILE_USER_AGENT)
            .send()
            .await?
            .text()
            .await?;

        if !html.contains("waf-jschallenge") && !html.contains("_wafchallengeid") {
            return Ok(html);
        }

        let challenge_html = html.clone();
        let cookie = tokio::task::spawn_blocking(move || solve_waf_challenge(&challenge_html))
            .await
            .map_err(|e| anyhow!("抖音 WAF 挑战计算任务失败: {}", e))??;

        tracing::info!("检测到抖音 WAF 挑战，已完成工作量证明并重试");
        Ok(client
            .get(fallback_url)
            .header("User-Agent", crate::utils::DOUYIN_MOBILE_USER_AGENT)
            .header(COOKIE, cookie)
            .send()
            .await?
            .text()
            .await?)
    }

    async fn fetch_web_detail_data(
        &self,
        client: &reqwest::Client,
        referer: &str,
        video_id: &str,
    ) -> Result<Value> {
        let encoded_id =
            url::form_urlencoded::byte_serialize(video_id.as_bytes()).collect::<String>();
        let url = format!(
            "https://www.douyin.com/aweme/v1/web/aweme/detail/?aweme_id={}&device_platform=webapp&aid=6383&channel=channel_pc_web",
            encoded_id
        );
        let response = client
            .get(url)
            .header("User-Agent", crate::utils::DOUYIN_MOBILE_USER_AGENT)
            .header(REFERER, referer)
            .send()
            .await?
            .error_for_status()?;
        let json: Value = response.json().await?;
        json.get("aweme_detail")
            .filter(|detail| detail.is_object())
            .cloned()
            .ok_or_else(|| anyhow!("抖音 Web 详情接口未返回作品数据"))
    }
    async fn parse_app_share_url(&self, share_url: &str) -> Result<VideoParseInfo> {
        let client = create_no_redirect_client()?;

        let response = client
            .get(share_url)
            .header("User-Agent", crate::utils::DEFAULT_USER_AGENT)
            .send()
            .await?;

        // 获取重定向地址
        let location = response
            .headers()
            .get("location")
            .ok_or_else(|| anyhow!("未找到重定向地址"))?
            .to_str()?;

        let video_id = self.extract_video_id_from_path(location)?;

        if location.contains("ixigua.com") {
            return Err(anyhow!("西瓜视频暂不支持"));
        }

        self.parse_video_id(&video_id).await
    }

    async fn parse_pc_share_url(&self, share_url: &str) -> Result<VideoParseInfo> {
        let video_id = self.extract_video_id_from_path(share_url)?;
        self.parse_video_id(&video_id).await
    }

    fn extract_video_id_from_path(&self, url_path: &str) -> Result<String> {
        let url = url::Url::parse(url_path)
            .or_else(|_| url::Url::parse(&format!("https://example.com{}", url_path)))?;

        if let Some(modal_id) = url.query_pairs().find(|(k, _)| k == "modal_id") {
            return Ok(modal_id.1.to_string());
        }

        // 从路径中提取
        let path = url.path().trim_matches('/');
        let id = path.split('/').next_back().unwrap_or("");
        if id.is_empty() {
            return Err(anyhow!("无法从路径中提取视频ID"));
        }
        Ok(id.to_string())
    }

    fn check_is_note(&self, html: &str) -> bool {
        // 查找 canonical link
        let document = Html::parse_document(html);
        let selector = Selector::parse("link[rel='canonical']").unwrap();

        if let Some(element) = document.select(&selector).next() {
            if let Some(href) = element.value().attr("href") {
                return href.contains("/note/");
            }
        }

        false
    }

    async fn parse_note_data(&self, video_id: &str) -> Result<Value> {
        let web_id = format!("75{}", generate_numeric_id(15));
        let a_bogus = generate_random_string(64);

        let url = format!(
            "https://www.iesdouyin.com/web/api/v2/aweme/slidesinfo/?reflow_source=reflow_page&web_id={}&device_id={}&aweme_ids=%5B{}%5D&request_source=200&a_bogus={}",
            web_id, web_id, video_id, a_bogus
        );

        let client = create_http_client()?;
        let response = client
            .get(&url)
            .header("User-Agent", crate::utils::DEFAULT_USER_AGENT)
            .send()
            .await?;

        let json: Value = response.json().await?;

        json.get("aweme_details")
            .and_then(|v| v.get(0))
            .cloned()
            .ok_or_else(|| anyhow!("获取图集数据失败"))
    }

    fn parse_video_data_from_html(&self, html: &str, video_id: &str) -> Result<Value> {
        let pattern = r"window\._ROUTER_DATA\s*=\s*(.*?)</script>";
        let json_str = extract_json_from_html(html, pattern)?;

        let json: Value = serde_json::from_str(&json_str)?;

        // 调试：打印JSON结构
        tracing::debug!(keys = ?json.as_object().map(|o| o.keys().collect::<Vec<_>>()), "解析得到的 JSON 顶层键");

        if let Some(loader_data) = json.get("loaderData") {
            if let Some(obj) = loader_data.as_object() {
                tracing::debug!(keys = ?obj.keys().collect::<Vec<_>>(), "loaderData 子键");

                for (key, value) in obj {
                    tracing::debug!(key = %key, "检查 loaderData 键");

                    if let Some(video_info_res) = value.get("videoInfoRes") {
                        if let Some(items) =
                            video_info_res.get("item_list").and_then(Value::as_array)
                        {
                            if let Some(item) = items.iter().find(|item| {
                                item.get("aweme_id").and_then(Value::as_str) == Some(video_id)
                            }) {
                                tracing::info!(key = %key, "在 loaderData 中定位到视频数据");
                                return Ok(item.clone());
                            }
                        }

                        if let Some(filter_list) = video_info_res.get("filter_list") {
                            if let Some(filters) = filter_list.as_array() {
                                for filter in filters {
                                    if filter.get("aweme_id").and_then(|v| v.as_str())
                                        == Some(video_id)
                                    {
                                        return Err(anyhow!(
                                            "视频被过滤: {} - {}",
                                            filter
                                                .get("filter_reason")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or(""),
                                            filter
                                                .get("detail_msg")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("")
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(anyhow!("无法从抖音页面提取视频数据"))
    }

    /// 从JSON数据中提取视频信息
    async fn extract_video_info(&self, data: &Value, is_note: bool) -> Result<VideoParseInfo> {
        let mut info = VideoParseInfo::new();

        // 提取作者信息
        info.author = Author {
            uid: data
                .pointer("/author/sec_uid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            name: data
                .pointer("/author/nickname")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            avatar: data
                .pointer("/author/avatar_thumb/url_list/0")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        };

        // 提取标题
        info.title = data
            .pointer("/desc")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        info.music_url = self.extract_music_url(data);

        // 提取封面
        info.cover_url = self.get_non_webp_url(data.pointer("/video/cover/url_list"));

        // Extract image posts from all known web/App payload layouts.
        for image in self.extract_image_items(data) {
            if let Some(img_url) = self.extract_image_url(image) {
                info.images.push(ImgInfo {
                    url: img_url,
                    live_photo_url: self.extract_live_photo_url(data, image),
                });
            }
        }

        if !is_note && info.images.is_empty() {
            if let Some(video_url) = data
                .pointer("/video/play_addr/url_list/0")
                .and_then(|v| v.as_str())
            {
                let video_url = video_url.replace("playwm", "play");
                info.video_url = Some(self.get_redirect_url(&video_url).await.unwrap_or(video_url));
            }
        }

        if !info.images.is_empty() {
            info.video_url = None;
        }

        if info.video_url.is_none() && info.images.is_empty() {
            return Err(anyhow!("没有找到视频或图集内容"));
        }

        Ok(info)
    }

    fn extract_music_url(&self, data: &Value) -> Option<String> {
        first_url_at(
            data,
            &[
                "/music/play_url/url_list/0",
                "/music/play_url/uri",
                "/music/play_url_uri",
            ],
        )
        .or_else(|| {
            // Image posts may expose the soundtrack as the top-level audio/mp4
            // asset while omitting music.play_url from the web payload.
            let music_duration = duration_at(data.pointer("/music/duration"));
            let video_duration = duration_at(data.pointer("/video/duration")).unwrap_or(0.0);
            if music_duration.is_some_and(|duration| duration > 0.0) && video_duration == 0.0 {
                first_url_at(
                    data,
                    &["/video/play_addr/uri", "/video/play_addr/url_list/0"],
                )
            } else {
                None
            }
        })
    }

    fn extract_image_items<'a>(&self, data: &'a Value) -> Vec<&'a Value> {
        [
            "/images",
            "/image_infos",
            "/image_post_info/images",
            "/image_post_info/image_list",
        ]
        .iter()
        .filter_map(|path| data.pointer(path).and_then(Value::as_array))
        .flat_map(|images| images.iter())
        .collect()
    }

    fn extract_image_url(&self, image: &Value) -> Option<String> {
        let url_list = [
            "/url_list",
            "/display_image/url_list",
            "/download_addr/url_list",
            "/download_url/url_list",
            "/download_url_list",
        ]
        .iter()
        .find_map(|path| image.pointer(path));
        self.get_non_webp_url(url_list)
            .or_else(|| first_url_at(image, &["/url", "/uri"]))
    }

    fn extract_live_photo_url(&self, data: &Value, image: &Value) -> Option<String> {
        let image_url = first_url_at(
            image,
            &[
                "/video/play_addr/url_list/0",
                "/video/play_addr/uri",
                "/video/play_addr_h264/url_list/0",
                "/video/download_addr/url_list/0",
                "/live_photo/play_addr/url_list/0",
                "/live_photo/play_addr/uri",
                "/live_photo/url_list/0",
                "/motion_photo/play_addr/url_list/0",
                "/motion_photo/play_addr/uri",
                "/dynamic_image/play_addr/url_list/0",
            ],
        );
        if image_url.is_some() {
            return image_url;
        }

        let marked_live = [
            "/live_photo_type",
            "/is_live_photo",
            "/video/is_live_photo",
            "/video/clip_type",
        ]
        .iter()
        .any(|path| is_truthy_marker(data.pointer(path)) || is_truthy_marker(image.pointer(path)));

        if marked_live {
            return first_url_at(
                data,
                &[
                    "/video/play_addr/url_list/0",
                    "/video/play_addr/uri",
                    "/video/play_addr_h264/url_list/0",
                    "/video/download_addr/url_list/0",
                ],
            );
        }

        None
    }

    fn get_non_webp_url(&self, url_list: Option<&Value>) -> Option<String> {
        let array = url_list?.as_array()?;

        for url in array {
            if let Some(url_str) = url.as_str() {
                if !url_str.contains(".webp") {
                    return Some(url_str.to_string());
                }
            }
        }

        array
            .first()
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// 获取302重定向后的URL
    async fn get_redirect_url(&self, url: &str) -> Result<String> {
        let client = create_no_redirect_client()?;

        let response = client
            .get(url)
            .header("User-Agent", crate::utils::DEFAULT_USER_AGENT)
            .send()
            .await?;

        if let Some(location) = response.headers().get("location") {
            Ok(location.to_str()?.to_string())
        } else {
            Ok(url.to_string())
        }
    }
}

fn duration_at(value: Option<&Value>) -> Option<f64> {
    let value = value?;
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|duration| duration.parse().ok()))
}

fn is_truthy_marker(value: Option<&Value>) -> bool {
    let Some(value) = value else {
        return false;
    };
    value.as_bool().unwrap_or(false)
        || value.as_i64().is_some_and(|marker| marker > 0)
        || value
            .as_str()
            .is_some_and(|marker| matches!(marker, "1" | "true" | "live" | "live_photo"))
}

fn first_url_at(value: &Value, paths: &[&str]) -> Option<String> {
    paths.iter().find_map(|path| {
        let value = value.pointer(path)?;
        if let Some(url) = value.as_str() {
            return (!url.is_empty()).then(|| url.to_string());
        }
        value
            .as_array()
            .and_then(|items| items.iter().find_map(Value::as_str))
            .filter(|url| !url.is_empty())
            .map(str::to_string)
    })
}
fn decode_base64_padded(value: &str) -> Result<Vec<u8>> {
    let mut padded = value.to_string();
    let padding = (4 - padded.len() % 4) % 4;
    padded.extend(std::iter::repeat_n('=', padding));
    STANDARD
        .decode(padded)
        .map_err(|e| anyhow!("抖音 WAF Base64 解码失败: {}", e))
}

fn solve_waf_challenge(html: &str) -> Result<String> {
    let re = Regex::new(r#"cs="([^"]+)""#)?;
    let encoded = re
        .captures(html)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str())
        .ok_or_else(|| anyhow!("无法提取抖音 WAF 挑战数据"))?;

    let challenge_json = decode_base64_padded(encoded)?;
    let mut challenge: Value = serde_json::from_slice(&challenge_json)?;
    let prefix = challenge
        .pointer("/v/a")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("抖音 WAF 挑战缺少前缀"))?;
    let expected = challenge
        .pointer("/v/c")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("抖音 WAF 挑战缺少目标摘要"))?;
    let prefix = decode_base64_padded(prefix)?;
    let expected = decode_base64_padded(expected)?;

    let nonce = (0_u32..=1_000_000)
        .find(|nonce| {
            let mut hasher = Sha256::new();
            hasher.update(&prefix);
            hasher.update(nonce.to_string().as_bytes());
            hasher.finalize().as_slice() == expected.as_slice()
        })
        .ok_or_else(|| anyhow!("抖音 WAF 挑战计算失败"))?;

    challenge["d"] = Value::String(STANDARD.encode(nonce.to_string()));
    let cookie_value = STANDARD.encode(serde_json::to_vec(&challenge)?);
    Ok(format!("_wafchallengeid={}", cookie_value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_video_id_from_supported_urls() {
        let parser = DouyinParser;
        assert_eq!(
            parser
                .extract_video_id_from_path("https://www.douyin.com/video/7662291996245224357")
                .unwrap(),
            "7662291996245224357"
        );
        assert_eq!(
            parser
                .extract_video_id_from_path(
                    "https://www.douyin.com/discover?modal_id=7662291996245224357"
                )
                .unwrap(),
            "7662291996245224357"
        );
        assert_eq!(
            parser
                .extract_video_id_from_path(
                    "https://www.iesdouyin.com/share/video/7662291996245224357/?foo=bar"
                )
                .unwrap(),
            "7662291996245224357"
        );
    }

    #[test]
    fn rejects_empty_video_path() {
        assert!(DouyinParser
            .extract_video_id_from_path("https://www.douyin.com/")
            .is_err());
    }

    #[test]
    fn extracts_router_data_item() {
        let html = r#"<script>window._ROUTER_DATA = {"loaderData":{"video_(id)/page":{"videoInfoRes":{"item_list":[{"aweme_id":"1","desc":"ok"}]}}}}</script>"#;
        let item = DouyinParser.parse_video_data_from_html(html, "1").unwrap();
        assert_eq!(item["aweme_id"], "1");
    }

    #[test]
    fn selects_the_requested_router_data_item() {
        let html = r#"<script>window._ROUTER_DATA = {"loaderData":{"video_(id)/page":{"videoInfoRes":{"item_list":[{"aweme_id":"other"},{"aweme_id":"wanted","desc":"ok"}]}}}}</script>"#;
        let item = DouyinParser
            .parse_video_data_from_html(html, "wanted")
            .unwrap();
        assert_eq!(item["aweme_id"], "wanted");
    }

    #[test]
    fn solves_douyin_waf_challenge() {
        let prefix = b"challenge-prefix";
        let nonce = 42_u32;
        let expected = Sha256::digest([prefix.as_slice(), nonce.to_string().as_bytes()].concat());
        let challenge = json!({
            "v": {
                "a": STANDARD.encode(prefix),
                "b": 1,
                "c": STANDARD.encode(expected),
            },
            "s": "signature"
        });
        let encoded = STANDARD.encode(serde_json::to_vec(&challenge).unwrap());
        let html = format!(r#"<script>var cs="{}";</script>"#, encoded);

        let cookie = solve_waf_challenge(&html).unwrap();
        let encoded_cookie = cookie.strip_prefix("_wafchallengeid=").unwrap();
        let solved: Value =
            serde_json::from_slice(&STANDARD.decode(encoded_cookie).unwrap()).unwrap();

        assert_eq!(solved["d"], STANDARD.encode(nonce.to_string()));
        assert_eq!(solved["v"], challenge["v"]);
    }
    #[test]
    fn supports_app_image_layout_and_string_live_marker() {
        let data = json!({
            "video": {
                "clip_type": "live_photo",
                "play_addr": { "uri": "https://cdn.example/live.mp4" }
            },
            "image_post_info": {
                "image_list": [{
                    "display_image": { "url_list": ["https://cdn.example/photo.webp"] },
                    "video": {
                        "download_addr": { "url_list": ["https://cdn.example/live-item.mp4"] }
                    }
                }]
            }
        });
        let parser = DouyinParser;
        let images = parser.extract_image_items(&data);
        assert_eq!(images.len(), 1);
        assert_eq!(
            parser.extract_image_url(images[0]).as_deref(),
            Some("https://cdn.example/photo.webp")
        );
        assert_eq!(
            parser.extract_live_photo_url(&data, images[0]).as_deref(),
            Some("https://cdn.example/live-item.mp4")
        );
    }

    #[test]
    fn does_not_mark_unflagged_soundtrack_as_live_photo() {
        let data = json!({
            "music": { "duration": 68 },
            "video": {
                "duration": 0,
                "play_addr": { "uri": "https://cdn.example/audio.mp4" }
            },
            "images": [{ "url_list": ["https://cdn.example/photo.jpg"] }]
        });
        let parser = DouyinParser;
        assert_eq!(
            parser.extract_live_photo_url(&data, &data["images"][0]),
            None
        );
    }
    #[test]
    fn extracts_nested_live_photo_and_audio_fallback() {
        let data = json!({
            "music": { "duration": 68 },
            "video": {
                "duration": 0,
                "play_addr": { "uri": "https://cdn.example/audio.mp4" }
            },
            "images": [{
                "url_list": ["https://cdn.example/photo.jpg"],
                "video": {
                    "play_addr": { "url_list": ["https://cdn.example/live.mp4"] }
                }
            }]
        });
        let parser = DouyinParser;
        assert_eq!(
            parser.extract_music_url(&data).as_deref(),
            Some("https://cdn.example/audio.mp4")
        );
        assert_eq!(
            parser
                .extract_live_photo_url(&data, &data["images"][0])
                .as_deref(),
            Some("https://cdn.example/live.mp4")
        );
    }

    #[test]
    fn extracts_marked_top_level_live_photo() {
        let data = json!({
            "live_photo_type": 1,
            "video": {
                "duration": 3,
                "play_addr": { "url_list": ["https://cdn.example/live.mp4"] }
            },
            "images": [{ "url_list": ["https://cdn.example/photo.jpg"] }]
        });
        let parser = DouyinParser;
        assert_eq!(
            parser
                .extract_live_photo_url(&data, &data["images"][0])
                .as_deref(),
            Some("https://cdn.example/live.mp4")
        );
    }
}

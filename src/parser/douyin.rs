use crate::models::{Author, ImgInfo, VideoParseInfo};
use crate::parser::VideoParser;
use crate::utils::{
    create_cookie_http_client, create_http_client, create_no_redirect_client,
    extract_json_from_html, generate_numeric_id, generate_random_string,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use scraper::{Html, Selector};
use serde_json::Value;
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
                let response = client
                    .get(fallback_url)
                    .header("User-Agent", crate::utils::DOUYIN_MOBILE_USER_AGENT)
                    .send()
                    .await?;
                let html = response.text().await?;
                tracing::debug!(
                    html_len = html.len(),
                    has_aweme_id = html.contains("\"aweme_id\""),
                    has_item_id = html.contains("\"itemId\""),
                    "收到抖音移动分享页"
                );
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

        info.music_url = data
            .pointer("/music/play_url/url_list/0")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        // 提取封面
        info.cover_url = self.get_non_webp_url(data.pointer("/video/cover/url_list"));

        // 提取图集
        if let Some(images_array) = data.pointer("/images").and_then(|v| v.as_array()) {
            for image in images_array {
                if let Some(url_list) = image.pointer("/url_list") {
                    if let Some(img_url) = self.get_non_webp_url(Some(url_list)) {
                        info.images.push(ImgInfo {
                            url: img_url,
                            live_photo_url: image
                                .pointer("/video/play_addr/url_list/0")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        });
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}

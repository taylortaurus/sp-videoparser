use crate::models::VideoParseInfo;
use crate::parser::VideoParser;
use crate::utils::create_http_client;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use scraper::{Html, Selector};

pub struct MeipaiParser;

#[async_trait]
impl VideoParser for MeipaiParser {
    async fn parse_share_url(&self, share_url: &str) -> Result<VideoParseInfo> {
        let client = create_http_client()?;
        let response = client
            .get(share_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/79.0.3945.88 Safari/537.36",
            )
            .send()
            .await?;

        let html = response.text().await?;
        let doc = Html::parse_document(&html);

        let share_btn_sel = Selector::parse("#shareMediaBtn").unwrap();
        let video_bs64 = doc
            .select(&share_btn_sel)
            .next()
            .and_then(|el| el.value().attr("data-video"))
            .ok_or_else(|| anyhow!("parse video base64 from share url fail"))?;

        let video_url = Self::parse_video_bs64(video_bs64)
            .map_err(|_| anyhow!("parse video play url fail"))?;

        let cover_sel = Selector::parse("#detailVideo img").unwrap();
        let cover_url = doc
            .select(&cover_sel)
            .next()
            .and_then(|el| el.value().attr("src"))
            .unwrap_or("")
            .to_string();

        let avatar_sel = Selector::parse(".detail-avatar").unwrap();
        let avatar_el = doc.select(&avatar_sel).next();
        let user_name = avatar_el
            .and_then(|el| el.value().attr("alt"))
            .unwrap_or("")
            .to_string();
        let user_avatar = avatar_el
            .and_then(|el| el.value().attr("src"))
            .unwrap_or("");

        let title_sel = Selector::parse(".detail-cover-title").unwrap();
        let title = doc
            .select(&title_sel)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default();

        let mut info = VideoParseInfo::new();
        info.title = title;
        info.video_url = Some(video_url);
        info.cover_url = if cover_url.is_empty() { None } else { Some(cover_url) };
        info.author.name = user_name;
        info.author.avatar = format!("https:{}", user_avatar);

        Ok(info)
    }

    async fn parse_video_id(&self, video_id: &str) -> Result<VideoParseInfo> {
        let req_url = format!("https://www.meipai.com/video/{}", video_id);
        self.parse_share_url(&req_url).await
    }
}

impl MeipaiParser {
    /// 还原美拍 data-video 中混淆的视频地址（移植自 Go 版算法）
    fn parse_video_bs64(video_bs64: &str) -> Result<String> {
        let (hex_1, str_1) = Self::get_hex(video_bs64)?;
        let (pre, tail) = Self::get_dec(&hex_1)?;
        let d = Self::sub_str(&str_1, &pre)?;
        let p = Self::get_pos(&d, &tail)?;
        let kk = Self::sub_str(&d, &p)?;
        let decoded = STANDARD
            .decode(kk.as_bytes())
            .map_err(|e| anyhow!("base64解码失败: {}", e))?;
        Ok(format!("https:{}", String::from_utf8_lossy(&decoded)))
    }

    /// 取前 4 位反转作为 hex，其余作为待处理字符串
    fn get_hex(s: &str) -> Result<(String, String)> {
        if s.len() < 4 {
            return Err(anyhow!("data-video 长度不足"));
        }
        let hex: String = s[0..4].chars().rev().collect();
        let str_1 = s[4..].to_string();
        Ok((hex, str_1))
    }

    /// 将 hex 转为十进制后，最高两位为 pre，其余为 tail
    fn get_dec(hex: &str) -> Result<(Vec<usize>, Vec<usize>)> {
        let n = u32::from_str_radix(hex, 16)
            .map_err(|e| anyhow!("hex 解析失败: {}", e))?;
        let s = n.to_string();
        let digits: Vec<usize> = s.chars().map(|c| c.to_digit(10).unwrap() as usize).collect();
        if digits.len() < 2 {
            return Err(anyhow!("dec 位数不足"));
        }
        let pre = digits[0..2].to_vec();
        let tail = digits[2..].to_vec();
        Ok((pre, tail))
    }

    /// 等价于 Go 版 subStr: 拼接头部并删除指定子串
    fn sub_str(s: &str, b: &[usize]) -> Result<String> {
        if b.len() < 2 {
            return Err(anyhow!("substr param b length is not correct"));
        }
        let b0 = b[0];
        let b1 = b[1];
        if b0 > s.len() || b0 + b1 > s.len() {
            return Err(anyhow!("substr index out of range"));
        }
        let c = &s[0..b0];
        let d = &s[b0..b0 + b1];
        let temp = s[b0..].replace(d, "");
        Ok(format!("{}{}", c, temp))
    }

    /// 等价于 Go 版 getPos: 计算新的起始位置
    fn get_pos(s: &str, b: &[usize]) -> Result<Vec<usize>> {
        if b.len() < 2 {
            return Err(anyhow!("getpos param b length is not correct"));
        }
        if b[0] + b[1] > s.len() {
            return Err(anyhow!("getpos index out of range"));
        }
        Ok(vec![s.len() - b[0] - b[1], b[1]])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_dec() {
        // 0x000a = 10 -> "10" -> pre=[1,0], tail=[]
        let (pre, tail) = MeipaiParser::get_dec("000a").unwrap();
        assert_eq!(pre, vec![1, 0]);
        assert!(tail.is_empty());
    }

    #[test]
    fn test_get_hex() {
        let (hex, rest) = MeipaiParser::get_hex("abcd1234").unwrap();
        assert_eq!(hex, "dcba"); // 前4位反转
        assert_eq!(rest, "1234");
    }
}

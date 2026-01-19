// 文件名: news.rs

use reqwest::Client;
use anyhow::Result;

pub struct NewsSentinel {
    client: Client,
}

impl NewsSentinel {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// [修改] 仅负责抓取和清洗标题，不做任何情感判断
    /// 返回格式：纯文本列表
    pub async fn fetch_raw_headlines(&self, _symbol: &str) -> Result<String> {
        let url = "https://www.coindesk.com/arc/outboundfeeds/rss/";
        
        // 增加重试逻辑
        let mut content = String::new();
        for _ in 0..3 {
            match self.client.get(url).timeout(std::time::Duration::from_secs(15)).send().await {
                Ok(resp) => {
                    if let Ok(text) = resp.text().await {
                        content = text;
                        break;
                    }
                },
                Err(_) => continue,
            }
        }

        if content.is_empty() {
            return Ok("No news available (Network Error)".to_string());
        }
        
        let mut headlines = Vec::new();
        let parts: Vec<&str> = content.split("<item>").collect();
        
        // 获取前 15 条新闻 (既然上下文够大，就多拿点)
        for part in parts.iter().skip(1).take(15) {
            if let Some(start) = part.find("<title>") {
                if let Some(end) = part.find("</title>") {
                    let title = &part[start + 7..end];
                    let clean_title = title.replace("<![CDATA[", "").replace("]]>", "").trim().to_string();
                    if !clean_title.is_empty() {
                        headlines.push(clean_title);
                    }
                }
            }
        }

        if headlines.is_empty() {
            return Ok("No news headlines found.".to_string());
        }

        // 格式化为 Markdown 列表供 LLM 阅读
        let mut output = String::from("Recent Headlines:\n");
        for (i, h) in headlines.iter().enumerate() {
            output.push_str(&format!("{}. {}\n", i + 1, h));
        }

        Ok(output)
    }
}
use reqwest::Client;
use anyhow::{Result, Context, anyhow};
use serde_json::Value;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use std::sync::Arc;
use tracing::warn;

pub struct RedditSentinel {
    client: Client,
    client_id: String,
    client_secret: String,
    token_cache: Arc<Mutex<(String, u64)>>, 
}

impl RedditSentinel {
    pub fn new(client: Client) -> Self {
        Self { 
            client,
            client_id: env::var("REDDIT_CLIENT_ID").unwrap_or_default(),
            client_secret: env::var("REDDIT_CLIENT_SECRET").unwrap_or_default(),
            token_cache: Arc::new(Mutex::new(("".to_string(), 0))),
        }
    }

    async fn get_access_token(&self) -> Result<String> {
        let mut cache = self.token_cache.lock().await;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        if !cache.0.is_empty() && cache.1 > now + 60 {
            return Ok(cache.0.clone());
        }

        let url = "https://www.reddit.com/api/v1/access_token" ;
        let params = [("grant_type", "client_credentials")];

        let resp = self.client.post(url)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&params)
            .header("User-Agent", "rust_trader/5.6") 
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Status: {}", resp.status()));
        }

        let json: Value = resp.json().await?;
        let access_token = json["access_token"].as_str().context("No access_token")?.to_string();
        let expires_in = json["expires_in"].as_u64().unwrap_or(3600);
        
        *cache = (access_token.clone(), now + expires_in);
        Ok(access_token)
    }

    async fn fetch_public_json(&self) -> Result<String> {
        // [修改] 获取前 10 条以弥补信息量，因为只取标题
        let url = "https://www.reddit.com/r/CryptoCurrency/hot.json?limit=10";
        let resp: Value = self.client.get(url)
            .header("User-Agent", "rust_trader/5.6 (fallback)")
            .send()
            .await?
            .json()
            .await?;
        self.parse_json_response(resp)
    }

    // [修改] 只提取标题，不再拼接正文
    fn parse_json_response(&self, json: Value) -> Result<String> {
        let mut raw_content = String::new();

        if let Some(children) = json["data"]["children"].as_array() {
            for item in children.iter() { // 迭代所有获取到的条目
                let data = &item["data"];
                let title = data["title"].as_str().unwrap_or("");
                // 移除正文 selftext，大幅降低噪音
                
                // 使用列表格式，更清晰
                raw_content.push_str(&format!("• {}\n", title));
            }
        }

        if raw_content.is_empty() {
            Ok("No Reddit data found.".to_string())
        } else {
            Ok(raw_content)
        }
    }

    pub async fn analyze_sentiment(&self) -> Result<String> {
        // 尝试走 OAuth，失败走 Public Fallback
        if !self.client_id.is_empty() {
            match self.get_access_token().await {
                Ok(token) => {
                    // [修改] limit=10
                    let url = "https://oauth.reddit.com/r/CryptoCurrency/hot?limit=10";
                    let resp = self.client.get(url)
                        .header("Authorization", format!("Bearer {}", token))
                        .header("User-Agent", "rust_trader/5.6")
                        .send()
                        .await;
                    
                    match resp {
                        Ok(r) => {
                            if r.status().is_success() {
                                if let Ok(json) = r.json().await {
                                    return self.parse_json_response(json);
                                }
                            }
                        },
                        Err(_) => {} 
                    }
                },
                Err(e) => warn!("Reddit Key Error: {}. Using fallback...", e),
            }
        }
        self.fetch_public_json().await
    }
}
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use url::Url;
use std::env;
use std::sync::Arc;
use tracing::{info, error, warn};
use serde_json::{json, Value};
use dashmap::DashMap;
use std::time::Instant;

pub type PriceCache = Arc<DashMap<String, (f64, Instant)>>;

pub struct OkxWsClient {
    url: String,
    price_cache: PriceCache,
}

impl OkxWsClient {
    pub fn new(price_cache: PriceCache) -> Self {
        // [修改] 默认地址改为最标准的 ws.okx.com，香港节点连接最稳
        let url = env::var("OKX_WS_URL").unwrap_or("wss://ws.okx.com:8443/ws/v5/public".to_string());
        Self { url, price_cache }
    }

    pub async fn run(&self, symbols: Vec<String>) {
        let url = match Url::parse(&self.url) {
            Ok(u) => u,
            Err(e) => {
                error!("CRITICAL: Invalid WebSocket URL '{}': {}", self.url, e);
                return;
            }
        };
        
        loop {
            info!("🔌 Connecting to OKX WebSocket ({}) ...", self.url);
            match connect_async(url.clone()).await {
                Ok((ws_stream, _)) => {
                    info!("✅ OKX WebSocket Connected.");
                    let (mut write, mut read) = ws_stream.split();

                    let args: Vec<_> = symbols.iter().map(|s| {
                        json!({
                            "channel": "tickers",
                            "instId": s
                        })
                    }).collect();

                    let sub_msg = json!({
                        "op": "subscribe",
                        "args": args
                    });

                    if let Err(e) = write.send(Message::Text(sub_msg.to_string())).await {
                        error!("Failed to subscribe: {}", e);
                        continue;
                    }

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                                    if let Some(data) = parsed["data"].as_array() {
                                        for item in data {
                                            if let (Some(inst_id), Some(last)) = (item["instId"].as_str(), item["last"].as_str()) {
                                                if let Ok(price) = last.parse::<f64>() {
                                                    self.price_cache.insert(inst_id.to_string(), (price, Instant::now()));
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Ok(Message::Ping(_)) => {},
                            Err(e) => {
                                warn!("WS Error: {}", e);
                                break; 
                            },
                            _ => {}
                        }
                    }
                },
                Err(e) => {
                    error!("WS Connection Failed: {}. Retrying in 5s...", e);
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}
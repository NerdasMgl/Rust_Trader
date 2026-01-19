use reqwest::{Client, Method};
use anyhow::{Result, anyhow};
use std::env;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose};
use serde_json::{json, Value};
use tracing::{info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

// ----------------------------------------------------------------------------
// 数据结构定义
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PositionSummary {
    pub symbol: String,
    pub size: f64,
    pub upl: f64,
    pub side: String,
    // [新增] 满足通知需求的关键字段
    pub leverage: u32,
    pub notional_usd: f64, // 持仓名义价值
    pub margin_usd: f64,   // 保证金占用
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PnlRecord {
    pub symbol: String,
    pub pnl: f64,
    pub fee: f64,
    pub ts: i64,
    pub type_name: String,
    pub ord_id: String,
}

#[allow(dead_code)]
pub struct OrderResult {
    pub order_id: String,
    pub response: String,
}

#[derive(Debug, Clone)]
pub struct InstrumentMeta {
    pub face_value: f64, 
    pub tick_size: f64,  
    pub min_sz: f64,     
    pub lot_sz: f64,     
}

pub struct BalanceSummary {
    pub total_equity: f64,
    pub available_balance: f64,
}

pub struct TradeExecutor {
    client: Client,
    base_url: String,
    api_key: String,
    secret_key: String,
    passphrase: String,
    is_simulated: bool,
    is_dry_run: bool,
    
    instruments_cache: Arc<RwLock<HashMap<String, InstrumentMeta>>>,
}

impl TradeExecutor {
    pub fn new(client: Client) -> Self {
        let is_sim = env::var("OKX_SIMULATED").unwrap_or("0".to_string()) == "1";
        let is_dry = env::var("DRY_RUN").unwrap_or("0".to_string()) == "1";
        
        Self {
            client,
            base_url: env::var("OKX_BASE_URL").unwrap_or("https://www.okx.com".to_string()),
            api_key: env::var("OKX_API_KEY").unwrap_or_default(),
            secret_key: env::var("OKX_SECRET_KEY").unwrap_or_default(),
            passphrase: env::var("OKX_PASSPHRASE").unwrap_or_default(),
            is_simulated: is_sim,
            is_dry_run: is_dry,
            instruments_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ------------------------------------------------------------------------
    // 签名与请求辅助
    // ------------------------------------------------------------------------
    fn sign_request(&self, method: &str, path: &str, body: &str, timestamp: &str) -> String {
        let message = format!("{}{}{}{}", timestamp, method, path, body);
        let mut mac = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        let result = mac.finalize();
        general_purpose::STANDARD.encode(result.into_bytes())
    }

    async fn send_signed_request(&self, method: Method, path: &str, body_json: &Value) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S.000Z").to_string();
        let body_str = if method == Method::GET { "".to_string() } else { body_json.to_string() };

        let sign = self.sign_request(method.as_str(), path, &body_str, &timestamp);

        for attempt in 1..=3 {
            let mut retry_req = self.client.request(method.clone(), &url)
                .header("OK-ACCESS-KEY", &self.api_key)
                .header("OK-ACCESS-SIGN", &sign)
                .header("OK-ACCESS-TIMESTAMP", &timestamp)
                .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
                .header("Content-Type", "application/json");
            
            if self.is_simulated {
                retry_req = retry_req.header("x-simulated-trading", "1");
            }
            if method != Method::GET {
                retry_req = retry_req.json(body_json);
            }

            match retry_req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    
                    if status.is_success() {
                        let json_val: Value = serde_json::from_str(&text).unwrap_or(json!({}));
                        if json_val["code"].as_str().unwrap_or("1") == "0" {
                            return Ok(json_val);
                        } else {
                            warn!("❌ OKX Biz Error: {} | Msg: {} | Req Body: {}", json_val["code"], json_val["msg"], body_str);
                            return Err(anyhow!("OKX Biz Error: {} | Msg: {}", json_val["code"], json_val["msg"]));
                        }
                    } else {
                        warn!("⚠️ OKX HTTP {} (Attempt {}/3): {}", status, attempt, text);
                    }
                },
                Err(e) => {
                    warn!("⚠️ OKX Network Error (Attempt {}/3): {}", attempt, e);
                }
            }
            sleep(Duration::from_millis(500 * attempt as u64)).await;
        }

        Err(anyhow!("OKX Request Failed after 3 attempts: {}", path))
    }

    // ------------------------------------------------------------------------
    // 元数据管理
    // ------------------------------------------------------------------------
    pub async fn init_instruments_cache(&self) -> Result<()> {
        info!("⏳ Fetching Instrument Metadata from OKX...");
        
        let resp = self.send_signed_request(Method::GET, "/api/v5/public/instruments?instType=SWAP", &json!({})).await?;
        
        let mut cache = self.instruments_cache.write().await;
        cache.clear();

        if let Some(data) = resp["data"].as_array() {
            for item in data {
                let inst_id = item["instId"].as_str().unwrap_or_default().to_string();
                if inst_id.is_empty() { continue; }

                let face_val = item["ctVal"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                let tick_sz = item["tickSz"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                let min_sz = item["minSz"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                let lot_sz = item["lotSz"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

                cache.insert(inst_id, InstrumentMeta {
                    face_value: face_val,
                    tick_size: tick_sz,
                    min_sz,
                    lot_sz,
                });
            }
            info!("✅ Instruments Meta Cache Initialized: {} symbols loaded.", cache.len());
        }
        Ok(())
    }

    pub async fn get_face_value(&self, symbol: &str) -> f64 {
        let cache = self.instruments_cache.read().await;
        cache.get(symbol).map(|m| m.face_value).unwrap_or(0.0)
    }

    pub async fn get_min_size(&self, symbol: &str) -> f64 {
        let cache = self.instruments_cache.read().await;
        cache.get(symbol).map(|m| m.min_sz).unwrap_or(1.0)
    }

    async fn format_sz(&self, symbol: &str, size: f64) -> String {
        let cache = self.instruments_cache.read().await;
        if let Some(meta) = cache.get(symbol) {
            if meta.lot_sz > 0.0 {
                let epsilon = 1e-9;
                let steps = ((size + epsilon) / meta.lot_sz).floor();
                let aligned = steps * meta.lot_sz;
                
                let decimals = if meta.lot_sz < 1.0 {
                    meta.lot_sz.log10().abs().ceil() as usize
                } else { 0 };
                
                return format!("{:.*}", decimals, aligned);
            }
        }
        format!("{}", size)
    }

    async fn format_price_dynamic(&self, symbol: &str, price: f64) -> String {
        let cache = self.instruments_cache.read().await;
        if let Some(meta) = cache.get(symbol) {
            if meta.tick_size > 0.0 {
                let decimals = meta.tick_size.log10().abs().ceil() as usize;
                return format!("{:.*}", decimals, price);
            }
        }
        let decimals = if price < 0.01 { 6 } else if price < 1.0 { 4 } else if price < 10.0 { 3 } else { 2 };
        format!("{:.*}", decimals, price)
    }

    // ------------------------------------------------------------------------
    // 核心交易功能
    // ------------------------------------------------------------------------
    
    pub async fn fetch_account_summary(&self) -> Result<BalanceSummary> {
        let resp = self.send_signed_request(Method::GET, "/api/v5/account/balance?ccy=USDT", &json!({})).await?;
        
        let details = &resp["data"][0]["details"][0];
        let equity = details["eq"].as_str().unwrap_or("0").parse::<f64>()?;
        let avail = details["availEq"].as_str().unwrap_or("0").parse::<f64>()?; 
        
        Ok(BalanceSummary {
            total_equity: equity,
            available_balance: avail,
        })
    }

    pub async fn fetch_positions(&self) -> Result<Vec<PositionSummary>> {
        let resp = self.send_signed_request(Method::GET, "/api/v5/account/positions?instType=SWAP", &json!({})).await?;
        
        let mut list = Vec::new();
        if let Some(data) = resp["data"].as_array() {
            for item in data {
                let sz = item["pos"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                if sz == 0.0 { continue; }
                
                list.push(PositionSummary {
                    symbol: item["instId"].as_str().unwrap_or("").to_string(),
                    size: sz,
                    upl: item["upl"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0),
                    side: item["posSide"].as_str().unwrap_or("net").to_string(),
                    // [新增] 提取更多字段用于通知
                    leverage: item["lever"].as_str().unwrap_or("1").parse::<u32>().unwrap_or(1),
                    notional_usd: item["notionalUsd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0),
                    margin_usd: item["mgn"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0),
                });
            }
        }
        Ok(list)
    }

    pub async fn execute_order(
        &self, 
        symbol: &str, 
        side: &str, 
        pos_side: &str, 
        size: f64, 
        current_price: f64,
        tp_pct: f64,
        sl_pct: f64,
        leverage: Option<u32>
    ) -> Result<OrderResult> {
        if let Some(lev) = leverage {
            let lev_body = json!({
                "instId": symbol,
                "lever": lev.to_string(),
                "mgnMode": "cross"
            });
            let _ = self.send_signed_request(Method::POST, "/api/v5/account/set-leverage", &lev_body).await;
        }

        let sz_str = self.format_sz(symbol, size).await;
        if sz_str.parse::<f64>().unwrap_or(0.0) == 0.0 {
            return Err(anyhow!("Order size {} too small after formatting (sz_str: {})", size, sz_str));
        }

        let mut body_map = serde_json::Map::new();
        body_map.insert("instId".to_string(), json!(symbol));
        body_map.insert("tdMode".to_string(), json!("cross"));
        body_map.insert("side".to_string(), json!(side));
        body_map.insert("posSide".to_string(), json!(pos_side));
        body_map.insert("ordType".to_string(), json!("market"));
        body_map.insert("sz".to_string(), json!(sz_str));

        if tp_pct > 0.0 && sl_pct > 0.0 {
            let (tp_price, sl_price) = if pos_side == "long" {
                (current_price * (1.0 + tp_pct), current_price * (1.0 - sl_pct))
            } else {
                (current_price * (1.0 - tp_pct), current_price * (1.0 + sl_pct))
            };

            if tp_price > 0.0 && sl_price > 0.0 {
                let tp_str = self.format_price_dynamic(symbol, tp_price).await;
                let sl_str = self.format_price_dynamic(symbol, sl_price).await;
                
                info!("🛡️ Attaching Algo: TP {} ({}%) / SL {} ({}%)", tp_str, tp_pct*100.0, sl_str, sl_pct*100.0);
                
                body_map.insert("attachAlgoOrds".to_string(), json!([{
                    "tpTriggerPx": tp_str,
                    "tpOrdPx": "-1", 
                    "slTriggerPx": sl_str,
                    "slOrdPx": "-1"
                }]));
            } else {
                warn!("⚠️ TPSL Skipped: Calculated prices invalid. TP: {}, SL: {}", tp_price, sl_price);
            }
        }

        if self.is_dry_run {
            info!("🧪 [DRY RUN] Order: {} {} {} sz={}", side, pos_side, symbol, sz_str);
            return Ok(OrderResult { order_id: "dry-run".to_string(), response: "ok".to_string() });
        }

        info!("🚀 Placing Atomic Order for {} (sz: {})...", symbol, sz_str);
        let res = self.send_signed_request(Method::POST, "/api/v5/trade/order", &Value::Object(body_map)).await?;
        
        let ord_id = res["data"][0]["ordId"].as_str().unwrap_or("unknown").to_string();
        info!("✅ OKX Order Success: ID {}", ord_id);
        Ok(OrderResult { order_id: ord_id, response: res.to_string() })
    }

    pub async fn fetch_recent_pnl(&self) -> Result<Vec<PnlRecord>> {
        let resp = self.send_signed_request(Method::GET, "/api/v5/account/bills?instType=SWAP&type=2", &json!({})).await?;
        
        let mut list = Vec::new();
        if let Some(data) = resp["data"].as_array() {
            for item in data {
                list.push(PnlRecord {
                    symbol: item["instId"].as_str().unwrap_or("").to_string(),
                    pnl: item["pnl"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
                    fee: item["fee"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
                    ts: item["ts"].as_str().unwrap_or("0").parse().unwrap_or(0),
                    type_name: item["type"].as_str().unwrap_or("").to_string(),
                    ord_id: item["ordId"].as_str().unwrap_or("").to_string(),
                });
            }
        }
        Ok(list)
    }
}
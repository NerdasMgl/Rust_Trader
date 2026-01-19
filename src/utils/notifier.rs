use reqwest::Client;
use serde_json::json;
use std::env;
use tracing::error; // [修改] 移除了 info 和 warn，只保留 error
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose};
use std::time::{SystemTime, UNIX_EPOCH};
use url::form_urlencoded;

/// [新增] 用于构建友好的持仓报告
pub struct PositionReportItem {
    pub symbol: String,
    pub side: String,
    pub notional_usdt: f64, 
    pub margin_usdt: f64,   
    pub upl: f64,           
    pub leverage: u32,      
}

pub struct DingTalkNotifier {
    client: Client,
    webhook_url: String,
    secret: String,
    keyword: String, 
}

impl DingTalkNotifier {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            webhook_url: env::var("DINGTALK_WEBHOOK").unwrap_or_default(),
            secret: env::var("DINGTALK_SECRET").unwrap_or_default(),
            keyword: env::var("DINGTALK_KEYWORD").unwrap_or("Trading".to_string()),
        }
    }

    fn get_signed_url(&self) -> String {
        if self.secret.is_empty() {
            return self.webhook_url.clone();
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();

        let string_to_sign = format!("{}\n{}", timestamp, self.secret);
        
        let mut mac = Hmac::<Sha256>::new_from_slice(self.secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let signature = general_purpose::STANDARD.encode(mac.finalize().into_bytes());
        
        let encoded_val: String = form_urlencoded::byte_serialize(signature.as_bytes()).collect();

        if self.webhook_url.contains('?') {
            format!("{}&timestamp={}&sign={}", self.webhook_url, timestamp, encoded_val)
        } else {
            format!("{}?timestamp={}&sign={}", self.webhook_url, timestamp, encoded_val)
        }
    }

    fn attach_keyword(&self, content: &str) -> String {
        if self.keyword.is_empty() {
            return content.to_string();
        }
        if content.contains(&self.keyword) {
            return content.to_string();
        }
        format!("{}\n\n[{}]", content, self.keyword)
    }

    async fn send(&self, body: &serde_json::Value) {
        if self.webhook_url.is_empty() { return; }
        
        let url = self.get_signed_url();
        match self.client.post(&url).json(body).send().await {
            Ok(resp) => {
                match resp.text().await {
                    Ok(text) => {
                        if let Ok(json_resp) = serde_json::from_str::<serde_json::Value>(&text) {
                            if json_resp["errcode"].as_i64().unwrap_or(-1) != 0 {
                                error!("❌ DingTalk Error: {}", text);
                            }
                        }
                    },
                    Err(e) => error!("❌ Failed to read response body: {}", e),
                }
            },
            Err(e) => error!("❌ DingTalk Network Error: {}", e),
        }
    }

    pub async fn send_alert(&self, content: &str) {
        let prefix = "⚠️ [RustTrader Alert]";
        let safe_content = self.attach_keyword(content); 
        
        let body = json!({
            "msgtype": "text",
            "text": {
                "content": format!("{}\n{}", prefix, safe_content)
            }
        });
        self.send(&body).await;
    }

    pub async fn send_trade_signal(
        &self, 
        symbol: &str, 
        action: &str, 
        size: f64, 
        price: f64, 
        reason: &str, 
        tp_pct: f64, 
        sl_pct: f64
    ) {
        let title = format!("{} {} (Signal)", action.to_uppercase(), symbol);
        
        let side_color = if action.to_lowercase().contains("buy") || action.to_lowercase().contains("long") {
            "#00AA00" 
        } else {
            "#FF0000" 
        };

        let (tp_price, sl_price) = if action.to_lowercase().contains("buy") {
            (price * (1.0 + tp_pct), price * (1.0 - sl_pct))
        } else {
            (price * (1.0 - tp_pct), price * (1.0 + sl_pct))
        };

        let raw_text = format!(
            "### <font color='{}'>🚀 交易执行: {}</font>\n\n\
            **标的**: {}\n\
            **数量**: {:.4} 张\n\
            **成交价**: ${:.2}\n\
            \n---\n\
            **🎯 计划止盈**: ${:.2} ({:.1}%)\n\
            **🛡️ 计划止损**: ${:.2} ({:.1}%)\n\
            \n---\n\
            **🧠 AI 决策逻辑**:\n> {}\n",
            side_color, action.to_uppercase(), symbol, size, price,
            tp_price, tp_pct * 100.0,
            sl_price, sl_pct * 100.0,
            reason
        );

        let safe_text = self.attach_keyword(&raw_text); 
        self.send_markdown_raw(&title, &safe_text).await;
    }

    pub async fn send_startup_report(
        &self,
        initial_capital: f64,
        start_time: &str,
        positions: Vec<PositionReportItem>
    ) {
        let title = "🚀 系统已启动 (Boot)";
        
        let mut pos_desc = String::new();
        if positions.is_empty() {
            pos_desc = "> *当前无持仓 (Flat)*".to_string();
        } else {
            for p in positions {
                let side_icon = if p.side.to_lowercase().contains("long") { "🟢" } else { "🔴" };
                let pnl_color = if p.upl >= 0.0 { "#FF0000" } else { "#00AA00" };
                let pnl_sign = if p.upl >= 0.0 { "+" } else { "" };
                
                pos_desc.push_str(&format!(
                    "- {} **{}** ({}x)\n   📦 **仓位价值**: `${:.0}`\n   🔒 **投入本金**: `${:.0}`\n   💰 **浮动盈亏**: <font color='{}'>{}${:.2}</font>\n\n",
                    side_icon, 
                    p.symbol.split('-').next().unwrap_or(&p.symbol),
                    p.leverage,
                    p.notional_usdt,
                    p.margin_usdt,
                    pnl_color, pnl_sign, p.upl
                ));
            }
        }

        let raw_text = format!(
            "### Rust Trader V6.0 (HK Node)\n\n\
            ---\n\
            💰 **初始本金**: `${:.2}`\n\
            🕒 **启动时间**: {}\n\
            📊 **本轮收益**: `0.00%` (基准已建立)\n\
            \n---\n\
            #### 🏷️ 初始持仓详情\n\
            {}",
            initial_capital, start_time, pos_desc
        );

        let safe_text = self.attach_keyword(&raw_text);
        self.send_markdown_raw(title, &safe_text).await;
    }

    pub async fn send_status_report(
        &self, 
        equity: f64, 
        pnl_pct: f64, 
        positions: Vec<PositionReportItem>
    ) {
        let title = "📊 运行周报";
        let pnl_color = if pnl_pct >= 0.0 { "#FF0000" } else { "#00AA00" }; 
        let pnl_sign = if pnl_pct >= 0.0 { "+" } else { "" };

        let mut pos_desc = String::new();
        if positions.is_empty() {
            pos_desc = "> *当前无持仓 (Flat)*".to_string();
        } else {
            for p in positions {
                let side_icon = if p.side.to_lowercase().contains("long") { "🟢" } else { "🔴" };
                let item_pnl_color = if p.upl >= 0.0 { "#FF0000" } else { "#00AA00" };
                
                pos_desc.push_str(&format!(
                    "- {} **{}** ({}x)\n   `${:.0}`(仓位) | `${:.0}`(本金) | <font color='{}'>${:.2}</font>\n",
                    side_icon, 
                    p.symbol.split('-').next().unwrap_or(&p.symbol),
                    p.leverage,
                    p.notional_usdt,
                    p.margin_usdt,
                    item_pnl_color, p.upl
                ));
            }
        }

        let raw_text = format!(
            "### 🤖 系统运行状态\n\n\
            💰 **当前权益**: `${:.2}`\n\
            📈 **累计收益**: <font color='{}'>{}{:.2}%</font>\n\n\
            🏷️ **持仓资金分布**:\n{}",
            equity, pnl_color, pnl_sign, pnl_pct, pos_desc
        );
        
        let safe_text = self.attach_keyword(&raw_text);
        self.send_markdown_raw(title, &safe_text).await;
    }

    /// [修改] 增加 #[allow(dead_code)] 避免未使用的警告
    #[allow(dead_code)]
    pub async fn send_evolution_log(&self, log_type: &str, symbol: &str, content: &str) {
        let title = format!("🧬 AI Evolution: {}", log_type);
        let color = if log_type == "MISTAKE" { "#FF9900" } else { "#0066FF" };
        
        let raw_text = format!(
            "### <font color='{}'>🧬 进化日志: {}</font>\n\n\
            **标的**: {}\n\n\
            **内容摘要**:\n> {}",
            color, log_type, symbol, content
        );
        
        let safe_text = self.attach_keyword(&raw_text);
        self.send_markdown_raw(&title, &safe_text).await;
    }

    async fn send_markdown_raw(&self, title: &str, text: &str) {
        let body = json!({
            "msgtype": "markdown",
            "markdown": {
                "title": title,
                "text": text
            }
        });
        self.send(&body).await;
    }
    
    // [修改] 增加 #[allow(dead_code)] 避免未使用的警告
    #[allow(dead_code)]
    pub async fn send_markdown(&self, title: &str, text: &str) {
        let safe_text = self.attach_keyword(text);
        self.send_markdown_raw(title, &safe_text).await;
    }
    
    pub async fn send_text(&self, content: &str) {
        self.send_alert(content).await;
    }
}
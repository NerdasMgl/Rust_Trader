use reqwest::Client;
use anyhow::{Result, anyhow, Context};
use serde_json::{json, Value};
use std::env;
use std::time::Duration;
use tokio::time::sleep;
// 引用路径改为 utils，确保文件结构正确
use crate::modules::perception::structs::MarketState;

use tracing::{info, warn};

pub struct DecisionMaker {
    client: Client,
    ds_key: String,
    ds_url: String,
    strategy_version: String,
}

// [关键修改] 添加 PartialEq, Clone 以支持主程序中的比较逻辑
#[derive(Debug, PartialEq, Clone)]
pub enum TradeAction {
    Buy,
    Sell,
    CloseLong,
    CloseShort,
    Hold,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct AiDecision {
    pub action: TradeAction,
    pub reason: String,
    pub tp_pct: f64, 
    pub sl_pct: f64,
    pub leverage: u32,
    pub win_rate: f64,       
    pub kelly_fraction: f64, 
    pub risk_reward_ratio: f64,
    pub strategy_version: String,
}

impl AiDecision {
    #[allow(dead_code)]
    pub fn action_name(&self) -> String {
        match self.action {
            TradeAction::Buy => "OPEN LONG".to_string(),
            TradeAction::Sell => "OPEN SHORT".to_string(),
            TradeAction::CloseLong => "CLOSE LONG".to_string(),
            TradeAction::CloseShort => "CLOSE SHORT".to_string(),
            TradeAction::Hold => "HOLD".to_string(),
        }
    }
}

impl DecisionMaker {
    pub fn new(client: Client) -> Self {
        Self { 
            client, 
            ds_key: env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
            ds_url: env::var("DEEPSEEK_BASE_URL").unwrap_or("https://api.deepseek.com".to_string()),
            strategy_version: env::var("STRATEGY_VERSION").unwrap_or("v6.0-Deep-Reasoning".to_string()),
        }
    }

    pub async fn analyze(&self, state: &MarketState, memories: &[String], position_info: &str, max_leverage: f64) -> Result<AiDecision> {
        if self.ds_key.is_empty() {
            return Err(anyhow!("DeepSeek API Key missing. Check .env"));
        }

        let memory_text = if memories.is_empty() {
            "No historical similarity found.".to_string()
        } else {
            memories.join("\n")
        };

        let position_state_str = if position_info.contains("No active positions") { 
            "FLAT (No Position)".to_string() 
        } else { 
            format!("INVESTED (Holding Position)\nDetails: {}", position_info) 
        };

        // [New] 计算 ATR 占比 (波动率百分比)
        let atr_pct = if state.price > 0.0 {
            (state.indicators.atr_14 / state.price) * 100.0
        } else {
            0.0
        };

        info!("🧠 [DeepSeek Reasoner] Ingesting Full Context (ATR: {:.2}%)...", atr_pct);

        // [UPGRADE] System Prompt: CIO Edition (No Bias, Friction Aware, ATR Driven)
        let system_prompt = r#"You are a seasoned Crypto Hedge Fund CIO powered by DeepSeek-R1. 
Your goal is to maximize Alpha while strictly managing Risk of Ruin.

### CORE PHILOSOPHY:
1. **Trend Follower**: We trade with the trend (EMA20/50), not against it.
2. **Friction Averse**: Trading costs money (Fees + Slippage). DO NOT flip positions (Close -> Open) unless the signal reversal is STRONG.
3. **Data-Driven**: Your feelings don't matter. Only Price, Volume, and Volatility (ATR) matter.
4. **History Rhymes**: Use the RAG Memory. If a setup failed before ("PAST MISTAKE"), DO NOT repeat it.

### TASK:
Analyze the provided Market Snapshot, Position, and Memories. Output a JSON decision.

### RISK MANAGEMENT RULES (STRICT):
- **Stop Loss (SL)**: MUST be calculated based on volatility. Typically 1.5x - 3.0x ATR. 
  - If Volatility is HIGH, widen SL to avoid noise.
  - If Volatility is LOW, tighten SL.
- **Take Profit (TP)**: Aim for >1.5 Risk-Reward Ratio.
- **Confidence**: If the signal is weak, output action: "HOLD".

### OUTPUT FORMAT (JSON ONLY - NO COMMENTARY OUTSIDE JSON):
{
  "action": "BUY" | "SELL" | "CLOSE_LONG" | "CLOSE_SHORT" | "HOLD",
  "reason": "Concise reasoning citing specific indicators (e.g. 'RSI div', 'Price > EMA20')...",
  "tp": 0.0, // Target Profit (Decimal, e.g. 0.06 for 6%)
  "sl": 0.0, // Stop Loss (Decimal, e.g. 0.02 for 2%)
  "leverage": 1, // Integer, max constraint applies
  "win_rate": 0.0, // Estimated probability (0.0-1.0) based on signal quality & memory match
  "risk_reward_ratio": 0.0 // Expected Payoff (e.g. 2.5)
}"#;

        // [UPGRADE] User Prompt: Injected ATR Context
        let user_prompt = format!(
            r#"
=== 1. MARKET DATA SNAPSHOT ===
{}

[VOLATILITY INTEL]
Current ATR (1H): {:.2}% of Price. 
(Normal volatility is ~0.5%. If higher, expect whipsaws.)

=== 2. CURRENT POSITION ===
{}

=== 3. HISTORICAL MEMORY (RAG) ===
{}

=== 4. CONSTRAINTS ===
Max Leverage: {}x
"#,
            state, atr_pct, position_state_str, memory_text, max_leverage as u32
        );

        // 打印 Prompt 供调试
        info!("\n================ [DEBUG] LLM FULL PROMPT START ================\n{}\n\n[USER MESSAGE]:\n{}\n================ [DEBUG] LLM FULL PROMPT END ================", system_prompt, user_prompt);

        let response = self.call_llm("deepseek-reasoner", &self.ds_url, &self.ds_key, system_prompt, &user_prompt, 0.1).await
            .context("DeepSeek Analysis Failed")?;
        
        self.parse_decision(&response, max_leverage)
    }

    fn clean_reasoning_content(&self, raw: &str) -> String {
        let mut clean = raw.to_string();
        if let Some(start) = clean.find("<think>") {
            if let Some(end) = clean.find("</think>") {
                if end + 8 <= clean.len() {
                    let mut before = clean[..start].to_string();
                    let after = clean[end + 8..].to_string();
                    before.push_str(&after);
                    clean = before;
                }
            }
        }
        clean
    }

    fn extract_json(&self, raw_response: &str) -> Result<Value> {
        let cleaned_response = self.clean_reasoning_content(raw_response);
        if let Ok(v) = serde_json::from_str::<Value>(&cleaned_response) { return Ok(v); }
        if let Some(start) = cleaned_response.find("```json") {
            if let Some(_end) = cleaned_response[start..].find("```") { 
                 let after_start = &cleaned_response[start+7..];
                 if let Some(real_end) = after_start.find("```") {
                     let json_str = &after_start[..real_end];
                     if let Ok(v) = serde_json::from_str::<Value>(json_str) { return Ok(v); }
                 }
            }
        }
        if let Some(start) = cleaned_response.find('{') {
            if let Some(end) = cleaned_response.rfind('}') {
                if end > start {
                    let json_str = &cleaned_response[start..=end];
                    if let Ok(v) = serde_json::from_str::<Value>(json_str) { return Ok(v); }
                }
            }
        }
        Err(anyhow!("Failed to extract JSON from response"))
    }

    async fn call_llm(&self, model: &str, base_url: &str, key: &str, sys_prompt: &str, user_prompt: &str, temp: f64) -> Result<String> {
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let body = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": sys_prompt}, 
                {"role": "user", "content": user_prompt}
            ],
            "temperature": temp, 
        });

        for _attempt in 1..=3 {
            let resp_result = self.client.post(&url)
                .header("Authorization", format!("Bearer {}", key))
                .json(&body)
                .send()
                .await;

            match resp_result {
                Ok(r) => {
                    if !r.status().is_success() {
                        let err = r.text().await.unwrap_or_default();
                        warn!("⚠️ {} API Error: {}", model, err);
                        sleep(Duration::from_secs(3)).await;
                        continue;
                    }
                    let content_str = r.text().await.unwrap_or_default();
                    if let Ok(json_res) = serde_json::from_str::<Value>(&content_str) {
                        if let Some(content) = json_res["choices"][0]["message"]["content"].as_str() {
                            return Ok(content.to_string());
                        }
                    }
                },
                Err(e) => {
                    warn!("⚠️ {} Network Error: {}", model, e);
                    sleep(Duration::from_secs(3)).await;
                }
            }
        }
        Err(anyhow!("{} Failed after 3 attempts", model))
    }

    fn parse_decision(&self, content: &str, max_leverage: f64) -> Result<AiDecision> {
        let decision_json = self.extract_json(content)?;
        let action_str = decision_json["action"].as_str().unwrap_or("HOLD").to_uppercase();
        let action = match action_str.as_str() {
            "BUY" | "OPEN_LONG" => TradeAction::Buy,
            "SELL" | "OPEN_SHORT" => TradeAction::Sell,
            "CLOSE_LONG" => TradeAction::CloseLong,
            "CLOSE_SHORT" => TradeAction::CloseShort,
            _ => TradeAction::Hold,
        };

        let mut tp_pct = decision_json["tp"].as_f64().unwrap_or(0.04);
        let mut sl_pct = decision_json["sl"].as_f64().unwrap_or(0.02);
        
        // [单位换算] 唯一的容错逻辑：防止 AI 把 5% 写成 5.0
        if tp_pct > 1.0 { tp_pct /= 100.0; }
        if sl_pct > 1.0 { sl_pct /= 100.0; }
        
        // 兜底极小值 (防止 API 报错说价格太近)
        if (matches!(action, TradeAction::Buy) || matches!(action, TradeAction::Sell)) && tp_pct < 0.005 {
            tp_pct = 0.008; 
        }

        let raw_leverage = decision_json["leverage"].as_u64().unwrap_or(1) as u32;
        let leverage = if raw_leverage > max_leverage as u32 { max_leverage as u32 } else if raw_leverage < 1 { 1 } else { raw_leverage };

        let p = decision_json["win_rate"].as_f64().unwrap_or(0.5);
        let b = decision_json["risk_reward_ratio"].as_f64().unwrap_or(1.5);
        let kelly_fraction = if b > 0.0 { p - ((1.0 - p) / b) } else { 0.0 };
        let (final_action, final_kelly) = if kelly_fraction <= 0.0 && (matches!(action, TradeAction::Buy) || matches!(action, TradeAction::Sell)) {
            warn!("⚠️ Kelly negative ({:.2}). Force HOLD. (WinRate={:.2}, Odds={:.2})", kelly_fraction, p, b);
            (TradeAction::Hold, 0.0)
        } else {
            (action, kelly_fraction.max(0.0))
        };

        Ok(AiDecision {
            action: final_action,
            reason: decision_json["reason"].as_str().unwrap_or("No reason").to_string(),
            tp_pct,
            sl_pct,
            leverage,
            win_rate: p,
            risk_reward_ratio: b,
            kelly_fraction: final_kelly,
            strategy_version: self.strategy_version.clone(),
        })
    }
}

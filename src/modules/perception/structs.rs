use serde::{Serialize, Deserialize};
// [修复] 删除了多余的 use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Indicators {
    pub rsi_14: f64,
    pub atr_14: f64,
    pub ema_20: f64,
    pub ema_50: f64,
    pub trend_signal: String, 
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketState {
    pub timestamp: i64,
    pub symbol: String,
    pub price: f64,
    pub indicators: Indicators,
    pub funding_rate: f64,
    pub open_interest: f64,
    pub reddit_sentiment: String,
    pub news_sentiment: String,
}

impl MarketState {
    /// [核心升级] 生成用于 RAG 检索的原始全息数据 (JSON)
    /// 这将被发送给 Embedding 模型 (2560维)。
    pub fn to_context_string(&self) -> String {
        // 1. 技术面叙事
        let rsi_desc = if self.indicators.rsi_14 > 70.0 { "Overbought" } 
                      else if self.indicators.rsi_14 < 30.0 { "Oversold" } 
                      else { "Neutral" };
        
        let ema_desc = if self.price > self.indicators.ema_20 { "Above short-term trend" } else { "Below short-term trend" };

        let funding_pct = self.funding_rate * 100.0;
        let funding_desc = if funding_pct > 0.01 { "High Positive Funding (Longs paying Shorts)" }
                          else if funding_pct < -0.01 { "High Negative Funding (Shorts paying Longs)" }
                          else { "Neutral Funding" };

        // 3. 组合成自然语言段落
        format!(
            "Market Context for {}:\n\
            - Price Action: ${:.2}, Trend is {}. Price is {}.\n\
            - Momentum: RSI is {:.2} ({}), Volatility (ATR) is {:.2}.\n\
            - Derivatives: {}, Open Interest is {:.0}.\n\
            - Market Sentiment Summary:\n\
            [News Headlines]: {}\n\
            [Social Discussion]: {}",
            self.symbol,
            self.price, self.indicators.trend_signal, ema_desc,
            self.indicators.rsi_14, rsi_desc, self.indicators.atr_14,
            funding_desc, self.open_interest,
            self.news_sentiment.chars().take(2000).collect::<String>(), 
            self.reddit_sentiment.chars().take(2000).collect::<String>()
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Kline {
    pub open_time: i64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
}

impl Kline {
    pub fn close_price(&self) -> f64 {
        self.close.parse().unwrap_or(0.0)
    }
    pub fn high_price(&self) -> f64 {
        self.high.parse().unwrap_or(0.0)
    }
    pub fn low_price(&self) -> f64 {
        self.low.parse().unwrap_or(0.0)
    }
}
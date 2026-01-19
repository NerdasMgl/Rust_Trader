use reqwest::Client;
use anyhow::{Result, Context};
use serde_json::Value;
use super::structs::{Kline, MarketState};
use super::math::TechnicalAnalysis;
use chrono::Utc;

pub struct MarketDataFetcher {
    client: Client,
    base_url: String,
}

impl MarketDataFetcher {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            base_url: "https://www.okx.com".to_string(),
        }
    }

    /// 获取 K 线数据 (1小时级别)
    pub async fn fetch_klines(&self, symbol: &str) -> Result<Vec<Kline>> {
        let url = format!("{}/api/v5/market/candles", self.base_url);
        let params = [
            ("instId", symbol),
            ("bar", "1H"),
            ("limit", "100"),
        ];

        let resp: Value = self.client.get(&url)
            .query(&params)
            .send()
            .await?
            .json()
            .await?;

        let data = resp["data"].as_array().context("No data in OKX response")?;

        let mut klines: Vec<Kline> = data.iter().map(|raw| Kline {
            open_time: raw[0].as_str().unwrap_or("0").parse::<i64>().unwrap_or(0),
            open: raw[1].as_str().unwrap_or("0").to_string(),
            high: raw[2].as_str().unwrap_or("0").to_string(),
            low: raw[3].as_str().unwrap_or("0").to_string(),
            close: raw[4].as_str().unwrap_or("0").to_string(),
            volume: raw[5].as_str().unwrap_or("0").to_string(),
        }).collect();

        klines.reverse(); 

        Ok(klines)
    }

    pub async fn fetch_funding_rate(&self, symbol: &str) -> Result<f64> {
        let url = format!("{}/api/v5/public/funding-rate", self.base_url);
        let resp: Value = self.client.get(&url)
            .query(&[("instId", symbol)])
            .send()
            .await?
            .json()
            .await?;
        
        let rate = resp["data"][0]["fundingRate"]
            .as_str()
            .unwrap_or("0.0")
            .parse::<f64>()?;
        Ok(rate)
    }

    pub async fn fetch_open_interest(&self, symbol: &str) -> Result<f64> {
        let url = format!("{}/api/v5/public/open-interest", self.base_url);
        let resp: Value = self.client.get(&url)
            .query(&[("instId", symbol)])
            .send()
            .await?
            .json()
            .await?;

        let oi = resp["data"][0]["oi"]
            .as_str()
            .unwrap_or("0.0")
            .parse::<f64>()?;
        Ok(oi)
    }

    pub async fn snapshot(&self, symbol: &str, reddit_sentiment: String, news_sentiment: String) -> Result<MarketState> {
        // [核心修复] 使用 tokio::join! 并行请求，而不是 try_join!
        // 这样即使资金费率或OI获取失败，只要K线还在，我们就能继续交易，不至于全盘崩溃
        let (klines_res, funding_res, oi_res) = tokio::join!(
            self.fetch_klines(symbol),
            self.fetch_funding_rate(symbol),
            self.fetch_open_interest(symbol)
        );

        // K线是必须的，如果失败则抛出错误
        let klines = klines_res?;
        
        // 次要数据如果失败，降级为默认值 0.0，不阻断流程
        let funding_rate = funding_res.unwrap_or_else(|_| 0.0);
        let open_interest = oi_res.unwrap_or_else(|_| 0.0);

        let current_price = klines.last().context("No klines fetched")?.close_price();
        let indicators = TechnicalAnalysis::analyze(&klines);

        Ok(MarketState {
            timestamp: Utc::now().timestamp(),
            symbol: symbol.to_string(),
            price: current_price,
            indicators,
            funding_rate,
            open_interest,
            reddit_sentiment,
            news_sentiment,
        })
    }
}
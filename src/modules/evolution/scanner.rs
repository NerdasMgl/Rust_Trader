use std::sync::Arc;
use sqlx::PgPool;
use anyhow::Result;
use crate::modules::perception::MarketDataFetcher;
use crate::modules::brain::MemorySystem;
use tracing::info;
use serde_json::json;

pub struct OpportunityScanner {
    pool: PgPool,
    fetcher: Arc<MarketDataFetcher>,
    memory: Arc<MemorySystem>,
}

impl OpportunityScanner {
    pub fn new(pool: PgPool, fetcher: Arc<MarketDataFetcher>, memory: Arc<MemorySystem>) -> Self {
        Self { pool, fetcher, memory }
    }

    pub async fn scan_missed_opportunities(&self, symbol: &str) -> Result<()> {
        let klines = self.fetcher.fetch_klines(symbol).await?;
        
        // [修复 1] 需要至少 3 根 K 线才能回溯到暴涨"前"的状态
        if klines.len() < 3 { return Ok(()); }

        let current = klines.last().unwrap();
        let prev = &klines[klines.len() - 2]; 
        // 核心修正：取暴涨前的那根 K 线 (pre_pump) 作为上下文
        // 这样 AI 记住的是"暴涨前的宁静"，而不是"暴涨后的高位"
        let pre_pump = &klines[klines.len() - 3]; 

        let prev_close = prev.close_price();
        if prev_close == 0.0 { return Ok(()); }
        
        // 计算最近一小时的涨幅 (判定是否发生了 Pump)
        let price_change_pct = (current.close_price() - prev_close) / prev_close;

        // 阈值：涨幅超过 5% 视为机会
        if price_change_pct > 0.05 { 
            // [修复 2] 扩大查询范围到 12 小时
            // 如果过去 12 小时内有买入，说明我们可能已经在车上了，不算踏空
            let recent_trades: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM trade_logs 
                 WHERE symbol = $1 AND direction = 'buy' 
                 AND created_at > NOW() - INTERVAL '12 hours'"
            )
            .bind(symbol)
            .fetch_one(&self.pool)
            .await?;

            if recent_trades == 0 {
                // [修复 1] 构建暴涨"前"的上下文
                let simplified_context = json!({
                    "symbol": symbol,
                    "price_before_pump": pre_pump.close_price(),
                    "indicators": {
                        "note": "Snapshot taken 1h BEFORE the 5% pump",
                        "volume": pre_pump.volume, // 记录暴涨前的量能特征
                        "structure": "Potential accumulation"
                    }
                });

                // [修复 3] 结论前置
                let lesson = format!(
                    "💡 OPPORTUNITY: Price pumped {:.2}% shortly after this state. Look for these signs!\n\nPRE-PUMP CONTEXT: {}",
                    price_change_pct * 100.0, simplified_context.to_string()
                );
                
                info!("🧬 Scanner found FOMO for {}: Pumped {:.2}%", symbol, price_change_pct * 100.0);
                self.memory.store_memory("missed_opportunity", &lesson).await?;
            }
        }

        Ok(())
    }
}
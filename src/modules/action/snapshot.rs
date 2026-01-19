use sqlx::PgPool;
use anyhow::Result;
use serde_json::json;
use crate::modules::perception::MarketState;
use std::env;

pub struct LogManager {
    pool: PgPool,
}

impl LogManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // [修改] 接收 initial_margin 参数
    pub async fn log_trade(&self, symbol: &str, direction: &str, state: &MarketState, order_id: &str, initial_margin: f64) -> Result<()> {
        let strategy_ver = env::var("STRATEGY_VERSION").unwrap_or("unknown".to_string());

        sqlx::query(
            "INSERT INTO trade_logs (symbol, direction, context_snapshot, okx_order_id, strategy_version, initial_margin) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(symbol)
        .bind(direction)
        .bind(json!(state))
        .bind(order_id) 
        .bind(strategy_ver)
        .bind(initial_margin) // 记录初始投入
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
use std::sync::Arc;
use sqlx::PgPool;
use anyhow::Result;
use crate::modules::action::TradeExecutor;
use tracing::{info, warn};

pub struct PnlMonitor {
    pool: PgPool,
    executor: Arc<TradeExecutor>,
}

impl PnlMonitor {
    pub fn new(pool: PgPool, executor: Arc<TradeExecutor>) -> Self {
        Self { pool, executor }
    }

    pub async fn sync_realized_pnl(&self) -> Result<()> {
        // [修复] 添加显式类型注解，解决编译器无法推断 bills 类型的问题
        let bills: Vec<crate::modules::action::executor::PnlRecord> = match self.executor.fetch_recent_pnl().await {
            Ok(b) => b,
            Err(e) => {
                warn!("Failed to fetch bills from OKX: {}", e);
                return Ok(());
            }
        };

        if bills.is_empty() {
            return Ok(());
        }

        info!("📥 Synced {} pnl records. Updating DB...", bills.len());

        for bill in bills {
            let net_pnl = bill.pnl + bill.fee;

            if bill.ord_id.is_empty() {
                continue;
            }

            let result = sqlx::query(
                "UPDATE trade_logs 
                 SET realized_pnl = $1 
                 WHERE okx_order_id = $2 AND realized_pnl IS NULL"
            )
            .bind(net_pnl)
            .bind(&bill.ord_id)
            .execute(&self.pool)
            .await?;

            if result.rows_affected() > 0 {
                info!("💰 PnL Updated for Order {}: ${:.2}", bill.ord_id, net_pnl);
            }
        }

        Ok(())
    }
}
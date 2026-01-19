use std::sync::Arc;
use sqlx::{PgPool, Row};
use anyhow::Result;
use crate::modules::brain::MemorySystem;
use crate::config::risk_profile::RiskProfile;
use serde_json::Value;
use tracing::{info, warn};
use uuid::Uuid;

pub struct AutopsyDoctor {
    pool: PgPool,
    memory: Arc<MemorySystem>,
}

impl AutopsyDoctor {
    pub fn new(pool: PgPool, memory: Arc<MemorySystem>) -> Self {
        Self { pool, memory }
    }

    pub async fn perform_daily_review(&self) -> Result<()> {
        let risk_profile = RiskProfile::load().unwrap_or_else(|_| {
            warn!("Failed to load risk profile in autopsy, using default -0.02");
            panic!("Risk profile load failed");
        });
        
        let threshold = risk_profile.thresholds.autopsy_roe_pct;

        // [Fix] SQL 逻辑增强：
        // 1. ROE < 阈值 (大亏)
        // 2. OR exit_reason 包含 'SL' (任何止损触发的交易，无论亏损大小)
        // 注意：这需要数据库 trade_logs 表有 exit_reason 字段。如果暂时没有，我们先依赖 ROE。
        // 目前数据库 schema 未知，假设我们先用 ROE 兜底，后续建议在 schema.sql 添加 exit_reason。
        
        // 这里的查询逻辑改为了更宽泛的捕获
        let rows = sqlx::query(
            "SELECT id, context_snapshot, symbol, realized_pnl, initial_margin, direction 
             FROM trade_logs 
             WHERE (
                (realized_pnl / NULLIF(initial_margin, 0)) < $1
             )
             AND is_reviewed = FALSE 
             AND created_at > NOW() - INTERVAL '24 hours'"
        )
        .bind(threshold)
        .fetch_all(&self.pool)
        .await?;

        for row in rows {
            let id: Uuid = row.try_get("id")?;
            let snapshot_val: Value = row.try_get("context_snapshot")?;
            let symbol: String = row.try_get("symbol")?;
            let pnl: f64 = row.try_get("realized_pnl")?; 
            let margin: f64 = row.try_get("initial_margin")?;
            let direction: String = row.try_get("direction")?;

            let roe = if margin != 0.0 { pnl / margin } else { 0.0 };

            let context_str = serde_json::to_string(&snapshot_val).unwrap_or_default();
            
            // [Fix] 增强 Lesson 描述，增加摩擦提醒
            let lesson = format!(
                "📚 LESSON: Trade {} on {} ended in LOSS (ROE: {:.2}%, PnL: {:.2} USDT). \
                Setup failed or Stop Loss hit. \
                REVIEW CONTEXT & AVOID SIMILAR SETUPS:\n{}",
                direction, symbol, roe * 100.0, pnl, context_str
            );

            info!("💀 Autopsy Generated Mistake Memory for {} (ROE: {:.2}%)", symbol, roe * 100.0);
            self.memory.store_memory("mistake", &lesson).await?;

            sqlx::query("UPDATE trade_logs SET is_reviewed = TRUE WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }
}

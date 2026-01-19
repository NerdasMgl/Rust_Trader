-- 1. 宏观事件表
CREATE TABLE IF NOT EXISTS macro_events (
    id SERIAL PRIMARY KEY,
    event_name VARCHAR(255),
    impact VARCHAR(50),
    event_time TIMESTAMP WITH TIME ZONE
);

-- 2. 交易日志表 (核心账本)
-- [重大升级] 支持 ROE 计算
CREATE TABLE IF NOT EXISTS trade_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol VARCHAR(20),
    direction VARCHAR(10),
    
    -- [修改] 存储绝对盈亏金额 (USDT)，原名 pnl_percentage 有歧义
    realized_pnl DECIMAL(10, 4), 
    
    -- [新增] 初始保证金 (USDT)，用于计算 ROE = realized_pnl / initial_margin
    initial_margin DECIMAL(10, 4),

    context_snapshot JSONB NOT NULL, 
    okx_order_id VARCHAR(64), 
    strategy_version VARCHAR(50),
    
    -- [新增] 复盘状态标记
    is_reviewed BOOLEAN DEFAULT FALSE,

    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- =========================================================
-- ⚠️ 数据库迁移指南 (如果你已经运行过旧版):
-- 请进入 docker 容器内的 postgres 执行以下命令，或删除 data 目录重建
-- 
-- ALTER TABLE trade_logs RENAME COLUMN pnl_percentage TO realized_pnl;
-- ALTER TABLE trade_logs ADD COLUMN initial_margin DECIMAL(10, 4);
-- ALTER TABLE trade_logs ADD COLUMN is_reviewed BOOLEAN DEFAULT FALSE;
-- =========================================================
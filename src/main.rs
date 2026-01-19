mod config;
mod database;
mod utils;
mod modules;

use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::time::sleep;
use tracing::{info, error, warn};
use sqlx::postgres::{PgPoolOptions, PgPool};
use dotenvy::dotenv;
use std::env;
use std::fs;
use chrono::Local;
use dashmap::DashMap;

use crate::config::risk_profile::RiskProfile;
use crate::utils::http_client::HttpClientFactory;
use crate::utils::notifier::{DingTalkNotifier, PositionReportItem};
use crate::modules::perception::{MarketDataFetcher, NewsSentinel, RedditSentinel, OkxWsClient};
use crate::modules::brain::{MemorySystem, DecisionMaker, llm::TradeAction};
use crate::modules::action::{TradeExecutor, LogManager};
use crate::modules::evolution::{AutopsyDoctor, OpportunityScanner, PnlMonitor};

async fn calculate_position_size_kelly(
    equity: f64, 
    available_equity: f64, 
    kelly_fraction: f64, 
    max_pct_limit: f64, 
    leverage: u32, 
    price: f64, 
    symbol: &str, 
    executor: &TradeExecutor
) -> f64 {
    let safe_kelly = kelly_fraction * 0.5;
    let actual_pct = if safe_kelly > max_pct_limit { max_pct_limit } else if safe_kelly < 0.01 { 0.01 } else { safe_kelly };
    
    let face_val = executor.get_face_value(symbol).await;
    let min_sz = executor.get_min_size(symbol).await; 

    if price * face_val == 0.0 { return 0.0; }

    let min_cost_margin = (price * face_val * min_sz) / (leverage as f64);
    
    if available_equity < min_cost_margin {
        warn!("💰 资金不足: {} 最小 {}张合约需 ${:.2} (杠杆{}x)，但可用余额仅 ${:.2}。跳过。", 
            symbol, min_sz, min_cost_margin, leverage, available_equity);
        return 0.0; 
    }

    let mut margin_amount = equity * actual_pct; 
    
    if margin_amount > available_equity {
        margin_amount = available_equity * 0.95; 
    }

    let notional_value = margin_amount * (leverage as f64);
    let mut contracts = notional_value / (price * face_val);
    
    if contracts < min_sz {
        contracts = min_sz;
    }
    
    let final_cost = (contracts * price * face_val) / (leverage as f64);
    if final_cost > available_equity {
        return 0.0;
    }
    
    contracts
}

async fn init_database(pool: &PgPool) -> anyhow::Result<()> {
    info!("Checking database schema...");
    let schema_path = "src/database/schema.sql";
    match fs::read_to_string(schema_path) {
        Ok(sql) => {
            let statements: Vec<&str> = sql.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            for stmt in statements {
                if stmt.is_empty() { continue; }
                if let Err(e) = sqlx::query(stmt).execute(pool).await {
                    if !e.to_string().contains("already exists") && !e.to_string().contains("duplicate column") {
                        warn!("Schema warning: {}", e);
                    }
                }
            }
            info!("Database schema check complete.");
        }
        Err(e) => warn!("Could not read schema: {}", e),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    info!("Starting Rust Trader V6.0 (HK Direct Mode - Upgraded)...");

    // 1. 基础设施初始化
    let risk_profile = RiskProfile::load().expect("Failed to load risk config");
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let qdrant_url = env::var("QDRANT_URL").unwrap_or("http://localhost:6334".to_string()); 
    let max_drawdown = env::var("MAX_DRAWDOWN_LIMIT").unwrap_or("0.10".to_string()).parse::<f64>().unwrap_or(0.10);

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&db_url)
        .await
        .map_err(|e| {
            error!("CRITICAL: DB Connection Failed! Is Docker running?");
            e
        })?;

    init_database(&pool).await?;

    // 2. 模块初始化
    let std_client = HttpClientFactory::create()?;
    let direct_client = HttpClientFactory::create_direct()?;
    
    let notifier = Arc::new(DingTalkNotifier::new(direct_client.clone()));
    let fetcher = Arc::new(MarketDataFetcher::new(std_client.clone()));
    let news_sentinel = Arc::new(NewsSentinel::new(std_client.clone()));
    let reddit_sentinel = Arc::new(RedditSentinel::new(std_client.clone()));
    
    let memory_sys = Arc::new(MemorySystem::new(qdrant_url, direct_client.clone()).expect("Failed to init Qdrant client"));
    if let Err(e) = memory_sys.init().await {
        error!("Failed to initialize Qdrant collection: {}", e);
    }

    let brain = Arc::new(DecisionMaker::new(direct_client.clone()));
    let executor = Arc::new(TradeExecutor::new(std_client.clone()));
    let logger = Arc::new(LogManager::new(pool.clone()));
    let autopsy = AutopsyDoctor::new(pool.clone(), memory_sys.clone());
    let scanner = OpportunityScanner::new(pool.clone(), fetcher.clone(), memory_sys.clone());
    let pnl_monitor = PnlMonitor::new(pool.clone(), executor.clone());

    // 3. 交易所元数据同步
    if let Err(e) = executor.init_instruments_cache().await {
        error!("CRITICAL: Init instruments failed: {}. System cannot start.", e);
        return Err(e); 
    }

    // 4. 获取初始资金基准
    info!("💰 Establishing Risk Baseline...");
    let mut initial_capital = 0.0;
    for i in 1..=5 {
        match executor.fetch_account_summary().await {
            Ok(cap) => {
                initial_capital = cap.total_equity;
                info!("✅ Risk Baseline Set: ${:.2}", initial_capital);
                break;
            }
            Err(e) => {
                warn!("⚠️ Failed to fetch capital (Attempt {}/5): {}. Retrying...", i, e);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }

    if initial_capital == 0.0 {
        let msg = "🔥 CRITICAL: Could not fetch Initial Capital!";
        error!("{}", msg);
        notifier.send_text(msg).await;
    } else {
        let startup_positions = match executor.fetch_positions().await {
            Ok(p) => p,
            Err(e) => { warn!("Failed to fetch positions on startup: {}", e); vec![] }
        };

        let report_items: Vec<PositionReportItem> = startup_positions.iter().map(|p| PositionReportItem {
            symbol: p.symbol.clone(),
            side: p.side.clone(),
            notional_usdt: p.notional_usd,
            margin_usdt: p.margin_usd,
            upl: p.upl,
            leverage: p.leverage,
        }).collect();

        notifier.send_startup_report(
            initial_capital, 
            &Local::now().format("%Y-%m-%d %H:%M:%S").to_string(), 
            report_items
        ).await;
    }

    // 5. 启动 WebSocket
    let price_cache = Arc::new(DashMap::new());
    let ws_client = OkxWsClient::new(price_cache.clone());
    let symbols_clone = risk_profile.allowed_symbols.clone();
    tokio::spawn(async move {
        ws_client.run(symbols_clone).await;
    });

    // 6. 循环变量
    let mut last_evolution_time = Instant::now();
    let mut last_report_time = Instant::now();
    
    let evolution_interval = Duration::from_secs(risk_profile.timing.evolution_sec);
    let report_interval = Duration::from_secs(3600); 
    let base_rest_interval = Duration::from_secs(risk_profile.timing.cycle_rest_sec);

    info!("✅ System initialized. Loop starting...");

    loop {
        info!("==================== 📊 SYSTEM STATUS ====================");
        
        let (equity, available_equity) = match executor.fetch_account_summary().await {
            Ok(balance) => (balance.total_equity, balance.available_balance),
            Err(e) => { error!("Failed to fetch balance: {}", e); (0.0, 0.0) }
        };

        if initial_capital > 0.0 && equity > 0.0 {
            let drawdown = (initial_capital - equity) / initial_capital;
            if drawdown > max_drawdown {
                let alert = format!("🔥 严重警告: 最大回撤触发! ({:.2}%). 系统已暂停.", drawdown * 100.0);
                error!("{}", alert);
                notifier.send_text(&alert).await;
            }
        }

        let all_positions = match executor.fetch_positions().await {
            Ok(p) => p, 
            Err(e) => { error!("Failed to fetch positions: {}", e); vec![] }
        };

        if last_report_time.elapsed() >= report_interval && equity > 0.0 {
            let total_pnl_pct = (equity - initial_capital) / initial_capital * 100.0;
            let report_items: Vec<PositionReportItem> = all_positions.iter().map(|p| PositionReportItem {
                symbol: p.symbol.clone(),
                side: p.side.clone(),
                notional_usdt: p.notional_usd,
                margin_usdt: p.margin_usd,
                upl: p.upl,
                leverage: p.leverage,
            }).collect();
            notifier.send_status_report(equity, total_pnl_pct, report_items).await;
            last_report_time = Instant::now();
        }

        info!("==========================================================");

        let raw_reddit = match reddit_sentinel.analyze_sentiment().await {
            Ok(t) => t, Err(e) => format!("Error fetching Reddit: {}", e),
        };
        let raw_news = match news_sentinel.fetch_raw_headlines("GLOBAL").await {
            Ok(m) => m, Err(e) => format!("Error fetching News: {}", e),
        };

        info!("📰 Global Context Ready: News ({} chars), Reddit ({} chars)", raw_news.len(), raw_reddit.len());

        // [New] Dynamic Heartbeat variables
        let mut max_atr_pct = 0.0;

        for symbol in &risk_profile.allowed_symbols {
            info!("🔍 Analyzing {}...", symbol);

            let market_state_res = fetcher.snapshot(symbol, raw_reddit.clone(), raw_news.clone()).await;
            
            let mut market_state = match market_state_res {
                Ok(s) => s,
                Err(e) => {
                    error!("Fetch error for {}: {}", symbol, e);
                    continue; 
                }
            };

            // Calculate ATR % for heartbeat logic
            if market_state.price > 0.0 {
                let current_atr_pct = (market_state.indicators.atr_14 / market_state.price) * 100.0;
                if current_atr_pct > max_atr_pct {
                    max_atr_pct = current_atr_pct;
                }
            }

            if let Some(entry) = price_cache.get(symbol) {
                let (ws_price, ts) = *entry.value();
                if ts.elapsed() < Duration::from_secs(60) {
                    market_state.price = ws_price;
                } else {
                    warn!("⚠️ WS Data Stale for {} ({:?} ago). Falling back to REST price.", symbol, ts.elapsed());
                }
            }

            let ctx_str = market_state.to_context_string();
            info!("\n================ [DEBUG] EMBEDDING INPUT START ================\n{}\n================ [DEBUG] EMBEDDING INPUT END ================", ctx_str);

            let memories = memory_sys.recall_memories(&ctx_str).await.unwrap_or_default();

            let long_pos = all_positions.iter().find(|p| p.symbol == *symbol && p.side == "long" && p.size > 0.0);
            let short_pos = all_positions.iter().find(|p| p.symbol == *symbol && p.side == "short" && p.size > 0.0);
            
            let pos_info = match (long_pos, short_pos) {
                (Some(l), Some(s)) => format!("Long: {} (PnL ${}), Short: {} (PnL ${})", l.size, l.upl, s.size, s.upl),
                (Some(l), None) => format!("Long: {} (PnL ${})", l.size, l.upl),
                (None, Some(s)) => format!("Short: {} (PnL ${})", s.size, s.upl),
                (None, None) => "No active positions".to_string(),
            };

            match brain.analyze(&market_state, &memories, &pos_info, risk_profile.max_leverage).await {
                Ok(mut decision) => {
                    info!("[{}] 🎯 Decision: {:?} (Reason: {})", symbol, decision.action, decision.reason);

                    match decision.action {
                        TradeAction::Buy | TradeAction::Sell => {
                            // [Fix] Win Rate Soft Cap
                            // 强制将胜率限制在 0.75 以内，防止凯利公式全仓梭哈
                            if decision.win_rate > 0.75 {
                                warn!("⚠️ AI WinRate ({:.2}) capped to 0.75 for safety.", decision.win_rate);
                                decision.win_rate = 0.75;
                                // 重新计算 kelly fraction
                                let p = decision.win_rate;
                                let b = decision.risk_reward_ratio;
                                decision.kelly_fraction = if b > 0.0 { p - ((1.0 - p) / b) } else { 0.0 };
                            }

                            let qty = calculate_position_size_kelly(
                                equity, available_equity, decision.kelly_fraction, risk_profile.max_order_size_pct, 
                                decision.leverage, market_state.price, symbol, &executor
                            ).await;

                            if qty > 0.0 {
                                let side = if let TradeAction::Buy = decision.action { "buy" } else { "sell" };
                                let pos_side = if let TradeAction::Buy = decision.action { "long" } else { "short" };
                                
                                for attempt in 1..=10 {
                                    match executor.execute_order(symbol, side, pos_side, qty, market_state.price, decision.tp_pct, decision.sl_pct, Some(decision.leverage)).await {
                                        Ok(res) => {
                                            info!("✅ [{}] Order Sent: {}", symbol, res.order_id);
                                            let face_val = executor.get_face_value(symbol).await;
                                            let initial_margin = (qty * market_state.price * face_val) / (decision.leverage as f64);
                                            let _ = logger.log_trade(symbol, side, &market_state, &res.order_id, initial_margin).await;
                                            notifier.send_trade_signal(
                                                symbol, side, qty, market_state.price, 
                                                &decision.reason, decision.tp_pct, decision.sl_pct
                                            ).await;
                                            break; 
                                        },
                                        Err(e) => {
                                            warn!("❌ [{}] Order Failed (Attempt {}/10): {}. Retrying in 1s...", symbol, attempt, e);
                                            sleep(Duration::from_secs(1)).await;
                                        }
                                    }
                                }
                            }
                        },
                        TradeAction::CloseLong => {
                            if let Some(pos) = long_pos {
                                for attempt in 1..=10 {
                                    if let Ok(_) = executor.execute_order(symbol, "sell", "long", pos.size, market_state.price, 0.0, 0.0, None).await {
                                        info!("Long Closed: {}", symbol);
                                        notifier.send_trade_signal(symbol, "CLOSE LONG", pos.size, market_state.price, &decision.reason, 0.0, 0.0).await;
                                        break;
                                    } else {
                                        warn!("❌ Close Long Failed (Attempt {}/10). Retrying...", attempt);
                                        sleep(Duration::from_secs(1)).await;
                                    }
                                }
                            }
                        },
                        TradeAction::CloseShort => {
                            if let Some(pos) = short_pos {
                                for attempt in 1..=10 {
                                    if let Ok(_) = executor.execute_order(symbol, "buy", "short", pos.size, market_state.price, 0.0, 0.0, None).await {
                                        info!("Short Closed: {}", symbol);
                                        notifier.send_trade_signal(symbol, "CLOSE SHORT", pos.size, market_state.price, &decision.reason, 0.0, 0.0).await;
                                        break;
                                    } else {
                                        warn!("❌ Close Short Failed (Attempt {}/10). Retrying...", attempt);
                                        sleep(Duration::from_secs(1)).await;
                                    }
                                }
                            }
                        },
                        TradeAction::Hold => {}
                    }
                },
                Err(e) => error!("[{}] Brain Error: {}", symbol, e),
            }
            sleep(Duration::from_millis(500)).await;
        }

        if last_evolution_time.elapsed() > evolution_interval {
            info!("🧬 Running Evolution...");
            if let Err(e) = pnl_monitor.sync_realized_pnl().await { error!("PnL Sync Failed: {}", e); }
            let _ = autopsy.perform_daily_review().await;
            for symbol in &risk_profile.allowed_symbols { let _ = scanner.scan_missed_opportunities(symbol).await; }
            last_evolution_time = Instant::now();
        }

        // [New] Dynamic Sleep Logic
        // Base is 0.5% volatility (ATR). If vol is 1.0% (2x), sleep time halves.
        // Min sleep is 60s to prevent API spam.
        let dynamic_rest = if max_atr_pct > 0.0 {
            let volatility_ratio = max_atr_pct / 0.5; // normalized to 0.5%
            let adjusted_secs = (base_rest_interval.as_secs_f64() / volatility_ratio.max(0.5)).max(60.0);
            Duration::from_secs(adjusted_secs as u64)
        } else {
            base_rest_interval
        };

        info!("💤 Cycle done. Volatility: {:.2}%. Sleeping {}s...", max_atr_pct, dynamic_rest.as_secs());
        sleep(dynamic_rest).await;
    }
}

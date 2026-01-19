use serde::Deserialize;
use config::{Config, File};
use anyhow::Result;

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct TimingConfig {
    pub cycle_rest_sec: u64,
    pub evolution_sec: u64,
    pub symbol_gap_sec: u64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct IndicatorConfig {
    pub kline_interval: String,
    pub rsi_period: usize,
    pub atr_period: usize,
    pub ema_fast: usize,
    pub ema_slow: usize,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct ThresholdConfig {
    // [修改] 改名为 autopsy_roe_pct
    pub autopsy_roe_pct: f64,
    pub scanner_pump_pct: f64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct RiskProfile {
    pub max_leverage: f64,
    pub max_order_size_pct: f64,
    pub daily_drawdown_limit: f64,
    pub allowed_symbols: Vec<String>,
    pub timing: TimingConfig,
    pub indicators: IndicatorConfig,
    pub thresholds: ThresholdConfig,
}

impl RiskProfile {
    pub fn load() -> Result<Self> {
        let settings = Config::builder()
            .add_source(File::with_name("risk_config"))
            .build()?;

        let profile: RiskProfile = settings.try_deserialize()?;
        Ok(profile)
    }
    
    #[allow(dead_code)]
    pub fn is_symbol_allowed(&self, symbol: &str) -> bool {
        self.allowed_symbols.contains(&symbol.to_string())
    }
}
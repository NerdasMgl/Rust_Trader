use super::structs::{Indicators, Kline};

pub struct TechnicalAnalysis;

impl TechnicalAnalysis {
    pub fn analyze(klines: &[Kline]) -> Indicators {
        let closes: Vec<f64> = klines.iter().map(|k| k.close_price()).collect();
        
        let rsi = Self::calculate_rsi(&closes, 14);
        let atr = Self::calculate_atr(klines, 14);
        let ema_20 = Self::calculate_ema(&closes, 20);
        let ema_50 = Self::calculate_ema(&closes, 50);

        let trend = if ema_20 > ema_50 {
            "Bullish".to_string()
        } else if ema_20 < ema_50 {
            "Bearish".to_string()
        } else {
            "Neutral".to_string()
        };

        Indicators {
            rsi_14: rsi,
            atr_14: atr,
            ema_20,
            ema_50,
            trend_signal: trend,
        }
    }

    /// 标准 RSI 计算 (Wilder's Smoothing)
    fn calculate_rsi(prices: &[f64], period: usize) -> f64 {
        if prices.len() < period + 1 { return 50.0; }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in 1..=period {
            let change = prices[i] - prices[i-1];
            if change > 0.0 { gains += change; } else { losses -= change; }
        }

        let mut avg_gain = gains / period as f64;
        let mut avg_loss = losses / period as f64;

        for i in (period + 1)..prices.len() {
            let change = prices[i] - prices[i-1];
            let (current_gain, current_loss) = if change > 0.0 { (change, 0.0) } else { (0.0, change.abs()) };
            
            avg_gain = ((avg_gain * (period as f64 - 1.0)) + current_gain) / period as f64;
            avg_loss = ((avg_loss * (period as f64 - 1.0)) + current_loss) / period as f64;
        }

        if avg_loss == 0.0 { return 100.0; }
        let rs = avg_gain / avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    }

    fn calculate_atr(klines: &[Kline], period: usize) -> f64 {
        if klines.len() < period + 1 { return 0.0; }
        
        let mut tr_sum = 0.0;
        for i in 1..=period {
            let high = klines[i].high_price();
            let low = klines[i].low_price();
            let prev_close = klines[i-1].close_price();
            
            let tr = (high - low)
                .max((high - prev_close).abs())
                .max((low - prev_close).abs());
            tr_sum += tr;
        }
        
        tr_sum / period as f64
    }

    // [核心修复] 使用 SMA 初始化 EMA，防止早期数据失真
    fn calculate_ema(prices: &[f64], period: usize) -> f64 {
        if prices.len() < period { return prices.last().cloned().unwrap_or(0.0); }
        
        // 1. 计算前 period 个数据的 SMA 作为 EMA 种子
        let sma_seed: f64 = prices.iter().take(period).sum::<f64>() / period as f64;
        
        let k = 2.0 / (period as f64 + 1.0);
        let mut ema = sma_seed;
        
        // 2. 从 period 索引开始迭代计算 EMA
        for price in prices.iter().skip(period) {
            ema = (price * k) + (ema * (1.0 - k));
        }
        ema
    }
}
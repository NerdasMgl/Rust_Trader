use std::fmt;
use super::structs::MarketState;

impl fmt::Display for MarketState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let funding_pct = self.funding_rate * 100.0;
        let funding_warning = if funding_pct.abs() > 0.05 { "(HIGH RISK)" } else { "" };
        
        write!(f, 
            "\n--- MARKET SNAPSHOT ---\n\
            [Basic] Symbol: {} | Price: ${:.2}\n\
            [Technical] Trend: {} | RSI: {:.2} | ATR: {:.2}\n\
            [Derivatives] Funding: {:.4}% {} | OI: {:.0}\n\
            [Sentiment Analysis]\n\
            > News: {}\n\n\
            > Reddit: {}\n\
            -----------------------",
            self.symbol, self.price,
            self.indicators.trend_signal, self.indicators.rsi_14, self.indicators.atr_14,
            funding_pct, funding_warning, self.open_interest,
            self.news_sentiment, self.reddit_sentiment
        )
    }
}
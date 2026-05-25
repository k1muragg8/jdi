use super::market_provider::MarketDataProvider;
use crate::models::{Candle, MarketPrice};
use anyhow::{Result, anyhow};
use chrono::Local;

pub struct MockMarketProvider;

impl MockMarketProvider {
    pub fn new() -> Self {
        Self
    }
}

impl MarketDataProvider for MockMarketProvider {
    fn fetch_latest_price(&self, symbol: &str) -> Result<MarketPrice> {
        let price = match symbol {
            "QQQ" => 450.50,
            "SPY" => 520.20,
            "RSP" => 165.30,
            "IBB" => 135.10,
            "XBI" => 90.50,
            "^N225" => 38500.0,
            "TLT" => 92.40,
            _ => return Err(anyhow!("Symbol not found in mock: {}", symbol)),
        };

        Ok(MarketPrice {
            symbol: symbol.to_string(),
            price,
            date: Local::now().format("%Y-%m-%d").to_string(),
            currency: "USD".to_string(),
            source: "mock".to_string(),
            is_stale: false,
        })
    }

    fn fetch_daily_candles(&self, symbol: &str, lookback_days: usize) -> Result<Vec<Candle>> {
        let latest = self.fetch_latest_price(symbol)?;
        let mut candles = Vec::new();

        for i in 0..lookback_days {
            let date = (Local::now() - chrono::Duration::days(i as i64))
                .format("%Y-%m-%d")
                .to_string();

            candles.push(Candle {
                symbol: symbol.to_string(),
                date,
                open: latest.price * 0.99,
                high: latest.price * 1.01,
                low: latest.price * 0.98,
                close: latest.price,
                volume: 1000000,
                source: "mock".to_string(),
            });
        }

        Ok(candles)
    }
}

use super::fx_provider::FxProvider;
use crate::models::Candle;
use crate::models::market::FxRate;
use anyhow::Result;
use chrono::Utc;

pub struct MockFxProvider;

impl FxProvider for MockFxProvider {
    fn fetch_latest_rate(&self, pair: &str) -> Result<FxRate> {
        if pair == "NON_EXISTENT" {
            return Err(anyhow::anyhow!("Pair not found"));
        }
        let now = Utc::now();
        let date = now.format("%Y-%m-%d").to_string();

        let rate = if pair.contains("USD") && pair.contains("CNH") {
            7.25
        } else {
            1.0
        };

        let parts: Vec<&str> = if pair.contains('/') {
            pair.split('/').collect()
        } else if pair.starts_with("USDCNH") {
            vec!["USD", "CNH"]
        } else {
            vec!["?", "?"]
        };

        Ok(FxRate {
            pair: pair.to_string(),
            base_currency: parts.get(0).unwrap_or(&"USD").to_string(),
            quote_currency: parts.get(1).unwrap_or(&"CNH").to_string(),
            rate,
            date,
            source: "mock".to_string(),
            is_stale: false,
            is_estimated: true,
        })
    }

    fn fetch_daily_rates(&self, pair: &str, lookback_days: usize) -> Result<Vec<Candle>> {
        if pair == "NON_EXISTENT" {
            return Err(anyhow::anyhow!("Pair not found"));
        }
        let mut candles = Vec::new();
        let now = Utc::now();

        let base_rate = if pair.contains("USD") && pair.contains("CNH") {
            7.25
        } else {
            1.0
        };

        for i in 0..lookback_days {
            let date = (now - chrono::Duration::days(i as i64))
                .format("%Y-%m-%d")
                .to_string();
            candles.push(Candle {
                symbol: pair.to_string(),
                date,
                open: 0.0,
                high: 0.0,
                low: 0.0,
                close: base_rate + (i as f64 * 0.001),
                volume: 0,
                source: "mock".to_string(),
            });
        }

        Ok(candles)
    }
}

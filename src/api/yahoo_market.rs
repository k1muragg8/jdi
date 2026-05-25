use super::market_provider::MarketDataProvider;
use crate::models::{Candle, MarketPrice};
use anyhow::{Context, Result, anyhow};
use chrono::{TimeZone, Utc};
use serde_json::Value;
use std::time::Duration;

pub struct YahooMarketProvider {
    client: reqwest::blocking::Client,
}

impl YahooMarketProvider {
    pub fn new(timeout: u64) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(timeout))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();
        Self { client }
    }

    fn fetch_chart_data(&self, symbol: &str, range: &str, interval: &str) -> Result<Value> {
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?range={}&interval={}",
            symbol, range, interval
        );

        let resp = self.client.get(&url).send().map_err(|e| {
            anyhow!(
                "Failed to send request to Yahoo Chart API: {} (URL: {}, Provider: yahoo)",
                e,
                url
            )
        })?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "Yahoo Chart API returned status: {} (URL: {}, Provider: yahoo)",
                resp.status(),
                url
            ));
        }

        resp.json()
            .context("Failed to parse Yahoo Chart response as JSON")
    }
}

impl MarketDataProvider for YahooMarketProvider {
    fn fetch_latest_price(&self, symbol: &str) -> Result<MarketPrice> {
        let data = self.fetch_chart_data(symbol, "1d", "1m")?;

        let result = data["chart"]["result"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| anyhow!("No data found for symbol: {}", symbol))?;

        let meta = &result["meta"];
        let price = meta["regularMarketPrice"]
            .as_f64()
            .ok_or_else(|| anyhow!("Missing market price for symbol: {}", symbol))?;

        let timestamp = meta["regularMarketTime"]
            .as_i64()
            .ok_or_else(|| anyhow!("Missing market time for symbol: {}", symbol))?;

        let currency = meta["currency"].as_str().unwrap_or("USD").to_string();

        let dt = Utc.timestamp_opt(timestamp, 0).unwrap();
        let date = dt.format("%Y-%m-%d").to_string();

        Ok(MarketPrice {
            symbol: symbol.to_string(),
            price,
            date,
            currency,
            source: "yahoo".to_string(),
            is_stale: false,
        })
    }

    fn fetch_daily_candles(&self, symbol: &str, lookback_days: usize) -> Result<Vec<Candle>> {
        let range = match lookback_days {
            0..=5 => "5d",
            6..=30 => "1mo",
            31..=90 => "3mo",
            91..=180 => "6mo",
            181..=365 => "1y",
            _ => "5y",
        };

        let data = self.fetch_chart_data(symbol, range, "1d")?;

        let result = data["chart"]["result"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| anyhow!("No history data found for symbol: {}", symbol))?;

        let timestamps = result["timestamp"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing timestamps in history data"))?;

        let indicators = &result["indicators"]["quote"][0];
        let opens = indicators["open"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing opens"))?;
        let highs = indicators["high"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing highs"))?;
        let lows = indicators["low"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing lows"))?;
        let closes = indicators["close"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing closes"))?;
        let volumes = indicators["volume"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing volumes"))?;

        let mut candles = Vec::new();
        for i in (0..timestamps.len()).rev().take(lookback_days) {
            let ts = timestamps[i].as_i64().unwrap();
            let dt = Utc.timestamp_opt(ts, 0).unwrap();

            // Yahoo API sometimes returns null for some fields in the middle of data
            let close = closes[i].as_f64();
            if close.is_none() {
                continue;
            }

            candles.push(Candle {
                symbol: symbol.to_string(),
                date: dt.format("%Y-%m-%d").to_string(),
                open: opens[i].as_f64().unwrap_or(0.0),
                high: highs[i].as_f64().unwrap_or(0.0),
                low: lows[i].as_f64().unwrap_or(0.0),
                close: close.unwrap(),
                volume: volumes[i].as_u64().unwrap_or(0),
                source: "yahoo".to_string(),
            });
        }

        Ok(candles)
    }
}

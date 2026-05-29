use super::fx_provider::FxProvider;
use crate::models::Candle;
use crate::models::market::FxRate;
use anyhow::{Context, Result, anyhow};
use chrono::{TimeZone, Utc};
use serde_json::Value;
use std::time::Duration;

pub struct YahooFxProvider {
    client: reqwest::blocking::Client,
}

impl YahooFxProvider {
    pub fn new(timeout: u64) -> Self {
        let client = std::thread::spawn(move || {
            reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(timeout))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .build()
                .unwrap_or_default()
        }).join().unwrap();
        Self { client }
    }

    fn fetch_chart_data(&self, symbol: &str, range: &str, interval: &str) -> Result<Value> {
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?range={}&interval={}",
            symbol, range, interval
        );

        let client_ref = &self.client;
        let url_ref = &url;
        tokio::task::block_in_place(move || {
            let resp = client_ref.get(url_ref).send().map_err(|e| {
                anyhow!(
                    "Failed to send request to Yahoo FX API: {} (URL: {}, Provider: yahoo)",
                    e,
                    url_ref
                )
            })?;
            if !resp.status().is_success() {
                return Err(anyhow!(
                    "Yahoo FX API returned status: {} (URL: {}, Provider: yahoo)",
                    resp.status(),
                    url_ref
                ));
            }
            resp.json()
                .context("Failed to parse Yahoo FX response as JSON")
        })
    }
}

impl FxProvider for YahooFxProvider {
    fn fetch_latest_rate(&self, pair: &str) -> Result<FxRate> {
        // pair is usually "USD/CNH", we need to map it to symbol if needed,
        // but for now let's assume the caller passes the yahoo symbol or we handle it here.
        // The user said: YahooFxProvider using configurable symbol, default: USDCNH=X
        // So the symbol might be "USDCNH=X"
        let data = self.fetch_chart_data(pair, "1d", "1m")?;

        let result = data["chart"]["result"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| anyhow!("No data found for FX pair: {}", pair))?;

        let meta = &result["meta"];
        let rate = meta["regularMarketPrice"]
            .as_f64()
            .ok_or_else(|| anyhow!("Missing rate for FX pair: {}", pair))?;

        let timestamp = meta["regularMarketTime"]
            .as_i64()
            .ok_or_else(|| anyhow!("Missing time for FX pair: {}", pair))?;

        let dt = Utc.timestamp_opt(timestamp, 0).unwrap();
        let date = dt.format("%Y-%m-%d").to_string();

        // Parse pair "USD/CNH" or similar
        let parts: Vec<&str> = if pair.contains('/') {
            pair.split('/').collect()
        } else if pair.starts_with("USDCNH") {
            vec!["USD", "CNH"]
        } else {
            vec!["?", "?"]
        };

        Ok(FxRate {
            pair: pair.to_string(),
            base_currency: parts.first().unwrap_or(&"USD").to_string(),
            quote_currency: parts.get(1).unwrap_or(&"CNH").to_string(),
            rate,
            date,
            source: "yahoo".to_string(),
            is_stale: false,
            is_estimated: false,
        })
    }

    fn fetch_daily_rates(&self, pair: &str, lookback_days: usize) -> Result<Vec<Candle>> {
        let range = match lookback_days {
            0..=5 => "5d",
            6..=30 => "1mo",
            31..=90 => "3mo",
            91..=180 => "6mo",
            181..=365 => "1y",
            _ => "5y",
        };

        let mut data = self.fetch_chart_data(pair, range, "1d")?;

        // For some FX symbols like USDCNH=X, Yahoo returns only the latest day for interval=1d
        // even if range is larger. We check if we got enough data.
        let needs_more = lookback_days > 1;
        let got_little = data["chart"]["result"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|r| r["timestamp"].as_array())
            .map_or(0, |a| a.len())
            <= 1;

        if needs_more && got_little {
            if let Ok(data_60m) = self.fetch_chart_data(pair, range, "60m") {
                data = data_60m;
            }
        }

        let result = data["chart"]["result"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| anyhow!("No history data found for FX pair: {}", pair))?;

        let timestamps = result["timestamp"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing timestamps in history data"))?;

        let indicators = &result["indicators"]["quote"][0];
        let closes = indicators["close"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing closes"))?;

        let mut candles = Vec::new();
        let mut last_date = String::new();

        for i in (0..timestamps.len()).rev() {
            let ts = timestamps[i].as_i64().unwrap();
            let dt = Utc.timestamp_opt(ts, 0).unwrap();
            let date = dt.format("%Y-%m-%d").to_string();

            // Aggregate multiple data points for the same day (take the latest one)
            if date == last_date {
                continue;
            }

            let close = closes[i].as_f64();
            if close.is_none() {
                continue;
            }

            candles.push(Candle {
                symbol: pair.to_string(),
                date: date.clone(),
                open: 0.0,
                high: 0.0,
                low: 0.0,
                close: close.unwrap(),
                volume: 0,
                source: "yahoo".to_string(),
            });

            last_date = date;
            if candles.len() >= lookback_days {
                break;
            }
        }

        Ok(candles)
    }
}

use super::instrument_provider::InstrumentProvider;
use super::market_provider::MarketDataProvider;
use crate::models::{Candle, InstrumentCandle, InstrumentConfig, InstrumentQuote, MarketPrice};
use anyhow::{Context, Result, anyhow};
use chrono::{TimeZone, Utc};
use serde_json::Value;
use std::time::Duration;

pub struct YahooMarketProvider {
    client: reqwest::blocking::Client,
}

fn extract_yahoo_previous_close(meta: &Value, result: &Value) -> Option<f64> {
    for key in [
        "regularMarketPreviousClose",
        "previousClose",
        "chartPreviousClose",
    ] {
        if let Some(v) = meta[key].as_f64() {
            if v > 0.0 {
                return Some(v);
            }
        }
    }
    if let Some(closes) = result["indicators"]["quote"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|q| q["close"].as_array())
    {
        let valid: Vec<f64> = closes.iter().filter_map(|v| v.as_f64()).collect();
        if valid.len() >= 2 {
            let prev = valid[valid.len() - 2];
            if prev > 0.0 {
                return Some(prev);
            }
        }
    }
    None
}

impl YahooMarketProvider {
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
                    "Failed to send request to Yahoo Chart API: {} (URL: {}, Provider: yahoo)",
                    e,
                    url_ref
                )
            })?;

            if !resp.status().is_success() {
                return Err(anyhow!(
                    "Yahoo Chart API returned status: {} (URL: {}, Provider: yahoo)",
                    resp.status(),
                    url_ref
                ));
            }

            resp.json()
                .context("Failed to parse Yahoo Chart response as JSON")
        })
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

        let previous_close = extract_yahoo_previous_close(meta, result);
        let change_direct = meta["regularMarketChange"].as_f64();
        let change_pct_direct = meta["regularMarketChangePercent"].as_f64();

        let (change, change_percent) =
            if let (Some(ch), Some(pct)) = (change_direct, change_pct_direct) {
                (Some(ch), Some(pct))
            } else if let Some(prev) = previous_close {
                if prev.abs() > 1e-12 {
                    let ch = price - prev;
                    let ch_pct = (ch / prev) * 100.0;
                    (Some(ch), Some(ch_pct))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

        let price = crate::engine::market_quote::normalize_market_price(MarketPrice {
            symbol: symbol.to_string(),
            price,
            date,
            currency,
            source: "yahoo".to_string(),
            is_stale: false,
            previous_close,
            change,
            change_percent,
        });

        Ok(price)
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

impl InstrumentProvider for YahooMarketProvider {
    fn latest(&self, instrument: &InstrumentConfig) -> Result<InstrumentQuote> {
        let market_price = self.fetch_latest_price(&instrument.provider_symbol)?;
        Ok(InstrumentQuote {
            instrument_id: instrument.instrument_id.clone(),
            symbol: instrument.symbol.clone(),
            name: instrument.name.clone(),
            name_zh: instrument.name_zh.clone(),
            category_zh: instrument.category_zh.clone(),
            asset_class: instrument.asset_class.clone(),
            latest_price: market_price.price,
            latest_date: market_price.date,
            currency: market_price.currency,
            quote_unit: instrument.quote_unit.clone(),
            provider: "yahoo".to_string(),
            source: market_price.source,
            status: "正常".to_string(),
            warning: if market_price.is_stale {
                Some("数据可能已过期".to_string())
            } else {
                None
            },
        })
    }

    fn history(&self, instrument: &InstrumentConfig, days: usize) -> Result<Vec<InstrumentCandle>> {
        let candles = self.fetch_daily_candles(&instrument.provider_symbol, days)?;
        Ok(candles
            .into_iter()
            .map(|c| InstrumentCandle {
                instrument_id: instrument.instrument_id.clone(),
                symbol: instrument.symbol.clone(),
                date: c.date,
                open: Some(c.open),
                high: Some(c.high),
                low: Some(c.low),
                close: c.close,
                volume: Some(c.volume as f64),
                source: c.source,
            })
            .collect())
    }
}

use super::instrument_provider::InstrumentProvider;
use super::market_provider::MarketDataProvider;
use crate::models::{Candle, InstrumentCandle, InstrumentConfig, InstrumentQuote, MarketPrice};
use anyhow::{Context, Result, anyhow};
use chrono::Local;
use serde_json::Value;
use std::time::Duration;

/// Eastmoney push2 quote API for domestic / SGE symbols (e.g. AU9999 via secid 118.AU9999).
pub struct EastMoneyMarketProvider {
    client: reqwest::blocking::Client,
}

impl EastMoneyMarketProvider {
    pub fn new(timeout: u64) -> Self {
        let client = std::thread::spawn(move || {
            reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(timeout))
                .user_agent(
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                )
                .build()
                .unwrap_or_default()
        })
        .join()
        .unwrap();
        Self { client }
    }

    /// Map user/provider symbol to Eastmoney `secid` (market.code).
    pub fn resolve_secid(symbol: &str) -> Result<String> {
        let s = symbol.trim();
        if s.is_empty() {
            return Err(anyhow!("东方财富代码映射错误：symbol 为空"));
        }
        if s.contains('.') {
            return Ok(s.to_string());
        }
        if let Some(rest) = s.strip_prefix("globalfuture/") {
            let rest = rest.trim();
            if rest.contains('.') {
                return Ok(rest.to_string());
            }
            return Ok(format!("118.{rest}"));
        }
        if s.eq_ignore_ascii_case("AU9999") {
            return Ok("118.AU9999".to_string());
        }
        Err(anyhow!(
            "东方财富代码映射错误：无法识别代码 {}，请使用 118.AU9999 等形式",
            s
        ))
    }

    fn fetch_quote_json(&self, secid: &str) -> Result<Value> {
        let url = format!(
            "https://push2.eastmoney.com/api/qt/stock/get?secid={}&fields=f43,f44,f45,f46,f57,f58,f60,f169,f170",
            secid
        );
        let client_ref = &self.client;
        let url_ref = &url;
        let body: Value = tokio::task::block_in_place(move || {
            let resp = client_ref
                .get(url_ref)
                .send()
                .map_err(|e| anyhow!("数据源暂不可用：东方财富请求失败 ({})", e))?;
            if !resp.status().is_success() {
                return Err(anyhow!(
                    "数据源暂不可用：东方财富返回 HTTP {}",
                    resp.status()
                ));
            }
            resp.json().context("东方财富未返回行情：响应非 JSON")
        })?;
        Ok(body)
    }

    fn scaled_price(raw: Option<f64>) -> Option<f64> {
        raw.filter(|v| *v > 0.0).map(|v| v / 100.0)
    }

    pub fn parse_quote(symbol: &str, secid: &str, body: &Value) -> Result<MarketPrice> {
        if body["rc"].as_i64().unwrap_or(-1) != 0 {
            return Err(anyhow!("东方财富未返回行情"));
        }
        let data = body
            .get("data")
            .filter(|d| !d.is_null())
            .ok_or_else(|| anyhow!("东方财富未返回行情"))?;

        let price = Self::scaled_price(data["f43"].as_f64())
            .ok_or_else(|| anyhow!("东方财富未返回行情"))?;
        let previous_close = Self::scaled_price(data["f60"].as_f64());

        let change = Self::scaled_price(data["f169"].as_f64()).or_else(|| {
            previous_close.map(|prev| {
                let ch = price - prev;
                if ch.abs() < 1e-12 { 0.0 } else { ch }
            })
        });

        let change_percent = data["f170"].as_f64().map(|v| v / 100.0).or_else(|| {
            previous_close.and_then(|prev| {
                if prev.abs() > 1e-12 {
                    Some((price - prev) / prev * 100.0)
                } else {
                    None
                }
            })
        });

        let display_code = data["f57"].as_str().unwrap_or(symbol).to_string();
        let date = Local::now().format("%Y-%m-%d").to_string();

        let mut mp = MarketPrice {
            symbol: display_code,
            price,
            date,
            currency: "CNY".to_string(),
            source: "eastmoney".to_string(),
            is_stale: false,
            previous_close,
            change,
            change_percent,
        };
        mp = crate::engine::market_quote::normalize_market_price(mp);
        let _ = secid; // secid used for fetch only
        Ok(mp)
    }
}

impl MarketDataProvider for EastMoneyMarketProvider {
    fn fetch_latest_price(&self, symbol: &str) -> Result<MarketPrice> {
        let secid = Self::resolve_secid(symbol)?;
        let body = self.fetch_quote_json(&secid)?;
        Self::parse_quote(symbol, &secid, &body)
    }

    fn fetch_daily_candles(&self, symbol: &str, _lookback_days: usize) -> Result<Vec<Candle>> {
        let secid = Self::resolve_secid(symbol)?;
        let url = format!(
            "https://push2.eastmoney.com/api/qt/stock/kline/get?secid={}&klt=101&fqt=1&lmt=120&fields1=f1&fields2=f51,f52,f53,f54,f55,f56",
            secid
        );
        let client_ref = &self.client;
        let url_ref = &url;
        let body: Value = tokio::task::block_in_place(move || {
            let resp = client_ref
                .get(url_ref)
                .send()
                .map_err(|e| anyhow!("Eastmoney kline request failed: {}", e))?;
            resp.json().context("Failed to parse Eastmoney kline JSON")
        })?;

        let mut candles = Vec::new();
        if let Some(klines) = body["data"]["klines"].as_array() {
            for line in klines {
                if let Some(s) = line.as_str() {
                    let parts: Vec<&str> = s.split(',').collect();
                    if parts.len() >= 5 {
                        if let (Ok(open), Ok(close)) =
                            (parts[1].parse::<f64>(), parts[2].parse::<f64>())
                        {
                            let high = parts[3].parse().unwrap_or(close);
                            let low = parts[4].parse().unwrap_or(close);
                            candles.push(Candle {
                                symbol: symbol.to_string(),
                                date: parts[0].to_string(),
                                open,
                                high,
                                low,
                                close,
                                volume: 0,
                                source: "eastmoney".to_string(),
                            });
                        }
                    }
                }
            }
        }
        Ok(candles)
    }
}

impl InstrumentProvider for EastMoneyMarketProvider {
    fn latest(&self, instrument: &InstrumentConfig) -> Result<InstrumentQuote> {
        let fetch_sym = if instrument.provider_symbol.is_empty() {
            instrument.symbol.as_str()
        } else {
            instrument.provider_symbol.as_str()
        };
        let market_price = self.fetch_latest_price(fetch_sym)?;
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
            provider: "eastmoney".to_string(),
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
        let fetch_sym = if instrument.provider_symbol.is_empty() {
            instrument.symbol.as_str()
        } else {
            instrument.provider_symbol.as_str()
        };
        let candles = self.fetch_daily_candles(fetch_sym, days)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_secid_au9999() {
        assert_eq!(
            EastMoneyMarketProvider::resolve_secid("AU9999").unwrap(),
            "118.AU9999"
        );
        assert_eq!(
            EastMoneyMarketProvider::resolve_secid("118.AU9999").unwrap(),
            "118.AU9999"
        );
    }

    #[test]
    fn test_parse_eastmoney_quote_sample() {
        let body: Value = serde_json::json!({
            "rc": 0,
            "data": {
                "f43": 96899,
                "f57": "AU9999",
                "f60": 97450,
                "f169": -551,
                "f170": -57
            }
        });
        let p = EastMoneyMarketProvider::parse_quote("AU9999", "118.AU9999", &body).unwrap();
        assert!((p.price - 968.99).abs() < 0.01);
        assert!((p.previous_close.unwrap() - 974.50).abs() < 0.01);
        assert!(p.change.is_some());
        assert!(p.change_percent.is_some());
        assert_eq!(p.source, "eastmoney");
    }
}

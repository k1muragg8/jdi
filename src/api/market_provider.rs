use crate::models::{Candle, MarketPrice};
use anyhow::Result;

pub trait MarketDataProvider {
    fn fetch_latest_price(&self, symbol: &str) -> Result<MarketPrice>;
    fn fetch_daily_candles(&self, symbol: &str, lookback_days: usize) -> Result<Vec<Candle>>;
}

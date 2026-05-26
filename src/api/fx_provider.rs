use crate::models::Candle;
use crate::models::market::FxRate;
use anyhow::Result;

pub trait FxProvider {
    fn fetch_latest_rate(&self, pair: &str) -> Result<FxRate>;
    fn fetch_daily_rates(&self, pair: &str, lookback_days: usize) -> Result<Vec<Candle>>;
}

use crate::models::{InstrumentCandle, InstrumentConfig, InstrumentQuote};
use anyhow::Result;

pub trait InstrumentProvider {
    fn latest(&self, instrument: &InstrumentConfig) -> Result<InstrumentQuote>;
    fn history(&self, instrument: &InstrumentConfig, days: usize) -> Result<Vec<InstrumentCandle>>;
}

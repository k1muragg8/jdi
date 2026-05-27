use crate::api::create_instrument_provider;
use crate::models::{InstrumentCandle, InstrumentConfig, InstrumentQuote, MarketConfig};
use anyhow::{Result, anyhow};

pub fn lookup_instrument(
    config: &MarketConfig,
    instruments: &[InstrumentConfig],
    symbol_or_id: &str,
) -> Result<InstrumentQuote> {
    let instrument = instruments
        .iter()
        .find(|i| i.instrument_id == symbol_or_id || i.symbol == symbol_or_id)
        .ok_or_else(|| anyhow!("Instrument not found: {}", symbol_or_id))?;

    if !instrument.enabled {
        return Err(anyhow!("Instrument is disabled: {}", symbol_or_id));
    }

    let provider = create_instrument_provider(config, Some(&instrument.provider));
    provider.latest(instrument)
}

pub fn get_instrument_history(
    config: &MarketConfig,
    instruments: &[InstrumentConfig],
    symbol_or_id: &str,
    days: usize,
) -> Result<Vec<InstrumentCandle>> {
    let instrument = instruments
        .iter()
        .find(|i| i.instrument_id == symbol_or_id || i.symbol == symbol_or_id)
        .ok_or_else(|| anyhow!("Instrument not found: {}", symbol_or_id))?;

    if !instrument.enabled {
        return Err(anyhow!("Instrument is disabled: {}", symbol_or_id));
    }

    let provider = create_instrument_provider(config, Some(&instrument.provider));
    provider.history(instrument, days)
}

pub fn validate_instruments(
    config: &MarketConfig,
    instruments: &[InstrumentConfig],
) -> Vec<(String, Result<InstrumentQuote>)> {
    instruments
        .iter()
        .filter(|i| i.enabled)
        .map(|i| {
            let provider = create_instrument_provider(config, Some(&i.provider));
            (i.instrument_id.clone(), provider.latest(i))
        })
        .collect()
}

use crate::models::{AssetClass, InstrumentConfig, InstrumentRegistry};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_instruments<P: AsRef<Path>>(path: P) -> Result<Vec<InstrumentConfig>> {
    if !path.as_ref().exists() {
        return Ok(get_default_instruments());
    }
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read instruments file at {:?}", path.as_ref()))?;

    // Support both TOML and JSON based on extension
    if path.as_ref().extension().map_or(false, |ext| ext == "json") {
        let registry: InstrumentRegistry = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse instruments JSON at {:?}", path.as_ref()))?;
        Ok(registry.instruments)
    } else {
        let registry: InstrumentRegistry = toml::from_str(&content)
            .with_context(|| format!("Failed to parse instruments TOML at {:?}", path.as_ref()))?;
        Ok(registry.instruments)
    }
}

pub fn save_instruments<P: AsRef<Path>>(path: P, instruments: &[InstrumentConfig]) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let registry = InstrumentRegistry {
        instruments: instruments.to_vec(),
    };

    let content = if path.as_ref().extension().map_or(false, |ext| ext == "json") {
        serde_json::to_string_pretty(&registry)
            .with_context(|| "Failed to serialize instruments to JSON")?
    } else {
        toml::to_string_pretty(&registry)
            .with_context(|| "Failed to serialize instruments to TOML")?
    };

    fs::write(path.as_ref(), content)
        .with_context(|| format!("Failed to write instruments file to {:?}", path.as_ref()))?;
    Ok(())
}

pub fn get_default_instruments() -> Vec<InstrumentConfig> {
    vec![
        InstrumentConfig {
            instrument_id: "nasdaq_qqq".to_string(),
            symbol: "QQQ".to_string(),
            display_symbol: Some("QQQ".to_string()),
            name: "Nasdaq 100 ETF".to_string(),
            asset_class: AssetClass::Etf,
            provider: "yahoo".to_string(),
            provider_symbol: "QQQ".to_string(),
            market: Some("US".to_string()),
            exchange: Some("NASDAQ".to_string()),
            currency: "USD".to_string(),
            quote_unit: "share".to_string(),
            price_unit: "USD/share".to_string(),
            timezone: Some("America/New_York".to_string()),
            enabled: true,
            priority: 10,
            tags: vec!["tech".to_string(), "growth".to_string()],
            note: None,
        },
        InstrumentConfig {
            instrument_id: "sp500_spy".to_string(),
            symbol: "SPY".to_string(),
            display_symbol: Some("SPY".to_string()),
            name: "S&P 500 ETF".to_string(),
            asset_class: AssetClass::Etf,
            provider: "yahoo".to_string(),
            provider_symbol: "SPY".to_string(),
            market: Some("US".to_string()),
            exchange: Some("NYSE".to_string()),
            currency: "USD".to_string(),
            quote_unit: "share".to_string(),
            price_unit: "USD/share".to_string(),
            timezone: Some("America/New_York".to_string()),
            enabled: true,
            priority: 9,
            tags: vec!["core".to_string()],
            note: None,
        },
        InstrumentConfig {
            instrument_id: "vix_index".to_string(),
            symbol: "^VIX".to_string(),
            display_symbol: Some("VIX".to_string()),
            name: "CBOE Volatility Index".to_string(),
            asset_class: AssetClass::Index,
            provider: "yahoo".to_string(),
            provider_symbol: "^VIX".to_string(),
            market: Some("US".to_string()),
            exchange: Some("CBOE".to_string()),
            currency: "USD".to_string(),
            quote_unit: "point".to_string(),
            price_unit: "points".to_string(),
            timezone: Some("America/New_York".to_string()),
            enabled: true,
            priority: 8,
            tags: vec!["risk".to_string()],
            note: None,
        },
        InstrumentConfig {
            instrument_id: "us30y_yield".to_string(),
            symbol: "^TYX".to_string(),
            display_symbol: Some("US30Y".to_string()),
            name: "Treasury Yield 30 Years".to_string(),
            asset_class: AssetClass::Rate,
            provider: "yahoo".to_string(),
            provider_symbol: "^TYX".to_string(),
            market: Some("US".to_string()),
            exchange: Some("Chicago Options".to_string()),
            currency: "USD".to_string(),
            quote_unit: "percent".to_string(),
            price_unit: "%".to_string(),
            timezone: Some("America/New_York".to_string()),
            enabled: true,
            priority: 7,
            tags: vec!["rate".to_string()],
            note: None,
        },
        InstrumentConfig {
            instrument_id: "usd_cnh".to_string(),
            symbol: "USDCNH=X".to_string(),
            display_symbol: Some("USD/CNH".to_string()),
            name: "USD/CNH FX".to_string(),
            asset_class: AssetClass::Fx,
            provider: "yahoo".to_string(),
            provider_symbol: "USDCNH=X".to_string(),
            market: Some("FX".to_string()),
            exchange: Some("CCY".to_string()),
            currency: "CNH".to_string(),
            quote_unit: "cnh".to_string(),
            price_unit: "CNH".to_string(),
            timezone: Some("UTC".to_string()),
            enabled: true,
            priority: 6,
            tags: vec!["fx".to_string()],
            note: None,
        },
        InstrumentConfig {
            instrument_id: "btc_usd".to_string(),
            symbol: "BTC-USD".to_string(),
            display_symbol: Some("BTC".to_string()),
            name: "Bitcoin USD".to_string(),
            asset_class: AssetClass::Crypto,
            provider: "yahoo".to_string(),
            provider_symbol: "BTC-USD".to_string(),
            market: Some("Crypto".to_string()),
            exchange: Some("CCC".to_string()),
            currency: "USD".to_string(),
            quote_unit: "btc".to_string(),
            price_unit: "USD".to_string(),
            timezone: Some("UTC".to_string()),
            enabled: true,
            priority: 5,
            tags: vec!["crypto".to_string()],
            note: None,
        },
        InstrumentConfig {
            instrument_id: "eth_usd".to_string(),
            symbol: "ETH-USD".to_string(),
            display_symbol: Some("ETH".to_string()),
            name: "Ethereum USD".to_string(),
            asset_class: AssetClass::Crypto,
            provider: "yahoo".to_string(),
            provider_symbol: "ETH-USD".to_string(),
            market: Some("Crypto".to_string()),
            exchange: Some("CCC".to_string()),
            currency: "USD".to_string(),
            quote_unit: "eth".to_string(),
            price_unit: "USD".to_string(),
            timezone: Some("UTC".to_string()),
            enabled: true,
            priority: 4,
            tags: vec!["crypto".to_string()],
            note: None,
        },
        InstrumentConfig {
            instrument_id: "sol_usd".to_string(),
            symbol: "SOL-USD".to_string(),
            display_symbol: Some("SOL".to_string()),
            name: "Solana USD".to_string(),
            asset_class: AssetClass::Crypto,
            provider: "yahoo".to_string(),
            provider_symbol: "SOL-USD".to_string(),
            market: Some("Crypto".to_string()),
            exchange: Some("CCC".to_string()),
            currency: "USD".to_string(),
            quote_unit: "sol".to_string(),
            price_unit: "USD".to_string(),
            timezone: Some("UTC".to_string()),
            enabled: true,
            priority: 3,
            tags: vec!["crypto".to_string()],
            note: None,
        },
        InstrumentConfig {
            instrument_id: "gold_gc".to_string(),
            symbol: "GC=F".to_string(),
            display_symbol: Some("Gold Futures".to_string()),
            name: "COMEX Gold Futures".to_string(),
            asset_class: AssetClass::Futures,
            provider: "yahoo".to_string(),
            provider_symbol: "GC=F".to_string(),
            market: Some("COMEX".to_string()),
            exchange: Some("NYMEX".to_string()),
            currency: "USD".to_string(),
            quote_unit: "oz".to_string(),
            price_unit: "USD/oz".to_string(),
            timezone: Some("America/New_York".to_string()),
            enabled: true,
            priority: 2,
            tags: vec!["commodity".to_string(), "gold".to_string()],
            note: None,
        },
        InstrumentConfig {
            instrument_id: "crude_cl".to_string(),
            symbol: "CL=F".to_string(),
            display_symbol: Some("WTI Crude".to_string()),
            name: "WTI Crude Oil Futures".to_string(),
            asset_class: AssetClass::Futures,
            provider: "yahoo".to_string(),
            provider_symbol: "CL=F".to_string(),
            market: Some("NYMEX".to_string()),
            exchange: Some("NYMEX".to_string()),
            currency: "USD".to_string(),
            quote_unit: "barrel".to_string(),
            price_unit: "USD/barrel".to_string(),
            timezone: Some("America/New_York".to_string()),
            enabled: true,
            priority: 1,
            tags: vec!["commodity".to_string(), "energy".to_string()],
            note: None,
        },
        InstrumentConfig {
            instrument_id: "copper_hg".to_string(),
            symbol: "HG=F".to_string(),
            display_symbol: Some("Copper".to_string()),
            name: "COMEX Copper Futures".to_string(),
            asset_class: AssetClass::Futures,
            provider: "yahoo".to_string(),
            provider_symbol: "HG=F".to_string(),
            market: Some("COMEX".to_string()),
            exchange: Some("NYMEX".to_string()),
            currency: "USD".to_string(),
            quote_unit: "lb".to_string(),
            price_unit: "USD/lb".to_string(),
            timezone: Some("America/New_York".to_string()),
            enabled: true,
            priority: 0,
            tags: vec!["commodity".to_string(), "industrial".to_string()],
            note: None,
        },
        InstrumentConfig {
            instrument_id: "au9999".to_string(),
            symbol: "AU9999".to_string(),
            display_symbol: Some("AU9999".to_string()),
            name: "上海黄金交易所 AU9999".to_string(),
            asset_class: AssetClass::SpotCommodity,
            provider: "manual".to_string(),
            provider_symbol: "AU9999".to_string(),
            market: Some("SGE".to_string()),
            exchange: Some("SGE".to_string()),
            currency: "CNY".to_string(),
            quote_unit: "g".to_string(),
            price_unit: "CNY/g".to_string(),
            timezone: Some("Asia/Shanghai".to_string()),
            enabled: false,
            priority: 0,
            tags: vec!["gold".to_string(), "spot".to_string()],
            note: Some(
                "需要接入支持上海黄金交易所 AU9999 的 provider；不得用 GLD/XAUUSD 代替。"
                    .to_string(),
            ),
        },
    ]
}

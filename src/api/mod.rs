pub mod eastmoney;
pub mod eastmoney_market;
pub mod fund_provider;
pub mod fx_provider;
pub mod generic_http;
pub mod instrument_provider;
pub mod market_provider;
pub mod mock_fund;
pub mod mock_fx;
pub mod mock_market;
pub mod yahoo_fx;
pub mod yahoo_market;

pub use eastmoney::EastMoneyFundProvider;
pub use eastmoney_market::EastMoneyMarketProvider;
pub use fund_provider::FundProvider;
pub use fx_provider::FxProvider;
pub use generic_http::GenericHttpFundProvider;
pub use market_provider::MarketDataProvider;
pub use mock_fund::MockFundProvider;
pub use mock_fx::MockFxProvider;
pub use mock_market::MockMarketProvider;
pub use yahoo_fx::YahooFxProvider;
pub use yahoo_market::YahooMarketProvider;

use crate::models::{ApiConfig, FxConfig, MarketConfig};

pub fn create_fund_provider(config: &ApiConfig) -> Box<dyn FundProvider> {
    match config.default_fund_provider.as_str() {
        "eastmoney" => Box::new(EastMoneyFundProvider::new(
            config.fund_provider_timeout_seconds,
        )),
        "generic_http" => Box::new(GenericHttpFundProvider::new(
            config.fund_provider_timeout_seconds,
            config.fund_provider_retry_count,
        )),
        _ => Box::new(MockFundProvider::new()),
    }
}

pub fn create_market_provider(
    config: &MarketConfig,
    provider_override: Option<&str>,
) -> Box<dyn MarketDataProvider> {
    let provider_name = provider_override.unwrap_or(config.default_market_provider.as_str());
    match provider_name {
        "yahoo" => Box::new(YahooMarketProvider::new(
            config.market_provider_timeout_seconds,
        )),
        "eastmoney" => Box::new(EastMoneyMarketProvider::new(
            config.market_provider_timeout_seconds,
        )),
        "mock" => Box::new(MockMarketProvider::new()),
        _ => Box::new(MockMarketProvider::new()),
    }
}

/// Fetch a quote using the instrument's configured market provider (yahoo / eastmoney / mock).
pub fn fetch_market_price(
    config: &MarketConfig,
    provider: &str,
    provider_symbol: &str,
) -> anyhow::Result<crate::models::MarketPrice> {
    let p = provider.trim().to_lowercase();
    let sym = provider_symbol.trim();
    if sym.is_empty() {
        anyhow::bail!("provider_symbol 为空");
    }
    let market_provider = create_market_provider(config, Some(&p));
    market_provider.fetch_latest_price(sym)
}

pub fn create_fx_provider(
    config: &FxConfig,
    provider_override: Option<&str>,
) -> Box<dyn FxProvider> {
    let provider_name = provider_override.unwrap_or(config.default_fx_provider.as_str());
    match provider_name {
        "yahoo" => Box::new(YahooFxProvider::new(20)),
        "mock" => Box::new(MockFxProvider),
        _ => Box::new(MockFxProvider),
    }
}

pub fn create_instrument_provider(
    config: &MarketConfig,
    provider_override: Option<&str>,
) -> Box<dyn instrument_provider::InstrumentProvider> {
    let provider_name = provider_override.unwrap_or(config.default_market_provider.as_str());
    match provider_name {
        "yahoo" => Box::new(YahooMarketProvider::new(
            config.market_provider_timeout_seconds,
        )),
        "eastmoney" => Box::new(EastMoneyMarketProvider::new(
            config.market_provider_timeout_seconds,
        )),
        "mock" => Box::new(MockMarketProvider::new()),
        "manual" => Box::new(ManualInstrumentProvider),
        _ => Box::new(UnsupportedInstrumentProvider {
            name: provider_name.to_string(),
        }),
    }
}

struct ManualInstrumentProvider;

impl instrument_provider::InstrumentProvider for ManualInstrumentProvider {
    fn latest(
        &self,
        instrument: &crate::models::InstrumentConfig,
    ) -> anyhow::Result<crate::models::InstrumentQuote> {
        Ok(crate::models::InstrumentQuote {
            instrument_id: instrument.instrument_id.clone(),
            symbol: instrument.symbol.clone(),
            name: instrument.name.clone(),
            name_zh: instrument.name_zh.clone(),
            category_zh: instrument.category_zh.clone(),
            asset_class: instrument.asset_class.clone(),
            latest_price: 0.0,
            latest_date: "N/A".to_string(),
            currency: instrument.currency.clone(),
            quote_unit: instrument.quote_unit.clone(),
            provider: "manual".to_string(),
            source: "manual".to_string(),
            status: "不支持".to_string(),
            warning: Some("手动模式尚未接入数据源".to_string()),
        })
    }

    fn history(
        &self,
        _instrument: &crate::models::InstrumentConfig,
        _days: usize,
    ) -> anyhow::Result<Vec<crate::models::InstrumentCandle>> {
        Ok(vec![])
    }
}

struct UnsupportedInstrumentProvider {
    name: String,
}

impl instrument_provider::InstrumentProvider for UnsupportedInstrumentProvider {
    fn latest(
        &self,
        instrument: &crate::models::InstrumentConfig,
    ) -> anyhow::Result<crate::models::InstrumentQuote> {
        Ok(crate::models::InstrumentQuote {
            instrument_id: instrument.instrument_id.clone(),
            symbol: instrument.symbol.clone(),
            name: instrument.name.clone(),
            name_zh: instrument.name_zh.clone(),
            category_zh: instrument.category_zh.clone(),
            asset_class: instrument.asset_class.clone(),
            latest_price: 0.0,
            latest_date: "N/A".to_string(),
            currency: instrument.currency.clone(),
            quote_unit: instrument.quote_unit.clone(),
            provider: self.name.clone(),
            source: "unsupported".to_string(),
            status: "不支持".to_string(),
            warning: Some(format!("不支持的提供商: {}", self.name)),
        })
    }

    fn history(
        &self,
        _instrument: &crate::models::InstrumentConfig,
        _days: usize,
    ) -> anyhow::Result<Vec<crate::models::InstrumentCandle>> {
        Ok(vec![])
    }
}

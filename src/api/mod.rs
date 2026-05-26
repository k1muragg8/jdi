pub mod eastmoney;
pub mod fund_provider;
pub mod fx_provider;
pub mod generic_http;
pub mod market_provider;
pub mod mock_fund;
pub mod mock_fx;
pub mod mock_market;
pub mod yahoo_fx;
pub mod yahoo_market;

pub use eastmoney::EastMoneyFundProvider;
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
        "mock" => Box::new(MockMarketProvider::new()),
        _ => Box::new(MockMarketProvider::new()),
    }
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

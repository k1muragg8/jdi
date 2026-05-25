pub mod eastmoney;
pub mod fund_provider;
pub mod generic_http;
pub mod market_provider;
pub mod mock_fund;
pub mod mock_market;
pub mod yahoo_market;

pub use eastmoney::EastMoneyFundProvider;
pub use fund_provider::FundProvider;
pub use generic_http::GenericHttpFundProvider;
pub use market_provider::MarketDataProvider;
pub use mock_fund::MockFundProvider;
pub use mock_market::MockMarketProvider;
pub use yahoo_market::YahooMarketProvider;

use crate::models::{ApiConfig, MarketConfig};

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

pub fn create_market_provider(config: &MarketConfig) -> Box<dyn MarketDataProvider> {
    match config.default_market_provider.as_str() {
        "yahoo" => Box::new(YahooMarketProvider::new(
            config.market_provider_timeout_seconds,
        )),
        _ => Box::new(MockMarketProvider::new()),
    }
}

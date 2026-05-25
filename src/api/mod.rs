pub mod fund_provider;
pub mod generic_http;
pub mod mock_fund;

pub use fund_provider::FundProvider;
pub use generic_http::GenericHttpFundProvider;
pub use mock_fund::MockFundProvider;

use crate::models::ApiConfig;

pub fn create_fund_provider(config: &ApiConfig) -> Box<dyn FundProvider> {
    match config.default_fund_provider.as_str() {
        "generic_http" => Box::new(GenericHttpFundProvider::new(
            config.fund_provider_timeout_seconds,
            config.fund_provider_retry_count,
        )),
        _ => Box::new(MockFundProvider::new()),
    }
}

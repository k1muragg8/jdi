use super::fund_provider::FundProvider;
use crate::models::{FundInfo, FundNav};
use anyhow::{Result, anyhow};

pub struct GenericHttpFundProvider {
    pub timeout: u64,
    pub retry: u32,
}

impl GenericHttpFundProvider {
    pub fn new(timeout: u64, retry: u32) -> Self {
        Self { timeout, retry }
    }
}

impl FundProvider for GenericHttpFundProvider {
    fn fetch_latest_nav(&self, fund_code: &str) -> Result<FundNav> {
        // This is a placeholder for real HTTP implementation
        Err(anyhow!(
            "HTTP provider not fully implemented for code: {}",
            fund_code
        ))
    }

    fn search_fund_by_code(&self, fund_code: &str) -> Result<FundInfo> {
        // This is a placeholder for real HTTP implementation
        Err(anyhow!(
            "HTTP provider not fully implemented for code: {}",
            fund_code
        ))
    }
}

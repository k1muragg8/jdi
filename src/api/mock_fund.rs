use super::fund_provider::FundProvider;
use crate::models::{FundInfo, FundNav};
use anyhow::{Result, anyhow};

pub struct MockFundProvider;

impl Default for MockFundProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockFundProvider {
    pub fn new() -> Self {
        Self
    }
}

impl FundProvider for MockFundProvider {
    fn fetch_latest_nav(&self, fund_code: &str) -> Result<FundNav> {
        match fund_code {
            "006327" => Ok(FundNav {
                fund_code: "006327".to_string(),
                nav: 5.38,
                accumulated_nav: Some(5.38),
                nav_date: "2026-05-22".to_string(),
                currency: "CNY".to_string(),
                source: "mock".to_string(),
                is_stale: false,
                is_estimated: false,
            }),
            "000001" => Ok(FundNav {
                fund_code: "000001".to_string(),
                nav: 1.25,
                accumulated_nav: Some(3.50),
                nav_date: "2026-05-22".to_string(),
                currency: "CNY".to_string(),
                source: "mock".to_string(),
                is_stale: false,
                is_estimated: false,
            }),
            _ => Err(anyhow!("Fund not found for code: {}", fund_code)),
        }
    }

    fn search_fund_by_code(&self, fund_code: &str) -> Result<FundInfo> {
        match fund_code {
            "006327" => Ok(FundInfo {
                fund_code: "006327".to_string(),
                fund_name: "纳斯达克100基金".to_string(),
                fund_type: "QDII".to_string(),
                currency: "CNY".to_string(),
                source: "mock".to_string(),
            }),
            "000001" => Ok(FundInfo {
                fund_code: "000001".to_string(),
                fund_name: "标普500基金".to_string(),
                fund_type: "指数型".to_string(),
                currency: "CNY".to_string(),
                source: "mock".to_string(),
            }),
            _ => Err(anyhow!("Fund not found for code: {}", fund_code)),
        }
    }
}

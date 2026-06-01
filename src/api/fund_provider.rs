use crate::models::{FundInfo, FundNav};
use anyhow::Result;

pub trait FundProvider {
    fn fetch_latest_nav(&self, fund_code: &str) -> Result<FundNav>;
    fn fetch_nav_history(&self, fund_code: &str) -> Result<Vec<FundNav>>;
    fn search_fund_by_code(&self, fund_code: &str) -> Result<FundInfo>;
}

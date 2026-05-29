use super::fund_provider::FundProvider;
use crate::models::{FundInfo, FundNav};
use anyhow::{Context, Result, anyhow};
use chrono::{TimeZone, Utc};
use serde_json::Value;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct EastMoneyFundProvider {
    client: reqwest::blocking::Client,
}

impl EastMoneyFundProvider {
    pub fn new(timeout: u64) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(timeout))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .build()
            .unwrap_or_default();
        Self { client }
    }

    fn fetch_js_data(&self, fund_code: &str) -> Result<String> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let url = format!(
            "https://fund.eastmoney.com/pingzhongdata/{}.js?v={}",
            fund_code, ts
        );

        let resp = self.client.get(&url).send().map_err(|e| {
            anyhow!(
                "Failed to send request to EastMoney pingzhongdata API: {} (URL: {}, Provider: eastmoney)",
                e,
                url
            )
        })?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "EastMoney pingzhongdata API returned status: {} (URL: {}, Provider: eastmoney)",
                resp.status(),
                url
            ));
        }

        resp.text()
            .context("Failed to read EastMoney response as text")
    }

    fn parse_variable(content: &str, var_name: &str) -> Result<String> {
        // Simple search for var name = "value"; or var name = [value];
        let pattern = format!("var {} = ", var_name);
        if let Some(start_idx) = content.find(&pattern) {
            let value_start = start_idx + pattern.len();
            if let Some(end_idx) = content[value_start..].find(';') {
                let mut value = content[value_start..value_start + end_idx].trim();
                // Remove quotes if present
                if (value.starts_with('\"') && value.ends_with('\"'))
                    || (value.starts_with('\'') && value.ends_with('\''))
                {
                    value = &value[1..value.len() - 1];
                }
                return Ok(value.to_string());
            }
        }
        Err(anyhow!("Variable {} not found in JS content", var_name))
    }
}

impl FundProvider for EastMoneyFundProvider {
    fn fetch_latest_nav(&self, fund_code: &str) -> Result<FundNav> {
        let content = self.fetch_js_data(fund_code)?;

        // Parse netWorthTrend
        let net_worth_json = Self::parse_variable(&content, "Data_netWorthTrend")?;
        let net_worth_data: Value = serde_json::from_str(&net_worth_json)
            .context("Failed to parse Data_netWorthTrend as JSON")?;

        let last_item = net_worth_data
            .as_array()
            .and_then(|a| a.last())
            .ok_or_else(|| anyhow!("No NAV data found for fund code: {}", fund_code))?;

        let timestamp = last_item["x"]
            .as_i64()
            .ok_or_else(|| anyhow!("Missing timestamp in NAV data"))?;
        let nav = last_item["y"]
            .as_f64()
            .ok_or_else(|| anyhow!("Missing NAV value"))?;

        // Convert timestamp (ms) to date string
        let dt = Utc.timestamp_millis_opt(timestamp).unwrap();
        let nav_date = dt.format("%Y-%m-%d").to_string();

        // Parse accumulated NAV if available
        let mut accumulated_nav = None;
        if let Some(ac_worth_data) = Self::parse_variable(&content, "Data_ACWorthTrend")
            .ok()
            .and_then(|json| serde_json::from_str::<Value>(&json).ok())
        {
            if let Some(last_ac_item) = ac_worth_data.as_array().and_then(|a| a.last()) {
                // Data_ACWorthTrend items are [timestamp, value]
                if let Some(val) = last_ac_item
                    .as_array()
                    .and_then(|a| a.get(1))
                    .and_then(|v| v.as_f64())
                {
                    accumulated_nav = Some(val);
                }
            }
        }

        Ok(FundNav {
            fund_code: fund_code.to_string(),
            nav,
            accumulated_nav,
            nav_date,
            currency: "CNY".to_string(),
            source: "eastmoney".to_string(),
            is_stale: false,
            is_estimated: false,
        })
    }

    fn search_fund_by_code(&self, fund_code: &str) -> Result<FundInfo> {
        let content = self.fetch_js_data(fund_code).map_err(|_| {
            anyhow!(
                "未找到基金代码 {}，或东方财富未返回有效基金信息。",
                fund_code
            )
        })?;

        let fund_name = Self::parse_variable(&content, "fS_name").map_err(|_| {
            anyhow!(
                "未找到基金代码 {}，或东方财富未返回有效基金信息。",
                fund_code
            )
        })?;

        // Optional variables
        let fund_type =
            Self::parse_variable(&content, "fS_type").unwrap_or_else(|_| "基金".to_string());

        Ok(FundInfo {
            fund_code: fund_code.to_string(),
            fund_name,
            fund_type,
            currency: "CNY".to_string(),
            source: "eastmoney".to_string(),
        })
    }
}

use crate::api::{FundProvider, MockFundProvider};
use crate::models::{ConfigRoot, NavCache, NavCacheEntry, PortfolioState};
use anyhow::Result;
use chrono::{Local, NaiveDate};

pub fn mark_to_market(
    config: &ConfigRoot,
    state: &mut PortfolioState,
    fund_provider: &dyn FundProvider,
    cache: &mut NavCache,
) -> Result<()> {
    let now = Local::now();
    let fetched_at = now.to_rfc3339();

    for holding in &mut state.asset_holdings {
        let asset_config = config
            .assets
            .iter()
            .find(|a| a.asset_id == holding.asset_id && a.enabled);

        if let Some(asset_config) = asset_config {
            if holding.fund_code.is_empty() {
                holding.fund_code = asset_config.fund_code.clone();
            }

            let mut nav_data = None;

            // 1. Try primary provider
            match fund_provider.fetch_latest_nav(&holding.fund_code) {
                Ok(data) => {
                    let mut data = data;
                    data.is_stale = false;
                    nav_data = Some(data);

                    // Check for name mismatch if we have fund info
                    if let Some(info) = fund_provider
                        .search_fund_by_code(&holding.fund_code)
                        .ok()
                        .filter(|info| asset_config.fund_name != info.fund_name)
                    {
                        println!(
                            "警告：资产 {} 的本地基金名称与真实基金名称不一致。",
                            holding.asset_id
                        );
                        println!("本地名称：{}", asset_config.fund_name);
                        println!("真实名称：{}", info.fund_name);
                    }
                }
                Err(_) => {
                    println!(
                        "警告：基金 {} 净值获取失败，尝试从缓存获取。",
                        holding.fund_code
                    );
                }
            }

            // 2. Try cache if primary failed
            if nav_data.is_none() {
                if let Some(entry) = cache
                    .entries
                    .iter()
                    .find(|e| e.fund_code == holding.fund_code)
                {
                    let is_stale = if let Ok(nav_date) =
                        NaiveDate::parse_from_str(&entry.nav_date, "%Y-%m-%d")
                    {
                        let days = now
                            .naive_local()
                            .date()
                            .signed_duration_since(nav_date)
                            .num_days();
                        days > config.api.fund_nav_stale_days
                    } else {
                        true
                    };

                    nav_data = Some(crate::models::FundNav {
                        fund_code: entry.fund_code.clone(),
                        nav: entry.nav,
                        accumulated_nav: entry.accumulated_nav,
                        nav_date: entry.nav_date.clone(),
                        currency: entry.currency.clone(),
                        source: entry.source.clone(),
                        is_stale,
                        is_estimated: false,
                    });
                }
            }

            // 3. Try Mock fallback if allowed and still no data
            if nav_data.is_none() && config.api.allow_mock_fallback {
                if let Ok(data) = MockFundProvider::new().fetch_latest_nav(&holding.fund_code) {
                    nav_data = Some(data);
                }
            }

            // Update state and cache if we got something
            if let Some(data) = nav_data {
                holding.latest_nav = Some(data.nav);
                holding.latest_nav_date = Some(data.nav_date.clone());
                holding.latest_nav_source = Some(data.source.clone());

                let status = if data.source == "mock" {
                    "模拟".to_string()
                } else if data.is_stale {
                    "过期".to_string()
                } else if data.is_estimated {
                    "估算".to_string()
                } else {
                    "正常".to_string()
                };
                holding.latest_nav_status = Some(status);

                if holding.units > 0.0 {
                    holding.last_market_value = holding.units * data.nav;
                }

                // Update cache if the source was a provider (not cache itself)
                // Actually, if we got it from primary provider, we update cache.
                // If we got it from mock fallback, we could also update cache?
                // Let's only update cache if it's not from cache.
                if !data.is_stale || data.source != "cache" {
                    // source "mock" or "generic_http" etc.
                    let entry = NavCacheEntry {
                        fund_code: data.fund_code.clone(),
                        nav: data.nav,
                        accumulated_nav: data.accumulated_nav,
                        nav_date: data.nav_date.clone(),
                        currency: data.currency.clone(),
                        source: data.source.clone(),
                        fetched_at: fetched_at.clone(),
                    };

                    if let Some(existing) = cache
                        .entries
                        .iter_mut()
                        .find(|e| e.fund_code == data.fund_code)
                    {
                        *existing = entry;
                    } else {
                        cache.entries.push(entry);
                    }
                }
            } else {
                holding.latest_nav_status = Some("获取失败".to_string());
                println!("警告：基金 {} 净值获取失败，已跳过。", holding.fund_code);
            }
        }
    }

    Ok(())
}

use crate::api;
use crate::engine;
use crate::models;
use crate::repository::{Repository, RepositoryContext};
use anyhow::Result;
use chrono::Local;

pub async fn refresh_fund_navs(
    repo: &dyn Repository,
    ctx: &RepositoryContext,
    config: &models::ConfigRoot,
) -> Result<usize> {
    let (mut cache, results) = {
        let config = config.clone();
        let cache = repo.load_nav_cache(ctx).await?;
        tokio::task::spawn_blocking(move || {
            let provider = api::create_fund_provider(&config.api);
            let mut results = Vec::new();
            for asset in &config.assets {
                if !asset.enabled {
                    continue;
                }
                let res = provider.fetch_latest_nav(&asset.fund_code);
                results.push((asset.asset_id.clone(), asset.fund_code.clone(), res));
            }
            (cache, results)
        })
        .await?
    };

    let mut success_count = 0;
    for (_asset_id, fund_code, nav_res) in results {
        if let Ok(nav) = nav_res {
            success_count += 1;
            if let Some(entry) = cache.entries.iter_mut().find(|e| e.fund_code == fund_code) {
                entry.nav = nav.nav;
                entry.accumulated_nav = nav.accumulated_nav;
                entry.nav_date = nav.nav_date;
                entry.fetched_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            } else {
                cache.entries.push(models::NavCacheEntry {
                    fund_code: fund_code.clone(),
                    nav: nav.nav,
                    accumulated_nav: nav.accumulated_nav,
                    nav_date: nav.nav_date,
                    currency: "CNY".to_string(),
                    source: "eastmoney".to_string(),
                    fetched_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                });
            }
        }
    }

    repo.save_nav_cache(ctx, &cache).await?;
    Ok(success_count)
}

pub async fn refresh_market_data(
    repo: &dyn Repository,
    ctx: &RepositoryContext,
    config: &models::ConfigRoot,
) -> Result<usize> {
    let symbols: Vec<String> = config
        .assets
        .iter()
        .filter(|a| a.enabled)
        .filter_map(|a| {
            a.reference_instrument_symbol
                .clone()
                .or(a.reference_index_symbol.clone())
        })
        .collect();

    if symbols.is_empty() {
        return Ok(0);
    }

    let (mut cache, mut regime_cache, results) = {
        let config = config.clone();
        let symbols = symbols.clone();
        let cache = repo.load_market_cache(ctx).await?;
        let regime_cache = repo.load_regime_cache(ctx).await?;
        tokio::task::spawn_blocking(move || {
            let provider = api::create_market_provider(&config.market, Some("yahoo"));
            let mut results = Vec::new();
            for sym in &symbols {
                let price_res = provider.fetch_latest_price(sym);
                let regime_res = if price_res.is_ok() {
                    provider.fetch_daily_candles(sym, config.regime.default_lookback_days)
                } else {
                    Err(anyhow::anyhow!("Price fetch failed"))
                };
                results.push((sym.clone(), price_res, regime_res));
            }
            (cache, regime_cache, results)
        })
        .await?
    };

    let mut success_count = 0;
    for (sym, price_res, regime_res) in results {
        if let Ok(price) = price_res {
            success_count += 1;
            if let Some(entry) = cache.entries.iter_mut().find(|e| e.symbol == sym) {
                entry.price = price.price;
                entry.date = price.date;
                entry.fetched_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            } else {
                cache.entries.push(models::MarketCacheEntry {
                    symbol: sym.clone(),
                    price: price.price,
                    date: price.date,
                    currency: price.currency,
                    source: price.source,
                    fetched_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                });
            }

            if let Ok(candles) = regime_res {
                let regime =
                    engine::regime::calculate_market_regime(&sym, &candles, &config.regime);
                if let Some(entry) = regime_cache.entries.iter_mut().find(|e| e.symbol == sym) {
                    entry.result = regime;
                } else {
                    regime_cache.entries.push(models::RegimeCacheEntry {
                        symbol: sym.clone(),
                        result: regime,
                    });
                }
            }
        }
    }

    regime_cache.fetched_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    repo.save_market_cache(ctx, &cache).await?;
    repo.save_regime_cache(ctx, &regime_cache).await?;

    Ok(success_count)
}

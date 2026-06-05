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
) -> Result<(usize, usize, usize)> {
    let mut symbols: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 1. From Asset Configs
    for a in &config.assets {
        if a.enabled {
            if let Some(s) = &a.reference_instrument_symbol {
                symbols.insert(s.clone());
            }
            if let Some(s) = &a.reference_index_symbol {
                symbols.insert(s.clone());
            }
        }
    }

    // 2. From Risk Config
    symbols.insert(config.risk.vix_symbol.clone());
    symbols.insert(config.risk.us30y_symbol.clone());
    for s in &config.risk.crypto_symbols {
        symbols.insert(s.clone());
    }
    for s in &config.risk.equity_symbols {
        symbols.insert(s.clone());
    }

    // 3. From FX Config
    symbols.insert(config.fx.usd_cnh_symbol.clone());

    // 4. From Instrument Registry (active watchlist only)
    if let Ok(instruments) = repo.load_instruments(ctx).await {
        for inst in instruments {
            if inst.enabled && !engine::instrument_watchlist::is_instrument_archived(&inst) {
                symbols.insert(inst.symbol.clone());
            }
        }
    }

    // 5. From current holdings (if they have benchmark mapping)
    if let Ok(state) = repo.load_state(ctx).await {
        for holding in &state.asset_holdings {
            if holding.units > 0.0 {
                if let Some(asset) = config
                    .assets
                    .iter()
                    .find(|a| a.asset_id == holding.asset_id)
                {
                    if let Some(s) = &asset.reference_instrument_symbol {
                        symbols.insert(s.clone());
                    }
                    if let Some(s) = &asset.reference_index_symbol {
                        symbols.insert(s.clone());
                    }
                }
            }
        }
    }

    // Build fetch list: cache key (display symbol) -> (provider, provider_symbol)
    let mut fetch_pairs: Vec<(String, String, String)> = Vec::new();
    let mut instruments = repo.load_instruments(ctx).await.unwrap_or_default();
    let mut instruments_dirty = false;
    for inst in instruments.iter_mut() {
        let before_provider = inst.provider.clone();
        let before_sym = inst.provider_symbol.clone();
        engine::instrument_watchlist::migrate_au9999_provider(inst);
        if inst.provider != before_provider || inst.provider_symbol != before_sym {
            instruments_dirty = true;
        }
    }
    if instruments_dirty {
        let _ = repo.save_instruments(ctx, &instruments).await;
    }
    let inst_by_symbol: std::collections::HashMap<String, &models::InstrumentConfig> =
        instruments.iter().map(|i| (i.symbol.clone(), i)).collect();

    for inst in &instruments {
        if inst.enabled && !engine::instrument_watchlist::is_instrument_archived(inst) {
            let fetch_sym = if inst.provider_symbol.is_empty() {
                inst.symbol.clone()
            } else {
                inst.provider_symbol.clone()
            };
            fetch_pairs.push((inst.symbol.clone(), inst.provider.clone(), fetch_sym));
        }
    }
    for sym in symbols {
        if sym.is_empty() {
            continue;
        }
        if fetch_pairs.iter().any(|(k, _, _)| k == &sym) {
            continue;
        }
        if let Some(inst) = inst_by_symbol.get(&sym) {
            let fetch_sym = if inst.provider_symbol.is_empty() {
                sym.clone()
            } else {
                inst.provider_symbol.clone()
            };
            fetch_pairs.push((sym.clone(), inst.provider.clone(), fetch_sym));
        } else {
            fetch_pairs.push((sym.clone(), "yahoo".to_string(), sym.clone()));
        }
    }

    if fetch_pairs.is_empty() {
        return Ok((0, 0, 0));
    }

    let (mut cache, mut regime_cache, results) = {
        let config = config.clone();
        let fetch_pairs = fetch_pairs.clone();
        let cache = repo.load_market_cache(ctx).await?;
        let regime_cache = repo.load_regime_cache(ctx).await?;
        tokio::task::spawn_blocking(move || {
            let mut results = Vec::new();
            for (cache_key, provider_name, fetch_sym) in &fetch_pairs {
                let provider =
                    api::create_market_provider(&config.market, Some(provider_name.as_str()));
                let price_res = provider.fetch_latest_price(fetch_sym);
                let regime_res = if price_res.is_ok() {
                    provider.fetch_daily_candles(fetch_sym, config.regime.default_lookback_days)
                } else {
                    Err(anyhow::anyhow!("Price fetch failed"))
                };
                results.push((
                    cache_key.clone(),
                    provider_name.clone(),
                    price_res,
                    regime_res,
                ));
            }
            (cache, regime_cache, results)
        })
        .await?
    };

    let mut success_count = 0;
    let mut failed_count = 0;

    let fetched_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    for (cache_key, provider_name, price_res, regime_res) in results {
        if let Ok(price) = price_res {
            let price = engine::market_quote::normalize_market_price(price);
            success_count += 1;
            if let Some(entry) = cache.entries.iter_mut().find(|e| e.symbol == cache_key) {
                engine::market_quote::apply_price_to_cache_entry(entry, &price, &fetched_at);
            } else {
                cache
                    .entries
                    .push(engine::market_quote::new_cache_entry_from_price(
                        &price,
                        &cache_key,
                        &fetched_at,
                    ));
            }

            if let Ok(candles) = regime_res {
                let regime =
                    engine::regime::calculate_market_regime(&cache_key, &candles, &config.regime);
                if let Some(entry) = regime_cache
                    .entries
                    .iter_mut()
                    .find(|e| e.symbol == cache_key)
                {
                    entry.result = regime;
                } else {
                    regime_cache.entries.push(models::RegimeCacheEntry {
                        symbol: cache_key.clone(),
                        result: regime,
                    });
                }
            }
        } else {
            failed_count += 1;
            let err_msg = price_res
                .err()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "fetch failed".to_string());
            let currency = if provider_name == "eastmoney" {
                "CNY"
            } else {
                "USD"
            };
            if let Some(entry) = cache.entries.iter_mut().find(|e| e.symbol == cache_key) {
                entry.status = Some("failed".to_string());
                entry.error_message = Some(err_msg.clone());
                entry.source = provider_name.clone();
                entry.fetched_at = fetched_at.clone();
            } else {
                cache.entries.push(models::MarketCacheEntry {
                    symbol: cache_key.clone(),
                    price: 0.0,
                    date: Local::now().format("%Y-%m-%d").to_string(),
                    currency: currency.to_string(),
                    source: provider_name.clone(),
                    fetched_at: fetched_at.clone(),
                    previous_close: None,
                    change: None,
                    change_percent: None,
                    status: Some("failed".to_string()),
                    error_message: Some(err_msg),
                });
            }
        }
    }

    regime_cache.fetched_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    repo.save_market_cache(ctx, &cache).await?;
    repo.save_regime_cache(ctx, &regime_cache).await?;

    Ok((success_count, 0, failed_count))
}

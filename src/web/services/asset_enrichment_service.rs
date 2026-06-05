//! Web-facing asset/fund enrichment (provider + persistence).

use crate::api;
use crate::engine::asset_enrichment::{FundLookupResult, apply_fund_info_to_asset, lookup_fund};
use crate::models::{ConfigRoot, FundNav, NavCacheEntry};
use crate::repository::RepositoryContext;
use crate::web::state::AppState;
use anyhow::Result;
use std::sync::Arc;

pub fn fund_provider_for(config: &ConfigRoot) -> Box<dyn api::FundProvider> {
    api::create_fund_provider(&config.api)
}

pub fn lookup_fund_code(config: &ConfigRoot, fund_code: &str) -> FundLookupResult {
    let provider = fund_provider_for(config);
    lookup_fund(provider.as_ref(), fund_code)
}

pub async fn enrich_asset_by_id(
    state: &Arc<AppState>,
    ctx: &RepositoryContext,
    asset_id: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut config = state.repo.load_config(ctx).await?;
    let (changed, warnings, nav_opt) = {
        let idx = config
            .assets
            .iter()
            .position(|a| a.asset_id == asset_id)
            .ok_or_else(|| anyhow::anyhow!("资产未找到"))?;
        let fund_code = config.assets[idx].fund_code.clone();
        if fund_code.trim().is_empty() {
            anyhow::bail!("请先填写基金代码");
        }
        let lookup = lookup_fund_code(&config, &fund_code);
        if !lookup.success {
            anyhow::bail!(lookup.message.unwrap_or_else(|| "基金查询失败".to_string()));
        }
        let info = crate::models::FundInfo {
            fund_code: lookup.fund_code.clone(),
            fund_name: lookup.fund_name.clone().unwrap_or_default(),
            fund_type: lookup.fund_type.clone().unwrap_or_default(),
            currency: lookup.currency.clone().unwrap_or_else(|| "CNY".to_string()),
            source: lookup.source.clone().unwrap_or_default(),
        };
        let nav = if let (Some(nav), Some(date)) = (lookup.nav, lookup.nav_date) {
            Some(FundNav {
                fund_code: lookup.fund_code.clone(),
                nav,
                accumulated_nav: None,
                nav_date: date,
                currency: info.currency.clone(),
                source: info.source.clone(),
                is_stale: false,
                is_estimated: false,
            })
        } else {
            None
        };
        let apply = apply_fund_info_to_asset(&mut config.assets[idx], &info, nav.as_ref());
        (apply.changed_fields, apply.warnings, nav)
    };
    state.repo.save_config(ctx, &config).await?;
    if let Some(n) = nav_opt {
        let mut cache = state.repo.load_nav_cache(ctx).await.unwrap_or_default();
        let fetched_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let entry = NavCacheEntry {
            fund_code: n.fund_code.clone(),
            nav: n.nav,
            accumulated_nav: n.accumulated_nav,
            nav_date: n.nav_date.clone(),
            currency: n.currency.clone(),
            source: n.source.clone(),
            fetched_at,
        };
        if let Some(e) = cache
            .entries
            .iter_mut()
            .find(|e| e.fund_code == entry.fund_code)
        {
            *e = entry;
        } else {
            cache.entries.push(entry);
        }
        state.repo.save_nav_cache(ctx, &cache).await?;
    }
    Ok((changed, warnings))
}

pub async fn enrich_all_assets(
    state: &Arc<AppState>,
    ctx: &RepositoryContext,
) -> Result<(usize, Vec<String>)> {
    let mut config = state.repo.load_config(ctx).await?;
    let (changed, warnings) = {
        let provider = fund_provider_for(&config);
        let mut changed = 0usize;
        let mut warnings = Vec::new();
        for asset in &mut config.assets {
            if !asset.enabled || asset.fund_code.trim().is_empty() {
                continue;
            }
            let lookup = lookup_fund(provider.as_ref(), &asset.fund_code);
            if !lookup.success {
                warnings.push(format!(
                    "{}: {}",
                    asset.asset_id,
                    lookup.message.unwrap_or_default()
                ));
                continue;
            }
            let info = crate::models::FundInfo {
                fund_code: lookup.fund_code.clone(),
                fund_name: lookup.fund_name.clone().unwrap_or_default(),
                fund_type: lookup.fund_type.clone().unwrap_or_default(),
                currency: lookup.currency.clone().unwrap_or_else(|| "CNY".to_string()),
                source: lookup.source.clone().unwrap_or_default(),
            };
            let nav = lookup.nav.map(|nav| FundNav {
                fund_code: lookup.fund_code.clone(),
                nav,
                accumulated_nav: None,
                nav_date: lookup.nav_date.clone().unwrap_or_default(),
                currency: info.currency.clone(),
                source: info.source.clone(),
                is_stale: false,
                is_estimated: false,
            });
            let apply = apply_fund_info_to_asset(asset, &info, nav.as_ref());
            if !apply.changed_fields.is_empty() {
                changed += 1;
            }
            warnings.extend(apply.warnings);
        }
        (changed, warnings)
    };
    state.repo.save_config(ctx, &config).await?;
    Ok((changed, warnings))
}

pub async fn auto_classify_assets(state: &Arc<AppState>, ctx: &RepositoryContext) -> Result<usize> {
    let mut config = state.repo.load_config(ctx).await?;
    let changed = crate::engine::classify_unassigned_assets(&mut config.assets);
    if changed > 0 {
        state.repo.save_config(ctx, &config).await?;
    }
    Ok(changed)
}

pub fn asset_row_source(asset: &crate::models::AssetConfig) -> &'static str {
    if asset.fund_code.is_empty() {
        "手动"
    } else if asset.market_data_provider.as_deref() == Some("eastmoney") {
        "基金/NAV"
    } else {
        "配置"
    }
}

//! Holdings page data (repository only; no HTML).

use crate::engine;
use crate::models::{
    AlipaySnapshot, ConfigRoot, FundNav, PortfolioState, PortfolioSummary, WebAdminAudit,
};
use crate::web::state::AppState;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub struct HoldingsPageData {
    pub config: ConfigRoot,
    pub portfolio_state: PortfolioState,
    pub summary: PortfolioSummary,
    pub latest_snaps: HashMap<String, AlipaySnapshot>,
}

pub async fn load_holdings_page(state: &Arc<AppState>) -> Result<HoldingsPageData> {
    let ctx = &state.ctx;
    let config = state.repo.load_config(ctx).await?;
    let portfolio_state = state.repo.load_state(ctx).await?;
    let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
    let snapshots = state
        .repo
        .load_alipay_snapshots(ctx)
        .await
        .unwrap_or_default();

    let mut latest_snaps: HashMap<String, AlipaySnapshot> = HashMap::new();
    for s in &snapshots {
        let key = if s.asset_id.is_empty() {
            format!("unmatched_{}", s.fund_code)
        } else {
            s.asset_id.clone()
        };
        let entry = latest_snaps.entry(key).or_insert(s.clone());
        if s.snapshot_date >= entry.snapshot_date {
            *entry = s.clone();
        }
    }

    Ok(HoldingsPageData {
        config,
        portfolio_state,
        summary,
        latest_snaps,
    })
}

/// Compute latest snapshots keyed by asset or unmatched fund.
fn latest_alipay_snapshots(snaps: &[AlipaySnapshot]) -> HashMap<String, AlipaySnapshot> {
    let mut latest: HashMap<String, AlipaySnapshot> = HashMap::new();
    for s in snaps {
        let key = if s.asset_id.is_empty() {
            format!("unmatched_{}", s.fund_code)
        } else {
            s.asset_id.clone()
        };
        let e = latest.entry(key).or_insert_with(|| s.clone());
        if s.snapshot_date >= e.snapshot_date {
            *e = s.clone();
        }
    }
    latest
}

pub async fn bootstrap_holdings_from_alipay(state: &Arc<AppState>) -> Result<String> {
    let ctx = &state.ctx;
    let mut config = state.repo.load_config(ctx).await?;
    let portfolio_state = state.repo.load_state(ctx).await?;
    let snapshots = state.repo.load_alipay_snapshots(ctx).await?;
    let latest = latest_alipay_snapshots(&snapshots);
    let candidates = crate::web::product::snapshots_to_candidates(&latest);
    if candidates.is_empty() {
        anyhow::bail!("无支付宝快照可初始化");
    }
    let (created, _, _) =
        engine::alipay_holding::bootstrap_assets_from_holdings(&mut config, &candidates);
    state.repo.save_config(ctx, &config).await?;
    let nav_cache = state.repo.load_nav_cache(ctx).await.unwrap_or_default();
    let nav_map: HashMap<String, FundNav> = nav_cache
        .entries
        .iter()
        .map(|e| {
            (
                e.fund_code.clone(),
                FundNav {
                    fund_code: e.fund_code.clone(),
                    nav: e.nav,
                    accumulated_nav: e.accumulated_nav,
                    nav_date: e.nav_date.clone(),
                    currency: e.currency.clone(),
                    source: e.source.clone(),
                    is_stale: false,
                    is_estimated: false,
                },
            )
        })
        .collect();
    let preview = engine::alipay_holding::preview_bootstrap_local(
        &config,
        &portfolio_state,
        &candidates,
        &nav_map,
        true,
    );
    let (new_state, n) = engine::alipay_holding::apply_bootstrap_local(portfolio_state, &preview);
    state.repo.save_state(ctx, &new_state).await?;
    Ok(format!(
        "已用支付宝快照初始化 {} 项持仓（新建资产 {} 个）",
        n, created
    ))
}

fn make_audit(
    ctx: &crate::repository::RepositoryContext,
    action: &str,
    target_id: &str,
    oldv: &str,
    newv: &str,
) -> WebAdminAudit {
    WebAdminAudit {
        audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
        timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        actor: "local_web".to_string(),
        actor_user_id: Some(ctx.actor_user_id.clone()),
        target_user_id: Some(ctx.target_user_id.clone()),
        portfolio_id: Some(ctx.portfolio_id.clone()),
        role: Some(ctx.role.clone()),
        action: action.to_string(),
        target_file: "config.toml".to_string(),
        target_id: Some(target_id.to_string()),
        old_value_summary: oldv.to_string(),
        new_value_summary: newv.to_string(),
        status: "success".to_string(),
        note: None,
    }
}

pub async fn set_asset_fund_code(
    state: &Arc<AppState>,
    asset_id: &str,
    fund_code: &str,
) -> Result<()> {
    let ctx = &state.ctx;
    let mut config = state.repo.load_config(ctx).await?;
    if let Some(a) = config.assets.iter_mut().find(|a| a.asset_id == asset_id) {
        let old = a.fund_code.clone();
        a.fund_code = fund_code.to_string();
        state.repo.save_config(ctx, &config).await?;
        let audit = make_audit(
            ctx,
            "set_asset_fund_code",
            asset_id,
            &format!("fund_code: {}", old),
            &format!("fund_code: {}", fund_code),
        );
        state.repo.append_web_admin_audit(ctx, audit).await?;
        Ok(())
    } else {
        anyhow::bail!("资产未找到")
    }
}

pub async fn rename_asset(state: &Arc<AppState>, asset_id: &str, fund_name: &str) -> Result<()> {
    let ctx = &state.ctx;
    let mut config = state.repo.load_config(ctx).await?;
    if let Some(a) = config.assets.iter_mut().find(|a| a.asset_id == asset_id) {
        let old = a.fund_name.clone();
        a.fund_name = fund_name.to_string();
        state.repo.save_config(ctx, &config).await?;
        let audit = make_audit(
            ctx,
            "rename_asset",
            asset_id,
            &format!("fund_name: {}", old),
            &format!("fund_name: {}", fund_name),
        );
        state.repo.append_web_admin_audit(ctx, audit).await?;
        Ok(())
    } else {
        anyhow::bail!("资产未找到")
    }
}

pub async fn set_asset_sector(state: &Arc<AppState>, asset_id: &str, sector: &str) -> Result<()> {
    let ctx = &state.ctx;
    let mut config = state.repo.load_config(ctx).await?;
    if let Some(a) = config.assets.iter_mut().find(|a| a.asset_id == asset_id) {
        let old = a.sector.clone();
        a.sector = sector.to_string();
        state.repo.save_config(ctx, &config).await?;
        let audit = make_audit(
            ctx,
            "set_asset_sector",
            asset_id,
            &format!("sector: {}", old),
            &format!("sector: {}", sector),
        );
        state.repo.append_web_admin_audit(ctx, audit).await?;
        Ok(())
    } else {
        anyhow::bail!("资产未找到")
    }
}

pub async fn set_asset_enabled(state: &Arc<AppState>, asset_id: &str, enabled: bool) -> Result<()> {
    let ctx = &state.ctx;
    let mut config = state.repo.load_config(ctx).await?;
    if let Some(a) = config.assets.iter_mut().find(|a| a.asset_id == asset_id) {
        a.enabled = enabled;
        state.repo.save_config(ctx, &config).await?;
        Ok(())
    } else {
        anyhow::bail!("资产未找到")
    }
}

pub async fn add_asset(
    state: &Arc<AppState>,
    fund_name: &str,
    fund_code: &str,
    sector: Option<&str>,
) -> Result<String> {
    let ctx = &state.ctx;
    let mut config = state.repo.load_config(ctx).await?;
    let asset_id = if fund_code.trim().is_empty() {
        format!("asset_{}", chrono::Local::now().timestamp_millis())
    } else {
        fund_code.to_string()
    };
    if config.assets.iter().any(|a| a.asset_id == asset_id) {
        anyhow::bail!("资产 ID {} 已存在", asset_id);
    }
    let new_asset = crate::models::AssetConfig {
        asset_id: asset_id.clone(),
        fund_name: fund_name.to_string(),
        fund_code: fund_code.to_string(),
        sector: sector.unwrap_or("未分类").to_string(),
        enabled: true,
        currency: "CNY".to_string(),
        valuation_method: "nav".to_string(),
        reference_index_symbol: None,
        reference_instrument_symbol: None,
        market_data_provider: Some("eastmoney".to_string()),
        ..Default::default()
    };
    config.assets.push(new_asset);
    state.repo.save_config(ctx, &config).await?;
    // audit omitted for brevity, or add
    Ok(asset_id)
}

pub async fn remove_asset(state: &Arc<AppState>, asset_id: &str) -> Result<String> {
    let ctx = &state.ctx;
    let mut config = state.repo.load_config(ctx).await?;
    let mut found = false;
    let mut referenced = false;
    let holdings = state
        .repo
        .load_state(ctx)
        .await
        .unwrap_or_default()
        .asset_holdings;
    let dca_plans: Vec<crate::models::DcaPlan> =
        state.repo.load_plans(ctx).await.unwrap_or_default();
    let snaps = state
        .repo
        .load_alipay_snapshots(ctx)
        .await
        .unwrap_or_default();
    for a in &mut config.assets {
        if a.asset_id == asset_id {
            found = true;
            if holdings.iter().any(|h| h.asset_id == asset_id) {
                referenced = true;
            }
            if dca_plans.iter().any(|d| d.asset_id == asset_id) {
                referenced = true;
            }
            if snaps.iter().any(|s| s.asset_id == asset_id) {
                referenced = true;
            }
            a.enabled = false;
            if !a.sector.contains("已归档") {
                a.sector = if a.sector.is_empty() {
                    "已归档".to_string()
                } else {
                    format!("{} (已归档)", a.sector)
                };
            }
            break;
        }
    }
    if !found {
        anyhow::bail!("资产未找到");
    }
    state.repo.save_config(ctx, &config).await?;
    if referenced {
        Ok("该资产仍被持仓/交易/DCA/快照引用，已改为禁用归档。".to_string())
    } else {
        Ok("资产已禁用/归档。".to_string())
    }
}

pub async fn update_asset(
    state: &Arc<AppState>,
    asset_id: &str,
    fund_name: Option<String>,
    fund_code: Option<String>,
    sector: Option<String>,
    currency: Option<String>,
    enabled: Option<bool>,
    reference_index_symbol: Option<String>,
    reference_instrument_symbol: Option<String>,
    market_data_provider: Option<String>,
    valuation_method: Option<String>,
) -> Result<()> {
    let ctx = &state.ctx;
    let mut config = state.repo.load_config(ctx).await?;
    let asset = config
        .assets
        .iter_mut()
        .find(|a| a.asset_id == asset_id)
        .ok_or_else(|| anyhow::anyhow!("资产未找到"))?;
    if let Some(v) = fund_name {
        asset.fund_name = v;
    }
    if let Some(v) = fund_code {
        asset.fund_code = v;
    }
    if let Some(v) = sector {
        asset.sector = v;
    }
    if let Some(v) = currency {
        asset.currency = v;
    }
    if let Some(v) = enabled {
        asset.enabled = v;
    }
    if let Some(v) = reference_index_symbol {
        asset.reference_index_symbol = Some(v);
    }
    if let Some(v) = reference_instrument_symbol {
        asset.reference_instrument_symbol = Some(v);
    }
    if let Some(v) = market_data_provider {
        asset.market_data_provider = Some(v);
    }
    if let Some(v) = valuation_method {
        asset.valuation_method = v;
    }
    state.repo.save_config(ctx, &config).await?;
    Ok(())
}

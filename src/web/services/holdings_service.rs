//! Holdings page data (repository only; no HTML).

use crate::engine;
use crate::models::{
    ConfigRoot, DcaFrequency, DcaPlan, PortfolioState, PortfolioSummary, WebAdminAudit,
};
use crate::web::state::AppState;
use anyhow::Result;
use std::sync::Arc;

pub struct HoldingsPageData {
    pub config: ConfigRoot,
    pub portfolio_state: PortfolioState,
    pub summary: PortfolioSummary,
    pub dca_plans: Vec<DcaPlan>,
}

pub async fn load_holdings_page(state: &Arc<AppState>) -> Result<HoldingsPageData> {
    let ctx = &state.ctx;
    // resolve any legacy plans linked by fund_code only (no silent loss)
    let _ = resolve_legacy_dca_plans(state).await;
    let config = state.repo.load_config(ctx).await?;
    let portfolio_state = state.repo.load_state(ctx).await?;
    let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
    let dca_plans = state.repo.load_plans(ctx).await.unwrap_or_default();

    Ok(HoldingsPageData {
        config,
        portfolio_state,
        summary,
        dca_plans,
    })
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
    let mut already_arch = false;
    let holdings = state
        .repo
        .load_state(ctx)
        .await
        .unwrap_or_default()
        .asset_holdings;
    let dca_plans: Vec<crate::models::DcaPlan> =
        state.repo.load_plans(ctx).await.unwrap_or_default();
    for a in &mut config.assets {
        if a.asset_id == asset_id {
            found = true;
            already_arch = !a.enabled || a.sector.contains("已归档");
            if !already_arch {
                if holdings.iter().any(|h| h.asset_id == asset_id) {
                    // ref
                }
                if dca_plans.iter().any(|d| d.asset_id == asset_id) {
                    // ref
                }
                a.enabled = false;
                if !a.sector.contains("已归档") {
                    a.sector = if a.sector.is_empty() {
                        "已归档".to_string()
                    } else {
                        format!("{} (已归档)", a.sector)
                    };
                }
            }
            break;
        }
    }
    if !found {
        anyhow::bail!("资产未找到");
    }
    if already_arch {
        // hard delete
        config.assets.retain(|a| a.asset_id != asset_id);
        state.repo.save_config(ctx, &config).await?;
        Ok("资产已永久删除。".to_string())
    } else {
        state.repo.save_config(ctx, &config).await?;
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

pub async fn restore_asset(state: &Arc<AppState>, asset_id: &str) -> Result<()> {
    let ctx = &state.ctx;
    let mut config = state.repo.load_config(ctx).await?;
    if let Some(a) = config.assets.iter_mut().find(|a| a.asset_id == asset_id) {
        a.enabled = true;
        if a.sector.contains("已归档") {
            a.sector = a
                .sector
                .replace(" (已归档)", "")
                .replace("已归档", "")
                .trim()
                .to_string();
            if a.sector.is_empty() {
                a.sector = "未分类".to_string();
            }
        }
        state.repo.save_config(ctx, &config).await?;
        Ok(())
    } else {
        anyhow::bail!("资产未找到")
    }
}

/// Resolve legacy DCA plans (that may have empty/wrong asset_id but matching fund_code) to current assets.
/// Persists if changes made. Returns number of plans updated.
pub async fn resolve_legacy_dca_plans(state: &Arc<AppState>) -> Result<usize> {
    let ctx = &state.ctx;
    let mut plans = state.repo.load_plans(ctx).await.unwrap_or_default();
    if plans.is_empty() {
        return Ok(0);
    }
    let config = state.repo.load_config(ctx).await?;
    let mut updated = 0usize;
    for p in &mut plans {
        let has_match = config.assets.iter().any(|a| a.asset_id == p.asset_id);
        if !has_match || p.asset_id.is_empty() {
            if let Some(a) = config
                .assets
                .iter()
                .find(|a| !a.fund_code.is_empty() && a.fund_code == p.fund_code)
            {
                p.asset_id = a.asset_id.clone();
                p.fund_name = a.fund_name.clone();
                updated += 1;
            }
        }
    }
    if updated > 0 {
        state.repo.save_plans(ctx, &plans).await?;
    }
    Ok(updated)
}

pub async fn upsert_dca_for_asset(
    state: &Arc<AppState>,
    asset_id: &str,
    amount: f64,
    frequency: &str,
    day: Option<u32>,
    note: Option<String>,
) -> Result<String> {
    let ctx = &state.ctx;
    let config = state.repo.load_config(ctx).await?;
    let asset = config
        .assets
        .iter()
        .find(|a| a.asset_id == asset_id)
        .ok_or_else(|| anyhow::anyhow!("资产未找到"))?;
    let mut plans = state.repo.load_plans(ctx).await.unwrap_or_default();
    let freq = match frequency {
        "daily" => DcaFrequency::Daily,
        "weekly" => DcaFrequency::Weekly,
        "monthly" => DcaFrequency::Monthly,
        _ => return Err(anyhow::anyhow!("无效的频率")),
    };
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if let Some(idx) = plans.iter().position(|p| p.asset_id == asset_id) {
        {
            let p = &mut plans[idx];
            p.amount = amount;
            p.frequency = freq.clone();
            if frequency == "weekly" {
                p.weekday = day;
                p.month_day = None;
            } else if frequency == "monthly" {
                p.month_day = day;
                p.weekday = None;
            }
            if let Some(n) = note.clone() {
                p.note = Some(n);
            }
            p.updated_at = now_str.clone();
        }
        state.repo.save_plans(ctx, &plans).await?;
        return Ok(plans[idx].plan_id.clone());
    }
    let plan_id = format!("plan_{}", chrono::Local::now().timestamp_millis());
    let new_plan = DcaPlan {
        plan_id: plan_id.clone(),
        asset_id: asset_id.to_string(),
        fund_code: asset.fund_code.clone(),
        fund_name: asset.fund_name.clone(),
        amount,
        currency: "CNY".to_string(),
        frequency: freq,
        weekday: if frequency == "weekly" { day } else { None },
        month_day: if frequency == "monthly" { day } else { None },
        start_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
        end_date: None,
        enabled: true,
        priority: 0,
        note: note.or(Some("从持仓设置".to_string())),
        created_at: now_str.clone(),
        updated_at: now_str,
    };
    plans.push(new_plan);
    state.repo.save_plans(ctx, &plans).await?;
    Ok(plan_id)
}

pub async fn pause_dca_for_asset(state: &Arc<AppState>, asset_id: &str) -> Result<()> {
    let ctx = &state.ctx;
    let mut plans = state.repo.load_plans(ctx).await?;
    if let Some(p) = plans.iter_mut().find(|p| p.asset_id == asset_id) {
        p.enabled = false;
        p.updated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        state.repo.save_plans(ctx, &plans).await?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("该资产无定投计划"))
    }
}

pub async fn resume_dca_for_asset(state: &Arc<AppState>, asset_id: &str) -> Result<()> {
    let ctx = &state.ctx;
    let mut plans = state.repo.load_plans(ctx).await?;
    if let Some(p) = plans.iter_mut().find(|p| p.asset_id == asset_id) {
        p.enabled = true;
        p.updated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        state.repo.save_plans(ctx, &plans).await?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("该资产无定投计划"))
    }
}

pub async fn archive_dca_for_asset(state: &Arc<AppState>, asset_id: &str) -> Result<()> {
    let ctx = &state.ctx;
    let mut plans = state.repo.load_plans(ctx).await?;
    let before = plans.len();
    plans.retain(|p| p.asset_id != asset_id);
    if plans.len() < before {
        state.repo.save_plans(ctx, &plans).await?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("该资产无定投计划"))
    }
}

//! POST actions: assets

use crate::web::handlers::forms::{AssetIdForm, CashAdjustForm, CashSetForm};
use super::types::*;
use crate::web::state::AppState;
use crate::{engine, models};
use axum::extract::{Form, State};
use axum::response::Redirect;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

pub async fn admin_asset_set_fund_code_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetFundCodeForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut config = state.repo.load_config(&ctx).await?;
        if let Some(a) = config
            .assets
            .iter_mut()
            .find(|a| a.asset_id == form.asset_id)
        {
            let old_code = a.fund_code.clone();
            a.fund_code = form.fund_code.clone();
            state.repo.save_config(&ctx, &config).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "set_asset_fund_code".to_string(),
                target_file: "config.toml".to_string(),
                target_id: Some(form.asset_id.clone()),
                old_value_summary: format!("fund_code: {}", old_code),
                new_value_summary: format!("fund_code: {}", form.fund_code),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("资产未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/holdings?success=基金代码设置成功"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

pub async fn admin_asset_rename_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetRenameForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut config = state.repo.load_config(&ctx).await?;
        if let Some(a) = config
            .assets
            .iter_mut()
            .find(|a| a.asset_id == form.asset_id)
        {
            let old_name = a.fund_name.clone();
            a.fund_name = form.fund_name.clone();
            state.repo.save_config(&ctx, &config).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "rename_asset".to_string(),
                target_file: "config.toml".to_string(),
                target_id: Some(form.asset_id.clone()),
                old_value_summary: format!("fund_name: {}", old_name),
                new_value_summary: format!("fund_name: {}", form.fund_name),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("资产未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/holdings?success=资产更名成功"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

pub async fn admin_asset_set_sector_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetSectorForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut config = state.repo.load_config(&ctx).await?;
        if let Some(a) = config
            .assets
            .iter_mut()
            .find(|a| a.asset_id == form.asset_id)
        {
            let old_sector = a.sector.clone();
            a.sector = form.sector.clone();
            state.repo.save_config(&ctx, &config).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "set_asset_sector".to_string(),
                target_file: "config.toml".to_string(),
                target_id: Some(form.asset_id.clone()),
                old_value_summary: format!("sector: {}", old_sector),
                new_value_summary: format!("sector: {}", form.sector),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("资产未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/holdings?success=资产板块设置成功"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}


pub async fn admin_asset_enable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetIdForm>,
) -> Redirect {
    set_asset_enabled(&state, &form.asset_id, true).await
}


pub async fn admin_asset_disable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetIdForm>,
) -> Redirect {
    set_asset_enabled(&state, &form.asset_id, false).await
}


pub async fn set_asset_enabled(state: &Arc<AppState>, asset_id: &str, enabled: bool) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut config = state.repo.load_config(&ctx).await?;
        let asset = config
            .assets
            .iter_mut()
            .find(|a| a.asset_id == asset_id)
            .ok_or_else(|| anyhow::anyhow!("资产未找到"))?;
        asset.enabled = enabled;
        state.repo.save_config(&ctx, &config).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    match result {
        Ok(_) => Redirect::to(&format!(
            "/holdings?success={}",
            if enabled {
                "资产已启用"
            } else {
                "资产已禁用"
            }
        )),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}


pub async fn admin_asset_add_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetAddForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut config = state.repo.load_config(&ctx).await?;

        // Generate a new asset_id if it doesn't exist
        let asset_id = form.fund_code.clone();
        if config.assets.iter().any(|a| a.asset_id == asset_id) {
            anyhow::bail!("资产 ID {} 已存在", asset_id);
        }

        let new_asset = models::AssetConfig {
            asset_id: asset_id.clone(),
            fund_code: form.fund_code.clone(),
            fund_name: form.fund_name.clone(),
            sector: form.sector.unwrap_or_default(),
            currency: "CNY".to_string(),
            market_data_provider: Some("eastmoney".to_string()),
            enabled: true,
            ..Default::default()
        };

        config.assets.push(new_asset);
        state.repo.save_config(&ctx, &config).await?;

        let audit = models::WebAdminAudit {
            audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            actor: "local_web".to_string(),
            actor_user_id: Some(ctx.actor_user_id.clone()),
            target_user_id: Some(ctx.target_user_id.clone()),
            portfolio_id: Some(ctx.portfolio_id.clone()),
            role: Some(ctx.role.clone()),
            action: "add_asset".to_string(),
            target_file: "config.json".to_string(),
            target_id: Some(asset_id),
            old_value_summary: "".to_string(),
            new_value_summary: format!("{:?}", form.fund_name),
            status: "success".to_string(),
            note: None,
        };
        state.repo.append_web_admin_audit(&ctx, audit).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/holdings?success=资产已添加"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}


pub async fn admin_asset_remove_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetIdForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut config = state.repo.load_config(&ctx).await?;
        let mut found = false;
        let mut referenced = false;
        // Check for references before archive
        let holdings = state
            .repo
            .load_state(&ctx)
            .await
            .unwrap_or_default()
            .asset_holdings;
        let dca_plans: Vec<models::DcaPlan> = state.repo.load_plans(&ctx).await.unwrap_or_default();
        let snaps = state
            .repo
            .load_alipay_snapshots(&ctx)
            .await
            .unwrap_or_default();
        for a in &mut config.assets {
            if a.asset_id == form.asset_id {
                found = true;
                // ref checks
                if holdings.iter().any(|h| h.asset_id == form.asset_id) {
                    referenced = true;
                }
                if dca_plans.iter().any(|d| d.asset_id == form.asset_id) {
                    referenced = true;
                }
                if snaps.iter().any(|s| s.asset_id == form.asset_id) {
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
            return Err(anyhow::anyhow!("资产未找到"));
        }
        state.repo.save_config(&ctx, &config).await?;
        if referenced {
            Ok("该资产仍被持仓/交易/DCA/快照引用，已改为禁用归档。")
        } else {
            Ok("资产已禁用/归档。")
        }
    }
    .await;

    match result {
        Ok(msg) => Redirect::to(&format!("/holdings?success={}", msg)),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}


pub async fn api_assets_auto_classify_handler(State(state): State<Arc<AppState>>) -> Redirect {
    let ctx = &state.ctx;
    match crate::web::services::asset_enrichment_service::auto_classify_assets(&state, ctx).await
    {
        Ok(n) => Redirect::to(&format!(
            "/holdings?success=自动分类完成，更新 {} 个资产",
            n
        )),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

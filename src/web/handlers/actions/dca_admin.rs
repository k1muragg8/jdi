//! POST actions: dca_admin

use crate::web::handlers::forms::{AssetIdForm, CashAdjustForm, CashSetForm};
use super::types::*;
use crate::web::state::AppState;
use crate::{engine, models};
use axum::extract::{Form, State};
use axum::response::Redirect;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

pub async fn admin_dca_add_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<DcaAddForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let asset = config.assets.iter().find(|a| a.asset_id == form.asset_id);

        if let Some(a) = asset {
            let mut plans = state.repo.load_plans(&ctx).await?;
            let freq = match form.frequency.as_str() {
                "daily" => models::DcaFrequency::Daily,
                "weekly" => models::DcaFrequency::Weekly,
                "monthly" => models::DcaFrequency::Monthly,
                _ => return Err(anyhow::anyhow!("无效的频率")),
            };

            let plan_id = format!("plan_{}", chrono::Local::now().timestamp_millis());
            let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let new_plan = models::DcaPlan {
                plan_id: plan_id.clone(),
                asset_id: form.asset_id.clone(),
                fund_code: a.fund_code.clone(),
                fund_name: a.fund_name.clone(),
                amount: form.amount,
                currency: "CNY".to_string(),
                frequency: freq,
                weekday: if form.frequency == "weekly" {
                    form.day
                } else {
                    None
                },
                month_day: if form.frequency == "monthly" {
                    form.day
                } else {
                    None
                },
                start_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                end_date: None,
                enabled: true,
                priority: 0,
                note: Some("Via Web Admin".to_string()),
                created_at: now_str.clone(),
                updated_at: now_str,
            };

            plans.push(new_plan.clone());
            state.repo.save_plans(&ctx, &plans).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "add_dca_plan".to_string(),
                target_file: "dca_plans.json".to_string(),
                target_id: Some(plan_id),
                old_value_summary: "none".to_string(),
                new_value_summary: format!("{:?}", new_plan),
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
        Ok(_) => Redirect::to("/holdings?success=定投计划新增成功"),
        Err(e) => Redirect::to(&format!("/dca?error={}", e)),
    }
}

pub async fn admin_dca_update_amount_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<DcaUpdateAmountForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        if let Some(p) = plans.iter_mut().find(|p| p.plan_id == form.plan_id) {
            let old_amount = p.amount;
            p.amount = form.amount;
            state.repo.save_plans(&ctx, &plans).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "update_dca_amount".to_string(),
                target_file: "dca_plans.json".to_string(),
                target_id: Some(form.plan_id.clone()),
                old_value_summary: format!("amount: {}", old_amount),
                new_value_summary: format!("amount: {}", form.amount),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/holdings?success=定投金额更新成功"),
        Err(e) => Redirect::to(&format!("/dca?error={}", e)),
    }
}


pub async fn admin_dca_enable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<DcaIdForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        if let Some(p) = plans.iter_mut().find(|p| p.plan_id == form.plan_id) {
            p.enabled = true;
            state.repo.save_plans(&ctx, &plans).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "enable_dca_plan".to_string(),
                target_file: "dca_plans.json".to_string(),
                target_id: Some(form.plan_id.clone()),
                old_value_summary: "enabled: false".to_string(),
                new_value_summary: "enabled: true".to_string(),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/holdings?success=计划已启用"),
        Err(e) => Redirect::to(&format!("/dca?error={}", e)),
    }
}


pub async fn admin_dca_disable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<DcaIdForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        if let Some(p) = plans.iter_mut().find(|p| p.plan_id == form.plan_id) {
            p.enabled = false;
            state.repo.save_plans(&ctx, &plans).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "disable_dca_plan".to_string(),
                target_file: "dca_plans.json".to_string(),
                target_id: Some(form.plan_id.clone()),
                old_value_summary: "enabled: true".to_string(),
                new_value_summary: "enabled: false".to_string(),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/holdings?success=计划已禁用"),
        Err(e) => Redirect::to(&format!("/dca?error={}", e)),
    }
}


pub async fn admin_dca_remove_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<DcaIdForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        if let Some(idx) = plans.iter().position(|p| p.plan_id == form.plan_id) {
            let removed = plans.remove(idx);
            state.repo.save_plans(&ctx, &plans).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "remove_dca_plan".to_string(),
                target_file: "dca_plans.json".to_string(),
                target_id: Some(form.plan_id.clone()),
                old_value_summary: format!("{:?}", removed),
                new_value_summary: "removed".to_string(),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/holdings?success=计划已删除"),
        Err(e) => Redirect::to(&format!("/dca?error={}", e)),
    }
}

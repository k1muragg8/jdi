//! POST form actions and templates (redirect responses).

use crate::web::state::AppState;
use crate::{engine, models};
use axum::extract::{Form, State};
use axum::response::Redirect;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

pub use super::api::{AssetIdForm, CashAdjustForm, CashSetForm};

#[derive(Deserialize)]
pub struct AddSnapshotForm {
    asset_id: String,
    snapshot_date: String,
    market_value: f64,
    units: Option<f64>,
    cost_basis: Option<f64>,
    nav: Option<f64>,
    nav_date: Option<String>,
    total_pnl: Option<f64>,
}

#[derive(Deserialize)]
pub struct DcaAddForm {
    asset_id: String,
    amount: f64,
    frequency: String,
    day: Option<u32>,
}

#[derive(Deserialize)]
pub struct AssetFundCodeForm {
    asset_id: String,
    fund_code: String,
}

#[derive(Deserialize)]
pub struct InstrumentIdForm {
    instrument_id: String,
}

#[derive(Deserialize)]
pub struct InstrumentAddForm {
    symbol: String,
    instrument_id: Option<String>,
    name_zh: Option<String>,
    asset_class: Option<String>,
    provider: Option<String>,
    currency: Option<String>,
}

pub async fn admin_add_snapshot_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AddSnapshotForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let mut snapshots = state.repo.load_alipay_snapshots(&ctx).await?;

        let asset = config.assets.iter().find(|a| a.asset_id == form.asset_id);
        if let Some(a) = asset {
            if form.market_value < 0.0 {
                return Err(anyhow::anyhow!("金额不能为负数"));
            }

            let snapshot_id = format!(
                "snap_{}_{}",
                form.asset_id,
                chrono::Local::now().format("%Y%m%d%H%M%S")
            );

            // Handle empty strings from form as None
            let parse_opt_f64 = |opt: Option<f64>| opt.filter(|&v| v > 0.0);

            let parse_opt_string = |opt: Option<String>| {
                if let Some(s) = opt {
                    if s.trim().is_empty() {
                        None
                    } else {
                        Some(s.trim().to_string())
                    }
                } else {
                    None
                }
            };

            let new_snapshot = models::AlipaySnapshot {
                snapshot_id: snapshot_id.clone(),
                asset_id: form.asset_id.clone(),
                fund_code: a.fund_code.clone(),
                fund_name: a.fund_name.clone(),
                snapshot_date: form.snapshot_date.clone(),
                market_value: form.market_value,
                units: parse_opt_f64(form.units),
                cost_basis: parse_opt_f64(form.cost_basis),
                nav: parse_opt_f64(form.nav),
                nav_date: parse_opt_string(form.nav_date),
                daily_pnl: None,
                total_pnl: form.total_pnl,
                source: "alipay".to_string(),
                note: Some("Via Web Admin".to_string()),
                created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            snapshots.push(new_snapshot.clone());
            state.repo.save_alipay_snapshots(&ctx, &snapshots).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "add_alipay_snapshot".to_string(),
                target_file: "alipay_snapshots.json".to_string(),
                target_id: Some(snapshot_id),
                old_value_summary: "none".to_string(),
                new_value_summary: format!("{:?}", new_snapshot),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("未找到资产 {}", form.asset_id))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/holdings?success=快照录入成功"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

#[derive(Deserialize)]
pub struct ReconcileApplyForm {
    snapshot_id: String,
    confirm: Option<String>,
}

pub async fn admin_reconcile_apply_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ReconcileApplyForm>,
) -> Redirect {
    if form.confirm.as_deref() != Some("true") {
        return Redirect::to("/holdings?error=未确认校准操作");
    }

    let ctx = &state.ctx;
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let mut portfolio_state = state.repo.load_state(&ctx).await?;
        let snapshots = state.repo.load_alipay_snapshots(&ctx).await?;

        let snapshot = snapshots.iter().find(|s| s.snapshot_id == form.snapshot_id);

        if let Some(s) = snapshot {
            let res = engine::reconciliation::reconcile_asset(&config, &portfolio_state, s);
            if let Some(suggest) = engine::reconciliation::generate_calibration_suggestion(&res) {
                let audit_record =
                    engine::reconciliation::apply_calibration(&mut portfolio_state, &suggest);

                // Save updated state
                state.repo.save_state(&ctx, &portfolio_state).await?;

                // Save domain audit
                let mut audits = state
                    .repo
                    .load_reconciliation_audits(&ctx)
                    .await
                    .unwrap_or_default();
                audits.push(audit_record.clone());
                state.repo.save_reconciliation_audits(&ctx, &audits).await?;

                // Save web admin audit
                let web_audit = models::WebAdminAudit {
                    audit_id: format!("audit_web_{}", chrono::Local::now().timestamp_millis()),
                    timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    actor: "local_web".to_string(),
                    actor_user_id: Some(ctx.actor_user_id.clone()),
                    target_user_id: Some(ctx.target_user_id.clone()),
                    portfolio_id: Some(ctx.portfolio_id.clone()),
                    role: Some(ctx.role.clone()),
                    action: "apply_calibration".to_string(),
                    target_file: "portfolio_state.json".to_string(),
                    target_id: Some(s.asset_id.clone()),
                    old_value_summary: format!(
                        "units:{}, cost:{}",
                        audit_record.old_units, audit_record.old_cost_basis
                    ),
                    new_value_summary: format!(
                        "units:{}, cost:{}",
                        audit_record.new_units, audit_record.new_cost_basis
                    ),
                    status: "success".to_string(),
                    note: Some(format!("Based on snapshot {}", s.snapshot_id)),
                };
                state.repo.append_web_admin_audit(&ctx, web_audit).await?;

                Ok::<(), anyhow::Error>(())
            } else {
                Err(anyhow::anyhow!("资产 {} 状态一致，无需校准", s.asset_id))
            }
        } else {
            Err(anyhow::anyhow!("未找到快照 {}", form.snapshot_id))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/holdings?success=校准执行成功，持仓已更新"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

#[derive(Deserialize)]
pub struct AddSettlementForm {
    asset_id: String,
    plan_id: Option<String>,
    deduction_date: String,
    confirmation_date: String,
    amount: f64,
    confirmed_nav: f64,
    confirmed_units: f64,
    fee: Option<f64>,
    note: Option<String>,
}

pub async fn admin_add_settlement_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AddSettlementForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let asset = config.assets.iter().find(|a| a.asset_id == form.asset_id);

        if let Some(a) = asset {
            let mut settlements = state.repo.load_settlements(&ctx).await?;
            let settlement_id = format!("settle_{}", chrono::Local::now().timestamp_millis());

            let new_settlement = models::DcaSettlement {
                settlement_id: settlement_id.clone(),
                plan_id: form.plan_id.clone(),
                asset_id: form.asset_id.clone(),
                fund_code: a.fund_code.clone(),
                fund_name: a.fund_name.clone(),
                scheduled_date: None,
                deduction_date: form.deduction_date.clone(),
                confirmation_date: form.confirmation_date.clone(),
                amount: form.amount,
                confirmed_nav: form.confirmed_nav,
                confirmed_units: form.confirmed_units,
                fee: form.fee,
                currency: "CNY".to_string(),
                source: "alipay".to_string(),
                status: models::DcaSettlementStatus::Confirmed,
                applied: false,
                note: form.note.clone(),
                created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            settlements.push(new_settlement.clone());
            state.repo.save_settlements(&ctx, &settlements).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "add_dca_settlement".to_string(),
                target_file: "dca_settlements.json".to_string(),
                target_id: Some(settlement_id),
                old_value_summary: "none".to_string(),
                new_value_summary: format!("{:?}", new_settlement),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("未找到资产 {}", form.asset_id))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/holdings?success=结算录入成功"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

#[derive(Deserialize)]
pub struct SettlementApplyForm {
    settlement_id: String,
    confirm: String,
}

pub async fn admin_settlement_apply_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<SettlementApplyForm>,
) -> Redirect {
    if form.confirm != "true" {
        return Redirect::to("/holdings?error=未确认应用操作");
    }

    let ctx = &state.ctx;
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let mut portfolio_state = state.repo.load_state(&ctx).await?;
        let mut settlements = state.repo.load_settlements(&ctx).await?;

        let settlement_idx = settlements
            .iter()
            .position(|s| s.settlement_id == form.settlement_id);

        if let Some(idx) = settlement_idx {
            if settlements[idx].applied {
                return Err(anyhow::anyhow!(
                    "结算 {} 已经应用过，请勿重复操作",
                    form.settlement_id
                ));
            }

            let s = &settlements[idx];
            let asset_id = s.asset_id.clone();
            let settlement_id = s.settlement_id.clone();
            let impact =
                engine::dca_settlement::calculate_settlement_impact(&config, &portfolio_state, s);

            let audit_record =
                engine::dca_settlement::apply_settlement(&mut portfolio_state, s, &impact);

            // Mark as applied
            settlements[idx].applied = true;

            // Save updated state and settlements
            state.repo.save_state(&ctx, &portfolio_state).await?;
            state.repo.save_settlements(&ctx, &settlements).await?;

            // Save web admin audit
            let web_audit = models::WebAdminAudit {
                audit_id: format!("audit_web_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "apply_dca_settlement".to_string(),
                target_file: "portfolio_state.json".to_string(),
                target_id: Some(asset_id),
                old_value_summary: format!(
                    "units:{}, cost:{}",
                    audit_record.old_units, audit_record.old_cost_basis
                ),
                new_value_summary: format!(
                    "units:{}, cost:{}",
                    audit_record.new_units, audit_record.new_cost_basis
                ),
                status: "success".to_string(),
                note: Some(format!("Based on settlement {}", settlement_id)),
            };
            state.repo.append_web_admin_audit(&ctx, web_audit).await?;

            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("未找到结算记录 {}", form.settlement_id))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/holdings?success=结算执行成功，持仓已更新"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

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

#[derive(Deserialize)]
pub struct DcaIdForm {
    plan_id: String,
}

#[derive(Deserialize)]
pub struct DcaUpdateAmountForm {
    plan_id: String,
    amount: f64,
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

#[derive(Deserialize)]
pub struct AssetRenameForm {
    asset_id: String,
    fund_name: String,
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

#[derive(Deserialize)]
pub struct AssetSectorForm {
    asset_id: String,
    sector: String,
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

pub async fn admin_instrument_enable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentIdForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        if let Some(inst) = instruments
            .iter_mut()
            .find(|i| i.instrument_id == form.instrument_id)
        {
            inst.enabled = true;
            state.repo.save_instruments(&ctx, &instruments).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "enable_instrument".to_string(),
                target_file: "instruments.json".to_string(),
                target_id: Some(form.instrument_id.clone()),
                old_value_summary: "enabled: false".to_string(),
                new_value_summary: "enabled: true".to_string(),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("证券未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/market?success=证券已启用"),
        Err(e) => Redirect::to(&format!("/market?error={}", e)),
    }
}

pub async fn admin_instrument_disable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentIdForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        if let Some(inst) = instruments
            .iter_mut()
            .find(|i| i.instrument_id == form.instrument_id)
        {
            inst.enabled = false;
            state.repo.save_instruments(&ctx, &instruments).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "disable_instrument".to_string(),
                target_file: "instruments.json".to_string(),
                target_id: Some(form.instrument_id.clone()),
                old_value_summary: "enabled: true".to_string(),
                new_value_summary: "enabled: false".to_string(),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("证券未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/market?success=证券已禁用"),
        Err(e) => Redirect::to(&format!("/market?error={}", e)),
    }
}

#[derive(Deserialize)]
pub struct InstrumentMetadataForm {
    instrument_id: String,
    name_zh: Option<String>,
    display_label: Option<String>,
    provider: Option<String>,
    provider_symbol: Option<String>,
}

pub async fn admin_instrument_update_metadata_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentMetadataForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        let instrument_id = form.instrument_id.clone();

        let (old_meta, new_meta) = {
            if let Some(inst) = instruments
                .iter_mut()
                .find(|i| i.instrument_id == instrument_id)
            {
                let old_meta = format!(
                    "name_zh: {:?}, label: {:?}, provider: {}, symbol: {}",
                    inst.name_zh, inst.display_label, inst.provider, inst.provider_symbol
                );

                if let Some(n) = form.name_zh.filter(|n| !n.trim().is_empty()) {
                    inst.name_zh = Some(n.trim().to_string());
                }
                if let Some(l) = form.display_label.filter(|l| !l.trim().is_empty()) {
                    inst.display_label = Some(l.trim().to_string());
                }
                if let Some(p) = form.provider.filter(|p| !p.trim().is_empty()) {
                    inst.provider = p.trim().to_lowercase();
                }
                if let Some(ps) = form.provider_symbol.filter(|p| !p.trim().is_empty()) {
                    inst.provider_symbol = ps.trim().to_string();
                }
                engine::instrument_watchlist::migrate_au9999_provider(inst);

                let new_meta = format!(
                    "name_zh: {:?}, label: {:?}, provider: {}, symbol: {}",
                    inst.name_zh, inst.display_label, inst.provider, inst.provider_symbol
                );
                (old_meta, new_meta)
            } else {
                return Err(anyhow::anyhow!("证券未找到"));
            }
        };

        state.repo.save_instruments(&ctx, &instruments).await?;

        let audit = models::WebAdminAudit {
            audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            actor: "local_web".to_string(),
            actor_user_id: Some(ctx.actor_user_id.clone()),
            target_user_id: Some(ctx.target_user_id.clone()),
            portfolio_id: Some(ctx.portfolio_id.clone()),
            role: Some(ctx.role.clone()),
            action: "update_instrument_metadata".to_string(),
            target_file: "instruments.json".to_string(),
            target_id: Some(instrument_id),
            old_value_summary: old_meta,
            new_value_summary: new_meta,
            status: "success".to_string(),
            note: None,
        };
        state.repo.append_web_admin_audit(&ctx, audit).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/market?success=证券元数据更新成功"),
        Err(e) => Redirect::to(&format!("/market?error={}", e)),
    }
}

pub async fn admin_instrument_add_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentAddForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        let sym = form.symbol.trim().to_string();
        if sym.is_empty() {
            return Err(anyhow::anyhow!("symbol 不能为空"));
        }
        let id = form
            .instrument_id
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| sym.clone());
        let asset_class = match form.asset_class.as_deref().unwrap_or("Index") {
            "Etf" | "etf" => models::AssetClass::Etf,
            "Crypto" => models::AssetClass::Crypto,
            "Fx" => models::AssetClass::Fx,
            "SpotCommodity" => models::AssetClass::SpotCommodity,
            _ => models::AssetClass::Index,
        };
        let new_inst = models::InstrumentConfig {
            instrument_id: id.clone(),
            symbol: sym.clone(),
            display_symbol: Some(sym.clone()),
            name: form.name_zh.clone().unwrap_or_else(|| sym.clone()),
            name_zh: form.name_zh,
            name_en: None,
            description_zh: None,
            category_zh: None,
            display_label: None,
            asset_class,
            provider: form.provider.unwrap_or_else(|| "yahoo".to_string()),
            provider_symbol: sym.clone(),
            market: None,
            exchange: None,
            currency: form.currency.unwrap_or_else(|| "USD".to_string()),
            quote_unit: "1".to_string(),
            price_unit: "1".to_string(),
            timezone: None,
            enabled: true,
            archived: false,
            priority: 0,
            tags: vec![],
            note: Some("web added".to_string()),
        };
        engine::instrument_watchlist::upsert_instrument(&mut instruments, new_inst)?;
        state.repo.save_instruments(&ctx, &instruments).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/market?success=标的新增或更新成功"),
        Err(e) => Redirect::to(&format!("/market?error={}", e)),
    }
}

pub async fn admin_instrument_archive_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentIdForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        let config = state.repo.load_config(&ctx).await.unwrap_or_default();
        let mut found = false;
        let mut referenced = false;
        for inst in &mut instruments {
            if inst.instrument_id == form.instrument_id {
                found = true;
                for a in &config.assets {
                    if a.reference_index_symbol.as_deref() == Some(&inst.symbol)
                        || a.reference_index_symbol.as_deref() == Some(&inst.provider_symbol)
                        || a.reference_index_name
                            .as_deref()
                            .map(|n| n.contains(&inst.symbol))
                            .unwrap_or(false)
                    {
                        referenced = true;
                    }
                }
                engine::archive_instrument(inst);
                break;
            }
        }
        if !found {
            return Err(anyhow::anyhow!("标的未找到"));
        }
        state.repo.save_instruments(&ctx, &instruments).await?;
        if referenced {
            Ok(
                "该标的仍被资产或策略引用，已归档并禁用。已归档，不再显示在默认行情列表。"
                    .to_string(),
            )
        } else {
            Ok("已归档，不再显示在默认行情列表。".to_string())
        }
    }
    .await;

    match result {
        Ok(msg) => Redirect::to(&format!("/market?success={}", msg)),
        Err(e) => Redirect::to(&format!("/market?error={}", e)),
    }
}

#[derive(Deserialize, Default)]
pub struct RestoreDefaultsForm {
    cleanup_test: Option<String>,
}

pub async fn admin_instrument_restore_defaults_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<RestoreDefaultsForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let also_cleanup = form
        .cleanup_test
        .as_deref()
        .is_some_and(|v| v == "1" || v == "on");
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        engine::migrate_instrument_flags(&mut instruments);
        let (added, reactivated) =
            engine::restore_default_instruments(&mut instruments, also_cleanup);
        state.repo.save_instruments(&ctx, &instruments).await?;
        Ok::<(usize, usize), anyhow::Error>((added, reactivated))
    }
    .await;

    match result {
        Ok((added, reactivated)) => {
            let msg = if also_cleanup {
                format!(
                    "已恢复默认标的并清理测试行：新增 {}，重新启用 {}",
                    added, reactivated
                )
            } else {
                format!("已恢复默认标的：新增 {}，重新启用 {}", added, reactivated)
            };
            Redirect::to(&format!("/market?success={}", msg))
        }
        Err(e) => Redirect::to(&format!("/market?error={}", e)),
    }
}

#[derive(Deserialize, Default)]
pub struct CleanupTestForm {
    confirm: Option<String>,
}

pub async fn admin_instrument_cleanup_test_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CleanupTestForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        if form.confirm.as_deref() != Some("1") {
            return Err(anyhow::anyhow!("缺少确认参数"));
        }
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        let preview = engine::cleanup_test_instruments(&mut instruments.clone(), true);
        if preview == 0 {
            return Ok("未检测到待清理的测试标的".to_string());
        }
        let n = engine::cleanup_test_instruments(&mut instruments, false);
        state.repo.save_instruments(&ctx, &instruments).await?;
        Ok(format!("已归档 {} 个测试标的，不再显示在默认行情列表。", n))
    }
    .await;

    match result {
        Ok(msg) => Redirect::to(&format!("/market?success={}", msg)),
        Err(e) => Redirect::to(&format!("/market?error={}", e)),
    }
}

// --- Autonomous Operation Handlers ---

pub async fn api_cash_set_initial_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CashSetForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let tx = crate::models::Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        date: Local::now().format("%Y-%m-%d").to_string(),
        transaction_type: "cash_set".to_string(),
        asset_id: None,
        amount: form.amount,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "Web端初始现金设定".to_string(),
        source: "manual".to_string(),
        raw_description: "Initial cash set".to_string(),
    };
    let mut transactions = state.repo.load_transactions(&ctx).await.unwrap_or_default();
    transactions.push(tx);
    let _ = state.repo.save_transactions(&ctx, &transactions).await;
    if let Ok(new_state) =
        crate::engine::holdings::rebuild_holdings_from_transactions(&transactions)
    {
        let _ = state.repo.save_state(&ctx, &new_state).await;
    }
    Redirect::to("/holdings?success=初始现金已设置")
}

pub async fn api_cash_adjust_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CashAdjustForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let tx_type = if form.amount >= 0.0 {
        "cash_in"
    } else {
        "cash_out"
    };
    let amount = form.amount.abs();
    let tx = crate::models::Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        date: Local::now().format("%Y-%m-%d").to_string(),
        transaction_type: tx_type.to_string(),
        asset_id: None,
        amount,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "Web端现金调整".to_string(),
        source: "manual".to_string(),
        raw_description: format!("Cash {}", tx_type),
    };
    let mut transactions = state.repo.load_transactions(&ctx).await.unwrap_or_default();
    transactions.push(tx);
    let _ = state.repo.save_transactions(&ctx, &transactions).await;
    if let Ok(new_state) =
        crate::engine::holdings::rebuild_holdings_from_transactions(&transactions)
    {
        let _ = state.repo.save_state(&ctx, &new_state).await;
    }
    Redirect::to("/holdings?success=现金调整已记录")
}

#[derive(Deserialize)]
pub struct CashReverseForm {
    tx_id: String,
}

pub async fn api_cash_reverse_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CashReverseForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut transactions = state.repo.load_transactions(&ctx).await?;
        let tx = transactions
            .iter()
            .find(|t| t.id == form.tx_id)
            .ok_or_else(|| anyhow::anyhow!("流水未找到"))?;
        if tx.note.contains("已冲正") {
            anyhow::bail!("该流水已冲正");
        }
        let reverse_type = if tx.transaction_type == "cash_in" || tx.transaction_type == "现金转入"
        {
            "cash_out"
        } else {
            "cash_in"
        };
        let rev = crate::models::Transaction {
            id: uuid::Uuid::new_v4().to_string(),
            date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            transaction_type: reverse_type.to_string(),
            asset_id: None,
            amount: tx.amount,
            units: None,
            price: None,
            fee: 0.0,
            currency: tx.currency.clone(),
            note: format!("冲正流水 {} (Web)", form.tx_id),
            source: "manual".to_string(),
            raw_description: format!("Reverse {}", form.tx_id),
        };
        if let Some(orig) = transactions.iter_mut().find(|t| t.id == form.tx_id) {
            orig.note = format!("{} [已冲正]", orig.note);
        }
        transactions.push(rev);
        state.repo.save_transactions(&ctx, &transactions).await?;
        if let Ok(new_state) =
            crate::engine::holdings::rebuild_holdings_from_transactions(&transactions)
        {
            state.repo.save_state(&ctx, &new_state).await?;
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;
    match result {
        Ok(_) => Redirect::to("/cash?success=冲正成功"),
        Err(e) => Redirect::to(&format!("/cash?error={}", e)),
    }
}

pub async fn api_assets_auto_classify_handler(State(state): State<Arc<AppState>>) -> Redirect {
    let ctx = &state.ctx;
    if let Ok(mut config) = state.repo.load_config(&ctx).await {
        let mut changed = 0;
        for asset in &mut config.assets {
            if asset.sector.is_empty() || asset.sector == "未分类" || asset.sector == "待确认"
            {
                let name = asset.fund_name.to_lowercase();

                let mut new_sector = None;

                if name.contains("纳斯达克科技")
                    || name.contains("nasdaq tech")
                    || name.contains("nasdaq100")
                    || name.contains("纳斯达克100")
                    || name.contains("nasdaq")
                    || name.contains("qqq")
                {
                    new_sector = Some("美国科技".to_string());
                } else if name.contains("标普500")
                    || name.contains("s&p 500")
                    || name.contains("s&p500")
                    || name.contains("spy")
                    || name.contains("ivv")
                    || name.contains("voo")
                {
                    new_sector = Some("美国大盘".to_string());
                } else if name.contains("生物科技")
                    || name.contains("创新药")
                    || name.contains("医疗")
                    || name.contains("biotech")
                    || name.contains("医药")
                {
                    new_sector = Some("生物科技".to_string());
                } else if name.contains("日经") || name.contains("日本") || name.contains("nikkei")
                {
                    new_sector = Some("日本".to_string());
                } else if name.contains("越南") || name.contains("vietnam") {
                    new_sector = Some("越南".to_string());
                } else if name.contains("印度") || name.contains("india") {
                    new_sector = Some("印度".to_string());
                } else if name.contains("黄金") || name.contains("gold") {
                    new_sector = Some("黄金".to_string());
                } else if name.contains("债")
                    || name.contains("国开")
                    || name.contains("同业存单")
                    || name.contains("中短债")
                    || name.contains("美元债")
                    || name.contains("bond")
                {
                    new_sector = Some("债券".to_string());
                } else if name.contains("dax")
                    || name.contains("德国")
                    || name.contains("cac40")
                    || name.contains("法国")
                    || name.contains("欧洲")
                    || name.contains("euro")
                {
                    new_sector = Some("欧洲".to_string());
                } else if name.contains("商品")
                    || name.contains("抗通胀")
                    || name.contains("commodity")
                {
                    new_sector = Some("商品".to_string());
                } else if name.contains("富时100") || name.contains("英国") || name.contains("ftse")
                {
                    new_sector = Some("欧洲".to_string());
                }

                if let Some(s) = new_sector {
                    if asset.sector != s {
                        asset.sector = s;
                        changed += 1;
                    }
                } else if asset.sector.is_empty() || asset.sector == "未分类" {
                    asset.sector = "待确认".to_string();
                    changed += 1;
                }
            }
        }
        if changed > 0 {
            let _ = state.repo.save_config(&ctx, &config).await;
        }
    }
    Redirect::to("/holdings?success=自动分类已执行(部分资产可能仍需手动确认)")
}

pub async fn template_transactions_handler() -> (axum::http::HeaderMap, String) {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_static("attachment; filename=transactions_template.csv"),
    );

    let content = "交易日期,交易类型,资产代码,资产名称,金额,份额,价格,手续费,币种,来源,备注\n\
        2024-01-01,buy,000216,华安黄金ETF联接A,1000.0,2.5,400.0,1.2,CNY,manual,示例买入\n\
        2024-01-02,sell,000216,华安黄金ETF联接A,500.0,1.25,400.0,0.6,CNY,manual,示例卖出"
        .to_string();

    (headers, content)
}

pub async fn template_alipay_holdings_handler() -> (axum::http::HeaderMap, String) {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_static("attachment; filename=alipay_holdings_template.csv"),
    );

    let content = "基金代码,基金名称,持有份额,持有金额,最新净值,净值日期,投入本金,持有收益,持有收益率,来源\n\
        000216,华安黄金ETF联接A,124.45,49782.36,1.23,2024-06-02,45000.0,4782.36,10.6,alipay_screenshot\n\
        000042,财通资管积极配置,5678.9,10234.56,1.80,2024-06-02,10000.0,234.56,2.3,alipay_screenshot"
        .to_string();

    (headers, content)
}

#[derive(Deserialize)]
pub struct AssetAddForm {
    fund_name: String,
    fund_code: String,
    sector: Option<String>,
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

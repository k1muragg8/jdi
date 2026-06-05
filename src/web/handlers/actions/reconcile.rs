//! POST actions: reconcile

use crate::web::handlers::forms::{AssetIdForm, CashAdjustForm, CashSetForm};
use super::types::*;
use crate::web::state::AppState;
use crate::{engine, models};
use axum::extract::{Form, State};
use axum::response::Redirect;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

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

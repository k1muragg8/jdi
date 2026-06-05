//! API: dca

use crate::web::state::AppState;
use crate::{engine, models};
use axum::extract::State;
use axum::response::Json;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

pub async fn api_dca_run_due_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::DcaExecutionResult> {
    let ctx = &state.ctx;
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let date = Local::now().format("%Y-%m-%d").to_string();
        let res = engine::dca::auto_execute_dca(state.repo.as_ref(), &ctx, &config, &date).await?;
        Ok::<models::DcaExecutionResult, anyhow::Error>(res)
    }
    .await;

    match result {
        Ok(res) => Json(res),
        Err(e) => Json(models::DcaExecutionResult {
            success: false,
            message: format!("DCA execution failed: {}", e),
            ..Default::default()
        }),
    }
}

pub async fn api_dca_plans_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<models::DcaPlan>> {
    let ctx = &state.ctx;
    let plans = state.repo.load_plans(&ctx).await.unwrap_or_default();
    Json(plans)
}

#[derive(Deserialize)]
pub struct DcaPlanForm {
    asset_id: String,
    amount: f64,
    frequency: String,
    day: Option<u32>,
    note: Option<String>,
}

pub async fn api_dca_add_plan_handler(
    State(state): State<Arc<AppState>>,
    Json(form): Json<DcaPlanForm>,
) -> Json<models::DcaExecutionResult> {
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
                note: form.note.or(Some("Via Web API".to_string())),
                created_at: now_str.clone(),
                updated_at: now_str,
            };

            plans.push(new_plan);
            state.repo.save_plans(&ctx, &plans).await?;
            Ok::<String, anyhow::Error>(plan_id)
        } else {
            Err(anyhow::anyhow!("资产未找到"))
        }
    }
    .await;

    match result {
        Ok(id) => Json(models::DcaExecutionResult {
            success: true,
            message: format!("Plan created: {}", id),
            ..Default::default()
        }),
        Err(e) => Json(models::DcaExecutionResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

#[derive(Deserialize)]
pub struct DcaUpdateForm {
    amount: Option<f64>,
    frequency: Option<String>,
    day: Option<u32>,
    note: Option<String>,
    enabled: Option<bool>,
}

pub async fn api_dca_update_plan_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(plan_id): axum::extract::Path<String>,
    Json(form): Json<DcaUpdateForm>,
) -> Json<models::DcaExecutionResult> {
    let ctx = &state.ctx;
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        if let Some(p) = plans.iter_mut().find(|p| p.plan_id == plan_id) {
            if let Some(a) = form.amount {
                p.amount = a;
            }
            if let Some(f) = form.frequency {
                p.frequency = match f.as_str() {
                    "daily" => models::DcaFrequency::Daily,
                    "weekly" => models::DcaFrequency::Weekly,
                    "monthly" => models::DcaFrequency::Monthly,
                    _ => p.frequency.clone(),
                };
                if f == "weekly" {
                    p.weekday = form.day;
                    p.month_day = None;
                } else if f == "monthly" {
                    p.month_day = form.day;
                    p.weekday = None;
                }
            }
            if let Some(n) = form.note {
                p.note = Some(n);
            }
            if let Some(e) = form.enabled {
                p.enabled = e;
            }
            p.updated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            state.repo.save_plans(&ctx, &plans).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Json(models::DcaExecutionResult {
            success: true,
            message: "Plan updated".to_string(),
            ..Default::default()
        }),
        Err(e) => Json(models::DcaExecutionResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

pub async fn api_dca_remove_plan_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(plan_id): axum::extract::Path<String>,
) -> Json<models::DcaExecutionResult> {
    let ctx = &state.ctx;
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        let len_before = plans.len();
        plans.retain(|p| p.plan_id != plan_id);
        if plans.len() < len_before {
            state.repo.save_plans(&ctx, &plans).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Json(models::DcaExecutionResult {
            success: true,
            message: "Plan removed".to_string(),
            ..Default::default()
        }),
        Err(e) => Json(models::DcaExecutionResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

pub async fn api_dca_executions_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<models::DcaSettlement>> {
    let ctx = &state.ctx;
    let mut settlements = state.repo.load_settlements(&ctx).await.unwrap_or_default();
    // Sort by deduction_date DESC
    settlements.sort_by(|a, b| b.deduction_date.cmp(&a.deduction_date));
    Json(settlements)
}

//! API: backtest

use crate::web::state::{AppState, BackgroundRefreshStatus};
use crate::{api, engine, models};
use anyhow::Result;
use axum::extract::{Multipart, State};
use axum::response::Json;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct BacktestRunForm {
    start_date: String,
    end_date: String,
    initial_cash: f64,
    include_baseline: bool,
}

pub async fn api_backtest_run_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BacktestRunForm>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    let config = match state.repo.load_config(&ctx).await {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    };

    let req = models::BacktestRequest {
        start_date: payload.start_date,
        end_date: payload.end_date,
        initial_cash: payload.initial_cash,
        portfolio_id: ctx.portfolio_id.clone(),
        policy_override: None,
        include_baseline: payload.include_baseline,
    };

    match engine::backtest::run_backtest(state.repo.as_ref(), &ctx, &config, req).await {
        Ok(report) => {
            let mut last_report = state.last_backtest_report.write().await;
            *last_report = Some(report.clone());
            Json(serde_json::json!({ "success": true, "report": report }))
        }
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    }
}


pub async fn api_backtest_latest_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let report_opt = state.last_backtest_report.read().await;
    if let Some(report) = report_opt.as_ref() {
        Json(serde_json::json!({ "success": true, "report": report }))
    } else {
        Json(serde_json::json!({ "success": false, "message": "No backtest report found" }))
    }
}

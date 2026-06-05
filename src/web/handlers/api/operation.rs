//! API: operation

use crate::web::state::{AppState, BackgroundRefreshStatus};
use crate::{api, engine, models};
use anyhow::Result;
use axum::extract::{Multipart, State};
use axum::response::Json;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

pub async fn api_operation_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::OperationStatus> {
    let ctx = &state.ctx;
    let status = state
        .repo
        .load_operation_status(&ctx)
        .await
        .unwrap_or_else(|_| models::OperationStatus::default());
    Json(status)
}


pub async fn api_operation_report_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    let status = state
        .repo
        .load_operation_status(&ctx)
        .await
        .unwrap_or_else(|_| models::OperationStatus::default());

    if let Some(report) = status.last_report {
        Json(serde_json::to_value(report).unwrap())
    } else {
        Json(serde_json::json!({ "error": "No report available" }))
    }
}


pub async fn api_operation_run_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    let config_res = state.repo.load_config(&ctx).await;

    match config_res {
        Ok(config) => {
            // run_autonomous_operation now handles internal refresh if needed via evaluate_operation_state
            match engine::run_autonomous_operation(state.repo.as_ref(), &ctx, &config).await {
                Ok(report) => Json(serde_json::json!({ "success": true, "report": report })),
                Err(e) => Json(
                    serde_json::json!({ "success": false, "message": e.to_string() as String }),
                ),
            }
        }
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() as String })),
    }
}

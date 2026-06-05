//! Holdings (持仓) page handlers.

use crate::models::DcaExecutionResult;
use crate::web::response::AdminQuery;
use crate::web::services::holdings_service;
use crate::web::state::AppState;
use crate::web::view_models::holdings::build_holdings_vm;
use crate::web::views::{holdings, layout, layout_with_msg};
use axum::extract::{Json, Path, Query, State};
use axum::response::{Html, Json as AxumJson};
use std::sync::Arc;

pub async fn holdings_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminQuery>,
) -> Html<String> {
    match holdings_service::load_holdings_page(&state).await {
        Ok(data) => {
            let vm = build_holdings_vm(data, query.filter.as_deref());
            layout_with_msg("持仓", holdings::render(&vm), query.success, query.error)
        }
        Err(e) => layout(
            "持仓",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
    }
}

// DCA actions bound to holdings row (Part 8): POST /holdings/:asset_id/dca etc.
// These delegate to service (persist to repo json/pg), return json result for fetch callers.
// Normal UI uses these or /api/dca (existing), no raw JSON browsing.

#[derive(serde::Deserialize, Default)]
pub struct DcaInlineForm {
    pub amount: Option<f64>,
    pub frequency: Option<String>,
    pub day: Option<u32>,
    pub note: Option<String>,
    pub enabled: Option<bool>,
}

pub async fn holdings_dca_post_handler(
    State(state): State<Arc<AppState>>,
    Path(asset_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> AxumJson<DcaExecutionResult> {
    let amount = payload
        .get("amount")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let frequency = payload
        .get("frequency")
        .and_then(|v| v.as_str())
        .unwrap_or("monthly")
        .to_string();
    let day = payload
        .get("day")
        .and_then(|v| v.as_u64())
        .map(|u| u as u32);
    let note = payload
        .get("note")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let res =
        holdings_service::upsert_dca_for_asset(&state, &asset_id, amount, &frequency, day, note)
            .await;
    match res {
        Ok(id) => AxumJson(DcaExecutionResult {
            success: true,
            message: format!("saved:{}", id),
            executed_count: 0,
            skipped_count: 0,
            failed_count: 0,
        }),
        Err(e) => AxumJson(DcaExecutionResult {
            success: false,
            message: e.to_string(),
            executed_count: 0,
            skipped_count: 0,
            failed_count: 0,
        }),
    }
}

pub async fn holdings_dca_pause_handler(
    State(state): State<Arc<AppState>>,
    Path(asset_id): Path<String>,
) -> AxumJson<DcaExecutionResult> {
    let res = holdings_service::pause_dca_for_asset(&state, &asset_id).await;
    match res {
        Ok(_) => AxumJson(DcaExecutionResult {
            success: true,
            message: "paused".into(),
            ..Default::default()
        }),
        Err(e) => AxumJson(DcaExecutionResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

pub async fn holdings_dca_resume_handler(
    State(state): State<Arc<AppState>>,
    Path(asset_id): Path<String>,
) -> AxumJson<DcaExecutionResult> {
    let res = holdings_service::resume_dca_for_asset(&state, &asset_id).await;
    match res {
        Ok(_) => AxumJson(DcaExecutionResult {
            success: true,
            message: "resumed".into(),
            ..Default::default()
        }),
        Err(e) => AxumJson(DcaExecutionResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

pub async fn holdings_dca_archive_handler(
    State(state): State<Arc<AppState>>,
    Path(asset_id): Path<String>,
) -> AxumJson<DcaExecutionResult> {
    let res = holdings_service::archive_dca_for_asset(&state, &asset_id).await;
    match res {
        Ok(_) => AxumJson(DcaExecutionResult {
            success: true,
            message: "archived".into(),
            ..Default::default()
        }),
        Err(e) => AxumJson(DcaExecutionResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

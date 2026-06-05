//! API: reconcile

use crate::web::state::{AppState, BackgroundRefreshStatus};
use crate::{api, engine, models};
use anyhow::Result;
use axum::extract::{Multipart, State};
use axum::response::Json;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

pub async fn api_reconciliation_report_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::ReconciliationReport> {
    let ctx = &state.ctx;
    let result = async {
        let transactions = state.repo.load_transactions(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let report =
            engine::reconcile_portfolio(&ctx.portfolio_id, &portfolio_state, &transactions);
        Ok::<models::ReconciliationReport, anyhow::Error>(report)
    }
    .await;

    match result {
        Ok(r) => Json(r),
        Err(_e) => Json(models::ReconciliationReport {
            portfolio_id: "error".to_string(),
            generated_at: chrono::Local::now().to_rfc3339(),
            summary: models::ReconciliationSummary::default(),
            issues: vec![],
        }),
    }
}

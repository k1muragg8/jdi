//! Overview (概览) page handlers.

use crate::models;
use crate::web::services::overview_service::fetch_dashboard_summary;
use crate::web::state::AppState;
use crate::web::view_models::overview::build_overview_vm;
use crate::web::views::{layout, overview};
use axum::extract::State;
use axum::response::{Html, Json};
use chrono::Local;
use std::sync::Arc;

pub async fn dashboard_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = &state.ctx;
    match fetch_dashboard_summary(&state, ctx).await {
        Ok(summary) => {
            let portfolio_state = state.repo.load_state(ctx).await.unwrap_or_default();
            let config = state.repo.load_config(ctx).await.unwrap_or_default();
            match build_overview_vm(&state, &summary, &portfolio_state, &config).await {
                Ok(vm) => layout("概览", overview::render(&vm)),
                Err(e) => layout(
                    "概览",
                    format!(
                        "<div class='message-banner message-error'>页面渲染失败: {}</div>",
                        e
                    ),
                ),
            }
        }
        Err(e) => layout(
            "概览",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
    }
}

pub async fn api_dashboard_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::DashboardSummary> {
    let ctx = &state.ctx;
    match fetch_dashboard_summary(&state, &ctx).await {
        Ok(summary) => Json(summary),
        Err(e) => Json(models::DashboardSummary {
            portfolio: models::PortfolioSummary::default(),
            lifecycle: models::DcaLifecycleSummary::default(),
            cache_status: models::CacheStatusRegistry::default(),
            decision: models::DecisionExplanation {
                date: Local::now().format("%Y-%m-%d").to_string(),
                portfolio_id: "error".to_string(),
                base_currency: "CNY".to_string(),
                available_cash: 0.0,
                daily_budget: 0.0,
                target_equity_value: 0.0,
                current_equity_value: 0.0,
                equity_gap: 0.0,
                risk_summary: models::RiskAdjustmentExplanation {
                    score: 0.0,
                    label: "Error".to_string(),
                    multiplier: 0.0,
                    factors: vec![],
                },
                asset_explanations: vec![],
                sector_explanations: vec![],
                warnings: vec![format!("Error: {}", e)],
                global_caps: vec![],
            },
            risk_overlay: models::GlobalRiskOverlay::default(),
            operation_status: models::OperationStatus::default(),
            backend: state.repo.name(),
            portfolio_name: "Error".to_string(),
            date: Local::now().format("%Y-%m-%d").to_string(),
            alipay_total_value: None,
            alipay_snapshot_date: None,
            unclassified_asset_count: 0,
            reconciliation_issue_count: 0,
            alipay_mismatch_count: 0,
        }),
    }
}

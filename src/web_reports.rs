use crate::engine;
use crate::models::{self, StorageBackend};
use crate::repository::RepositoryContext;
use crate::web::AppState;
use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ReportQuery {
    pub date: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub month: Option<String>,
    pub portfolio_id: Option<String>,
}

pub async fn build_daily_report(
    state: &Arc<AppState>,
    params: &ReportQuery,
) -> models::InvestmentReport {
    let mut ctx = RepositoryContext::default();
    if let Some(pid) = &params.portfolio_id {
        ctx.portfolio_id = pid.clone();
    }

    let target_date = params
        .date
        .clone()
        .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());

    let config: models::ConfigRoot = state.repo.load_config(&ctx).await.unwrap_or_default();
    let p_state: models::PortfolioState = state.repo.load_state(&ctx).await.unwrap_or_default();
    let plans: Vec<models::DcaPlan> = state.repo.load_plans(&ctx).await.unwrap_or_default();
    let settlements: Vec<models::DcaSettlement> =
        state.repo.load_settlements(&ctx).await.unwrap_or_default();
    let snapshots: Vec<models::AlipaySnapshot> = state
        .repo
        .load_alipay_snapshots(&ctx)
        .await
        .unwrap_or_default();
    let transactions: Vec<models::Transaction> =
        state.repo.load_transactions(&ctx).await.unwrap_or_default();
    let nav_cache = state.repo.load_nav_cache(&ctx).await.unwrap_or_default();

    let summary = engine::calculate_portfolio_summary(&config, &p_state);
    let dca_lifecycle = engine::calculate_dca_lifecycle(
        &config,
        &plans,
        &settlements,
        &snapshots,
        &p_state,
        &nav_cache,
        &target_date,
    );

    let mut latest_snaps = std::collections::HashMap::new();
    for s in &snapshots {
        let entry = latest_snaps.entry(s.asset_id.clone()).or_insert(s.clone());
        if s.snapshot_date >= entry.snapshot_date {
            *entry = s.clone();
        }
    }
    let mut reconciliation_results = Vec::new();
    for asset in &config.assets {
        if let Some(s) = latest_snaps.get(&asset.asset_id) {
            reconciliation_results.push(engine::reconciliation::reconcile_asset(
                &config, &p_state, s,
            ));
        }
    }

    let backend_name = match config.storage.backend {
        StorageBackend::Postgres => "postgres",
        _ => "json",
    };

    let extended_summary = engine::report_summary::generate_report_summary(
        &ctx.portfolio_id,
        backend_name,
        &target_date,
        &target_date,
        &transactions,
        &p_state,
    );

    engine::report::generate_investment_report(
        models::ReportPeriod::Daily,
        &format!("每日复盘报告 - {}", target_date),
        &target_date,
        &target_date,
        Some(summary),
        Some(dca_lifecycle),
        None, // Skipping risk overlay for API simplicity unless needed
        None,
        &reconciliation_results,
        Some(extended_summary),
    )
}

pub async fn api_reports_daily_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ReportQuery>,
) -> impl IntoResponse {
    let report = build_daily_report(&state, &params).await;
    Json(report)
}

pub async fn build_weekly_report(
    state: &Arc<AppState>,
    params: &ReportQuery,
) -> models::InvestmentReport {
    let mut ctx = RepositoryContext::default();
    if let Some(pid) = &params.portfolio_id {
        ctx.portfolio_id = pid.clone();
    }

    let target_end = params
        .end
        .clone()
        .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
    let target_start = params.start.clone().unwrap_or_else(|| {
        let end_dt = chrono::NaiveDate::parse_from_str(&target_end, "%Y-%m-%d").unwrap();
        (end_dt - chrono::Duration::days(6))
            .format("%Y-%m-%d")
            .to_string()
    });

    let config: models::ConfigRoot = state.repo.load_config(&ctx).await.unwrap_or_default();
    let p_state: models::PortfolioState = state.repo.load_state(&ctx).await.unwrap_or_default();
    let transactions: Vec<models::Transaction> =
        state.repo.load_transactions(&ctx).await.unwrap_or_default();

    let summary = engine::calculate_portfolio_summary(&config, &p_state);

    let backend_name = match config.storage.backend {
        StorageBackend::Postgres => "postgres",
        _ => "json",
    };

    let extended_summary = engine::report_summary::generate_report_summary(
        &ctx.portfolio_id,
        backend_name,
        &target_start,
        &target_end,
        &transactions,
        &p_state,
    );

    engine::report::generate_investment_report(
        models::ReportPeriod::Weekly,
        &format!("每周复盘报告 ({} - {})", target_start, target_end),
        &target_start,
        &target_end,
        Some(summary),
        None,
        None,
        None,
        &[],
        Some(extended_summary),
    )
}

pub async fn api_reports_weekly_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ReportQuery>,
) -> impl IntoResponse {
    let report = build_weekly_report(&state, &params).await;
    Json(report)
}

pub async fn build_monthly_report(
    state: &Arc<AppState>,
    params: &ReportQuery,
) -> models::InvestmentReport {
    let mut ctx = RepositoryContext::default();
    if let Some(pid) = &params.portfolio_id {
        ctx.portfolio_id = pid.clone();
    }

    let target_month = params
        .month
        .clone()
        .unwrap_or_else(|| Local::now().format("%Y-%m").to_string());
    let target_start = format!("{}-01", target_month);
    let target_end = format!("{}-31", target_month);

    let config: models::ConfigRoot = state.repo.load_config(&ctx).await.unwrap_or_default();
    let p_state: models::PortfolioState = state.repo.load_state(&ctx).await.unwrap_or_default();
    let transactions: Vec<models::Transaction> =
        state.repo.load_transactions(&ctx).await.unwrap_or_default();

    let summary = engine::calculate_portfolio_summary(&config, &p_state);

    let backend_name = match config.storage.backend {
        StorageBackend::Postgres => "postgres",
        _ => "json",
    };

    let extended_summary = engine::report_summary::generate_report_summary(
        &ctx.portfolio_id,
        backend_name,
        &target_start,
        &target_end,
        &transactions,
        &p_state,
    );

    engine::report::generate_investment_report(
        models::ReportPeriod::Monthly,
        &format!("月度复盘报告 - {}", target_month),
        &target_start,
        &target_end,
        Some(summary),
        None,
        None,
        None,
        &[],
        Some(extended_summary),
    )
}

pub async fn api_reports_monthly_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ReportQuery>,
) -> impl IntoResponse {
    let report = build_monthly_report(&state, &params).await;
    Json(report)
}

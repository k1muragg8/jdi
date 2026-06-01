use axum::extract::{Query, State};
use axum::response::IntoResponse;
use serde_json::Value;
use std::sync::Arc;

use pendulum_kelly_cli::repository::JsonRepository;
use pendulum_kelly_cli::web::AppState;

use pendulum_kelly_cli::web_reports::{
    ReportQuery, api_reports_daily_handler, api_reports_monthly_handler, api_reports_weekly_handler,
};
use pendulum_kelly_cli::web_reports_html::{
    html_reports_daily_handler, html_reports_index_handler,
};

async fn setup_state() -> Arc<AppState> {
    let repo = Arc::new(JsonRepository::new(
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
    ));
    let refresh_status = Arc::new(tokio::sync::RwLock::new(
        pendulum_kelly_cli::web::BackgroundRefreshStatus {
            last_market_refresh: None,
            last_fund_refresh: None,
            is_running: true,
            last_error: None,
            latest_daily_report: None,
        },
    ));
    Arc::new(AppState {
        repo,
        refresh_status,
        last_backtest_report: Arc::new(tokio::sync::RwLock::new(None)),
    })
}

async fn get_body_bytes(response: axum::response::Response) -> axum::body::Bytes {
    axum::body::to_bytes(response.into_body(), 1024 * 1024 * 10)
        .await
        .unwrap()
}

#[tokio::test]
async fn test_reports_index_route_works() {
    let state = setup_state().await;
    let query = Query(ReportQuery {
        date: None,
        start: None,
        end: None,
        month: None,
        portfolio_id: None,
    });

    let response = html_reports_index_handler(State(state), query)
        .await
        .into_response();
    let body = get_body_bytes(response).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("复盘报告"));
    assert!(body_str.contains("/reports/daily?portfolio_id=default"));
}

#[tokio::test]
async fn test_daily_report_api_returns_valid_json() {
    let state = setup_state().await;
    let query = Query(ReportQuery {
        date: None,
        start: None,
        end: None,
        month: None,
        portfolio_id: None,
    });

    let response = api_reports_daily_handler(State(state), query)
        .await
        .into_response();
    let body = get_body_bytes(response).await;
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["report_type"], "daily");
    assert!(json.get("extended_summary").is_some());
}

#[tokio::test]
async fn test_weekly_report_api_returns_valid_json() {
    let state = setup_state().await;
    let query = Query(ReportQuery {
        date: None,
        start: None,
        end: None,
        month: None,
        portfolio_id: None,
    });

    let response = api_reports_weekly_handler(State(state), query)
        .await
        .into_response();
    let body = get_body_bytes(response).await;
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["report_type"], "weekly");
}

#[tokio::test]
async fn test_monthly_report_api_returns_valid_json() {
    let state = setup_state().await;
    let query = Query(ReportQuery {
        date: None,
        start: None,
        end: None,
        month: None,
        portfolio_id: None,
    });

    let response = api_reports_monthly_handler(State(state), query)
        .await
        .into_response();
    let body = get_body_bytes(response).await;
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["report_type"], "monthly");
}

#[tokio::test]
async fn test_html_report_page_renders_with_empty_data() {
    let state = setup_state().await;
    let query = Query(ReportQuery {
        date: None,
        start: None,
        end: None,
        month: None,
        portfolio_id: None,
    });

    let response = html_reports_daily_handler(State(state), query)
        .await
        .into_response();
    let body = get_body_bytes(response).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("每日复盘报告"));
    assert!(body_str.contains("投资组合:</strong> default"));
}

#[tokio::test]
async fn test_portfolio_isolation() {
    let state = setup_state().await;
    let query = Query(ReportQuery {
        date: None,
        start: None,
        end: None,
        month: None,
        portfolio_id: Some("test_portfolio".to_string()),
    });

    let response = html_reports_daily_handler(State(state), query)
        .await
        .into_response();
    let body = get_body_bytes(response).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("投资组合:</strong> test_portfolio"));
}

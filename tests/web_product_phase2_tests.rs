//! Phase 2: three-page product, legacy URL redirects.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use pendulum_kelly_cli::repository::RepositoryContext;
use pendulum_kelly_cli::repository::json::JsonRepository;
use pendulum_kelly_cli::web::routes::build_router;
use pendulum_kelly_cli::web::test_pages;
use pendulum_kelly_cli::web::{AppState, BackgroundRefreshStatus};
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;

fn make_state(dir: &str) -> Arc<AppState> {
    std::fs::create_dir_all(dir).ok();
    for (src, name) in [
        ("examples/config.toml", "config.toml"),
        ("examples/portfolio_state.json", "portfolio_state.json"),
        ("examples/transactions.json", "transactions.json"),
    ] {
        let dest = format!("{}/{}", dir, name);
        if !std::path::Path::new(&dest).exists() {
            let _ = std::fs::copy(src, &dest);
        }
    }
    let repo = Arc::new(JsonRepository::new_with_defaults(dir));
    Arc::new(AppState {
        repo,
        ctx: RepositoryContext::default(),
        refresh_status: Arc::new(tokio::sync::RwLock::new(BackgroundRefreshStatus {
            last_market_refresh: None,
            last_fund_refresh: None,
            is_running: false,
            last_error: None,
            latest_daily_report: None,
        })),
        last_backtest_report: Arc::new(tokio::sync::RwLock::new(None)),
        running_jobs: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
    })
}

#[test]
fn nav_only_three_product_tabs() {
    let nav = test_pages::render_nav_shell();
    assert!(nav.contains("概览"));
    assert!(nav.contains("市场"));
    assert!(nav.contains("持仓"));
    assert!(!nav.contains("流水线"));
    assert!(!nav.contains("导入"));
    assert!(!nav.contains("对账"));
    assert!(!nav.contains("系统"));
    assert!(!nav.contains("管理"));
    let o = nav.find("概览").unwrap();
    let m = nav.find("市场").unwrap();
    let h = nav.find("持仓").unwrap();
    assert!(o < m && m < h);
    assert!(nav.contains("href=\"/overview\""));
}

#[tokio::test]
async fn overview_market_holdings_render() {
    let dir = TempDir::new().unwrap().path().to_str().unwrap().to_string();
    let o = test_pages::render_overview(make_state(&dir)).await;
    assert!(o.contains("总资产"));
    assert!(!o.contains("操作台"));
    let m = test_pages::render_market(make_state(&dir)).await;
    assert!(m.contains("market-compact"));
    assert!(m.contains("instEditModal"));
    let h = test_pages::render_holdings(make_state(&dir)).await;
    assert!(h.contains("持仓"));
    assert!(
        h.contains("/api/holdings/bootstrap-alipay")
            || h.contains("资产配置")
            || h.contains("请导入支付宝持仓快照")
    );
}

#[tokio::test]
async fn root_and_legacy_urls_redirect() {
    let dir = TempDir::new().unwrap().path().to_str().unwrap().to_string();
    let app = build_router(make_state(&dir));
    for (path, loc) in [
        ("/", "/overview"),
        ("/daily", "/overview"),
        ("/import", "/holdings"),
        ("/system", "/overview"),
    ] {
        let res = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::PERMANENT_REDIRECT, "{path}");
        assert_eq!(
            res.headers().get("location").unwrap().to_str().unwrap(),
            loc,
            "{path}"
        );
    }
}

#[tokio::test]
async fn no_href_raw_json_in_product_pages() {
    let dir = TempDir::new().unwrap().path().to_str().unwrap().to_string();
    let state = make_state(&dir);
    for html in [
        test_pages::render_overview(state.clone()).await,
        test_pages::render_market(state.clone()).await,
        test_pages::render_holdings(state).await,
    ] {
        assert!(!html.contains("href=\"/api/"));
    }
}

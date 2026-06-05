use axum::body::Body;
use axum::http::{Request, StatusCode};
use pendulum_kelly_cli::models::AlipaySnapshot;
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

#[tokio::test]
async fn main_nav_only_overview_market_holdings() {
    let nav = test_pages::render_nav_shell();
    assert!(nav.contains("概览"));
    assert!(nav.contains("市场"));
    assert!(nav.contains("持仓"));
    let market_pos = nav.find("市场").expect("市场");
    let holdings_pos = nav.find("持仓").expect("持仓");
    let overview_pos = nav.find("概览").expect("概览");
    assert!(overview_pos < market_pos);
    assert!(market_pos < holdings_pos);
    assert!(!nav.contains("流水线"));
    assert!(!nav.contains(">导入<") && !nav.contains("href=\"/import\""));
    assert!(!nav.contains("对账"));
    assert!(!nav.contains("href=\"/system\""));
    assert!(!nav.contains("href=\"/daily\""));
}

#[tokio::test]
async fn overview_renders_portfolio_metrics() {
    let tmp = TempDir::new().unwrap();
    let html = test_pages::render_dashboard(make_state(tmp.path().to_str().unwrap())).await;
    assert!(html.contains("概览"));
    assert!(html.contains("我的资产分布、仓位比例与今日建议"));
    assert!(!html.contains("操作台"));
    assert!(html.contains("总资产"));
    assert!(html.contains("权益仓"));
    assert!(html.contains("债券"));
    assert!(html.contains("货币/现金"));
    assert!(html.contains("今日建议买入"));
    assert!(html.contains("大类资产分布"));
    assert!(html.contains("权益国家/地区"));
    assert!(html.contains("赛道分布"));
    assert!(!html.contains("每日操作流水线"));
}

#[tokio::test]
async fn market_compact_no_row_inline_edit_inputs() {
    let tmp = TempDir::new().unwrap();
    let html = test_pages::render_market(make_state(tmp.path().to_str().unwrap())).await;
    assert!(html.contains("市场"));
    assert!(html.contains("market-compact"));
    assert!(html.contains("instEditModal"));
    assert!(html.contains("openInstEdit"));
    assert!(!html.contains(
        "update-metadata\" method=\"POST\" class=\"market-crud-form\" style=\"margin-top"
    ));
    // row actions are proper buttons, not tiny ghost links
    assert!(html.contains("class=\"btn btn-sm\"") || html.contains("btn btn-sm btn-outline"));
    assert!(html.contains(">编辑<"));
    assert!(html.contains(">刷新<"));
    assert!(html.contains(">归档<"));
    // auto refresh UI and 60s logic present (no raw json nav)
    assert!(html.contains("marketAutoRefreshBar") || html.contains("自动刷新"));
    assert!(
        html.contains("60 秒") || html.contains("startAutoCountdown") || html.contains("60000")
    );
    assert!(!html.contains("href=\"/api/jobs/market/refresh\""));
}

#[tokio::test]
async fn holdings_bootstrap_when_alipay_only() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    let ctx = &state.ctx;
    let mut pf = state.repo.load_state(ctx).await.unwrap();
    pf.asset_holdings.clear();
    state.repo.save_state(ctx, &pf).await.unwrap();
    state
        .repo
        .save_alipay_snapshots(
            ctx,
            &[AlipaySnapshot {
                snapshot_id: "snap_test_1".to_string(),
                asset_id: String::new(),
                fund_code: "000001".to_string(),
                fund_name: "测试基金".to_string(),
                snapshot_date: "2026-06-01".to_string(),
                market_value: 50000.0,
                units: Some(1000.0),
                cost_basis: None,
                nav: Some(50.0),
                nav_date: Some("2026-06-01".to_string()),
                daily_pnl: None,
                total_pnl: None,
                source: "alipay".to_string(),
                created_at: "2026-06-01T00:00:00Z".to_string(),
                note: None,
            }],
        )
        .await
        .unwrap();

    let html = test_pages::render_holdings(make_state(dir)).await;
    // alipay init UI removed from normal holdings; local first
    assert!(!html.contains("用支付宝快照初始化持仓"));
    assert!(html.contains("暂无本地持仓"));
}

#[tokio::test]
async fn legacy_get_routes_redirect_safely() {
    let tmp = TempDir::new().unwrap();
    let state = make_state(tmp.path().to_str().unwrap());
    let app = build_router(state);

    let cases = [
        ("/", "/overview"),
        ("/daily", "/overview"),
        ("/import", "/holdings"),
        ("/reconcile", "/holdings"),
        ("/admin", "/overview"),
        ("/dashboard", "/overview"),
        ("/operation", "/overview"),
    ];
    for (path, expected) in cases {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::PERMANENT_REDIRECT,
            "unexpected status for {path}"
        );
        let location = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(location, expected, "redirect for {path}");
    }
}

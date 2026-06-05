//! Smoke tests after Web module split.

use pendulum_kelly_cli::repository::RepositoryContext;
use pendulum_kelly_cli::repository::json::JsonRepository;
use pendulum_kelly_cli::web::test_pages;
use pendulum_kelly_cli::web::{AppState, BackgroundRefreshStatus};
use std::sync::Arc;
use tempfile::TempDir;

fn make_state(dir: &str) -> Arc<AppState> {
    std::fs::create_dir_all(dir).ok();
    for (src, name) in [
        ("tests/fixtures/json_backend/config.toml", "config.toml"),
        (
            "tests/fixtures/json_backend/portfolio_state.json",
            "portfolio_state.json",
        ),
        (
            "tests/fixtures/json_backend/transactions.json",
            "transactions.json",
        ),
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
async fn product_pages_render_after_phase2() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    assert!(
        test_pages::render_overview(make_state(dir))
            .await
            .contains("概览")
    );
    assert!(
        test_pages::render_market(make_state(dir))
            .await
            .contains("市场")
    );
    assert!(
        test_pages::render_holdings(make_state(dir))
            .await
            .contains("持仓")
    );
}

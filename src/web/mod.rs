//! Web UI: thin root module — routes, state, handlers, views, services.

use crate::repository::{Repository, RepositoryContext};
use anyhow::Result;
use chrono::Local;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod errors;
pub mod handlers;
pub mod product;
pub mod response;
pub mod routes;
pub mod services;
pub mod state;
pub mod utils;
pub mod view_models;
pub mod views;

pub use product::*;
pub use response::{AdminQuery, redirect_with_flash};
pub use state::{AppState, BackgroundRefreshStatus};
pub use utils::*;
pub use views::{layout, layout_with_msg};

pub async fn start_server(
    port: u16,
    repo: Arc<dyn Repository>,
    ctx: RepositoryContext,
) -> Result<()> {
    let refresh_status = Arc::new(RwLock::new(BackgroundRefreshStatus {
        last_market_refresh: None,
        last_fund_refresh: None,
        is_running: true,
        last_error: None,
        latest_daily_report: None,
    }));

    let app_state = Arc::new(AppState {
        repo: repo.clone(),
        ctx: ctx.clone(),
        refresh_status: refresh_status.clone(),
        last_backtest_report: Arc::new(RwLock::new(None)),
        running_jobs: Arc::new(RwLock::new(std::collections::HashSet::new())),
    });

    let _ = repo.mark_stale_running_jobs_interrupted(&ctx).await;

    let repo_loop = repo.clone();
    let refresh_status_loop = refresh_status.clone();
    let ctx_loop = ctx.clone();
    tokio::spawn(async move {
        let ctx = ctx_loop;
        loop {
            let config_res = repo_loop.load_config(&ctx).await;
            if let Ok(config) = config_res {
                if config.market_refresh.enabled {
                    match crate::engine::refresh::refresh_market_data(
                        repo_loop.as_ref(),
                        &ctx,
                        &config,
                    )
                    .await
                    {
                        Ok(_) => {
                            let mut status = refresh_status_loop.write().await;
                            status.last_market_refresh =
                                Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                            status.last_error = None;
                        }
                        Err(e) => {
                            let mut status = refresh_status_loop.write().await;
                            status.last_error = Some(format!("Market refresh failed: {}", e));
                        }
                    }
                }

                let interval = config.market_refresh.interval_seconds;
                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
            } else {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        }
    });

    let app = routes::build_router(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Starting web server at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// HTML render helpers for integration tests (no HTTP server required).
pub mod test_pages {
    use super::AdminQuery;
    use super::handlers;
    use super::state::AppState;
    use super::views::layout;
    use axum::extract::{Query, State};
    use std::sync::Arc;

    pub async fn render_holdings(state: Arc<AppState>) -> String {
        handlers::holdings_handler(State(state), Query(AdminQuery::default()))
            .await
            .0
    }

    pub async fn render_market(state: Arc<AppState>) -> String {
        handlers::instruments_handler(State(state), Query(AdminQuery::default()))
            .await
            .0
    }

    pub async fn render_overview(state: Arc<AppState>) -> String {
        handlers::dashboard_handler(State(state)).await.0
    }

    /// Alias for tests that still call render_dashboard.
    pub async fn render_dashboard(state: Arc<AppState>) -> String {
        render_overview(state).await
    }

    pub fn render_nav_shell() -> String {
        layout("测试", String::new()).0
    }
}

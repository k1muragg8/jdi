//! Axum route registration only.

use crate::web::handlers;
use crate::web::state::AppState;
use axum::Router;
use axum::routing::{delete, get, patch, post};
use std::sync::Arc;

pub fn build_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        // Product pages
        .route("/", get(handlers::redirects::root_redirect))
        .route("/overview", get(handlers::dashboard_handler))
        .route("/dashboard", get(handlers::redirects::redirect_to_overview))
        .route("/market", get(handlers::instruments_handler))
        .route("/holdings", get(handlers::holdings_handler))
        // Legacy page URLs → product
        .route("/daily", get(handlers::redirects::redirect_to_overview))
        .route(
            "/daily-plan",
            get(handlers::redirects::redirect_to_overview),
        )
        .route("/import", get(handlers::redirects::redirect_to_holdings))
        .route(
            "/import/transactions",
            get(handlers::redirects::redirect_to_holdings),
        )
        .route("/reconcile", get(handlers::redirects::redirect_to_holdings))
        .route(
            "/reconcile/alipay",
            get(handlers::redirects::redirect_to_holdings),
        )
        .route("/admin", get(handlers::redirects::redirect_admin_hidden))
        .route("/system", get(handlers::redirects::redirect_admin_hidden))
        .route("/ops", get(handlers::redirects::redirect_admin_hidden))
        .route("/operation", get(handlers::redirects::redirect_to_overview))
        .route("/cash", get(handlers::redirects::redirect_to_holdings))
        .route("/instruments", get(handlers::redirects::redirect_to_market))
        .route("/sectors", get(handlers::redirects::redirect_to_overview))
        .route("/decisions", get(handlers::redirects::redirect_to_overview))
        .route("/decision", get(handlers::redirects::redirect_to_overview))
        .route(
            "/decision/adjusted",
            get(handlers::redirects::redirect_to_overview),
        )
        .route(
            "/transactions",
            get(handlers::redirects::redirect_to_holdings),
        )
        .route("/assets", get(handlers::redirects::redirect_to_holdings))
        .route("/kelly", get(handlers::redirects::redirect_to_overview))
        .route("/dca", get(handlers::redirects::redirect_to_holdings))
        .route(
            "/dca/settlements",
            get(handlers::redirects::redirect_to_holdings),
        )
        .route(
            "/dca/lifecycle",
            get(handlers::redirects::redirect_to_holdings),
        )
        .route(
            "/alipay/holdings",
            get(handlers::redirects::redirect_to_holdings),
        )
        .route("/backtest", get(handlers::redirects::redirect_to_overview))
        .route("/regime", get(handlers::redirects::redirect_to_overview))
        .route("/risk", get(handlers::redirects::redirect_to_overview))
        .route("/proxy", get(handlers::redirects::redirect_to_overview))
        .route(
            "/valuation/proxy",
            get(handlers::redirects::redirect_to_overview),
        )
        // Hidden admin GET pages → overview (POST actions remain)
        .route(
            "/admin/db-status",
            get(handlers::redirects::redirect_admin_hidden),
        )
        .route(
            "/admin/reconcile",
            get(handlers::redirects::redirect_admin_hidden),
        )
        .route(
            "/admin/dca-settlements",
            get(handlers::redirects::redirect_admin_hidden),
        )
        .route(
            "/admin/dca",
            get(handlers::redirects::redirect_admin_hidden),
        )
        .route(
            "/admin/assets",
            get(handlers::redirects::redirect_admin_hidden),
        )
        .route(
            "/admin/instruments",
            get(handlers::redirects::redirect_admin_hidden),
        )
        .route(
            "/admin/audit",
            get(handlers::redirects::redirect_admin_hidden),
        )
        // Holdings bootstrap
        .route(
            "/api/holdings/bootstrap-alipay",
            post(handlers::api_holdings_bootstrap_alipay_handler),
        )
        // Overview API
        .route("/api/dashboard", get(handlers::api_dashboard_handler))
        // Market jobs
        .route(
            "/api/market/refresh-status",
            get(handlers::api_market_refresh_status_handler),
        )
        .route(
            "/api/market/refresh",
            post(handlers::api_market_refresh_handler),
        )
        .route(
            "/api/jobs/market/refresh",
            post(handlers::api_jobs_market_refresh_handler),
        )
        .route(
            "/api/jobs/market/status",
            get(handlers::api_jobs_market_status_handler),
        )
        .route(
            "/api/market/refresh-symbol",
            post(handlers::api_market_refresh_symbol_handler),
        )
        // NAV / classify / daily jobs (overview auto-update)
        .route("/api/nav/refresh", post(handlers::api_nav_refresh_handler))
        .route(
            "/api/jobs/nav/refresh",
            post(handlers::api_jobs_nav_refresh_handler),
        )
        .route(
            "/api/jobs/assets/auto-classify",
            post(handlers::api_jobs_auto_classify_handler),
        )
        .route(
            "/api/assets/auto-classify",
            post(handlers::api_assets_auto_classify_handler),
        )
        .route("/api/daily/run", post(handlers::api_daily_run_handler))
        .route("/api/daily/status", get(handlers::api_daily_status_handler))
        .route("/api/daily/report", get(handlers::api_daily_report_handler))
        .route(
            "/api/jobs/daily/run",
            post(handlers::api_jobs_daily_run_handler),
        )
        .route(
            "/api/jobs/daily/status",
            get(handlers::api_jobs_daily_status_handler),
        )
        // DCA API (holdings inline create)
        .route("/api/dca/plans", get(handlers::api_dca_plans_handler))
        .route("/api/dca/plans", post(handlers::api_dca_add_plan_handler))
        .route(
            "/api/dca/plans/:id",
            patch(handlers::api_dca_update_plan_handler),
        )
        .route(
            "/api/dca/plans/:id",
            delete(handlers::api_dca_remove_plan_handler),
        )
        .route(
            "/api/dca/executions",
            get(handlers::api_dca_executions_handler),
        )
        .route("/api/dca/run-due", post(handlers::api_dca_run_due_handler))
        // Import / Alipay (holdings)
        .route(
            "/api/import/preview",
            post(handlers::api_import_preview_handler),
        )
        .route(
            "/api/import/commit",
            post(handlers::api_import_commit_handler),
        )
        .route(
            "/api/alipay/holdings/preview",
            post(handlers::api_alipay_holdings_preview_handler),
        )
        .route(
            "/api/alipay/holdings/align",
            post(handlers::api_alipay_holdings_align_handler),
        )
        .route(
            "/templates/alipay_holdings_snapshot.csv",
            get(handlers::template_alipay_holdings_handler),
        )
        .route(
            "/templates/transactions.csv",
            get(handlers::template_transactions_handler),
        )
        // Asset actions (holdings)
        .route("/admin/assets/add", post(handlers::admin_asset_add_handler))
        .route(
            "/admin/assets/set-fund-code",
            post(handlers::admin_asset_set_fund_code_handler),
        )
        .route(
            "/admin/assets/rename",
            post(handlers::admin_asset_rename_handler),
        )
        .route(
            "/admin/assets/set-sector",
            post(handlers::admin_asset_set_sector_handler),
        )
        .route(
            "/admin/assets/remove",
            post(handlers::admin_asset_remove_handler),
        )
        .route(
            "/admin/assets/enable",
            post(handlers::admin_asset_enable_handler),
        )
        .route(
            "/admin/assets/disable",
            post(handlers::admin_asset_disable_handler),
        )
        .route(
            "/admin/reconcile/alipay/add",
            post(handlers::admin_add_snapshot_handler),
        )
        .route(
            "/admin/reconcile/apply-confirm",
            post(handlers::admin_reconcile_apply_handler),
        )
        // Instrument actions (market)
        .route(
            "/admin/instruments/add",
            post(handlers::admin_instrument_add_handler),
        )
        .route(
            "/admin/instruments/update-metadata",
            post(handlers::admin_instrument_update_metadata_handler),
        )
        .route(
            "/admin/instruments/enable",
            post(handlers::admin_instrument_enable_handler),
        )
        .route(
            "/admin/instruments/disable",
            post(handlers::admin_instrument_disable_handler),
        )
        .route(
            "/admin/instruments/archive",
            post(handlers::admin_instrument_archive_handler),
        )
        .route(
            "/admin/instruments/restore-defaults",
            post(handlers::admin_instrument_restore_defaults_handler),
        )
        .route(
            "/admin/instruments/cleanup-test",
            post(handlers::admin_instrument_cleanup_test_handler),
        )
        // Cash API (optional adjustment from holdings/overview links)
        .route(
            "/api/cash/set-initial",
            post(handlers::api_cash_set_initial_handler),
        )
        .route("/api/cash/adjust", post(handlers::api_cash_adjust_handler))
        .route(
            "/api/cash/reverse",
            post(handlers::api_cash_reverse_handler),
        )
        .with_state(app_state)
}

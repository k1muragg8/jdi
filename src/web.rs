use crate::repository::{Repository, RepositoryContext};
use crate::{engine, models};
use anyhow::Result;
use axum::{
    Router,
    extract::{Form, Multipart, Query, State},
    response::{Html, Json, Redirect},
    routing::{delete, get, patch, post},
};
use chrono::Local;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundRefreshStatus {
    pub last_market_refresh: Option<String>,
    pub last_fund_refresh: Option<String>,
    pub is_running: bool,
    pub last_error: Option<String>,
    pub latest_daily_report: Option<models::DailyOperationReport>,
}

pub struct AppState {
    pub repo: Arc<dyn Repository>,
    pub refresh_status: Arc<RwLock<BackgroundRefreshStatus>>,
    pub last_backtest_report: Arc<RwLock<Option<models::BacktestReport>>>,
}

pub async fn start_server(port: u16, repo: Arc<dyn Repository>) -> Result<()> {
    let refresh_status = Arc::new(RwLock::new(BackgroundRefreshStatus {
        last_market_refresh: None,
        last_fund_refresh: None,
        is_running: true,
        last_error: None,
        latest_daily_report: None,
    }));

    let app_state = Arc::new(AppState {
        repo: repo.clone(),
        refresh_status: refresh_status.clone(),
        last_backtest_report: Arc::new(RwLock::new(None)),
    });

    // Start background refresh loop
    let repo_loop = repo.clone();
    let refresh_status_loop = refresh_status.clone();
    tokio::spawn(async move {
        let ctx = RepositoryContext::default();
        loop {
            let config_res = repo_loop.load_config(&ctx).await;
            if let Ok(config) = config_res {
                if config.market_refresh.enabled {
                    match engine::refresh::refresh_market_data(repo_loop.as_ref(), &ctx, &config)
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

    let app = Router::new()
        .route("/", get(dashboard_handler))
        .route("/dashboard", get(dashboard_handler))
        .route("/cash", get(cash_handler))
        .route("/api/cash/set-initial", post(api_cash_set_initial_handler))
        .route("/api/cash/adjust", post(api_cash_adjust_handler))
        .route("/api/dashboard", get(api_dashboard_handler))
        .route("/api/dca/plans", get(api_dca_plans_handler))
        .route("/api/dca/plans", post(api_dca_add_plan_handler))
        .route("/api/dca/plans/:id", patch(api_dca_update_plan_handler))
        .route("/api/dca/plans/:id", delete(api_dca_remove_plan_handler))
        .route("/api/dca/executions", get(api_dca_executions_handler))
        .route("/api/dca/run-due", post(api_dca_run_due_handler))
        .route("/api/nav/refresh", post(api_nav_refresh_handler))
        .route(
            "/api/market/refresh-status",
            get(api_market_refresh_status_handler),
        )
        .route("/api/market/refresh", post(api_market_refresh_handler))
        .route(
            "/api/reports/daily",
            get(crate::web_reports::api_reports_daily_handler),
        )
        .route(
            "/api/reports/weekly",
            get(crate::web_reports::api_reports_weekly_handler),
        )
        .route(
            "/api/reports/monthly",
            get(crate::web_reports::api_reports_monthly_handler),
        )
        .route(
            "/reports",
            get(crate::web_reports_html::html_reports_index_handler),
        )
        .route(
            "/reports/daily",
            get(crate::web_reports_html::html_reports_daily_handler),
        )
        .route(
            "/reports/weekly",
            get(crate::web_reports_html::html_reports_weekly_handler),
        )
        .route(
            "/reports/monthly",
            get(crate::web_reports_html::html_reports_monthly_handler),
        )
        .route("/operation", get(operation_page_handler))
        .route("/api/operation/status", get(api_operation_status_handler))
        .route("/api/operation/report", get(api_operation_report_handler))
        .route("/api/operation/run", post(api_operation_run_handler))
        .route(
            "/api/operation/policies",
            get(api_get_operation_policies_handler).post(api_save_operation_policies_handler),
        )
        .route("/ops", get(ops_handler))
        .route("/admin", get(admin_handler))
        .route("/admin/reconcile", get(admin_reconcile_handler))
        .route(
            "/admin/reconcile/alipay/add",
            post(admin_add_snapshot_handler),
        )
        .route(
            "/admin/reconcile/apply-confirm",
            post(admin_reconcile_apply_handler),
        )
        .route(
            "/templates/transactions.csv",
            get(template_transactions_handler),
        )
        .route(
            "/templates/alipay_holdings_snapshot.csv",
            get(template_alipay_holdings_handler),
        )
        .route("/admin/dca-settlements", get(admin_dca_settlements_handler))
        .route(
            "/admin/dca-settlements/add",
            post(admin_add_settlement_handler),
        )
        .route(
            "/admin/dca-settlements/apply-confirm",
            post(admin_settlement_apply_handler),
        )
        .route("/admin/dca", get(admin_dca_handler))
        .route("/admin/dca/add", post(admin_dca_add_handler))
        .route(
            "/admin/dca/update-amount",
            post(admin_dca_update_amount_handler),
        )
        .route("/admin/dca/enable", post(admin_dca_enable_handler))
        .route("/admin/dca/disable", post(admin_dca_disable_handler))
        .route("/admin/dca/remove", post(admin_dca_remove_handler))
        .route("/admin/assets", get(admin_assets_handler))
        .route("/admin/assets/add", post(admin_asset_add_handler))
        .route(
            "/admin/assets/set-fund-code",
            post(admin_asset_set_fund_code_handler),
        )
        .route(
            "/api/assets/auto-classify",
            post(api_assets_auto_classify_handler),
        )
        .route("/admin/assets/rename", post(admin_asset_rename_handler))
        .route(
            "/admin/assets/set-sector",
            post(admin_asset_set_sector_handler),
        )
        .route("/admin/assets/remove", post(admin_asset_remove_handler))
        .route("/admin/instruments", get(admin_instruments_handler))
        .route(
            "/admin/instruments/enable",
            post(admin_instrument_enable_handler),
        )
        .route(
            "/admin/instruments/disable",
            post(admin_instrument_disable_handler),
        )
        .route(
            "/admin/instruments/update-metadata",
            post(admin_instrument_update_metadata_handler),
        )
        .route("/admin/audit", get(admin_audit_handler))
        .route("/holdings", get(holdings_handler))
        .route("/sectors", get(sectors_handler))
        .route("/decisions", get(decisions_handler))
        .route("/decision", get(decisions_handler)) // Alias for stability
        .route("/decision/adjusted", get(adjusted_decision_handler))
        .route("/transactions", get(transactions_handler))
        .route("/assets", get(assets_handler))
        .route("/valuation/proxy", get(proxy_valuation_handler))
        .route("/proxy", get(proxy_valuation_handler)) // Alias for stability
        .route("/regime", get(regime_handler))
        .route("/risk", get(risk_handler))
        .route("/kelly", get(kelly_handler))
        .route("/daily-plan", get(kelly_handler)) // Alias
        .route("/daily", get(daily_handler))
        .route("/backtest", get(backtest_page_handler))
        .route("/api/backtest/run", post(api_backtest_run_handler))
        .route("/api/backtest/latest", get(api_backtest_latest_handler))
        .route("/api/daily/run", post(api_daily_run_handler))
        .route("/api/daily/status", get(api_daily_status_handler))
        .route("/api/daily/report", get(api_daily_report_handler))
        .route("/instruments", get(instruments_handler))
        .route("/market", get(instruments_handler))
        .route("/dca", get(dca_handler))
        .route("/dca/settlements", get(dca_settlements_handler))
        .route("/dca/lifecycle", get(dca_lifecycle_handler))
        .route("/import", get(import_handler))
        .route("/import/transactions", get(import_transactions_handler))
        .route("/api/import/preview", post(api_import_preview_handler))
        .route("/api/import/commit", post(api_import_commit_handler))
        .route("/alipay/holdings", get(alipay_holdings_handler))
        .route(
            "/api/alipay/holdings/preview",
            post(api_alipay_holdings_preview_handler),
        )
        .route(
            "/api/alipay/holdings/align",
            post(api_alipay_holdings_align_handler),
        )
        .route("/reconcile/alipay", get(alipay_reconcile_handler))
        .route("/reconcile", get(system_reconcile_handler))
        .route(
            "/api/reconciliation/report",
            get(api_reconciliation_report_handler),
        )
        .route("/api/decision/explain", get(api_decision_explain_handler))
        .route("/api/kelly/plan", get(api_kelly_plan_handler))
        .route("/api/daily-plan", get(api_kelly_plan_handler)) // Alias
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Starting web server at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub fn layout(title: &str, content: String) -> Html<String> {
    layout_with_msg(title, content, None, None)
}

fn layout_with_msg(
    title: &str,
    content: String,
    success: Option<String>,
    error: Option<String>,
) -> Html<String> {
    let mut msg_html = String::new();
    if let Some(s) = success {
        msg_html.push_str(&format!(
            r#"<div class="message-banner message-success">
                <span class="banner-icon">✓</span>
                <span>{}</span>
            </div>"#,
            s
        ));
    }
    if let Some(e) = error {
        msg_html.push_str(&format!(
            r#"<div class="message-banner message-error">
                <span class="banner-icon">✕</span>
                <span>{}</span>
            </div>"#,
            e
        ));
    }

    Html(format!(
        r#"
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <title>{} - JDI Portfolio</title>
    <style>
        :root {{
            --primary-color: #0052D9;
            --primary-light: #E8F3FF;
            --bg-color: #F2F3F5;
            --card-bg: #FFFFFF;
            --text-main: #1D2129;
            --text-muted: #86909C;
            --border-color: #E5E6EB;
            --up-color: #F53F3F;
            --down-color: #00B42A;
            --warn-color: #FF7D00;
            --info-color: #165DFF;
            --nav-bg: rgba(255, 255, 255, 0.8);
            --radius: 12px;
            --shadow-sm: 0 2px 8px rgba(0,0,0,0.04);
            --shadow-md: 0 4px 16px rgba(0,0,0,0.08);
        }}
        * {{ box-sizing: border-box; -webkit-tap-highlight-color: transparent; }}
        body {{ 
            font-family: -apple-system, BlinkMacSystemFont, "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif; 
            line-height: 1.6; 
            color: var(--text-main); 
            background-color: var(--bg-color);
            margin: 0;
            padding: 0;
            padding-bottom: 80px; 
        }}
        
        .container {{ max-width: 1200px; margin: 0 auto; padding: 32px 24px; }}
        
        header {{ 
            background: var(--nav-bg); 
            backdrop-filter: blur(10px);
            -webkit-backdrop-filter: blur(10px);
            border-bottom: 1px solid var(--border-color); 
            position: sticky; 
            top: 0; 
            z-index: 1000; 
        }}
        .header-wrap {{ display: flex; align-items: center; justify-content: space-between; padding: 0 24px; height: 64px; max-width: 1200px; margin: 0 auto; }}
        .logo {{ font-weight: 900; font-size: 1.25rem; color: var(--primary-color); text-decoration: none; letter-spacing: -0.5px; display: flex; align-items: center; gap: 8px; }}
        .logo::before {{ content: '📈'; font-size: 1.4rem; }}
        
        .nav-desktop {{ display: flex; gap: 4px; }}
        .nav-desktop a {{ 
            color: var(--text-main); 
            text-decoration: none; 
            padding: 8px 16px; 
            font-size: 0.95rem; 
            font-weight: 600;
            border-radius: 8px;
            transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
        }}
        .nav-desktop a:hover {{ background: var(--bg-color); color: var(--primary-color); }}
        .nav-desktop a.active {{ color: var(--primary-color); background: var(--primary-light); }}
        
        .card {{ 
            background: var(--card-bg); 
            border-radius: var(--radius); 
            padding: 24px; 
            margin-bottom: 24px; 
            box-shadow: var(--shadow-sm); 
            border: 1px solid var(--border-color); 
            transition: transform 0.2s, box-shadow 0.2s;
        }}
        .card:hover {{ box-shadow: var(--shadow-md); }}
        .card-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; padding-bottom: 12px; border-bottom: 1px solid var(--bg-color); }}
        .card-title {{ font-size: 0.9rem; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; }}
        .card-value {{ font-size: 2.25rem; font-weight: 900; letter-spacing: -1.5px; line-height: 1; margin: 8px 0; font-variant-numeric: tabular-nums; }}
        .card-sub {{ font-size: 0.85rem; color: var(--text-muted); font-weight: 500; }}
        
        .dashboard-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 24px; margin-bottom: 24px; }}
        
        h1 {{ font-size: 2rem; font-weight: 900; margin: 0 0 32px 0; color: var(--text-main); letter-spacing: -0.75px; }}
        h2 {{ font-size: 1.5rem; font-weight: 800; margin: 48px 0 24px 0; letter-spacing: -0.5px; display: flex; align-items: center; gap: 12px; }}
        h3 {{ font-size: 1.15rem; font-weight: 700; margin: 24px 0 16px 0; }}

        .table-container {{ background: var(--card-bg); border-radius: var(--radius); overflow: hidden; border: 1px solid var(--border-color); margin-bottom: 32px; box-shadow: var(--shadow-sm); }}
        .table-wrap {{ overflow-x: auto; }}
        table {{ width: 100%; border-collapse: collapse; font-size: 0.95rem; }}
        th {{ background: var(--bg-color); color: var(--text-muted); font-weight: 700; text-align: left; padding: 14px 20px; border-bottom: 1px solid var(--border-color); font-size: 0.75rem; text-transform: uppercase; letter-spacing: 1px; }}
        td {{ padding: 16px 20px; border-bottom: 1px solid var(--bg-color); vertical-align: middle; }}
        tr:hover td {{ background-color: rgba(0, 82, 217, 0.02); }}
        tr:last-child td {{ border-bottom: none; }}
        .text-right {{ text-align: right; }}
        .tabular {{ font-variant-numeric: tabular-nums; }}
        
        .badge {{ display: inline-flex; align-items: center; justify-content: center; padding: 4px 10px; border-radius: 6px; font-size: 0.75rem; font-weight: 800; color: #fff; background: var(--text-muted); white-space: nowrap; }}
        .badge-red {{ background: var(--up-color); }}
        .badge-green {{ background: var(--down-color); }}
        .badge-blue {{ background: var(--info-color); }}
        .badge-orange {{ background: var(--warn-color); }}
        .badge-gray {{ background: var(--text-muted); }}
        .badge-outline {{ background: transparent; border: 1.5px solid currentColor; color: inherit; }}
        
        .text-up {{ color: var(--up-color); font-weight: 800; }}
        .text-down {{ color: var(--down-color); font-weight: 800; }}
        .text-warn {{ color: var(--warn-color); font-weight: 700; }}
        
        .message-banner {{ padding: 16px 24px; margin-bottom: 24px; border-radius: var(--radius); font-size: 1rem; border: 1px solid transparent; font-weight: 600; display: flex; align-items: center; gap: 16px; animation: slideIn 0.3s ease-out; }}
        @keyframes slideIn {{ from {{ transform: translateY(-10px); opacity: 0; }} to {{ transform: translateY(0); opacity: 1; }} }}
        .message-success {{ background: #EFFFF1; color: #008026; border-color: #B2F0C1; }}
        .message-error {{ background: #FFF1F0; color: #AD352F; border-color: #FFCCC7; }}
        .message-warning {{ background: #FFF7E6; color: #996000; border-color: #FFE7BA; }}
        .banner-icon {{ font-size: 1.2rem; font-weight: 900; }}
        
        .form-group {{ margin-bottom: 24px; }}
        .form-group label {{ display: block; margin-bottom: 8px; font-size: 0.9rem; font-weight: 700; color: var(--text-main); }}
        input, select, textarea {{ 
            width: 100%; padding: 12px 16px; border: 1px solid var(--border-color); border-radius: 8px; font-size: 1rem; outline: none; transition: all 0.2s; background: #FFF; font-weight: 500;
        }}
        input:focus, select:focus, textarea:focus {{ border-color: var(--primary-color); box-shadow: 0 0 0 4px rgba(0, 82, 217, 0.1); }}
        
        .btn {{ 
            display: inline-flex; align-items: center; justify-content: center; padding: 10px 24px; background: var(--primary-color); color: #fff; text-decoration: none; border-radius: 8px; 
            font-size: 0.95rem; font-weight: 700; border: none; cursor: pointer; transition: all 0.2s; gap: 8px;
        }}
        .btn:hover {{ opacity: 0.9; transform: translateY(-1px); }}
        .btn:active {{ transform: translateY(0); }}
        .btn:disabled {{ opacity: 0.5; cursor: not-allowed; transform: none; }}
        .btn-sm {{ padding: 6px 16px; font-size: 0.85rem; border-radius: 6px; }}
        .btn-outline {{ background: transparent; border: 1px solid var(--border-color); color: var(--text-main); }}
        .btn-outline:hover {{ background: var(--bg-color); border-color: var(--text-muted); }}
        .btn-ghost {{ background: transparent; color: var(--primary-color); padding: 0; box-shadow: none; font-weight: 600; border: none; cursor: pointer; }}
        .btn-ghost:hover {{ text-decoration: underline; background: transparent; transform: none; }}

        .nav-bottom {{ 
            display: none; position: fixed; bottom: 0; left: 0; right: 0; height: 64px; background: var(--nav-bg); 
            backdrop-filter: blur(10px); -webkit-backdrop-filter: blur(10px); border-top: 1px solid var(--border-color); z-index: 1000;
            justify-content: space-around; align-items: center; padding-bottom: env(safe-area-inset-bottom);
        }}
        .nav-item {{ display: flex; flex-direction: column; align-items: center; text-decoration: none; color: var(--text-muted); font-size: 0.7rem; font-weight: 700; }}
        .nav-item.active {{ color: var(--primary-color); }}
        .nav-icon {{ font-size: 1.25rem; margin-bottom: 2px; }}

        .empty-state {{ text-align: center; padding: 80px 24px; background: var(--card-bg); border-radius: var(--radius); border: 2px dashed var(--border-color); }}
        .empty-state-icon {{ font-size: 3rem; margin-bottom: 16px; display: block; opacity: 0.5; }}
        .empty-state-text {{ color: var(--text-muted); font-size: 1rem; }}
        
        .action-group {{ display: flex; gap: 12px; flex-wrap: wrap; margin-top: 24px; }}
        
        @media (max-width: 768px) {{
            .container {{ padding: 24px 16px; }}
            .nav-desktop {{ display: none; }}
            .nav-bottom {{ display: flex; }}
            .dashboard-grid {{ grid-template-columns: 1fr; gap: 16px; }}
            h1 {{ font-size: 1.75rem; }}
            .card {{ padding: 20px; }}
            .card-value {{ font-size: 2rem; }}
            td, th {{ padding: 12px 16px; }}
        }}
    </style>
</head>
<body>
    <header>
        <div class="header-wrap">
            <a href="/" class="logo">JDI PORTFOLIO</a>
            <nav class="nav-desktop">
                <a href="/dashboard">操作台</a>
                <a href="/daily">流水线</a>
                <a href="/holdings">持仓</a>
                <a href="/import">导入</a>
                <a href="/reconcile">对账</a>
                <a href="/market">市场</a>
                <a href="/admin">管理</a>
            </nav>
        </div>
    </header>

    <main class="container">
        {}
        {}
    </main>

    <nav class="nav-bottom">
        <a href="/dashboard" class="nav-item">
            <span class="nav-icon">📊</span>
            <span>首页</span>
        </a>
        <a href="/holdings" class="nav-item">
            <span class="nav-icon">💰</span>
            <span>持仓</span>
        </a>
        <a href="/daily" class="nav-item">
            <span class="nav-icon">⚡</span>
            <span>流水</span>
        </a>
        <a href="/market" class="nav-item">
            <span class="nav-icon">📈</span>
            <span>市场</span>
        </a>
        <a href="/admin" class="nav-item">
            <span class="nav-icon">⚙️</span>
            <span>管理</span>
        </a>
    </nav>

    <script>
        document.querySelectorAll('.nav-desktop a, .nav-bottom a').forEach(link => {{
            const path = window.location.pathname;
            const href = link.getAttribute('href');
            if (path === href || (href !== '/' && path.startsWith(href))) {{
                link.classList.add('active');
            }}
        }});
    </script>
<script>
    async function refreshMarket(btn) {{
        const originalText = btn ? btn.innerText : '刷新';
        if (btn) {{
            btn.disabled = true;
            btn.innerText = '⏳ 正在刷新...';
        }}

        try {{
            const res = await fetch('/api/market/refresh', {{ method: 'POST' }});
            const result = await res.json();
            if (result.success) {{
                if (btn) btn.innerText = '✔️ 刷新成功';
                setTimeout(() => location.reload(), 500);
            }} else {{
                alert('刷新失败: ' + result.message);
                if (btn) {{
                    btn.disabled = false;
                    btn.innerText = originalText;
                }}
            }}
        }} catch (e) {{
            alert('网络错误: ' + e);
            if (btn) {{
                btn.disabled = false;
                btn.innerText = originalText;
            }}
        }}
    }}

    async function autoClassify(btn) {{
        if (btn) {{
            btn.disabled = true;
            btn.innerText = '⏳ 处理中...';
        }}
        try {{
            const res = await fetch('/api/assets/auto-classify', {{ method: 'POST' }});
            if (res.ok) {{
                location.reload();
            }} else {{
                alert('分类失败');
                if (btn) btn.disabled = false;
            }}
        }} catch (e) {{
            alert('网络错误: ' + e);
            if (btn) btn.disabled = false;
        }}
    }}

    async function refreshNav(btn) {{
        if (btn) {{
            btn.disabled = true;
            btn.innerText = '⏳ 正在刷新...';
        }}
        try {{
            const res = await fetch('/api/nav/refresh', {{ method: 'POST' }});
            const result = await res.json();
            if (result.success) {{
                if (btn) btn.innerText = '✔️ 刷新成功';
                setTimeout(() => location.reload(), 500);
            }} else {{
                alert('刷新失败: ' + result.message);
                if (btn) {{
                    btn.disabled = false;
                    btn.innerText = '💰 刷新净值';
                }}
            }}
        }} catch (e) {{
            alert('网络错误: ' + e);
            if (btn) {{
                btn.disabled = false;
                btn.innerText = '💰 刷新净值';
            }}
        }}
    }}

    async function runDueDca(btn) {{
        if (!confirm('确定要执行今日到期的定投计划吗？')) return;
        if (btn) {{
            btn.disabled = true;
            btn.innerText = '⏳ 正在执行...';
        }}
        try {{
            const res = await fetch('/api/dca/run-due', {{ method: 'POST' }});
            const result = await res.json();
            if (result.success) {{
                if (btn) btn.innerText = '✔️ 执行成功';
                setTimeout(() => location.reload(), 500);
            }} else {{
                alert('执行失败: ' + result.message);
                if (btn) {{
                    btn.disabled = false;
                    btn.innerText = '🤖 执行定投';
                }}
            }}
        }} catch (e) {{
            alert('网络错误: ' + e);
            if (btn) {{
                btn.disabled = false;
                btn.innerText = '🤖 执行定投';
            }}
        }}
    }}
</script>
</body>
</html>
"#,
        title,
        msg_html,
        content
    ))
}

pub fn fmt_f64_opt(val: Option<f64>, precision: usize) -> String {
    match val {
        Some(v) => format!("{:.1$}", v, precision),
        None => "-".to_string(),
    }
}

pub fn fmt_amount(val: f64) -> String {
    format!("{:.2}", val)
}

pub fn fmt_nav(val: f64) -> String {
    format!("{:.4}", val)
}

pub fn fmt_pct(val: f64) -> String {
    format!("{:.2}%", val * 100.0)
}

pub fn safe_div(num: f64, den: f64) -> String {
    if den.abs() < 0.000001 {
        "N/A".to_string()
    } else {
        fmt_pct(num / den)
    }
}

fn color_class(val: f64) -> &'static str {
    if val > 0.000001 {
        "text-up"
    } else if val < -0.000001 {
        "text-down"
    } else {
        ""
    }
}

fn badge_regime(label: &str) -> String {
    let color = match label {
        "极冷" | "偏冷" => "badge-green",
        "极热" | "偏热" | "过热" => "badge-red",
        "中性" => "badge-gray",
        _ => "badge-gray",
    };
    format!("<span class='badge {}'>{}</span>", color, label)
}

fn badge_risk(label: &str) -> String {
    let color = match label {
        "低风险" => "badge-green",
        "正常" | "正常风险" => "badge-blue",
        "偏高" => "badge-orange",
        "高风险" | "极高风险" => "badge-red",
        "查询失败" => "badge-gray",
        _ => "badge-gray",
    };
    format!("<span class='badge {}'>{}</span>", color, label)
}

pub fn badge_status(status: &str) -> String {
    let color = match status {
        "正常" | "均衡" => "badge-blue",
        "低配" => "badge-green",
        "超配" => "badge-red",
        "估算" | "模拟" => "badge-orange",
        "过期" | "查询失败" => "badge-gray",
        _ => "badge-gray",
    };
    format!("<span class='badge {}'>{}</span>", color, status)
}

async fn fetch_dashboard_summary(
    state: &AppState,
    ctx: &RepositoryContext,
) -> Result<models::DashboardSummary> {
    let config = state.repo.load_config(ctx).await?;
    let date = Local::now().format("%Y-%m-%d").to_string();

    let portfolio_state = state.repo.load_state(ctx).await?;
    let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);

    let dca_plans = state.repo.load_plans(ctx).await?;
    let settlements = state.repo.load_settlements(ctx).await?;
    let snapshots = state.repo.load_alipay_snapshots(ctx).await?;
    let nav_cache = state.repo.load_nav_cache(ctx).await?;

    let lifecycle = engine::dca_lifecycle::calculate_dca_lifecycle(
        &config,
        &dca_plans,
        &settlements,
        &snapshots,
        &portfolio_state,
        &nav_cache,
        &date,
    );

    let mut cache_status = state.repo.load_cache_status(ctx).await.unwrap_or_default();
    let market_cache = state.repo.load_market_cache(ctx).await.unwrap_or_default();
    cache_status.market_cache_size = market_cache.entries.len();
    cache_status.last_market_update = market_cache
        .entries
        .iter()
        .map(|e| &e.fetched_at)
        .max()
        .cloned();
    let risk_cache = state.repo.load_risk_cache(ctx).await?.unwrap_or_default();
    let regime_cache = state.repo.load_regime_cache(ctx).await?;

    let mut regimes = std::collections::HashMap::new();
    for entry in &regime_cache.entries {
        for asset in &config.assets {
            let symbol_opt = asset
                .reference_instrument_symbol
                .clone()
                .or(asset.reference_index_symbol.clone());
            if let Some(s) = symbol_opt {
                if s == entry.symbol {
                    regimes.insert(asset.asset_id.clone(), entry.result.clone());
                }
            }
        }
    }

    let decision = engine::explanation::explain_decision(
        &config,
        &portfolio_state,
        ctx.portfolio_id.clone(),
        date.clone(),
        &risk_cache.overlay,
        &regimes,
    );

    let operation_status = state
        .repo
        .load_operation_status(ctx)
        .await
        .unwrap_or_default();

    let mut alipay_total_value = None;
    let mut alipay_snapshot_date = None;
    if !snapshots.is_empty() {
        let mut latest_snaps: std::collections::HashMap<String, models::AlipaySnapshot> =
            std::collections::HashMap::new();
        for s in &snapshots {
            let key = if s.asset_id.is_empty() {
                format!("unmatched_{}", s.fund_code)
            } else {
                s.asset_id.clone()
            };
            let entry = latest_snaps.entry(key).or_insert(s.clone());
            if s.snapshot_date >= entry.snapshot_date {
                *entry = s.clone();
            }
        }
        alipay_total_value = Some(latest_snaps.values().map(|s| s.market_value).sum::<f64>());
        alipay_snapshot_date = latest_snaps
            .values()
            .map(|s| &s.snapshot_date)
            .max()
            .cloned();
    }

    let unclassified_asset_count = config
        .assets
        .iter()
        .filter(|a| a.enabled && (a.sector.is_empty() || a.sector == "未分类"))
        .count();

    let transactions = state.repo.load_transactions(ctx).await.unwrap_or_default();
    let report = crate::engine::portfolio_reconciliation::reconcile_portfolio(
        &ctx.portfolio_id,
        &portfolio_state,
        &transactions,
    );
    let reconciliation_issue_count = report.issues.len();

    let mut alipay_mismatch_count = 0;
    if let Ok(snapshots) = state.repo.load_alipay_snapshots(ctx).await {
        let mut latest_snaps = std::collections::HashMap::new();
        for s in &snapshots {
            let key = if s.asset_id.is_empty() {
                format!("unmatched_{}", s.fund_code)
            } else {
                s.asset_id.clone()
            };
            let entry = latest_snaps.entry(key).or_insert(s.clone());
            if s.snapshot_date >= entry.snapshot_date {
                *entry = s.clone();
            }
        }
        let mut processed_keys = std::collections::HashSet::new();
        for asset in &config.assets {
            if let Some(s) = latest_snaps.get(&asset.asset_id) {
                let res =
                    crate::engine::reconciliation::reconcile_asset(&config, &portfolio_state, s);
                if res.status == "需要校准"
                    || res.status == "份额不一致"
                    || res.status == "明显差异"
                    || res.status == "缺少系统持仓"
                {
                    alipay_mismatch_count += 1;
                }
                processed_keys.insert(asset.asset_id.clone());
            }
        }
        for (key, s) in latest_snaps {
            if !processed_keys.contains(&key) {
                let res =
                    crate::engine::reconciliation::reconcile_asset(&config, &portfolio_state, &s);
                if res.status == "需要校准"
                    || res.status == "份额不一致"
                    || res.status == "明显差异"
                    || res.status == "缺少系统持仓"
                {
                    alipay_mismatch_count += 1;
                }
            }
        }
    }

    Ok(models::DashboardSummary {
        portfolio: summary,
        lifecycle,
        cache_status,
        decision,
        risk_overlay: risk_cache.overlay,
        operation_status,
        backend: state.repo.name(),
        portfolio_name: config.portfolio.name,
        date,
        alipay_total_value,
        alipay_snapshot_date,
        unclassified_asset_count,
        reconciliation_issue_count,
        alipay_mismatch_count,
    })
}

async fn dashboard_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    match fetch_dashboard_summary(&state, &ctx).await {
        Ok(summary) => {
            // Hero section data
            let total_suggested = summary.decision.asset_explanations.iter().map(|a| a.final_suggested_buy).sum::<f64>();
            let target_equity_pct = summary.operation_status.policy.target_equity_weight * 100.0;
            let current_equity_pct = if summary.portfolio.total_asset_value > 0.0 { (summary.portfolio.equity_value / summary.portfolio.total_asset_value) * 100.0 } else { 0.0 };
            let equity_gap = summary.decision.equity_gap;
            
            // 1. Status Banners (High priority only)
            let mut banners = String::new();
            if summary.portfolio.available_cash < 0.0 {
                banners.push_str("<div class='message-banner message-error'>⚠️ <strong>现金余额异常:</strong> 账面现金为负，请检查流水或<a href='/cash'>补录现金</a>。</div>");
            }

            // 2. Action Required / To-do items
            let mut todo_items = Vec::new();
            if summary.unclassified_asset_count > 0 {
                todo_items.push(format!("<li><span class='text-warn'>{} 个资产未分类: </span>影响配置建议。 <button onclick='autoClassify(this)' class='btn-ghost'>自动分类</button></li>", summary.unclassified_asset_count));
            }
            if summary.cache_status.market_cache_size == 0 {
                todo_items.push("<li><span class='text-warn'>行情缓存为空: </span>无法计算最新建议。 <button onclick='refreshMarket(this)' class='btn-ghost'>一键刷新</button></li>".to_string());
            }
            if summary.reconciliation_issue_count > 0 {
                todo_items.push(format!("<li><a href='/reconcile' class='text-warn'>{} 项系统对账异常</a>: 内部账本数据不一致。</li>", summary.reconciliation_issue_count));
            }
            if summary.alipay_mismatch_count > 0 {
                todo_items.push(format!("<li><a href='/alipay/holdings' class='text-warn'>{} 项支付宝快照不匹配</a>: 外部持仓与本地不符。</li>", summary.alipay_mismatch_count));
            }
            if summary.operation_status.last_run_at.is_none() {
                todo_items.push("<li><a href='/daily' class='text-info'>今日流水线尚未运行</a>: 建议执行每日自动任务。</li>".to_string());
            }
            
            let todo_card = if todo_items.is_empty() {
                r#"<div class="card" style="border-color: var(--down-color); background: rgba(0, 180, 42, 0.02);">
                    <div class="card-header"><span class="card-title">🚀 运行状态</span></div>
                    <div style="color: var(--down-color); font-weight: 700;">✨ 状态良好，今日无待处理事项。</div>
                </div>"#.to_string()
            } else {
                format!(
                    r#"<div class="card" style="border-color: var(--warn-color); background: rgba(255, 125, 0, 0.02);">
                        <div class="card-header"><span class="card-title" style="color: var(--warn-color);">⚠️ 待处理事项 (Action Required)</span></div>
                        <ul style="margin: 0; padding-left: 20px; font-size: 0.9rem; line-height: 1.8;">
                            {}
                        </ul>
                    </div>"#,
                    todo_items.join("")
                )
            };

            // 3. Top Buys
            let mut top_buys_html = String::new();
            let mut top_buys: Vec<_> = summary.decision.asset_explanations.iter().filter(|a| a.final_suggested_buy > 0.0).collect();
            top_buys.sort_by(|a, b| b.final_suggested_buy.partial_cmp(&a.final_suggested_buy).unwrap());

            for asset in top_buys.iter().take(5) {
                top_buys_html.push_str(&format!(
                    r#"<div class="ranking-row">
                        <div style="display: flex; align-items: center; gap: 12px;">
                            <div class="metric-pill">{}</div>
                            <div>
                                <div style="font-weight: 700;">{}</div>
                                <div style="font-size: 0.75rem; color: var(--text-muted);"><code>{}</code></div>
                            </div>
                        </div>
                        <div class="text-right">
                            <div class="text-up tabular" style="font-weight: 900; font-size: 1.1rem;">{:.2}</div>
                            <div style="font-size: 0.7rem; color: var(--text-muted); font-weight: 600;">建议买入</div>
                        </div>
                    </div>"#,
                    asset.sector_id, asset.fund_name, asset.fund_code, asset.final_suggested_buy
                ));
            }

            if top_buys_html.is_empty() {
                top_buys_html = "<div class='empty-state' style='padding: 32px 0;'><span class='empty-state-icon'>☕</span><div class='empty-state-text'>今日暂无买入建议</div></div>".to_string();
            }

            // 4. Sector Allocation
            let mut allocation_rows = String::new();
            for s in &summary.portfolio.sector_summaries {
                let target_pct = s.target_weight * 100.0;
                let current_pct = s.current_weight * 100.0;
                let (status_text, color_class) = match s.status.as_str() {
                    "underweight" => ("低配", "badge-green"),
                    "overweight" => ("超配", "badge-red"),
                    _ => ("均衡", "badge-blue"),
                };

                allocation_rows.push_str(&format!(
                    r#"<tr>
                        <td><strong>{}</strong></td>
                        <td class="tabular">{:.1}%</td>
                        <td class="tabular">{:.1}%</td>
                        <td class="text-right"><span class="badge {}">{}</span></td>
                    </tr>"#,
                    s.sector_name, current_pct, target_pct, color_class, status_text
                ));
            }

            // 5. Quick Actions
            let quick_actions = r#"
                <div class="card">
                    <div class="card-header"><span class="card-title">⚡ 快速操作</span></div>
                    <div class="action-group" style="margin-top: 0;">
                        <button onclick="refreshMarket(this)" class="btn btn-sm btn-outline">🔄 刷新行情</button>
                        <button onclick="refreshNav(this)" class="btn btn-sm btn-outline">💰 刷新净值</button>
                        <a href="/import" class="btn btn-sm btn-outline">📥 导入数据</a>
                        <button onclick="runDueDca(this)" class="btn btn-sm btn-outline">🤖 执行定投</button>
                    </div>
                </div>
            "#;

            // 6. Alipay Sync Info
            let alipay_diff_html = if let Some(val) = summary.alipay_total_value {
                let diff = summary.portfolio.total_asset_value - val;
                let pct = if val > 0.0 { diff.abs() / val * 100.0 } else { 0.0 };
                let (sign, color) = if diff > 0.0 { ("+", "text-up") } else if diff < 0.0 { ("-", "text-down") } else { ("", "text-muted") };
                format!("<span class='{}' style='font-weight: 700;'>{}{} ({:.2}%)</span>", color, sign, format!("{:.2}", diff.abs()), pct)
            } else {
                "<span class='text-muted'>未同步</span>".to_string()
            };

            let summary_analysis = if summary.decision.risk_summary.factors.is_empty() {
                summary.decision.risk_summary.label.clone()
            } else {
                format!("{}:<br>{}", summary.decision.risk_summary.label, summary.decision.risk_summary.factors.join("<br>"))
            };

            let content = format!(
                r#"
                {}

                <div class="card" style="background: linear-gradient(135deg, #0052D9 0%, #003EB3 100%); color: white; border: none; padding: 32px; overflow: hidden; position: relative;">
                    <div style="position: absolute; right: -20px; top: -20px; font-size: 10rem; opacity: 0.1; transform: rotate(15deg);">📈</div>
                    <div style="display: flex; justify-content: space-between; align-items: flex-start; position: relative; z-index: 1;">
                        <div>
                            <div style="opacity: 0.8; font-size: 1rem; margin-bottom: 8px; font-weight: 600;">组合总市值 (Portfolio Value)</div>
                            <div style="font-size: 3rem; font-weight: 950; letter-spacing: -2px; margin-bottom: 4px;">{:.2} <small style="font-size: 1.2rem; font-weight: 600; opacity: 0.7;">CNY</small></div>
                            <div style="font-size: 0.9rem; opacity: 0.8;">支付宝差异: {}</div>
                        </div>
                        <div style="text-align: right;">
                            <div style="opacity: 0.8; font-size: 0.9rem; font-weight: 600;">今日建议买入 (Today)</div>
                            <div style="font-size: 2rem; font-weight: 900; color: #FFF; letter-spacing: -1px;">{:.2}</div>
                            <div style="font-size: 0.8rem; opacity: 0.9; margin-top: 4px;">{}</div>
                        </div>
                    </div>
                    <div style="display: flex; gap: 32px; font-size: 0.95rem; opacity: 0.95; border-top: 1px solid rgba(255,255,255,0.2); padding-top: 20px; margin-top: 24px; position: relative; z-index: 1; overflow-x: auto;">
                        <div style="white-space: nowrap;">可用现金: <strong class="tabular" style="font-size: 1.15rem;">{:.2}</strong></div>
                        <div style="white-space: nowrap;">权益仓位: <strong class="tabular" style="font-size: 1.15rem;">{:.1}% / {:.1}%</strong></div>
                        <div style="white-space: nowrap;">权益缺口: <strong class="tabular" style="font-size: 1.15rem;">{:.2}</strong></div>
                    </div>
                </div>

                <div class="dashboard-grid">
                    <div class="card">
                        <div class="card-header"><span class="card-title">📊 资产摘要</span></div>
                        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
                            <div>
                                <div style="font-size: 0.8rem; color: var(--text-muted); font-weight: 600;">本地权益市值</div>
                                <div style="font-weight: 800; font-size: 1.1rem;" class="tabular">{:.2}</div>
                            </div>
                            <div>
                                <div style="font-size: 0.8rem; color: var(--text-muted); font-weight: 600;">缓存状态</div>
                                <div style="font-weight: 800; font-size: 1.1rem;">{} 项</div>
                            </div>
                            <div style="grid-column: span 2;">
                                <div style="font-size: 0.8rem; color: var(--text-muted); font-weight: 600;">最近行情同步</div>
                                <div style="font-weight: 700; font-size: 0.95rem; color: var(--primary-color);">{}</div>
                            </div>
                        </div>
                    </div>
                    
                    {}

                    {}
                </div>

                <div style="display: grid; grid-template-columns: 1.2fr 0.8fr; gap: 24px;">
                    <div>
                        <h2>🔥 今日首选 (Top Picks)</h2>
                        {}
                        
                        <div class="card" style="margin-top: 24px; background: #F8F9FA;">
                            <div class="card-header"><span class="card-title">💡 决策分析 (Analysis)</span></div>
                            <div style="font-size: 0.9rem; color: var(--text-main); line-height: 1.7;">
                                {}
                            </div>
                        </div>
                    </div>
                    <div>
                        <h2>🧭 赛道分布 (Allocation)</h2>
                        <div class="table-container">
                            <table style="min-width: unset;">
                                <thead>
                                    <tr>
                                        <th>赛道</th>
                                        <th>当前</th>
                                        <th>目标</th>
                                        <th class="text-right">状态</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {}
                                </tbody>
                            </table>
                        </div>
                        <div style="text-align: right;">
                            <a href="/holdings" class="btn-ghost">查看完整持仓明细 &rarr;</a>
                        </div>
                    </div>
                </div>

                <div style="margin-top: 48px; border-top: 1px solid var(--border-color); padding-top: 24px; display: flex; justify-content: space-between; align-items: center; color: var(--text-muted); font-size: 0.8rem;">
                    <div>组合名称: <strong>{}</strong> · 后端: <strong>{}</strong></div>
                    <div>最后快照日期: {}</div>
                </div>
                "#,
                banners,
                summary.portfolio.total_asset_value,
                alipay_diff_html,
                total_suggested,
                badge_risk(&summary.risk_overlay.risk_label),
                summary.portfolio.available_cash,
                current_equity_pct, target_equity_pct,
                equity_gap,
                summary.portfolio.equity_value,
                summary.cache_status.market_cache_size,
                summary.cache_status.last_market_update.as_deref().unwrap_or("从未同步"),
                todo_card,
                quick_actions,
                top_buys_html,
                summary_analysis,
                allocation_rows,
                summary.portfolio_name,
                summary.backend,
                summary.alipay_snapshot_date.as_deref().unwrap_or("从未导入")
            );

            layout("操作台", content)
        }
        Err(e) => layout("操作台", format!("<div class='message-banner message-error'>数据加载失败: {}</div>", e)),
    }
}

async fn api_dashboard_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::DashboardSummary> {
    let ctx = RepositoryContext::default();
    match fetch_dashboard_summary(&state, &ctx).await {
        Ok(summary) => Json(summary),
        Err(e) => {
            // Return an error dashboard
            Json(models::DashboardSummary {
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
            })
        }
    }
}

async fn api_decision_explain_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::DecisionExplanation> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let date = Local::now().format("%Y-%m-%d").to_string();

        // Load caches for risk and regime
        let risk_cache = state.repo.load_risk_cache(&ctx).await?.unwrap_or_default();
        let regime_cache = state.repo.load_regime_cache(&ctx).await?;

        let mut regimes = std::collections::HashMap::new();
        for entry in &regime_cache.entries {
            for asset in &config.assets {
                let symbol_opt = asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone());
                if let Some(s) = symbol_opt {
                    if s == entry.symbol {
                        regimes.insert(asset.asset_id.clone(), entry.result.clone());
                    }
                }
            }
        }

        let explanation = engine::explanation::explain_decision(
            &config,
            &portfolio_state,
            ctx.portfolio_id.clone(),
            date,
            &risk_cache.overlay,
            &regimes,
        );
        Ok::<models::DecisionExplanation, anyhow::Error>(explanation)
    }
    .await;

    match result {
        Ok(e) => Json(e),
        Err(e) => {
            // Return an empty explanation with the error in warnings
            Json(models::DecisionExplanation {
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
                    factors: vec![e.to_string()],
                },
                asset_explanations: vec![],
                sector_explanations: vec![],
                warnings: vec![format!("Failed to generate explanation: {}", e)],
                global_caps: vec![],
            })
        }
    }
}

async fn api_kelly_plan_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::KellyPortfolioPreview> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date);

        // Load caches
        let risk_cache = state.repo.load_risk_cache(&ctx).await?;
        let regime_cache = state.repo.load_regime_cache(&ctx).await?.clone();

        let risk_overlay = if let Some(rc) = risk_cache {
            rc.overlay
        } else {
            models::GlobalRiskOverlay {
                risk_score: 0.0,
                risk_label: "未知".to_string(),
                factor_results: vec![],
                warnings: vec!["请运行 data refresh --risk".to_string()],
                explanation: "请运行 data refresh --risk".to_string(),
            }
        };

        let mut regimes = std::collections::HashMap::new();
        for entry in regime_cache.entries {
            for asset in &config.assets {
                let symbol_opt = asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone());
                if let Some(_s) = symbol_opt.filter(|s| *s == entry.symbol) {
                    regimes.insert(asset.asset_id.clone(), entry.result.clone());
                }
            }
        }

        let preview =
            engine::kelly::calculate_kelly_preview(&config, &decision, &risk_overlay, &regimes);

        Ok::<models::KellyPortfolioPreview, anyhow::Error>(preview)
    }
    .await;

    match result {
        Ok(p) => Json(p),
        Err(e) => Json(models::KellyPortfolioPreview {
            base_total_buy: 0.0,
            preview_total_buy: 0.0,
            total_multiplier: 0.0,
            global_risk_score: 0.0,
            global_risk_label: "错误".to_string(),
            results: vec![],
            warnings: vec![format!("加载 Kelly 数据失败: {}", e)],
        }),
    }
}

async fn api_dca_run_due_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::DcaExecutionResult> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let date = Local::now().format("%Y-%m-%d").to_string();
        let res = engine::dca::auto_execute_dca(state.repo.as_ref(), &ctx, &config, &date).await?;
        Ok::<models::DcaExecutionResult, anyhow::Error>(res)
    }
    .await;

    match result {
        Ok(res) => Json(res),
        Err(e) => Json(models::DcaExecutionResult {
            success: false,
            message: format!("DCA execution failed: {}", e),
            ..Default::default()
        }),
    }
}

async fn api_nav_refresh_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::import::ImportResult> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let count = engine::refresh::refresh_fund_navs(state.repo.as_ref(), &ctx, &config).await?;

        let mut status = state.refresh_status.write().await;
        status.last_fund_refresh = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());

        Ok::<usize, anyhow::Error>(count)
    }
    .await;

    match result {
        Ok(count) => Json(models::import::ImportResult {
            success: count > 0,
            inserted: count,
            message: if count > 0 {
                format!("成功刷新 {} 个基金净值", count)
            } else {
                "未发现需要刷新的活跃基金。请先启用资产并配置基金代码。".to_string()
            },
            ..Default::default()
        }),
        Err(e) => Json(models::import::ImportResult {
            success: false,
            message: format!("基金净值刷新失败: {}", e),
            ..Default::default()
        }),
    }
}

async fn api_market_refresh_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::import::ImportResult> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let count =
            engine::refresh::refresh_market_data(state.repo.as_ref(), &ctx, &config).await?;

        let mut status = state.refresh_status.write().await;
        status.last_market_refresh = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());

        Ok::<usize, anyhow::Error>(count)
    }
    .await;

    match result {
        Ok(count) => Json(models::import::ImportResult {
            success: count > 0,
            inserted: count,
            message: if count > 0 {
                format!("行情刷新成功：新增/更新 {} 条。", count)
            } else {
                "没有可刷新的活跃标的，请先配置持仓、锚定指数或启用证券标的。".to_string()
            },
            ..Default::default()
        }),
        Err(e) => Json(models::import::ImportResult {
            success: false,
            message: format!("行情刷新失败: {}", e),
            ..Default::default()
        }),
    }
}

async fn api_market_refresh_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<BackgroundRefreshStatus> {
    let status = state.refresh_status.read().await;
    Json(status.clone())
}

async fn api_dca_plans_handler(State(state): State<Arc<AppState>>) -> Json<Vec<models::DcaPlan>> {
    let ctx = RepositoryContext::default();
    let plans = state.repo.load_plans(&ctx).await.unwrap_or_default();
    Json(plans)
}

#[derive(Deserialize)]
struct DcaPlanForm {
    asset_id: String,
    amount: f64,
    frequency: String,
    day: Option<u32>,
    note: Option<String>,
}

async fn api_dca_add_plan_handler(
    State(state): State<Arc<AppState>>,
    Json(form): Json<DcaPlanForm>,
) -> Json<models::DcaExecutionResult> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let asset = config.assets.iter().find(|a| a.asset_id == form.asset_id);

        if let Some(a) = asset {
            let mut plans = state.repo.load_plans(&ctx).await?;
            let freq = match form.frequency.as_str() {
                "daily" => models::DcaFrequency::Daily,
                "weekly" => models::DcaFrequency::Weekly,
                "monthly" => models::DcaFrequency::Monthly,
                _ => return Err(anyhow::anyhow!("无效的频率")),
            };

            let plan_id = format!("plan_{}", chrono::Local::now().timestamp_millis());
            let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let new_plan = models::DcaPlan {
                plan_id: plan_id.clone(),
                asset_id: form.asset_id.clone(),
                fund_code: a.fund_code.clone(),
                fund_name: a.fund_name.clone(),
                amount: form.amount,
                currency: "CNY".to_string(),
                frequency: freq,
                weekday: if form.frequency == "weekly" {
                    form.day
                } else {
                    None
                },
                month_day: if form.frequency == "monthly" {
                    form.day
                } else {
                    None
                },
                start_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                end_date: None,
                enabled: true,
                priority: 0,
                note: form.note.or(Some("Via Web API".to_string())),
                created_at: now_str.clone(),
                updated_at: now_str,
            };

            plans.push(new_plan);
            state.repo.save_plans(&ctx, &plans).await?;
            Ok::<String, anyhow::Error>(plan_id)
        } else {
            Err(anyhow::anyhow!("资产未找到"))
        }
    }
    .await;

    match result {
        Ok(id) => Json(models::DcaExecutionResult {
            success: true,
            message: format!("Plan created: {}", id),
            ..Default::default()
        }),
        Err(e) => Json(models::DcaExecutionResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

#[derive(Deserialize)]
struct DcaUpdateForm {
    amount: Option<f64>,
    frequency: Option<String>,
    day: Option<u32>,
    note: Option<String>,
    enabled: Option<bool>,
}

async fn api_dca_update_plan_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(plan_id): axum::extract::Path<String>,
    Json(form): Json<DcaUpdateForm>,
) -> Json<models::DcaExecutionResult> {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        if let Some(p) = plans.iter_mut().find(|p| p.plan_id == plan_id) {
            if let Some(a) = form.amount {
                p.amount = a;
            }
            if let Some(f) = form.frequency {
                p.frequency = match f.as_str() {
                    "daily" => models::DcaFrequency::Daily,
                    "weekly" => models::DcaFrequency::Weekly,
                    "monthly" => models::DcaFrequency::Monthly,
                    _ => p.frequency.clone(),
                };
                if f == "weekly" {
                    p.weekday = form.day;
                    p.month_day = None;
                } else if f == "monthly" {
                    p.month_day = form.day;
                    p.weekday = None;
                }
            }
            if let Some(n) = form.note {
                p.note = Some(n);
            }
            if let Some(e) = form.enabled {
                p.enabled = e;
            }
            p.updated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            state.repo.save_plans(&ctx, &plans).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Json(models::DcaExecutionResult {
            success: true,
            message: "Plan updated".to_string(),
            ..Default::default()
        }),
        Err(e) => Json(models::DcaExecutionResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

async fn api_dca_remove_plan_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(plan_id): axum::extract::Path<String>,
) -> Json<models::DcaExecutionResult> {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        let len_before = plans.len();
        plans.retain(|p| p.plan_id != plan_id);
        if plans.len() < len_before {
            state.repo.save_plans(&ctx, &plans).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Json(models::DcaExecutionResult {
            success: true,
            message: "Plan removed".to_string(),
            ..Default::default()
        }),
        Err(e) => Json(models::DcaExecutionResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

async fn api_dca_executions_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<models::DcaSettlement>> {
    let ctx = RepositoryContext::default();
    let mut settlements = state.repo.load_settlements(&ctx).await.unwrap_or_default();
    // Sort by deduction_date DESC
    settlements.sort_by(|a, b| b.deduction_date.cmp(&a.deduction_date));
    Json(settlements)
}

async fn holdings_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
        let snapshots = state.repo.load_alipay_snapshots(&ctx).await.unwrap_or_default();
        Ok::<
            (
                models::ConfigRoot,
                models::PortfolioState,
                models::PortfolioSummary,
                Vec<models::AlipaySnapshot>,
            ),
            anyhow::Error,
        >((config, portfolio_state, summary, snapshots))
    }
    .await;

    match result {
        Ok((config, portfolio_state, summary, snapshots)) => {
            let mut latest_snaps = std::collections::HashMap::new();
            for s in &snapshots {
                let entry = latest_snaps.entry(s.asset_id.clone()).or_insert(s.clone());
                if s.snapshot_date >= entry.snapshot_date { *entry = s.clone(); }
            }

            let mut rows = String::new();
            for holding in &portfolio_state.asset_holdings {
                let asset_config = config.assets.iter().find(|a| a.asset_id == holding.asset_id);
                if !asset_config.map(|a| a.enabled).unwrap_or(false) { continue; }

                let name = asset_config.map(|a| a.fund_name.as_str()).unwrap_or("Unknown");
                let sector = asset_config.map(|a| a.sector.as_str()).unwrap_or("未分类");
                let is_unclassified = sector == "未分类" || sector.is_empty();

                let market_value = holding.last_market_value;
                let cost = holding.cost_basis;
                let pnl = market_value - cost;
                let pnl_pct = if cost.abs() > 0.01 { pnl / cost * 100.0 } else { 0.0 };

                let alipay_diff_html = if let Some(snap) = latest_snaps.get(&holding.asset_id) {
                    let diff = market_value - snap.market_value;
                    let diff_class = if diff.abs() < 1.0 { "text-muted" } else if diff > 0.0 { "text-up" } else { "text-down" };
                    format!("<div style='font-size: 0.8rem;' class='{} tabular'>{:+.2}</div>", diff_class, diff)
                } else {
                    "<span class='text-muted'>-</span>".to_string()
                };

                rows.push_str(&format!(
                    r#"<tr>
                        <td>
                            <div style="font-weight: 700;">{}</div>
                            <div style="font-size: 0.75rem; color: var(--text-muted);"><code>{}</code></div>
                        </td>
                        <td><span class="badge {}">{}</span></td>
                        <td class="text-right tabular" style="font-weight: 700;">{:.2}</td>
                        <td class="text-right tabular">{:.2}</td>
                        <td class="text-right tabular {}">
                            <div>{:+.2}</div>
                            <div style="font-size: 0.75rem;">{:+.2}%</div>
                        </td>
                        <td class="text-right">{}</td>
                        <td class="text-right">{}</td>
                    </tr>"#,
                    name, holding.fund_code,
                    if is_unclassified { "badge-orange" } else { "badge-outline" },
                    sector,
                    market_value,
                    holding.units,
                    color_class(pnl), pnl, pnl_pct,
                    alipay_diff_html,
                    badge_status(holding.latest_nav_status.as_deref().unwrap_or("正常"))
                ));
            }

            let alipay_total: f64 = latest_snaps.values().map(|s| s.market_value).sum();
            let diff = summary.equity_value - alipay_total;
            let diff_class = if diff.abs() < 10.0 { "text-muted" } else if diff > 0.0 { "text-up" } else { "text-down" };

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 32px;">
                    <div>
                        <h1 style="margin-bottom: 4px;">权益资产持仓 (Equity Holdings)</h1>
                        <p style="color: var(--text-muted); font-size: 0.95rem; margin: 0;">管理您的证券、基金持仓与外部账本偏差</p>
                    </div>
                    <div class="action-group" style="margin-top: 0;">
                        <a href="/admin/assets" class="btn btn-outline btn-sm">⚙️ 资产设置</a>
                        <a href="/reconcile" class="btn btn-outline btn-sm">⚖️ 对账中心</a>
                    </div>
                </div>

                <div class="dashboard-grid">
                    <div class="card">
                        <div class="card-header"><span class="card-title">系统账面总额</span></div>
                        <div class="card-value tabular">{:.2}</div>
                        <div class="card-sub">组合状态计算值</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">支付宝快照总额</span></div>
                        <div class="card-value tabular">{:.2}</div>
                        <div class="card-sub">外部导入对比值</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">持仓偏差</span></div>
                        <div class="card-value tabular {}">{:+.2}</div>
                        <div class="card-sub">系统 vs 支付宝</div>
                    </div>
                </div>

                <div class="table-container">
                    <div class="table-wrap">
                        <table>
                            <thead>
                                <tr>
                                    <th>资产名称 / 代码</th>
                                    <th>板块赛道</th>
                                    <th class="text-right">当前市值 (CNY)</th>
                                    <th class="text-right">持有份额</th>
                                    <th class="text-right">累计盈亏</th>
                                    <th class="text-right">支付宝差异</th>
                                    <th class="text-right">净值状态</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>
                </div>

                <div class="card" style="background: #F8F9FA; border-style: dashed;">
                    <h3 style="margin-top: 0;">💡 持仓说明</h3>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.6; margin-bottom: 0;">
                        1. <strong>系统账面:</strong> 基于您录入的历史交易流水推算出的当前持仓份额 × 最新净值。<br>
                        2. <strong>支付宝差异:</strong> 自动对比最新导入的支付宝快照。若差异较大，建议前往“对账中心”进行校准。<br>
                        3. <strong>板块赛道:</strong> 若显示为“未分类”，将无法获得准确的 Kelly 建议。
                    </p>
                </div>
                "#,
                summary.equity_value,
                alipay_total,
                diff_class, diff,
                rows
            );

            layout("权益持仓", content)
        }
        Err(e) => layout("权益持仓", format!("<div class='message-banner message-error'>数据加载失败: {}</div>", e)),
    }
}

async fn sectors_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
        Ok::<models::PortfolioSummary, anyhow::Error>(summary)
    }
    .await;

    match result {
        Ok(summary) => {
            let mut rows = String::new();
            for s in summary.sector_summaries {
                let target_pct = s.target_weight * 100.0;
                let current_pct = s.current_weight * 100.0;

                let (status_text, badge_class) = match s.status.as_str() {
                    "underweight" => ("低配", "badge-green"),
                    "neutral" => ("均衡", "badge-blue"),
                    "overweight" => ("超配", "badge-red"),
                    _ => (s.status.as_str(), "badge-gray"),
                };

                let gap_class = if s.gap_value > 1.0 {
                    "text-down"
                } else if s.gap_value < -1.0 {
                    "text-up"
                } else {
                    ""
                };

                rows.push_str(&format!(
                    "<tr>
                        <td><div style='font-weight: 700; font-size: 1rem;'>{}</div></td>
                        <td style='font-weight: 600;'>{:.2}</td>
                        <td class='{}' style='font-weight: 700;'>{:.2}</td>
                        <td>
                            <div style='display: flex; justify-content: space-between; margin-bottom: 4px;'>
                                <span style='font-size: 0.75rem; color: var(--text-muted);'>当前: {:.1}%</span>
                                <span style='font-size: 0.75rem; color: var(--text-muted);'>目标: {:.1}%</span>
                            </div>
                            <div style='width: 100%; height: 6px; background: #F2F3F5; border-radius: 3px; position: relative;'>
                                <div style='position: absolute; left: 0; top: 0; height: 100%; width: {}%; background: var(--primary-color); border-radius: 3px;'></div>
                                <div style='position: absolute; left: {}%; top: -3px; height: 12px; width: 2px; background: var(--text-muted);'></div>
                            </div>
                        </td>
                        <td><span class='badge {}'>{}</span></td>
                    </tr>",
                    s.sector_name,
                    s.current_value,
                    gap_class,
                    s.gap_value,
                    current_pct,
                    target_pct,
                    current_pct.clamp(0.0, 100.0),
                    target_pct.clamp(0.0, 100.0),
                    badge_class,
                    status_text
                ));
            }

            let content = format!(
                r#"
                <h1>赛道权重分布 (Sector Weights)</h1>
                
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>赛道名称</th>
                                <th>当前市值 (CNY)</th>
                                <th>配置缺口 (Gap)</th>
                                <th style='width: 30%;'>权重进度 (当前 vs 目标)</th>
                                <th>状态</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="card" style="background-color: #F7F8FA; border: 1px dashed var(--border-color);">
                    <p style="font-size: 0.85rem; color: var(--text-muted); margin: 0;">
                        💡 提示: <strong>配置缺口</strong> 为负表示当前仓位不足（低配），建议买入；为正表示仓位超出目标（超配）。
                    </p>
                </div>
                "#,
                rows
            );

            layout("赛道分布", content)
        }
        Err(e) => layout(
            "赛道分布",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
    }
}

async fn decisions_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let result = engine::generate_buy_suggestions(&config, &portfolio_state, date);
        Ok::<(models::ConfigRoot, engine::decision::DecisionResult), anyhow::Error>((
            config, result,
        ))
    }
    .await;

    match result {
        Ok((config, result)) => {
            let mut warnings = String::new();
            for warning in &result.warnings {
                warnings.push_str(&format!(
                    "<div class='warning-box'><strong>!</strong> {}</div>",
                    warning
                ));
            }

            let mut rows = String::new();
            if result.suggested_total_buy > 0.0 {
                for sector in result.sector_suggestions {
                    let sector_pct = safe_div(sector.suggested_buy, result.suggested_total_buy);

                    for asset in sector.asset_suggestions {
                        let asset_pct = safe_div(asset.suggested_buy, result.suggested_total_buy);
                        let buy_highlight = if asset.suggested_buy > 0.0 {
                            "style='background-color: #fff9f9;'"
                        } else {
                            ""
                        };
                        let amount_class = if asset.suggested_buy > 0.0 {
                            "text-up"
                        } else {
                            ""
                        };

                        rows.push_str(&format!(
                            "<tr {}>
                                <td><strong>{}</strong></td>
                                <td><strong>{}</strong></td>
                                <td><code>{}</code></td>
                                <td class='text-down'>{:.2}</td>
                                <td>{:.2} <small>({})</small></td>
                                <td class='{}'><strong>{:.2}</strong> <br><small>({})</small></td>
                                <td><small>{}</small></td>
                            </tr>",
                            buy_highlight,
                            asset.sector_name,
                            asset.fund_name,
                            asset.fund_code,
                            sector.gap_value,
                            sector.suggested_buy,
                            sector_pct,
                            amount_class,
                            asset.suggested_buy,
                            asset_pct,
                            asset.reason
                        ));
                    }
                }
            } else {
                rows.push_str("<tr><td colspan='7' style='text-align: center; padding: 2rem; color: var(--text-muted);'>今日无可买入建议</td></tr>");
            }

            let content = format!(
                r#"
                <h1>今日买入建议</h1>
                {}
                <div class="dashboard-grid">
                    <div class="card">
                        <h3>可用现金</h3>
                        <div class="value">{:.2} {}</div>
                    </div>
                    <div class="card">
                        <h3>建议总买入</h3>
                        <div class="value text-up">{:.2} {}</div>
                        <div class="sub-value">占单日上限: {}</div>
                    </div>
                    <div class="card">
                        <h3>单日买入上限</h3>
                        <div class="value">{:.2} {}</div>
                    </div>
                </div>
                
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>赛道</th>
                                <th>资产</th>
                                <th>代码</th>
                                <th>缺口金额</th>
                                <th>赛道分配 (占比)</th>
                                <th>建议买入 (占比)</th>
                                <th>建议原因</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>
                "#,
                warnings,
                result.available_cash,
                config.portfolio.base_currency,
                result.suggested_total_buy,
                config.portfolio.base_currency,
                safe_div(result.suggested_total_buy, result.max_daily_buy_total),
                result.max_daily_buy_total,
                config.portfolio.base_currency,
                rows
            );

            layout("今日买入建议", content)
        }
        Err(e) => layout(
            "今日买入建议",
            format!("<div class='warning-box'>行情数据获取失败: {}</div>", e),
        ),
    }
}

async fn transactions_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = state.repo.load_transactions(&ctx).await;

    match result {
        Ok(mut transactions) => {
            transactions.sort_by(|a, b| b.date.cmp(&a.date));
            let mut rows = String::new();
            for tx in transactions {
                let type_cn = match tx.transaction_type.as_str() {
                    "buy" => "买入",
                    "sell" => "卖出",
                    "cash_in" => "注入",
                    "cash_out" => "提取",
                    "expense" => "支出",
                    "dividend" => "分红",
                    "manual_cash_adjustment" | "cash_set" => "调节",
                    other => other,
                };

                let type_class = match tx.transaction_type.as_str() {
                    "buy" | "cash_in" | "dividend" => "text-up",
                    "sell" | "cash_out" | "expense" => "text-down",
                    _ => "",
                };

                rows.push_str(&format!(
                    r#"<tr>
                        <td><div style="font-size: 0.85rem; font-weight: 600;">{}</div></td>
                        <td><span class="badge badge-outline {}">{}</span></td>
                        <td><code>{}</code></td>
                        <td class="text-right tabular" style="font-weight: 700;">{:.2}</td>
                        <td class="text-right tabular">{}</td>
                        <td class="text-right tabular">{}</td>
                        <td class="text-right tabular">{:.2}</td>
                        <td><div style="font-size: 0.8rem; color: var(--text-muted); max-width: 200px;" title="{}">{}</div></td>
                        <td class="text-right"><div style="font-size: 0.7rem; color: var(--text-muted);"><code>{}</code></div></td>
                    </tr>"#,
                    tx.date,
                    type_class, type_cn,
                    tx.asset_id.as_deref().unwrap_or("-"),
                    tx.amount,
                    tx.units.map(|u| format!("{:.4}", u)).unwrap_or_else(|| "-".to_string()),
                    tx.price.map(|p| format!("{:.4}", p)).unwrap_or_else(|| "-".to_string()),
                    tx.fee,
                    tx.note, if tx.note.chars().count() > 20 { format!("{}...", tx.note.chars().take(17).collect::<String>()) } else { tx.note.clone() },
                    &tx.id[0..8]
                ));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 32px;">
                    <div>
                        <h1 style="margin-bottom: 4px;">交易明细 (Transactions)</h1>
                        <p style="color: var(--text-muted); font-size: 0.95rem; margin: 0;">所有买入、卖出及资金划转的历史记录</p>
                    </div>
                </div>

                <div class="table-container">
                    <div class="table-wrap">
                        <table>
                            <thead>
                                <tr>
                                    <th>日期</th>
                                    <th>类型</th>
                                    <th>资产 ID</th>
                                    <th class="text-right">金额 (CNY)</th>
                                    <th class="text-right">份额</th>
                                    <th class="text-right">成交价</th>
                                    <th class="text-right">费用</th>
                                    <th>备注</th>
                                    <th class="text-right">交易 ID</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>
                </div>
                "#,
                rows
            );

            layout("交易记录", content)
        }
        Err(e) => layout("交易记录", format!("<div class='message-banner message-error'>数据加载失败: {}</div>", e)),
    }
}

async fn assets_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = state.repo.load_config(&ctx).await;

    match result {
        Ok(config) => {
            let mut rows = String::new();
            for asset in config.assets {
                let status_badge = if asset.enabled {
                    "<span class='badge badge-blue'>正常</span>"
                } else {
                    "<span class='badge badge-gray'>禁用</span>"
                };

                rows.push_str(&format!(
                    r#"<tr>
                        <td>
                            <div style="font-weight: 700;">{}</div>
                            <div style="font-size: 0.75rem; color: var(--text-muted);">ID: <code>{}</code> · <code>{}</code></div>
                        </td>
                        <td><span class="badge badge-outline">{}</span></td>
                        <td><div style="font-size: 0.85rem; font-weight: 600;">{}</div><div style="font-size: 0.7rem; color: var(--text-muted);"><code>{}</code></div></td>
                        <td>{}</td>
                        <td>{}</td>
                        <td class="text-right">{}</td>
                    </tr>"#,
                    asset.fund_name, asset.asset_id, asset.fund_code,
                    asset.sector,
                    asset.reference_index_name.as_deref().unwrap_or("-"),
                    asset.reference_index_symbol.as_deref().unwrap_or("-"),
                    asset.currency,
                    asset.valuation_method,
                    status_badge
                ));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 32px;">
                    <div>
                        <h1 style="margin-bottom: 4px;">资产档案 (Assets)</h1>
                        <p style="color: var(--text-muted); font-size: 0.95rem; margin: 0;">组合支持的所有基金及标的元数据详情</p>
                    </div>
                    <div class="action-group" style="margin-top: 0;">
                        <a href="/admin/assets" class="btn btn-outline btn-sm">⚙️ 管理资产</a>
                    </div>
                </div>

                <div class="table-container">
                    <div class="table-wrap">
                        <table>
                            <thead>
                                <tr>
                                    <th>资产名称 / 识别码</th>
                                    <th>板块赛道</th>
                                    <th>参考基准</th>
                                    <th>币种</th>
                                    <th>估值方法</th>
                                    <th class="text-right">状态</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>
                </div>
                "#,
                rows
            );

            layout("资产列表", content)
        }
        Err(e) => layout("资产列表", format!("<div class='message-banner message-error'>数据加载失败: {}</div>", e)),
    }
}

async fn proxy_valuation_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = state.repo.load_proxy_cache(&ctx).await;

    match result {
        Ok(cache) => {
            let mut rows = String::new();
            if cache.results.is_empty() {
                rows.push_str("<tr><td colspan='13' style='text-align: center;'>暂无缓存数据。请运行 <code>data refresh --proxy</code>。</td></tr>");
            }

            for res in cache.results {
                let index_return_pct = fmt_pct(res.index_return);
                let index_class = color_class(res.index_return);

                let fx_return_pct = if res.use_fx_adjustment
                    && (res.status.contains("汇率")
                        || res.warning.as_ref().is_some_and(|w| w.contains("汇率")))
                {
                    if res.fx_return.abs() < 0.000001 {
                        "N/A".to_string()
                    } else {
                        fmt_pct(res.fx_return)
                    }
                } else {
                    fmt_pct(res.fx_return)
                };
                let fx_class = if fx_return_pct == "N/A" {
                    ""
                } else {
                    color_class(res.fx_return)
                };

                let combined_return_pct = fmt_pct(res.combined_proxy_return);
                let combined_class = color_class(res.combined_proxy_return);

                let fx_adj_str = if res.use_fx_adjustment { "是" } else { "否" };

                let diff = res.estimated_market_value - res.official_market_value;
                let deviation_pct_val = if res.official_market_value.abs() > 0.001 {
                    diff / res.official_market_value
                } else {
                    0.0
                };
                let deviation_pct_str = fmt_pct(deviation_pct_val);
                let dev_class = color_class(diff);

                let status_badge = badge_status(&res.status);
                let warning_text = if let Some(w) = &res.warning {
                    format!("<br><small style='color: var(--text-muted)'>{}</small>", w)
                } else {
                    "".to_string()
                };

                rows.push_str(&format!(
                    "<tr>
                        <td><code>{}</code></td>
                        <td>{}</td>
                        <td>{:.4} <br><small>{}</small></td>
                        <td>{:.2}</td>
                        <td><strong>{}</strong></td>
                        <td class='{}'>{}</td>
                        <td class='{}'>{}</td>
                        <td class='{}'>{}</td>
                        <td>{}</td>
                        <td>{:.4}</td>
                        <td>{:.2}</td>
                        <td class='{}'><strong>{}</strong></td>
                        <td>{}{}</td>
                    </tr>",
                    res.asset_id,
                    res.fund_name,
                    res.official_nav,
                    res.official_nav_date,
                    res.official_market_value,
                    res.reference_index_symbol,
                    index_class,
                    index_return_pct,
                    fx_class,
                    fx_return_pct,
                    combined_class,
                    combined_return_pct,
                    fx_adj_str,
                    res.estimated_nav,
                    res.estimated_market_value,
                    dev_class,
                    deviation_pct_str,
                    status_badge,
                    warning_text
                ));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: baseline;">
                    <h1>估值预览 (缓存)</h1>
                    <div style="font-size: 0.85rem; color: var(--text-muted);">
                        缓存更新时间: {}
                    </div>
                </div>
                <div class="warning-box">
                    <strong>提示:</strong> 估算净值仅用于当日实时参考，不参与当前建议买入金额的计算。请定期运行 <code>data refresh --proxy</code>。
                </div>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>资产ID</th>
                                <th>基金名称</th>
                                <th>官方净值 (日期)</th>
                                <th>官方市值</th>
                                <th>参考指数</th>
                                <th>指数涨跌</th>
                                <th>汇率涨跌</th>
                                <th>综合涨跌</th>
                                <th>汇率调</th>
                                <th>估算净值</th>
                                <th>估算市值</th>
                                <th>偏离比例</th>
                                <th>状态</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>
                "#,
                cache.fetched_at, rows
            );

            layout("估算净值", content)
        }
        Err(e) => layout(
            "估算净值",
            format!("<div class='warning-box'>加载估值数据失败: {}</div>", e),
        ),
    }
}

async fn regime_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = state.repo.load_regime_cache(&ctx).await;

    match result {
        Ok(cache) => {
            let mut rows = String::new();
            if cache.entries.is_empty() {
                rows.push_str("<tr><td colspan='11' style='text-align: center; padding: 2rem;'>暂无缓存数据，请先在 CLI 运行 <code>cargo run -- data refresh --market</code></td></tr>");
            }

            for entry in cache.entries {
                let res = entry.result;
                let mut window_cols = String::new();
                for w_days in &[20, 60, 120, 250] {
                    let z_val = res
                        .windows
                        .iter()
                        .find(|w| w.window_days == *w_days)
                        .and_then(|w| w.z_score);

                    let z_str = z_val
                        .map(|z| format!("{:.2}", z))
                        .unwrap_or_else(|| "-".to_string());
                    let z_class = z_val.map(color_class).unwrap_or("");
                    window_cols.push_str(&format!("<td class='{}'>{}</td>", z_class, z_str));
                }

                let latest_window = res
                    .windows
                    .iter()
                    .find(|w| w.window_days == 250)
                    .or(res.windows.first());

                let drawdown_pct = latest_window
                    .map(|w| format!("{:.2}%", w.drawdown * 100.0))
                    .unwrap_or_else(|| "-".to_string());
                let vol_pct = latest_window
                    .map(|w| format!("{:.2}%", w.annualized_volatility * 100.0))
                    .unwrap_or_else(|| "-".to_string());

                let pointer_pos = ((res.pendulum_score + 100.0) / 2.0).clamp(0.0, 100.0);

                rows.push_str(&format!(
                    "<tr>
                        <td><strong>{}</strong></td>
                        <td>{:.2}</td>
                        {}
                        <td class='text-down'>{}</td>
                        <td>{}</td>
                        <td>
                            <div class='score-meter-wrap'>
                                <div class='score-meter'>
                                    <div class='score-pointer' style='left: {:.1}%;'></div>
                                </div>
                                <div style='display:flex; justify-content:space-between; font-size: 0.7rem; color: var(--text-muted); margin-top:4px;'>
                                    <span>极冷</span>
                                    <span>中性</span>
                                    <span>过热</span>
                                </div>
                            </div>
                            <strong>{:.2}</strong>
                        </td>
                        <td>{}</td>
                        <td><small>{}</small></td>
                    </tr>",
                    res.symbol, res.latest_price, window_cols, drawdown_pct, vol_pct, pointer_pos, res.pendulum_score, badge_regime(&res.regime_label), res.warning.as_deref().unwrap_or("-")
                ));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: baseline;">
                    <h1>市场冷热分析 (缓存)</h1>
                    <div style="font-size: 0.85rem; color: var(--text-muted);">
                        缓存更新时间: {}
                    </div>
                </div>
                <p>基于均值偏离 (Z-score) 和历史波动计算的钟摆分数。红色代表过热，绿色代表极冷。</p>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>代码</th>
                                <th>最新价</th>
                                <th>20日 Z</th>
                                <th>60日 Z</th>
                                <th>120日 Z</th>
                                <th>250日 Z</th>
                                <th>最大回撤</th>
                                <th>年化波动</th>
                                <th>钟摆分数 (仪表盘)</th>
                                <th>市场状态</th>
                                <th>提示</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>
                <div class="warning-box">
                    <strong>风险提示:</strong> 金融市场收益并不严格服从正态分布，Z-score 仅用于衡量相对偏离程度，不应被理解为确定性预测。请定期运行 <code>data refresh --market</code>。
                </div>
                "#,
                cache.fetched_at, rows
            );

            layout("市场冷热", content)
        }
        Err(e) => layout(
            "市场冷热",
            format!("<div class='warning-box'>加载冷热数据失败: {}</div>", e),
        ),
    }
}

async fn risk_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = state.repo.load_risk_cache(&ctx).await;

    match result {
        Ok(Some(cache)) => {
            let overlay = &cache.overlay;
            let mut factor_rows = String::new();
            for f in &overlay.factor_results {
                let z_str = f
                    .z_score
                    .map(|z| format!("{:.2}", z))
                    .unwrap_or_else(|| "-".to_string());
                let z_class = f.z_score.map(color_class).unwrap_or("");

                let short_class = color_class(f.short_return);
                let medium_class = color_class(f.medium_return);

                factor_rows.push_str(&format!(
                    "<tr>
                        <td><strong>{}</strong></td>
                        <td><code>{}</code></td>
                        <td>{:.2}</td>
                        <td>{}</td>
                        <td class='{}'>{}</td>
                        <td class='{}'>{}</td>
                        <td class='{}'>{}</td>
                        <td class='text-down'>{:.2}%</td>
                        <td>{}</td>
                    </tr>",
                    f.name,
                    f.symbol,
                    f.latest_value,
                    f.latest_date,
                    short_class,
                    fmt_pct(f.short_return),
                    medium_class,
                    fmt_pct(f.medium_return),
                    z_class,
                    z_str,
                    f.drawdown * 100.0,
                    badge_status(&f.status)
                ));
            }

            let mut warning_html = String::new();
            if !overlay.warnings.is_empty() {
                warning_html.push_str("<div class='warning-box'><h3>风险警告</h3><ul>");
                for w in &overlay.warnings {
                    warning_html.push_str(&format!("<li>{}</li>", w));
                }
                warning_html.push_str("</ul></div>");
            }

            let mut explain_list = String::new();
            for line in overlay.explanation.split('；') {
                explain_list.push_str(&format!("<li>{}</li>", line.trim_end_matches('。')));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: baseline;">
                    <h1>全局风险分析 (缓存)</h1>
                    <div style="font-size: 0.85rem; color: var(--text-muted);">
                        缓存更新时间: {}
                    </div>
                </div>

                <div class="dashboard-grid">
                    <div class="card">
                        <h3>综合风险等级</h3>
                        <div class="value">{}</div>
                        <div class="sub-value">风险分数: {:.1} / 100</div>
                    </div>
                </div>

                {}

                <h2>主要风险来源</h2>
                <div class="card" style="margin-bottom: 2rem;">
                    <ul>{}</ul>
                </div>

                <h2>风险因子明细</h2>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>因子</th>
                                <th>代码</th>
                                <th>最新值</th>
                                <th>日期</th>
                                <th>20日变化</th>
                                <th>60日变化</th>
                                <th>Z-score</th>
                                <th>250日回撤</th>
                                <th>状态</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>
                <div class="warning-box">
                    <strong>风险提示:</strong> 该评分来自缓存，请定期运行 <code>data refresh --risk</code>。金融数据存在滞后性，请以实际行情为准。
                </div>
                "#,
                cache.fetched_at,
                badge_risk(&overlay.risk_label),
                overlay.risk_score,
                warning_html,
                explain_list,
                factor_rows
            );

            layout("全局风险", content)
        }
        Ok(None) => layout(
            "全局风险",
            "<div class='warning-box'>暂无风险缓存数据，请先在 CLI 运行 <code>cargo run -- data refresh --risk</code></div>".to_string(),
        ),
        Err(e) => layout(
            "全局风险",
            format!("<div class='warning-box'>风险数据加载失败: {}</div>", e),
        ),
    }
}

async fn kelly_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date);

        // Load caches
        let risk_cache = state.repo.load_risk_cache(&ctx).await?;
        let regime_cache = state.repo.load_regime_cache(&ctx).await?.clone();

        let risk_overlay = if let Some(rc) = risk_cache {
            rc.overlay
        } else {
            models::GlobalRiskOverlay {
                risk_score: 0.0,
                risk_label: "未知".to_string(),
                factor_results: vec![],
                warnings: vec!["请运行 data refresh --risk".to_string()],
                explanation: "请运行 data refresh --risk".to_string(),
            }
        };

        let mut regimes = std::collections::HashMap::new();
        for entry in regime_cache.entries {
            for asset in &config.assets {
                let symbol_opt = asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone());
                if let Some(_s) = symbol_opt.filter(|s| *s == entry.symbol) {
                    regimes.insert(asset.asset_id.clone(), entry.result.clone());
                }
            }
        }

        let preview =
            engine::kelly::calculate_kelly_preview(&config, &decision, &risk_overlay, &regimes);

        Ok::<
            (
                models::KellyPortfolioPreview,
                engine::decision::DecisionResult,
            ),
            anyhow::Error,
        >((preview, decision))
    }
    .await;

    match result {
        Ok((preview, decision)) => {
            let mut result_rows = String::new();
            let mut skipped_rows = String::new();

            for res in &preview.results {
                if res.capped_preview_buy_amount > 0.0 {
                    let mult_class = if res.kelly_multiplier > 1.0 {
                        "text-up"
                    } else if res.kelly_multiplier < 1.0 {
                        "text-down"
                    } else {
                        ""
                    };

                    result_rows.push_str(&format!(
                        "<tr>
                            <td>
                                <div style='font-weight: 700;'>{}</div>
                                <div style='font-size: 0.75rem; color: var(--text-muted);'>{}</div>
                            </td>
                            <td>
                                <code>{}</code><br>
                                <small>{}</small>
                            </td>
                            <td>
                                <div style='font-weight: 600;'>{:.2}%</div>
                                <div style='font-size: 0.7rem; color: var(--text-muted);'>{}</div>
                            </td>
                            <td>
                                <div style='font-weight: 700;'>{:.1}</div>
                                {}
                            </td>
                            <td>{}</td>
                            <td class='{}' style='font-weight: 800;'>{:.2}x</td>
                            <td>
                                <div style='font-size: 0.8rem; color: var(--text-muted);'>Base: {:.2}</div>
                                <div class='text-up' style='font-weight: 800; font-size: 1.1rem;'>{:.2}</div>
                            </td>
                        </tr>",
                        res.fund_name,
                        res.sector,
                        res.asset_id,
                        res.benchmark_symbol.as_deref().unwrap_or("-"),
                        res.volatility * 100.0,
                        if res.volatility > 0.0 { "年化波动" } else { "无历史数据" },
                        res.pendulum_score,
                        badge_regime(&res.market_regime_label),
                        badge_risk(&res.global_risk_label),
                        mult_class,
                        res.kelly_multiplier,
                        res.base_suggested_buy,
                        res.capped_preview_buy_amount
                    ));
                } else {
                    skipped_rows.push_str(&format!(
                        "<tr>
                            <td><strong>{}</strong></td>
                            <td><code>{}</code></td>
                            <td>{}</td>
                            <td><span class='badge badge-gray'>{}</span></td>
                            <td><small>{}</small></td>
                        </tr>",
                        res.fund_name, res.asset_id, res.sector, res.status, res.explanation
                    ));
                }
            }

            let mut warnings_html = String::new();
            if !preview.warnings.is_empty() {
                warnings_html.push_str(
                    "<div class='message-banner message-error'><strong>注意:</strong><ul>",
                );
                for w in &preview.warnings {
                    warnings_html.push_str(&format!("<li>{}</li>", w));
                }
                warnings_html.push_str("</ul></div>");
            }

            let skipped_section = if !skipped_rows.is_empty() {
                format!(
                    r#"<h2 style="margin-top: 40px;">跳过的资产 (Skipped)</h2>
                    <div class="table-container">
                        <table style="min-width: unset;">
                            <thead>
                                <tr>
                                    <th>资产名称</th>
                                    <th>代码</th>
                                    <th>赛道</th>
                                    <th>原因</th>
                                    <th>详细解释</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>"#,
                    skipped_rows
                )
            } else {
                "".to_string()
            };

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 24px;">
                    <h1>Kelly 每日执行计划 (预览)</h1>
                    <div style="font-size: 0.85rem; color: var(--text-muted);">日期: {}</div>
                </div>

                <div class="dashboard-grid">
                    <div class="card">
                        <div class="card-header"><span class="card-title">建议总买入</span></div>
                        <div class="card-value text-up">{:.2}</div>
                        <div class="card-sub">基础总额: {:.2} (倍率: {:.2}x)</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">可用现金</span></div>
                        <div class="card-value">{:.2}</div>
                        <div class="card-sub">单日预算上限: {:.2}</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">全局风险</span></div>
                        <div class="card-value">{}</div>
                        <div class="card-sub">风险分数: {:.1} / 100</div>
                    </div>
                </div>

                {}

                <h2>买入建议明细 (Kelly Adjusted)</h2>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>基金 / 赛道</th>
                                <th>资产 ID / 基准</th>
                                <th>波动率 (Vol)</th>
                                <th>周期 (Z/Regime)</th>
                                <th>全局风险</th>
                                <th>Kelly 倍率</th>
                                <th>最终执行建议</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                {}

                <div class="card" style="margin-top: 40px; background-color: #f0f7ff; border-left: 4px solid var(--primary-color);">
                    <h3 style="margin-top: 0;">Kelly 模型说明</h3>
                    <p style="font-size: 0.9rem; color: var(--text-main); line-height: 1.6;">
                        • <strong>波动率:</strong> 基于 250 日历史收益率计算。高波动会降低胜率 p。<br>
                        • <strong>周期调节:</strong> 极冷市场增加胜率，过热市场大幅降低胜率与赔率。<br>
                        • <strong>风险调节:</strong> 全局风险指数 (GRI) 升高时，强制压低所有资产的买入倍率。<br>
                        • <strong>Kelly 公式:</strong> <code>f* = p - (1-p)/b</code>。此处胜率 p 和赔率 b 均为基于模型参数的估计值。<br>
                        • <strong>安全性:</strong> 预览结果已应用单资产上限、组合总额上限和可用现金上限。
                    </p>
                    <div style="margin-top: 12px;">
                        <a href="/api/kelly/plan" class="btn btn-outline" style="font-size: 0.8rem; padding: 4px 12px;" target="_blank">查看 API 数据 (JSON)</a>
                    </div>
                </div>
                "#,
                decision.date,
                preview.preview_total_buy,
                preview.base_total_buy,
                preview.total_multiplier,
                decision.available_cash,
                decision.max_daily_buy_total,
                badge_risk(&preview.global_risk_label),
                preview.global_risk_score,
                warnings_html,
                result_rows,
                skipped_section
            );

            layout("Kelly 每日计划", content)
        }
        Err(e) => layout(
            "Kelly 每日计划",
            format!(
                "<div class='message-banner message-error'>生成 Kelly 计划失败: {}</div>",
                e
            ),
        ),
    }
}

async fn adjusted_decision_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date);

        // Load caches
        let risk_cache = state.repo.load_risk_cache(&ctx).await?;
        let regime_cache = state.repo.load_regime_cache(&ctx).await?.clone();

        let risk_overlay = if let Some(rc) = risk_cache {
            rc.overlay
        } else {
            models::GlobalRiskOverlay {
                risk_score: 0.0,
                risk_label: "未知".to_string(),
                factor_results: vec![],
                warnings: vec!["请运行 data refresh --risk".to_string()],
                explanation: "请运行 data refresh --risk".to_string(),
            }
        };

        let mut regimes = std::collections::HashMap::new();
        for entry in regime_cache.entries {
            for asset in &config.assets {
                let symbol_opt = asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone());
                if let Some(_s) = symbol_opt.filter(|s| *s == entry.symbol) {
                    regimes.insert(asset.asset_id.clone(), entry.result.clone());
                }
            }
        }

        let preview = engine::adjusted_decision::calculate_adjusted_decision(
            &config,
            &portfolio_state,
            &decision,
            &risk_overlay,
            &regimes,
        );

        Ok::<models::AdjustedDecisionPreview, anyhow::Error>(preview)
    }
    .await;

    match result {
        Ok(preview) => {
            let mut result_rows = String::new();
            for item in &preview.items {
                let pnl_class = if item.combined_multiplier > 1.0 {
                    "text-up"
                } else if item.combined_multiplier < 1.0 {
                    "text-down"
                } else {
                    ""
                };

                result_rows.push_str(&format!(
                    "<tr>
                        <td>{}</td>
                        <td><code>{}</code><br><small>{}</small></td>
                        <td>{:.2}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{:.2}x</td>
                        <td class='{}'><strong>{:.2}x</strong></td>
                        <td><strong>{:.2}</strong></td>
                        <td>{}</td>
                    </tr>",
                    item.sector,
                    item.asset_id,
                    item.fund_name,
                    item.base_suggested_buy,
                    badge_regime(&item.regime_label),
                    badge_risk(&item.global_risk_label),
                    item.kelly_multiplier,
                    pnl_class,
                    item.combined_multiplier,
                    item.capped_adjusted_buy,
                    badge_status(&item.status)
                ));
            }

            let mut warnings_html = String::new();
            if !preview.warnings.is_empty() {
                warnings_html.push_str("<div class='warning-box'><strong>注意:</strong><ul>");
                for w in &preview.warnings {
                    warnings_html.push_str(&format!("<li>{}</li>", w));
                }
                warnings_html.push_str("</ul></div>");
            }

            let content = format!(
                r#"
                <h1>风险调整买入建议</h1>
                
                <div class="dashboard-grid">
                    <div class="card">
                        <h3>综合总倍率</h3>
                        <div class="value">{:.2}x</div>
                        <div class="sub-value">相对于基础建议</div>
                    </div>
                    <div class="card">
                        <h3>基础总买入</h3>
                        <div class="value">{:.2}</div>
                        <div class="sub-value">未调节金额</div>
                    </div>
                    <div class="card">
                        <h3>调整后总买入</h3>
                        <div class="value">{:.2}</div>
                        <div class="sub-value">调节后最终金额</div>
                    </div>
                    <div class="card">
                        <h3>全局风险</h3>
                        <div class="value">{}</div>
                        <div class="sub-value">分数: {:.1}</div>
                    </div>
                </div>

                {}

                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>赛道</th>
                                <th>资产</th>
                                <th>基础建议</th>
                                <th>市场状态</th>
                                <th>全局风险</th>
                                <th>Kelly倍率</th>
                                <th>综合倍率</th>
                                <th>调整后建议</th>
                                <th>状态</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="warning-box" style="background-color: #fef9e7; border-left-color: #f1c40f; color: #7d6608;">
                    <strong>模型说明:</strong><br>
                    1. 该建议综合了<strong>基础缺口、市场冷热、全局风险、Kelly 仓位、以及数据质量</strong>等多重维度。<br>
                    2. <strong>极高风险</strong>或<strong>市场过热</strong>时，建议买入量会自动大幅缩减或归零。<br>
                    3. 若数据过期或使用模拟数据，系统会采取保守策略降低买入额。<br>
                    <br>
                    <strong>重要提示:</strong> 风险调整建议仅供参考，不作为自动交易指令。
                </div>
                "#,
                preview.total_multiplier,
                preview.base_total_buy,
                preview.adjusted_total_buy,
                badge_risk(&preview.global_risk_label),
                preview.global_risk_score,
                warnings_html,
                result_rows
            );

            layout("风险调整建议", content)
        }
        Err(e) => layout(
            "风险调整建议",
            format!("<div class='warning-box'>调整建议数据加载失败: {}</div>", e),
        ),
    }
}

async fn dca_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();

        // Auto triggers
        let _ = engine::refresh::refresh_fund_navs(state.repo.as_ref(), &ctx, &config).await;
        let _ = engine::dca::auto_execute_dca(state.repo.as_ref(), &ctx, &config, &date).await;

        let portfolio_state = state.repo.load_state(&ctx).await?;
        let plans = state.repo.load_plans(&ctx).await?;
        let nav_cache = state.repo.load_nav_cache(&ctx).await?;
        let settlements = state.repo.load_settlements(&ctx).await?;

        let dca_preview = engine::dca::calculate_dca_preview(&config, &plans, &nav_cache, &date);
        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date.clone());

        Ok::<
            (
                models::DcaPreviewSummary,
                Vec<models::DcaPlan>,
                Vec<models::DcaSettlement>,
                models::ConfigRoot,
                f64,
            ),
            anyhow::Error,
        >((
            dca_preview,
            plans,
            settlements,
            config,
            decision.suggested_total_buy,
        ))
    }
    .await;

    match result {
        Ok((preview, all_plans, settlements, config, _base_buy)) => {
            let mut plan_rows = String::new();
            for p in &all_plans {
                let asset = config.assets.iter().find(|a| a.asset_id == p.asset_id);
                let fund_name = asset.map(|a| a.fund_name.as_str()).unwrap_or("Unknown");

                let freq_str = match p.frequency {
                    models::DcaFrequency::Daily => "每日".to_string(),
                    models::DcaFrequency::Weekly => format!("每周(周{})", p.weekday.unwrap_or(1)),
                    models::DcaFrequency::Monthly => {
                        format!("每月({}日)", p.month_day.unwrap_or(1))
                    }
                };

                let status_badge = if p.enabled {
                    "<span class='badge badge-blue'>启用中</span>"
                } else {
                    "<span class='badge badge-gray'>已暂停</span>"
                };

                let actions = if p.enabled {
                    format!(
                        r#"<button onclick="updatePlanStatus('{}', false)" class="btn btn-outline" style="color: var(--warn-color); border-color: var(--warn-color); padding: 2px 8px; font-size: 0.75rem;">暂停</button>"#,
                        p.plan_id
                    )
                } else {
                    format!(
                        r#"<button onclick="updatePlanStatus('{}', true)" class="btn btn-outline" style="color: var(--down-color); border-color: var(--down-color); padding: 2px 8px; font-size: 0.75rem;">恢复</button>"#,
                        p.plan_id
                    )
                };

                plan_rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-weight: 700; color: var(--text-main);'>{}</div>
                            <div style='font-size: 0.75rem; color: var(--text-muted);'><code>{}</code></div>
                        </td>
                        <td style='font-weight: 800; font-size: 1.05rem;'>{:.2}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td style='font-size: 0.85rem;'>{}</td>
                        <td>
                            <div style='display: flex; gap: 8px;'>
                                <button onclick=\"editPlan('{}', {:.2})\" class='btn btn-outline' style='padding: 2px 8px; font-size: 0.75rem;'>改额</button>
                                {}
                                <button onclick=\"deletePlan('{}')\" class='btn btn-outline' style='color: var(--up-color); border-color: var(--up-color); padding: 2px 8px; font-size: 0.75rem;'>删除</button>
                            </div>
                        </td>
                    </tr>",
                    fund_name,
                    p.asset_id,
                    p.amount,
                    freq_str,
                    status_badge,
                    p.start_date,
                    p.plan_id, p.amount,
                    actions,
                    p.plan_id
                ));
            }

            let mut due_rows = String::new();
            for item in &preview.items {
                let status_class = if item.status == "今日应投" {
                    "text-up"
                } else {
                    "text-muted"
                };

                let nav_info = match (item.latest_nav, &item.nav_date) {
                    (Some(n), Some(d)) => format!("{:.4} ({})", n, d),
                    _ => "-".to_string(),
                };

                due_rows.push_str(&format!(
                    "<tr>
                        <td><strong>{}</strong></td>
                        <td>{:.2} {}</td>
                        <td>{}</td>
                        <td class='{}' style='font-weight: 700;'>{}</td>
                        <td>{}</td>
                    </tr>",
                    item.fund_name,
                    item.amount,
                    item.currency,
                    nav_info,
                    status_class,
                    item.status,
                    item.warnings.join(", ")
                ));
            }

            let mut history_rows = String::new();
            let mut recent_settlements = settlements.clone();
            recent_settlements.sort_by(|a, b| b.deduction_date.cmp(&a.deduction_date));
            for s in recent_settlements.iter().take(10) {
                history_rows.push_str(&format!(
                    "<tr>
                        <td>{}</td>
                        <td>{}</td>
                        <td style='font-weight: 600;'>{:.2}</td>
                        <td>{:.4}</td>
                        <td>{:.4}</td>
                        <td><span class='badge badge-blue'>已入账</span></td>
                    </tr>",
                    s.deduction_date, s.fund_name, s.amount, s.confirmed_nav, s.confirmed_units
                ));
            }

            let mut asset_options = String::new();
            for a in &config.assets {
                if a.enabled {
                    asset_options.push_str(&format!(
                        "<option value='{}'>{}</option>",
                        a.asset_id, a.fund_name
                    ));
                }
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 24px;">
                    <div>
                        <h1 style="margin-bottom: 4px;">定投控制中心 (DCA Center)</h1>
                        <p style="color: var(--text-muted); font-size: 0.9rem; margin: 0;">自动化交易执行与定投计划全生命周期管理</p>
                    </div>
                    <div style="display: flex; gap: 10px;">
                        <button onclick="runDueDca()" class="btn btn-success">🚀 立即执行今日到期</button>
                        <button onclick="refreshNav()" class="btn btn-outline">🔄 刷新全部净值</button>
                    </div>
                </div>

                <div class="dashboard-grid">
                    <div class="card">
                        <div class="card-header"><span class="card-title">今日定投计划</span></div>
                        <div class="card-value text-up">{:.2} <small style="font-size: 0.9rem;">CNY</small></div>
                        <div class="card-sub">应执行 {} 笔</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">活跃计划</span></div>
                        <div class="card-value">{}</div>
                        <div class="card-sub">启用中的定投规则</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">上一次执行</span></div>
                        <div class="card-value" style="font-size: 1.2rem; margin-top: 8px;">{}</div>
                        <div class="card-sub">历史记录最后日期</div>
                    </div>
                </div>

                <div style="display: grid; grid-template-columns: 2fr 1fr; gap: 24px; margin-top: 32px;">
                    <div>
                        <h2>定投计划列表 (DCA Rules)</h2>
                        <div class="table-container">
                            <table>
                                <thead>
                                    <tr>
                                        <th>基金资产</th>
                                        <th>单次金额</th>
                                        <th>频率</th>
                                        <th>状态</th>
                                        <th>开始日期</th>
                                        <th>操作</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {}
                                </tbody>
                            </table>
                        </div>

                        <h2 style="margin-top: 40px;">近期执行历史 (Recent Executions)</h2>
                        <div class="table-container">
                            <table>
                                <thead>
                                    <tr>
                                        <th>扣款日期</th>
                                        <th>基金名称</th>
                                        <th>金额</th>
                                        <th>成交净值</th>
                                        <th>成交份额</th>
                                        <th>状态</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {}
                                </tbody>
                            </table>
                        </div>
                    </div>

                    <div>
                        <h2>添加新计划</h2>
                        <div class="card">
                            <form id="addPlanForm">
                                <div class="form-group">
                                    <label>目标资产</label>
                                    <select id="new_asset_id" required>
                                        {}
                                    </select>
                                </div>
                                <div class="form-group">
                                    <label>频率</label>
                                    <select id="new_frequency" onchange="toggleDayInput()">
                                        <option value="daily">每日 (工作日)</option>
                                        <option value="weekly">每周</option>
                                        <option value="monthly">每月</option>
                                    </select>
                                </div>
                                <div class="form-group" id="dayInputGroup" style="display:none;">
                                    <label id="dayLabel">周几/几号</label>
                                    <input type="number" id="new_day" placeholder="1-7 或 1-31">
                                </div>
                                <div class="form-group">
                                    <label>定投金额 (CNY)</label>
                                    <input type="number" id="new_amount" step="0.01" required>
                                </div>
                                <button type="button" onclick="addNewPlan()" class="btn btn-primary" style="width:100%;">+ 创建计划</button>
                            </form>
                        </div>

                        <h2 style="margin-top: 32px;">今日预检 (Preview)</h2>
                        <div class="card" style="padding: 0; overflow: hidden;">
                            <table style="margin: 0; font-size: 0.85rem;">
                                <tbody id="previewBody">
                                    {}
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>

                <script>
                    function toggleDayInput() {{
                        const freq = document.getElementById('new_frequency').value;
                        const group = document.getElementById('dayInputGroup');
                        const label = document.getElementById('dayLabel');
                        if (freq === 'weekly') {{
                            group.style.display = 'block';
                            label.innerText = '每周几 (1-7, 1=周一)';
                        }} else if (freq === 'monthly') {{
                            group.style.display = 'block';
                            label.innerText = '每月几号 (1-31)';
                        }} else {{
                            group.style.display = 'none';
                        }}
                    }}

                    async function addNewPlan() {{
                        const data = {{
                            asset_id: document.getElementById('new_asset_id').value,
                            amount: parseFloat(document.getElementById('new_amount').value),
                            frequency: document.getElementById('new_frequency').value,
                            day: parseInt(document.getElementById('new_day').value) || null,
                        }};
                        try {{
                            const res = await fetch('/api/dca/plans', {{
                                method: 'POST',
                                headers: {{ 'Content-Type': 'application/json' }},
                                body: JSON.stringify(data)
                            }});
                            const result = await res.json();
                            if (result.success) location.reload();
                            else alert('Error: ' + result.message);
                        }} catch (e) {{ alert(e); }}
                    }}

                    async function updatePlanStatus(id, enabled) {{
                        try {{
                            await fetch(`/api/dca/plans/${{id}}`, {{
                                method: 'PATCH',
                                headers: {{ 'Content-Type': 'application/json' }},
                                body: JSON.stringify({{ enabled }})
                            }});
                            location.reload();
                        }} catch (e) {{ alert(e); }}
                    }}

                    async function editPlan(id, currentAmount) {{
                        const amount = prompt('请输入新的定投金额:', currentAmount);
                        if (amount === null) return;
                        try {{
                            await fetch(`/api/dca/plans/${{id}}`, {{
                                method: 'PATCH',
                                headers: {{ 'Content-Type': 'application/json' }},
                                body: JSON.stringify({{ amount: parseFloat(amount) }})
                            }});
                            location.reload();
                        }} catch (e) {{ alert(e); }}
                    }}

                    async function deletePlan(id) {{
                        if (!confirm('确定要删除此计划吗？历史成交数据将保留。')) return;
                        try {{
                            await fetch(`/api/dca/plans/${{id}}`, {{ method: 'DELETE' }});
                            location.reload();
                        }} catch (e) {{ alert(e); }}
                    }}

                    async function runDueDca() {{
                        if (!confirm('确定要执行今日到期的定投计划吗？')) return;
                        const res = await fetch('/api/dca/run-due', {{ method: 'POST' }});
                        const result = await res.json();
                        alert(result.message);
                        location.reload();
                    }}

                    async function refreshNav() {{
                        const res = await fetch('/api/nav/refresh', {{ method: 'POST' }});
                        const result = await res.json();
                        alert(result.message);
                        location.reload();
                    }}
                </script>
                "#,
                preview.total_due_amount,
                preview
                    .items
                    .iter()
                    .filter(|i| i.status == "今日应投")
                    .count(),
                all_plans.iter().filter(|p| p.enabled).count(),
                settlements
                    .first()
                    .map(|s| s.deduction_date.as_str())
                    .unwrap_or("从未"),
                plan_rows,
                history_rows,
                asset_options,
                due_rows
            );

            layout("定投计划", content)
        }
        Err(e) => layout(
            "定投计划",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
    }
}

async fn daily_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let status = state.refresh_status.read().await;
    let report_opt = &status.latest_daily_report;

    let mut timeline_html = String::new();
    let mut plan_summary_html = String::new();
    let mut status_badge = "<span class='badge badge-gray'>未运行</span>".to_string();

    if let Some(report) = report_opt {
        status_badge = match report.status {
            models::DailyOperationStatus::Success => "<span class='badge badge-green'>运行完成</span>",
            models::DailyOperationStatus::PartialSuccess => "<span class='badge badge-orange'>部分成功</span>",
            models::DailyOperationStatus::Failed => "<span class='badge badge-red'>运行失败</span>",
            models::DailyOperationStatus::Running => "<span class='badge badge-blue'>执行中...</span>",
            _ => "<span class='badge badge-gray'>未知</span>",
        }.to_string();

        for step in &report.steps {
            let (icon, color, bg) = match step.status {
                models::DailyOperationStatus::Success => ("✅", "#00B42A", "rgba(0, 180, 42, 0.05)"),
                models::DailyOperationStatus::PartialSuccess => ("⚠️", "#FF7D00", "rgba(255, 125, 0, 0.05)"),
                models::DailyOperationStatus::Failed => ("❌", "#F53F3F", "rgba(245, 63, 63, 0.05)"),
                models::DailyOperationStatus::Running => ("⏳", "#0052D9", "rgba(0, 82, 217, 0.05)"),
                models::DailyOperationStatus::Skipped => ("⏭️", "#86909C", "rgba(134, 144, 156, 0.05)"),
                _ => ("⚪", "#E5E6EB", "transparent"),
            };
            
            timeline_html.push_str(&format!(
                r#"<div style="display: flex; gap: 16px; margin-bottom: 20px; position: relative; padding-left: 24px;">
                    <div style="position: absolute; left: 6px; top: 24px; bottom: -20px; width: 2px; background: var(--border-color);"></div>
                    <div style="width: 14px; height: 14px; border-radius: 50%; background: {}; border: 3px solid #FFF; position: absolute; left: 0; top: 6px; z-index: 1; box-shadow: var(--shadow-sm);"></div>
                    <div class="card" style="flex: 1; margin-bottom: 0; padding: 16px 20px; background: {}; border-color: {};">
                        <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 4px;">
                            <div style="font-weight: 800; font-size: 1rem; color: var(--text-main);">{} {}</div>
                            <div style="font-size: 0.75rem; color: var(--text-muted); font-weight: 600;">{}</div>
                        </div>
                        <div style="font-size: 0.9rem; color: var(--text-main);">{}</div>
                    </div>
                </div>"#,
                color, bg, color, icon, step.name, step.completed_at.as_deref().unwrap_or(step.started_at.as_deref().unwrap_or("-")), step.message
            ));
        }

        if let Some(plan) = &report.plan {
            let mut item_rows = String::new();
            for item in &plan.items {
                if item.recommended_amount > 0.0 {
                    item_rows.push_str(&format!(
                        "<tr>
                            <td><strong>{}</strong></td>
                            <td><span class='badge badge-outline'>{}</span></td>
                            <td class='text-up tabular' style='font-weight: 900; font-size: 1.1rem;'>{:.2}</td>
                            <td class='text-right'>{}</td>
                        </tr>",
                        item.fund_name, item.sector, item.recommended_amount, item.status
                    ));
                }
            }

            if item_rows.is_empty() {
                item_rows = "<tr><td colspan='4' style='text-align: center; padding: 48px; color: var(--text-muted); font-weight: 500;'>☕ 今日无需执行买入，建议保持观望。</td></tr>".to_string();
            }

            plan_summary_html = format!(
                r#"
                <h2 style="margin-top: 48px;">📌 今日执行建议 (Execution Plan)</h2>
                <div class="dashboard-grid">
                    <div class="card" style="border-left: 4px solid var(--up-color);">
                        <div class="card-header"><span class="card-title">拟定总买入</span></div>
                        <div class="card-value text-up tabular">{:.2}</div>
                        <div class="card-sub">计划执行的资金总量</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">待处理项</span></div>
                        <div class="card-value tabular">{}</div>
                        <div class="card-sub">项基金买入建议</div>
                    </div>
                </div>

                <div class="table-container">
                    <div class="table-wrap">
                        <table>
                            <thead>
                                <tr>
                                    <th>基金名称</th>
                                    <th>资产赛道</th>
                                    <th>建议买入金额 (CNY)</th>
                                    <th class='text-right'>执行依据</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>
                </div>
                "#,
                plan.total_recommended_amount,
                plan.items.iter().filter(|i| i.recommended_amount > 0.0).count(),
                item_rows
            );
        }
    }

    if timeline_html.is_empty() {
        timeline_html = "<div class='empty-state'><span class='empty-state-icon'>💤</span><div class='empty-state-text'>流水线今日尚未启动。点击上方按钮开始自动化数据同步。</div></div>".to_string();
    }

    let content = format!(
        r#"
        <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 32px;">
            <div>
                <h1 style="margin-bottom: 4px;">每日操作流水线 (Daily Pipeline)</h1>
                <p style="color: var(--text-muted); font-size: 1rem; margin: 0;">全自动数据获取、定投执行与资产评估</p>
            </div>
            <div style="display: flex; gap: 12px;">
                <button id="runPipelineBtn" onclick="runPipeline(this)" class="btn">🚀 启动每日流水线</button>
            </div>
        </div>

        <div class="card" style="background: var(--primary-light); border: 1px solid #C0D9FB; padding: 24px;">
            <h3 style="margin-top: 0; color: var(--primary-color);">💡 流水线功能说明</h3>
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin-top: 16px;">
                <div style="font-size: 0.85rem; color: #4E5969; line-height: 1.6;"><strong>1. 同步净值:</strong> 从天天基金网获取最新单位净值。</div>
                <div style="font-size: 0.85rem; color: #4E5969; line-height: 1.6;"><strong>2. 刷新行情:</strong> 更新指数、期货及汇率的实时缓存。</div>
                <div style="font-size: 0.85rem; color: #4E5969; line-height: 1.6;"><strong>3. 执行定投:</strong> 自动结算今日到期的扣款记录。</div>
                <div style="font-size: 0.85rem; color: #4E5969; line-height: 1.6;"><strong>4. 对账审计:</strong> 检查数据完整性并评估赛道缺口。</div>
                <div style="font-size: 0.85rem; color: #4E5969; line-height: 1.6;"><strong>5. 生成建议:</strong> 计算今日最优买入基金及权重。</div>
            </div>
        </div>

        <div style="margin-bottom: 16px; display: flex; align-items: center; gap: 12px;">
            <span style="font-size: 1.25rem; font-weight: 800;">状态详情</span>
            {}
        </div>

        <div style="padding-top: 8px;">
            {}
        </div>

        {}

        <script>
            async function runPipeline(btn) {{
                if (btn) {{
                    btn.disabled = true;
                    btn.innerText = '⏳ 正在执行中...';
                }}
                
                try {{
                    const res = await fetch('/api/daily/run', {{ method: 'POST' }});
                    const result = await res.json();
                    if (result.success) {{
                        location.reload();
                    }} else {{
                        alert('运行失败: ' + result.message);
                        location.reload();
                    }}
                }} catch (e) {{
                    alert('网络错误: ' + e);
                    if (btn) {{
                        btn.disabled = false;
                        btn.innerText = '🚀 启动每日流水线';
                    }}
                }}
            }}
        </script>
        "#,
        status_badge,
        timeline_html,
        plan_summary_html
    );

    layout("每日流水线", content)
}

async fn dca_settlements_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = state.repo.load_settlements(&ctx).await;

    match result {
        Ok(mut settlements) => {
            settlements.sort_by(|a, b| b.deduction_date.cmp(&a.deduction_date));
            let mut rows = String::new();
            for s in settlements {
                let status_badge = match s.status {
                    models::DcaSettlementStatus::Confirmed => badge_status("正常"),
                    models::DcaSettlementStatus::Pending => {
                        "<span class='badge badge-orange'>处理中</span>".to_string()
                    }
                    models::DcaSettlementStatus::Failed => {
                        "<span class='badge badge-red'>失败</span>".to_string()
                    }
                };

                let (applied_text, applied_badge) = if s.applied {
                    ("已入账", "badge-green")
                } else {
                    ("待入账", "badge-outline")
                };

                rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-weight: 700; color: var(--text-main); font-size: 1.05rem;'>{}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted); margin-top: 2px;'><code>{}</code></div>
                        </td>
                        <td style='font-weight: 800; font-size: 1.1rem; font-family: DIN Alternate, Helvetica Neue;'>{:.2}</td>
                        <td>
                            <div style='font-size: 1.05rem; font-weight: 600;'>{:.4}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted);'>{:.2} 份</div>
                        </td>
                        <td>
                            <div style='font-size: 0.85rem;'>扣款: {}</div>
                            <div style='font-size: 0.85rem;'>确认: {}</div>
                        </td>
                        <td>{}</td>
                        <td><span class='badge {}'>{}</span></td>
                        <td><div style='font-size: 0.85rem; color: var(--text-muted);'>{}</div></td>
                    </tr>",
                    s.fund_name,
                    s.asset_id,
                    s.amount,
                    s.confirmed_nav,
                    s.confirmed_units,
                    s.deduction_date,
                    s.confirmation_date,
                    status_badge,
                    applied_badge,
                    applied_text,
                    s.note.as_deref().unwrap_or("-")
                ));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 24px; background: #FFF; padding: 20px; border-radius: 12px; border: 1px solid var(--border-color); box-shadow: var(--shadow);">
                    <div>
                        <h1 style="margin-bottom: 4px;">定投结算记录 (DCA Settlements)</h1>
                        <p style="color: var(--text-muted); font-size: 0.9rem; margin: 0;">记录历史成交确认单，追踪份额入账情况</p>
                    </div>
                    <div style="text-align: right;">
                        <a href="/admin/dca-settlements" class="btn">结算管理录入 &rarr;</a>
                    </div>
                </div>

                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>基金名称 / 资产ID</th>
                                <th>成交金额 (CNY)</th>
                                <th>确认净值 / 份额</th>
                                <th>日期 (扣款/确认)</th>
                                <th>单据状态</th>
                                <th>入账状态</th>
                                <th>备注说明</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="card" style="background-color: #F7F8FA; border: 1px dashed var(--border-color); padding: 20px;">
                    <p style="font-size: 0.9rem; color: var(--text-muted); margin: 0; line-height: 1.6;">
                        💡 <strong>入账说明:</strong><br>
                        • <strong>已入账:</strong> 表示该确认单对应的份额已累加到系统持仓中。<br>
                        • <strong>待入账:</strong> 表示已录入单据但尚未应用到持仓，您可以在管理后台手动触发应用。<br>
                        • 建议在收到平台确认短信/通知后及时录入真实成交净值与份额。
                    </p>
                </div>
                "#,
                rows
            );

            layout("定投结算", content)
        }
        Err(e) => layout(
            "定投结算",
            format!(
                "<div class='message-banner message-error'>定投结算数据加载失败: {}</div>",
                e
            ),
        ),
    }
}
async fn import_handler(State(_state): State<Arc<AppState>>) -> Html<String> {
    let content = r#"
        <div style="margin-bottom: 40px;">
            <h1>数据中心 (Import Center)</h1>
            <p style="color: var(--text-muted); font-size: 1.1rem; margin: 0;">同步您的交易流水与外部持仓状态</p>
        </div>

        <div class="dashboard-grid">
            <div class="card" style="display: flex; flex-direction: column; justify-content: space-between;">
                <div>
                    <div style="font-size: 2rem; margin-bottom: 16px;">📑</div>
                    <h2 style="margin-top: 0;">交易流水导入</h2>
                    <p style="color: var(--text-muted); font-size: 0.9rem; line-height: 1.6;">支持标准 CSV 格式。系统会自动解析日期、类型和金额，并根据流水推算份额和现金流。</p>
                    <div style="margin-top: 12px;">
                        <a href="/templates/transactions.csv" class="btn btn-sm btn-ghost" style="padding-left: 0;">📥 下载交易模板</a>
                    </div>
                </div>
                <div style="margin-top: 24px;">
                    <a href="/import/transactions" class="btn btn-block">去导入流水 &rarr;</a>
                </div>
            </div>

            <div class="card" style="display: flex; flex-direction: column; justify-content: space-between; border-color: var(--primary-color); background: rgba(0, 82, 217, 0.01);">
                <div>
                    <div style="font-size: 2rem; margin-bottom: 16px;">📸</div>
                    <h2 style="margin-top: 0;">支付宝快照导入</h2>
                    <p style="color: var(--text-muted); font-size: 0.9rem; line-height: 1.6;">导入支付宝 App 导出的“资产快照”CSV。用于一次性对齐所有标的的最新持有份额与市值。</p>
                    <div style="margin-top: 12px;">
                        <a href="/templates/alipay_holdings_snapshot.csv" class="btn btn-sm btn-ghost" style="padding-left: 0;">📥 下载快照模板</a>
                    </div>
                </div>
                <div style="margin-top: 24px;">
                    <a href="/alipay/holdings" class="btn btn-block">去导入快照 &rarr;</a>
                </div>
            </div>
        </div>

        <div class="card" style="background: #F8FAFC; border: 2px dashed var(--border-color); padding: 32px;">
            <h2 style="margin-top: 0;">💡 推荐数据同步流程</h2>
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 24px; margin-top: 24px;">
                <div>
                    <div style="font-weight: 800; font-size: 1.2rem; color: var(--primary-color); margin-bottom: 8px;">01.</div>
                    <div style="font-weight: 700; margin-bottom: 8px;">初始化/校准</div>
                    <div style="font-size: 0.85rem; color: var(--text-muted);">使用“支付宝快照导入”快速同步当前最新的资产持有情况。</div>
                </div>
                <div>
                    <div style="font-weight: 800; font-size: 1.2rem; color: var(--primary-color); margin-bottom: 8px;">02.</div>
                    <div style="font-weight: 700; margin-bottom: 8px;">日常维护</div>
                    <div style="font-size: 0.85rem; color: var(--text-muted);">每周或每月导入一次“交易流水”，以保持现金流和盈利统计的准确。</div>
                </div>
                <div>
                    <div style="font-weight: 800; font-size: 1.2rem; color: var(--primary-color); margin-bottom: 8px;">03.</div>
                    <div style="font-weight: 700; margin-bottom: 8px;">对账审计</div>
                    <div style="font-size: 0.85rem; color: var(--text-muted);">前往“<a href="/reconcile">对账中心</a>”检查系统账本与外部快照是否存在逻辑偏差。</div>
                </div>
            </div>
        </div>
    "#;

    layout("数据导入", content.to_string())
}

async fn import_transactions_handler(State(_state): State<Arc<AppState>>) -> Html<String> {
    let content = r#"
    <div style="display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 24px;">
        <h1>交易数据导入 (Transaction Import)</h1>
    </div>

    <div class="card">
        <div class="card-header"><span class="card-title">上传 CSV 文件</span></div>
        <form id="importForm" enctype="multipart/form-data">
            <div class="form-group">
                <label for="csvFile">选择 CSV 文件 (标准格式: Date,Type,Asset ID,Amount,Units,Price,Fee,Source,Note)</label>
                <input type="file" id="csvFile" name="file" accept=".csv" required style="margin-bottom: 12px;">
                <p style="font-size: 0.8rem; color: var(--text-muted);">提示: 重复数据将被自动检测并默认跳过。</p>
            </div>
            <div style="display: flex; gap: 12px;">
                <button type="button" id="previewBtn" class="btn" onclick="previewImport()">预览数据</button>
                <button type="button" id="commitBtn" class="btn btn-success" onclick="commitImport()" disabled>确认导入</button>
            </div>
        </form>
    </div>

    <div id="loading" style="display: none; text-align: center; padding: 40px;">
        <div style="font-size: 1.2rem; font-weight: 600; color: var(--primary-color);">正在处理中...</div>
    </div>

    <div id="previewContainer" style="display: none;">
        <div class="dashboard-grid" id="summaryGrid">
            <!-- Filled by JS -->
        </div>

        <div class="table-container">
            <table>
                <thead>
                    <tr>
                        <th>日期</th>
                        <th>类型</th>
                        <th>资产 ID</th>
                        <th>金额 (CNY)</th>
                        <th>份额</th>
                        <th>状态</th>
                    </tr>
                </thead>
                <tbody id="previewBody">
                    <!-- Filled by JS -->
                </tbody>
            </table>
        </div>
    </div>

    <script>
        async function previewImport() {
            const fileInput = document.getElementById('csvFile');
            if (!fileInput.files[0]) {
                alert('请先选择文件');
                return;
            }

            const formData = new FormData();
            formData.append('file', fileInput.files[0]);

            document.getElementById('loading').style.display = 'block';
            document.getElementById('previewContainer').style.display = 'none';
            document.getElementById('commitBtn').disabled = true;

            try {
                const response = await fetch('/api/import/preview', {
                    method: 'POST',
                    body: formData
                });
                const data = await response.json();
                renderPreview(data);
            } catch (error) {
                alert('预览失败: ' + error);
            } finally {
                document.getElementById('loading').style.display = 'none';
            }
        }

        function renderPreview(data) {
            const container = document.getElementById('previewContainer');
            const summaryGrid = document.getElementById('summaryGrid');
            const previewBody = document.getElementById('previewBody');

            summaryGrid.innerHTML = `
                <div class="card">
                    <div class="card-header"><span class="card-title">总行数</span></div>
                    <div class="card-value">${data.summary.total_rows}</div>
                </div>
                <div class="card">
                    <div class="card-header"><span class="card-title">可导入</span></div>
                    <div class="card-value text-up">${data.summary.valid_rows}</div>
                </div>
                <div class="card">
                    <div class="card-header"><span class="card-title">重复 (将跳过)</span></div>
                    <div class="card-value text-muted">${data.summary.duplicate_rows}</div>
                </div>
                <div class="card">
                    <div class="card-header"><span class="card-title">错误行</span></div>
                    <div class="card-value ${data.summary.error_rows > 0 ? 'text-up' : ''}">${data.summary.error_rows}</div>
                </div>
            `;

            previewBody.innerHTML = '';
            data.candidates.forEach((c, i) => {
                const isDuplicate = data.duplicates[i];
                const errors = data.errors[i];
                
                let statusHtml = '';
                if (errors.length > 0) {
                    statusHtml = '<span class="badge badge-red">错误</span>';
                } else if (isDuplicate) {
                    statusHtml = '<span class="badge badge-gray">重复</span>';
                } else {
                    statusHtml = '<span class="badge badge-blue">新增</span>';
                }

                const row = document.createElement('tr');
                if (errors.length > 0) row.style.backgroundColor = '#fff1f0';
                
                row.innerHTML = `
                    <td>${c.date}</td>
                    <td>${c.transaction_type}</td>
                    <td><code>${c.asset_id || '-'}</code></td>
                    <td style="font-weight: 600;">${c.amount.toFixed(2)}</td>
                    <td>${c.units ? c.units.toFixed(4) : '-'}</td>
                    <td>${statusHtml} ${errors.join(', ')}</td>
                `;
                previewBody.appendChild(row);
            });

            container.style.display = 'block';
            if (data.summary.valid_rows > 0 && data.summary.error_rows === 0) {
                document.getElementById('commitBtn').disabled = false;
            }
        }

        async function commitImport() {
            if (!confirm('确定要导入这些交易吗？')) return;

            const fileInput = document.getElementById('csvFile');
            const formData = new FormData();
            formData.append('file', fileInput.files[0]);

            document.getElementById('loading').style.display = 'block';
            document.getElementById('commitBtn').disabled = true;

            try {
                const response = await fetch('/api/import/commit', {
                    method: 'POST',
                    body: formData
                });
                const result = await response.json();
                if (result.success) {
                    alert(result.message);
                    window.location.href = '/transactions';
                } else {
                    alert('导入失败: ' + result.message);
                }
            } catch (error) {
                alert('提交失败: ' + error);
            } finally {
                document.getElementById('loading').style.display = 'none';
            }
        }
    </script>
    "#;
    layout("交易导入", content.to_string())
}

async fn api_import_preview_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<models::import::TransactionImportPreview> {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut content = String::new();
        while let Some(field) = multipart.next_field().await? {
            if field.name() == Some("file") {
                content = field.text().await?;
                break;
            }
        }

        if content.is_empty() {
            anyhow::bail!("Empty file or no file field found");
        }

        let transactions = state.repo.load_transactions(&ctx).await?;
        let candidates = engine::import::parse_transactions_from_csv(&content)?;
        let preview = engine::import::preview_import(candidates, &transactions);
        Ok::<models::import::TransactionImportPreview, anyhow::Error>(preview)
    }
    .await;

    match result {
        Ok(p) => Json(p),
        Err(_e) => Json(models::import::TransactionImportPreview {
            candidates: vec![],
            duplicates: vec![],
            warnings: vec![],
            errors: vec![],
            summary: models::import::ImportSummary {
                total_rows: 0,
                valid_rows: 0,
                error_rows: 1,
                warning_rows: 0,
                duplicate_rows: 0,
                new_rows: 0,
            },
        }),
    }
}

async fn api_import_commit_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<models::import::ImportResult> {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut content = String::new();
        while let Some(field) = multipart.next_field().await? {
            if field.name() == Some("file") {
                content = field.text().await?;
                break;
            }
        }

        if content.is_empty() {
            anyhow::bail!("Empty file or no file field found");
        }

        let mut transactions = state.repo.load_transactions(&ctx).await?;
        let mut portfolio_state = state.repo.load_state(&ctx).await?;
        let candidates = engine::import::parse_transactions_from_csv(&content)?;
        let preview = engine::import::preview_import(candidates, &transactions);

        if preview.summary.error_rows > 0 {
            anyhow::bail!("Import rejected: file contains errors.");
        }

        let import_result = engine::import::commit_import(
            &preview,
            &mut portfolio_state,
            &mut transactions,
            true, // skip duplicates
        );

        if import_result.inserted > 0 {
            state.repo.save_state(&ctx, &portfolio_state).await?;
            state.repo.save_transactions(&ctx, &transactions).await?;
        }

        Ok::<models::import::ImportResult, anyhow::Error>(import_result)
    }
    .await;

    match result {
        Ok(r) => Json(r),
        Err(e) => Json(models::import::ImportResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

async fn alipay_holdings_handler(State(_state): State<Arc<AppState>>) -> Html<String> {
    let content = r#"
    <div style="display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 24px;">
        <h1>支付宝持仓对齐 (Alipay Holding Alignment)</h1>
    </div>

    <div class="card">
        <div class="card-header"><span class="card-title">上传持仓导出文件</span></div>
        <form id="uploadForm">
            <div class="form-group">
                <label>选择 CSV 文件 (支持包含基金代码, 基金名称, 持有份额, 市值, 单位净值, 净值日期的文件)</label>
                <input type="file" id="holdingFile" accept=".csv" required style="margin-bottom: 12px;">
            </div>
            <div class="form-group">
                <label>对账日期 (Snapshot Date)</label>
                <input type="date" id="snapshotDate" required>
            </div>
            <div style="display: flex; gap: 12px; margin-top: 16px;">
                <button type="button" onclick="previewHoldings()" class="btn">预览差异</button>
                <button type="button" id="alignBtn" onclick="alignHoldings()" class="btn btn-success" disabled>保存快照并准备对齐</button>
            </div>
        </form>
    </div>

    <div id="loading" style="display: none; padding: 40px; text-align: center;">
        <div style="font-size: 1.2rem; font-weight: 600; color: var(--primary-color);">正在解析数据...</div>
    </div>

    <div id="previewContainer" style="display: none; margin-top: 32px;">
        <h2>对比结果 (Comparison Result)</h2>
        <div class="table-container">
            <table>
                <thead>
                    <tr>
                        <th>基金名称 / 代码</th>
                        <th>对齐资产 ID</th>
                        <th>Alipay 份额</th>
                        <th>本地份额</th>
                        <th>差异</th>
                        <th>市值 (CNY)</th>
                        <th>状态</th>
                    </tr>
                </thead>
                <tbody id="previewBody"></tbody>
            </table>
        </div>
    </div>

    <script>
        document.getElementById('snapshotDate').value = new Date().toISOString().split('T')[0];

        async function previewHoldings() {
            const fileInput = document.getElementById('holdingFile');
            const dateInput = document.getElementById('snapshotDate');
            if (!fileInput.files[0]) { alert('请先选择文件'); return; }

            const formData = new FormData();
            formData.append('file', fileInput.files[0]);
            formData.append('date', dateInput.value);

            document.getElementById('loading').style.display = 'block';
            document.getElementById('previewContainer').style.display = 'none';

            try {
                const res = await fetch('/api/alipay/holdings/preview', {
                    method: 'POST',
                    body: formData
                });
                const data = await res.json();
                renderPreview(data);
            } catch (e) {
                alert('预览失败: ' + e);
            } finally {
                document.getElementById('loading').style.display = 'none';
            }
        }

        function renderPreview(data) {
            const body = document.getElementById('previewBody');
            body.innerHTML = '';
            
            data.candidates.forEach((c, i) => {
                const matched = data.matched_asset_ids[i];
                const localUnits = data.system_units[i];
                const diff = data.unit_diffs[i];
                const errors = data.errors[i];
                const warnings = data.warnings[i];

                const row = document.createElement('tr');
                if (errors.length > 0) row.style.backgroundColor = '#fff1f0';
                
                let statusHtml = '';
                if (errors.length > 0) statusHtml = `<span class="badge badge-red">错误</span> <small>${errors.join(', ')}</small>`;
                else if (diff !== null && Math.abs(diff) > 0.0001) statusHtml = `<span class="badge badge-orange">差异</span> <small>${warnings.join(', ')}</small>`;
                else statusHtml = '<span class="badge badge-green">一致</span>';

                row.innerHTML = `
                    <td>
                        <div style="font-weight: 700;">${c.fund_name}</div>
                        <div style="font-size: 0.75rem; color: var(--text-muted);"><code>${c.fund_code}</code></div>
                    </td>
                    <td><code>${matched || '-'}</code></td>
                    <td style="font-weight: 600;">${c.units.toFixed(4)}</td>
                    <td>${localUnits !== null ? localUnits.toFixed(4) : '-'}</td>
                    <td class="${diff > 0 ? 'text-up' : (diff < 0 ? 'text-down' : '')}">${diff !== null ? diff.toFixed(4) : '-'}</td>
                    <td>${c.market_value.toFixed(2)}</td>
                    <td>${statusHtml}</td>
                `;
                body.appendChild(row);
            });

            document.getElementById('previewContainer').style.display = 'block';
            document.getElementById('alignBtn').disabled = false;
        }

        async function alignHoldings() {
            if (!confirm('确定要保存这些快照吗？这不会自动修改持仓，您可以在对账页面进行手动校准。')) return;
            
            const fileInput = document.getElementById('holdingFile');
            const dateInput = document.getElementById('snapshotDate');
            const formData = new FormData();
            formData.append('file', fileInput.files[0]);
            formData.append('date', dateInput.value);

            try {
                const res = await fetch('/api/alipay/holdings/align', {
                    method: 'POST',
                    body: formData
                });
                const result = await res.json();
                alert(result.message);
                if (result.success) window.location.href = '/reconcile/alipay';
            } catch (e) {
                alert('提交失败: ' + e);
            }
        }
    </script>
    "#;
    layout("支付宝持仓对齐", content.to_string())
}

async fn api_alipay_holdings_preview_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<models::AlipayHoldingImportPreview> {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut content = String::new();
        let mut date = String::new();
        while let Some(field) = multipart.next_field().await? {
            match field.name() {
                Some("file") => content = field.text().await?,
                Some("date") => date = field.text().await?,
                _ => {}
            }
        }

        if content.is_empty() {
            anyhow::bail!("Empty file or no file field found");
        }
        if date.is_empty() {
            date = Local::now().format("%Y-%m-%d").to_string();
        }

        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let candidates = engine::alipay_holding::parse_alipay_holdings_from_csv(&content)?;
        let preview = engine::alipay_holding::preview_alipay_holdings(
            &config,
            &portfolio_state,
            candidates,
            &date,
        );
        Ok::<models::AlipayHoldingImportPreview, anyhow::Error>(preview)
    }
    .await;

    match result {
        Ok(p) => Json(p),
        Err(_e) => Json(models::AlipayHoldingImportPreview::default()),
    }
}

async fn api_alipay_holdings_align_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<models::AlipayHoldingImportResult> {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut content = String::new();
        let mut date = String::new();
        while let Some(field) = multipart.next_field().await? {
            match field.name() {
                Some("file") => content = field.text().await?,
                Some("date") => date = field.text().await?,
                _ => {}
            }
        }

        if content.is_empty() {
            anyhow::bail!("Empty file or no file field found");
        }
        if date.is_empty() {
            date = Local::now().format("%Y-%m-%d").to_string();
        }

        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let candidates = engine::alipay_holding::parse_alipay_holdings_from_csv(&content)?;
        let preview = engine::alipay_holding::preview_alipay_holdings(
            &config,
            &portfolio_state,
            candidates,
            &date,
        );

        let snapshots = engine::alipay_holding::convert_to_snapshots(&preview);
        let imported_count = snapshots.len();

        if imported_count > 0 {
            let mut existing = state.repo.load_alipay_snapshots(&ctx).await?;
            existing.extend(snapshots);
            state.repo.save_alipay_snapshots(&ctx, &existing).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "web_user".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "IMPORT_ALIPAY_SNAPSHOTS".to_string(),
                target_file: "alipay_snapshots.json".to_string(),
                target_id: Some(date),
                old_value_summary: format!("existing: {}", existing.len() - imported_count),
                new_value_summary: format!("total: {}", existing.len()),
                status: "success".to_string(),
                note: Some(format!("Imported {} snapshots", imported_count)),
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
        }

        Ok::<usize, anyhow::Error>(imported_count)
    }
    .await;

    match result {
        Ok(count) => Json(models::AlipayHoldingImportResult {
            imported_count: count,
            success: true,
            message: format!(
                "成功导入 {} 笔快照。请前往对账页面查看并进行必要的手动校准。",
                count
            ),
            ..Default::default()
        }),
        Err(e) => Json(models::AlipayHoldingImportResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

async fn alipay_reconcile_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let snapshots = state.repo.load_alipay_snapshots(&ctx).await?;

        let mut latest_snaps = std::collections::HashMap::new();
        for s in &snapshots {
            let key = if s.asset_id.is_empty() {
                format!("unmatched_{}", s.fund_code)
            } else {
                s.asset_id.clone()
            };
            let entry = latest_snaps.entry(key).or_insert(s.clone());
            if s.snapshot_date >= entry.snapshot_date {
                *entry = s.clone();
            }
        }

        let mut results = Vec::new();
        let mut processed_keys = std::collections::HashSet::new();
        for asset in &config.assets {
            if let Some(s) = latest_snaps.get(&asset.asset_id) {
                let res = engine::reconciliation::reconcile_asset(&config, &portfolio_state, s);
                results.push(res);
                processed_keys.insert(asset.asset_id.clone());
            }
        }

        for (key, s) in latest_snaps {
            if !processed_keys.contains(&key) {
                let res = engine::reconciliation::reconcile_asset(&config, &portfolio_state, &s);
                results.push(res);
            }
        }

        Ok::<Vec<models::ReconciliationResult>, anyhow::Error>(results)
    }
    .await;

    match result {
        Ok(results) => {
            let mut result_rows = String::new();
            let mut warning_count = 0;
            let mut info_count = 0;

            for res in results {
                let diff_class = if res.market_value_diff.abs() > 50.0 {
                    if res.market_value_diff > 0.0 {
                        "text-up"
                    } else {
                        "text-down"
                    }
                } else {
                    ""
                };

                let status_badge = match res.status.as_str() {
                    "一致" => badge_status("一致"),
                    "需要校准" | "份额不一致" | "明显差异" => {
                        warning_count += 1;
                        "<span class='badge badge-red'>⚠️ 份额差异</span>".to_string()
                    }
                    "小幅差异" => {
                        info_count += 1;
                        "<span class='badge badge-orange'>小幅差异</span>".to_string()
                    }
                    "缺少系统持仓" => {
                        warning_count += 1;
                        "<span class='badge badge-gray'>未匹配资产</span>".to_string()
                    }
                    _ => format!("<span class='badge badge-gray'>{}</span>", res.status),
                };

                result_rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-weight: 700; color: var(--text-main);'>{}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted);'><code>{}</code></div>
                        </td>
                        <td style='font-size: 0.9rem;'>{}</td>
                        <td>
                            <div style='font-size: 0.9rem; font-weight: 600;'>账面: {:.2}</div>
                            <div style='font-size: 0.9rem; color: var(--text-muted);'>支付: {:.2}</div>
                        </td>
                        <td class='{}'>
                            <div style='font-weight: 700;'>{:+.2}</div>
                            <div style='font-size: 0.8rem;'>{:+.2}%</div>
                        </td>
                        <td>{}</td>
                        <td>{}</td>
                        <td><div style='font-weight: 600; font-size: 0.9rem;'>{}</div></td>
                    </tr>",
                    res.fund_name, res.fund_code, res.snapshot_date,
                    res.system_market_value, res.alipay_market_value,
                    diff_class, res.market_value_diff, res.market_value_diff_pct * 100.0,
                    res.alipay_units.map(|u| format!("{:.2}", u)).unwrap_or_else(|| {
                        if res.snapshot_date == "-" || res.snapshot_date.is_empty() {
                            "无快照数据".to_string()
                        } else {
                            if res.alipay_market_value > 0.0 && res.alipay_units.is_none() {
                                "截图未提供份额".to_string()
                            } else {
                                "无份额".to_string()
                            }
                        }
                    }),
                    status_badge, res.suggested_action
                ));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 24px;">
                    <div>
                        <h1 style="margin-bottom: 4px;">支付宝对账与校准 (Reconciliation)</h1>
                        <p style="color: var(--text-muted); font-size: 0.95rem; margin: 0;">对比系统账面记录与支付宝侧实测数据，识别并修复数据不一致</p>
                    </div>
                    <div class="action-group" style="margin-top: 0;">
                        <a href="/alipay/holdings" class="btn btn-outline btn-sm">导入最新截图</a>
                        <a href="/admin/reconcile" class="btn btn-sm">去执行手动校准 &rarr;</a>
                    </div>
                </div>

                <div class="dashboard-grid">
                    <div class="card" style="border-left: 4px solid var(--up-color);">
                        <div class="card-header"><span class="card-title">关键差异</span></div>
                        <div class="card-value text-up">{}</div>
                        <div class="card-sub">账面记录严重冲突</div>
                    </div>
                    <div class="card" style="border-left: 4px solid var(--warn-color);">
                        <div class="card-header"><span class="card-title">对账预警</span></div>
                        <div class="card-value text-warn">{}</div>
                        <div class="card-sub">支付宝快照与账面不符</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">普通提示</span></div>
                        <div class="card-value">{}</div>
                        <div class="card-sub">建议同步或补全快照</div>
                    </div>
                </div>

                <div class="table-container">
                    <div class="table-wrap">
                        <table>
                            <thead>
                                <tr>
                                    <th>资产名称 / 代码</th>
                                    <th>快照日期</th>
                                    <th>市值对比 (系统/支付)</th>
                                    <th>数值偏差</th>
                                    <th>支付宝份额</th>
                                    <th>状态</th>
                                    <th>建议操作</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>
                </div>
                "#,
                0, warning_count, info_count, result_rows
            );

            layout("对账校准", content)
        }
        Err(e) => layout(
            "对账校准",
            format!(
                "<div class='message-banner message-error'>对账数据计算失败: {}</div>",
                e
            ),
        ),
    }
}
async fn system_reconcile_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let transactions = state.repo.load_transactions(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let config = state.repo.load_config(&ctx).await?;
        let snapshots = state.repo.load_alipay_snapshots(&ctx).await.unwrap_or_default();
        let mut report = engine::portfolio_reconciliation::reconcile_portfolio(&ctx.portfolio_id, &portfolio_state, &transactions);

        let mut latest_snaps = std::collections::HashMap::new();
        for s in &snapshots {
            let key = if s.asset_id.is_empty() { format!("unmatched_{}", s.fund_code) } else { s.asset_id.clone() };
            let entry = latest_snaps.entry(key).or_insert(s.clone());
            if s.snapshot_date >= entry.snapshot_date { *entry = s.clone(); }
        }

        for asset in &config.assets {
            if !asset.enabled { continue; }
            if asset.sector.is_empty() || asset.sector == "未分类" {
                report.issues.push(models::ReconciliationIssue::UnclassifiedAsset {
                    asset_id: asset.asset_id.clone(),
                    severity: models::IssueSeverity::Info,
                });
            }
            if let Some(s) = latest_snaps.get(&asset.asset_id) {
                let res = engine::reconciliation::reconcile_asset(&config, &portfolio_state, s);
                if res.status == "需要校准" || res.status == "份额不一致" || res.status == "明显差异" {
                    report.issues.push(models::ReconciliationIssue::AlipayMismatch {
                        asset_id: asset.asset_id.clone(),
                        description: format!("支付宝与系统存在严重差异 (状态: {})", res.status),
                        severity: models::IssueSeverity::Warning,
                    });
                }
            } else {
                report.issues.push(models::ReconciliationIssue::MissingSnapshot {
                    asset_id: asset.asset_id.clone(),
                    severity: models::IssueSeverity::Info,
                });
            }
        }
        
        report.summary.total_issues = report.issues.len();
        report.summary.critical_issues = report.issues.iter().filter(|i| i.severity() == models::IssueSeverity::Critical).count();
        report.summary.warning_issues = report.issues.iter().filter(|i| i.severity() == models::IssueSeverity::Warning).count();
        
        Ok::<models::ReconciliationReport, anyhow::Error>(report)
    }.await;

    match result {
        Ok(report) => {
            let mut issue_rows = String::new();
            
            // Group issues by category
            let mut cash_issues = Vec::new();
            let mut ledger_issues = Vec::new();
            let mut metadata_issues = Vec::new();
            let mut alipay_issues = Vec::new();

            for issue in &report.issues {
                match issue {
                    models::ReconciliationIssue::CashMismatch { .. } => cash_issues.push(issue),
                    models::ReconciliationIssue::HoldingMismatch { .. } | models::ReconciliationIssue::NegativeQuantity { .. } | models::ReconciliationIssue::UnknownTransactionType { .. } | models::ReconciliationIssue::DateOutOfRange { .. } | models::ReconciliationIssue::DuplicateTransactionIssue { .. } | models::ReconciliationIssue::SuspiciousTransactionIssue { .. } => ledger_issues.push(issue),
                    models::ReconciliationIssue::UnclassifiedAsset { .. } | models::ReconciliationIssue::MissingPriceOrNav { .. } => metadata_issues.push(issue),
                    models::ReconciliationIssue::AlipayMismatch { .. } | models::ReconciliationIssue::MissingSnapshot { .. } => alipay_issues.push(issue),
                    _ => ledger_issues.push(issue),
                }
            }

            let config_res = async { state.repo.load_config(&ctx).await }.await;
            let config = config_res.unwrap_or_default();

            fn render_issue_group(title: &str, issues: &[&models::ReconciliationIssue], config: &models::ConfigRoot) -> String {
                if issues.is_empty() { return String::new(); }
                let mut html = format!("<tr><td colspan='4' style='background: #F8FAFC; font-weight: 800; font-size: 0.8rem; color: var(--text-muted); padding: 8px 20px;'>{}</td></tr>", title);
                for issue in issues {
                    let (severity_text, icon, color_class) = match issue.severity() {
                        models::IssueSeverity::Critical => ("严重", "🔴", "badge-red"),
                        models::IssueSeverity::Warning => ("警告", "🟠", "badge-orange"),
                        models::IssueSeverity::Info => ("提示", "🔵", "badge-blue"),
                    };
                    
                    let detail = match issue {
                        models::ReconciliationIssue::CashMismatch { currency, expected, actual, .. } => 
                            format!("现金 <strong>{}</strong> 不匹配: 账面 {:.2} vs 系统 {:.2}", currency, expected, actual),
                        models::ReconciliationIssue::HoldingMismatch { asset_id, expected, actual, .. } => {
                            let name = config.assets.iter().find(|a| a.asset_id == *asset_id).map(|a| a.fund_name.as_str()).unwrap_or(asset_id);
                            format!("份额不匹配: <strong>{}</strong> 账面 {:.4} vs 实际 {:.4}", name, expected, actual)
                        },
                        models::ReconciliationIssue::AlipayMismatch { asset_id, description, .. } => {
                            let name = config.assets.iter().find(|a| a.asset_id == *asset_id).map(|a| a.fund_name.as_str()).unwrap_or(asset_id);
                            format!("支付宝不符: <strong>{}</strong> - {}", name, description)
                        },
                        models::ReconciliationIssue::UnclassifiedAsset { asset_id, .. } => {
                            let name = config.assets.iter().find(|a| a.asset_id == *asset_id).map(|a| a.fund_name.as_str()).unwrap_or(asset_id);
                            format!("缺少分类: 资产 <strong>{}</strong> 未设置赛道", name)
                        },
                        models::ReconciliationIssue::MissingPriceOrNav { asset_id, .. } => {
                            let name = config.assets.iter().find(|a| a.asset_id == *asset_id).map(|a| a.fund_name.as_str()).unwrap_or(asset_id);
                            format!("行情缺失: <strong>{}</strong> 缺少最新价格", name)
                        },
                        _ => format!("{:?}", issue),
                    };

                    let action = match issue {
                        models::ReconciliationIssue::CashMismatch { .. } => "<a href='/cash' class='btn-ghost'>修正现金 &rarr;</a>",
                        models::ReconciliationIssue::AlipayMismatch { .. } | models::ReconciliationIssue::HoldingMismatch { .. } => "<a href='/reconcile/alipay' class='btn-ghost'>去对账校准 &rarr;</a>",
                        models::ReconciliationIssue::UnclassifiedAsset { .. } => "<a href='/admin/assets?filter=unclassified' class='btn-ghost'>设置分类 &rarr;</a>",
                        models::ReconciliationIssue::MissingPriceOrNav { .. } => "<button onclick='refreshMarket(this)' class='btn-ghost'>刷新行情</button>",
                        _ => "<span class='text-muted'>需手动检查</span>",
                    };

                    html.push_str(&format!(
                        "<tr>
                            <td style='width: 40px; text-align: center;'>{}</td>
                            <td style='width: 100px;'><span class='badge {}'>{}</span></td>
                            <td>{}</td>
                            <td class='text-right'>{}</td>
                        </tr>",
                        icon, color_class, severity_text, detail, action
                    ));
                }
                html
            }

            issue_rows.push_str(&render_issue_group("💸 现金与流水 (Cash)", &cash_issues, &config));
            issue_rows.push_str(&render_issue_group("📋 内部账本 (Ledger)", &ledger_issues, &config));
            issue_rows.push_str(&render_issue_group("📱 支付宝同步 (Alipay)", &alipay_issues, &config));
            issue_rows.push_str(&render_issue_group("🏷️ 元数据 (Metadata)", &metadata_issues, &config));

            if issue_rows.is_empty() {
                issue_rows = "<tr><td colspan='4' style='text-align: center; padding: 80px 24px;'><div style='font-size: 3rem; margin-bottom: 16px;'>✅</div><div style='font-weight: 700; color: var(--down-color);'>组合对账通过，数据完全一致。</div></td></tr>".to_string();
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 32px;">
                    <div>
                        <h1 style="margin-bottom: 4px;">对账中心 (Reconciliation)</h1>
                        <p style="color: var(--text-muted); font-size: 0.95rem; margin: 0;">自动审计流水、持仓与外部账本的一致性</p>
                    </div>
                    <div>
                        <a href="/reconcile/alipay" class="btn btn-outline">⚖️ 支付宝快照对账 &rarr;</a>
                    </div>
                </div>

                <div class="dashboard-grid">
                    <div class="card" style="border-left: 4px solid var(--up-color);">
                        <div class="card-header"><span class="card-title">待处理异常</span></div>
                        <div class="card-value {}">{}</div>
                        <div class="card-sub">严重: {}, 警告: {}</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">受影响资产</span></div>
                        <div class="card-value">{}</div>
                        <div class="card-sub">个标的存在记录偏差</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">审计深度</span></div>
                        <div class="card-value">{}</div>
                        <div class="card-sub">条已检查的历史交易</div>
                    </div>
                </div>

                <div class="table-container">
                    <div class="table-wrap">
                        <table>
                            <thead>
                                <tr>
                                    <th style='width: 40px;'></th>
                                    <th style='width: 100px;'>级别</th>
                                    <th>异常说明</th>
                                    <th class='text-right'>建议操作</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>
                </div>

                <div class="card" style="background: #FFF; border: 1px solid #FFE7BA; border-left: 4px solid var(--warn-color);">
                    <h3 style="margin-top: 0; color: #996000;">📖 对账指引</h3>
                    <div style="font-size: 0.9rem; color: #666; line-height: 1.8;">
                        • <strong>份额差异:</strong> 通常因为漏记了定投或手动调仓。建议从支付宝导出最新持仓进行一次快照对账。<br>
                        • <strong>现金差异:</strong> 请检查是否有未录入的分红现金入账，或手续费计算不准。<br>
                        • <strong>重复指纹:</strong> 导入 CSV 时可能重复提交了相同日期的记录，可在“交易明细”中删除重复项。
                    </div>
                </div>
                "#,
                if report.summary.critical_issues > 0 { "text-up" } else { "" },
                report.summary.total_issues,
                report.summary.critical_issues, report.summary.warning_issues,
                report.summary.affected_assets.len(),
                report.summary.total_transactions_checked,
                issue_rows
            );

            layout("对账中心", content)
        }
        Err(e) => layout("对账中心", format!("<div class='message-banner message-error'>对账计算失败: {}</div>", e)),
    }
}

async fn api_reconciliation_report_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::ReconciliationReport> {
    let ctx = RepositoryContext::default();
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

async fn api_daily_run_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::DailyOperationResult> {
    let ctx = RepositoryContext::default();

    // Set status to running immediately
    {
        let mut status = state.refresh_status.write().await;
        let report = models::DailyOperationReport {
            date: Local::now().format("%Y-%m-%d").to_string(),
            started_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            completed_at: None,
            status: models::DailyOperationStatus::Running,
            steps: Vec::new(),
            plan: None,
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        status.latest_daily_report = Some(report);
    }

    let result = engine::daily_operation::run_daily_operation(state.repo.as_ref(), &ctx).await;

    match result {
        Ok(report) => {
            let mut status = state.refresh_status.write().await;
            status.latest_daily_report = Some(report.clone());
            Json(models::DailyOperationResult {
                success: report.status != models::DailyOperationStatus::Failed,
                message: "Daily operation completed".to_string(),
            })
        }
        Err(e) => Json(models::DailyOperationResult {
            success: false,
            message: format!("Pipeline error: {}", e),
        }),
    }
}

async fn api_daily_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Option<models::DailyOperationReport>> {
    let status = state.refresh_status.read().await;
    Json(status.latest_daily_report.clone())
}

async fn api_daily_report_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Option<models::DailyOperationReport>> {
    let status = state.refresh_status.read().await;
    Json(status.latest_daily_report.clone())
}

async fn instruments_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let cache_status = state.repo.load_cache_status(&ctx).await.unwrap_or_default();
        let instruments = state.repo.load_instruments(&ctx).await?;
        let market_cache = state.repo.load_market_cache(&ctx).await.unwrap_or_default();
        Ok::<
            (
                models::ConfigRoot,
                models::CacheStatusRegistry,
                Vec<models::InstrumentConfig>,
                models::MarketCache,
            ),
            anyhow::Error,
        >((config, cache_status, instruments, market_cache))
    }
    .await;

    match result {
        Ok((config, cache, instruments, market_cache)) => {
            let mut inst_rows = String::new();
            let cache_map: std::collections::HashMap<String, &models::MarketCacheEntry> =
                market_cache
                    .entries
                    .iter()
                    .map(|e| (e.symbol.clone(), e))
                    .collect();

            for inst in instruments {
                let quote = cache_map.get(&inst.symbol);
                let price_html = if let Some(q) = quote {
                    format!(
                        "<div style='font-weight: 800;' class='tabular'>{}</div><div style='font-size: 0.7rem; color: var(--text-muted);'>{}</div>",
                        q.price, q.date
                    )
                } else {
                    "<span class='text-muted'>暂无价格</span>".to_string()
                };

                let status_badge = if inst.enabled {
                    "<span class='badge badge-blue'>监控中</span>"
                } else {
                    "<span class='badge badge-gray'>未启用</span>"
                };

                inst_rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-weight: 700;'>{}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted);'><code>{}</code></div>
                        </td>
                        <td><span class='badge badge-outline'>{:?}</span></td>
                        <td>{}</td>
                        <td>{}</td>
                        <td class='tabular'>{}</td>
                        <td class='text-right'>{}</td>
                    </tr>",
                    inst.name_zh.as_deref().unwrap_or(&inst.instrument_id),
                    inst.symbol,
                    inst.asset_class,
                    inst.provider,
                    inst.currency,
                    price_html,
                    status_badge
                ));
            }

            let last_refresh = cache.last_market_update.as_deref().unwrap_or("从未刷新");

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 32px;">
                    <div>
                        <h1 style="margin-bottom: 4px;">市场观察哨 (Market Watchlist)</h1>
                        <p style="color: var(--text-muted); font-size: 0.95rem; margin: 0;">实时跟踪全球指数、汇率及大宗商品行情</p>
                    </div>
                    <div class="action-group" style="margin-top: 0;">
                        <button onclick="refreshMarket(this)" class="btn">📈 刷新全部行情</button>
                        <a href="/admin/instruments" class="btn btn-outline">⚙️ 管理标的</a>
                    </div>
                </div>

                <div class="dashboard-grid">
                    <div class="card">
                        <div class="card-header"><span class="card-title">最近同步</span></div>
                        <div class="card-value" style="font-size: 1.25rem; color: var(--primary-color);">{}</div>
                        <div class="card-sub">行情中心更新时间</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">缓存深度</span></div>
                        <div class="card-value">{}</div>
                        <div class="card-sub">个有效的价格切片</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">监控标的</span></div>
                        <div class="card-value">{}</div>
                        <div class="card-sub">已启用的市场仪器</div>
                    </div>
                </div>

                <div class="table-container">
                    <div class="table-wrap">
                        <table>
                            <thead>
                                <tr>
                                    <th>标的名称 / 代码</th>
                                    <th>资产类型</th>
                                    <th>行情源</th>
                                    <th>基准币种</th>
                                    <th>最新价格</th>
                                    <th class="text-right">状态</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>
                </div>

                <div class="card" style="background: #F8F9FA; border-style: dashed;">
                    <h3 style="margin-top: 0;">💡 行情说明</h3>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.6; margin-bottom: 0;">
                        系统自动从 Yahoo Finance / 东财等接口获取延迟行情。行情数据主要用于 Kelly 计算、赛道估值调整及 QDII 溢价参考。<br>
                        如需新增标的或修改映射，请前往“标的管理”页面。
                    </p>
                </div>
                "#,
                last_refresh,
                cache.market_cache_size,
                config.assets.len(),
                inst_rows
            );

            layout("市场行情", content)
        }
        Err(e) => layout(
            "市场行情",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
    }
}

async fn dca_lifecycle_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let date = Local::now().format("%Y-%m-%d").to_string();

        let dca_plans = state.repo.load_plans(&ctx).await?;
        let settlements = state.repo.load_settlements(&ctx).await?;
        let snapshots = state.repo.load_alipay_snapshots(&ctx).await?;
        let nav_cache = state.repo.load_nav_cache(&ctx).await?;

        let summary = engine::dca_lifecycle::calculate_dca_lifecycle(
            &config,
            &dca_plans,
            &settlements,
            &snapshots,
            &portfolio_state,
            &nav_cache,
            &date,
        );

        Ok::<models::DcaLifecycleSummary, anyhow::Error>(summary)
    }
    .await;

    match result {
        Ok(summary) => {
            let mut item_rows = String::new();
            for item in summary.items {
                let status_badge = badge_status(&item.lifecycle_status);
                let action_class = if item.suggested_next_action == "无需处理" {
                    "badge-gray"
                } else {
                    "badge-blue"
                };

                item_rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-weight: 700; color: var(--text-main); font-size: 1.05rem;'>{}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted); margin-top: 2px;'><code>{}</code></div>
                        </td>
                        <td style='font-weight: 700; font-size: 1.1rem; font-family: DIN Alternate, Helvetica Neue;'>{:.2}</td>
                        <td style='font-weight: 700; font-size: 1.1rem;'>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td><span class='badge {}'>{}</span></td>
                    </tr>",
                    item.fund_name,
                    item.asset_id,
                    item.planned_amount,
                    item.settlement_amount
                        .map(|a| format!("{:.2}", a))
                        .unwrap_or_else(|| "-".to_string()),
                    status_badge,
                    badge_status(&item.reconciliation_status),
                    action_class,
                    item.suggested_next_action
                ));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 24px; background: #FFF; padding: 20px; border-radius: 12px; border: 1px solid var(--border-color); box-shadow: var(--shadow);">
                    <div>
                        <h1 style="margin-bottom: 4px;">定投闭环监控 (DCA Lifecycle)</h1>
                        <p style="color: var(--text-muted); font-size: 0.9rem; margin: 0;">监控从扣款到入账对账的全生命周期状态</p>
                    </div>
                    <div style="text-align: right;">
                        <div style="font-size: 0.85rem; color: var(--text-muted); font-weight: 600;">监控日期</div>
                        <div style="font-size: 1.2rem; font-weight: 700; color: var(--text-main);">{}</div>
                    </div>
                </div>

                <div class="dashboard-grid">
                    <div class="card">
                        <div class="card-header"><span class="card-title">今日定投计划</span></div>
                        <div class="card-value text-up">{:.2} <small style="font-size: 0.9rem; font-weight: 500; opacity: 0.8;">CNY</small></div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">待处理 (确认/入账)</span></div>
                        <div class="card-value">{} <small style="font-size: 1rem; color: var(--text-muted); font-weight: 400;">/ {}</small></div>
                        <div class="card-sub">确认单项 / 待入账项</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">异常关注项</span></div>
                        <div class="card-value text-warn">{}</div>
                        <div class="card-sub">需人工介入核对项</div>
                    </div>
                </div>

                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>基金名称 / ID</th>
                                <th>计划金额</th>
                                <th>确认金额</th>
                                <th>生命周期状态</th>
                                <th>对账状态</th>
                                <th>建议下一步动作</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="card" style="background-color: #F7F8FA; border: 1px dashed var(--border-color); padding: 20px;">
                    <h3 style="margin-top: 0;">ℹ 定投闭环生命周期说明</h3>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.6; margin-bottom: 0;">
                        定投闭环追踪从 <strong>计划 -> 扣款 -> 确认 -> 入账 -> 对账</strong> 的全过程。<br>
                        1. <strong>计划:</strong> 根据定投规则生成的今日扣款项。<br>
                        2. <strong>确认:</strong> 录入定投确认单后，状态变为“已确认待入账”。<br>
                        3. <strong>入账:</strong> 确认单应用后，份额增加，状态变为“已入账待对账”。<br>
                        4. <strong>对账:</strong> 录入支付宝持仓快照，若系统与支付宝份额一致，则完成最终闭环。
                    </p>
                </div>
                "#,
                summary.date,
                summary.total_planned_amount,
                summary.count_waiting_confirmation,
                summary.count_unapplied,
                summary.count_attention_required,
                item_rows
            );

            layout("定投闭环", content)
        }
        Err(e) => layout(
            "定投闭环",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
    }
}

async fn ops_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
        let date = Local::now().format("%Y-%m-%d").to_string();

        let dca_plans = state.repo.load_plans(&ctx).await?;
        let settlements = state.repo.load_settlements(&ctx).await?;
        let snapshots = state.repo.load_alipay_snapshots(&ctx).await?;
        let nav_cache = state.repo.load_nav_cache(&ctx).await?;

        let lifecycle = engine::dca_lifecycle::calculate_dca_lifecycle(
            &config,
            &dca_plans,
            &settlements,
            &snapshots,
            &portfolio_state,
            &nav_cache,
            &date,
        );

        let cache_status = state.repo.load_cache_status(&ctx).await.unwrap_or_default();
        let risk_cache = state.repo.load_risk_cache(&ctx).await.unwrap_or(None);

        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date.clone());
        let risk_overlay =
            risk_cache
                .map(|rc| rc.overlay)
                .unwrap_or_else(|| models::GlobalRiskOverlay {
                    risk_score: 0.0,
                    risk_label: "未知".to_string(),
                    factor_results: vec![],
                    warnings: vec![],
                    explanation: "请运行 ops refresh".to_string(),
                });

        Ok::<
            (
                models::PortfolioSummary,
                models::DcaLifecycleSummary,
                models::CacheStatusRegistry,
                engine::decision::DecisionResult,
                models::GlobalRiskOverlay,
            ),
            anyhow::Error,
        >((summary, lifecycle, cache_status, decision, risk_overlay))
    }
    .await;

    match result {
        Ok((summary, lifecycle, cache_status, decision, risk_overlay)) => {
            let mut cache_html = String::new();
            let keys = vec!["fund", "market", "risk", "instrument", "proxy"];
            for key in keys {
                let status = cache_status.statuses.iter().find(|s| s.key == key);
                let (status_text, color) = match status {
                    Some(s) if s.status == "正常" => ("正常", "badge-green"),
                    Some(s) => (s.status.as_str(), "badge-orange"),
                    None => ("缺失", "badge-red"),
                };
                cache_html.push_str(&format!(
                    "<span class='badge {}' style='margin-right: 4px; margin-bottom: 4px;'>{}: {}</span>",
                    color, key, status_text
                ));
            }

            let pending_items: Vec<_> = lifecycle
                .items
                .iter()
                .filter(|i| i.suggested_next_action != "无需处理" && i.lifecycle_status != "已暂停")
                .collect();

            let mut next_steps: Vec<String> = Vec::new();
            if cache_status.statuses.iter().any(|s| s.status != "正常")
                || cache_status.statuses.is_empty()
            {
                next_steps.push(
                    "运行 <code>cargo run -- data refresh --all</code> 刷新行情。".to_string(),
                );
            }
            if !pending_items.is_empty() {
                next_steps.push(format!(
                    "处理 <strong>{}</strong> 项待办定投事项（录入确认单或支付宝快照）。",
                    pending_items.len()
                ));
            }
            if lifecycle.count_due > 0 {
                next_steps.push("确认今日执行计划并执行手动买入。".to_string());
            }

            let next_steps_html = if next_steps.is_empty() {
                "<div class='text-down' style='font-weight: 700;'>[✓] 组合状态良好，暂无建议动作。</div>".to_string()
            } else {
                format!(
                    "<ul style='margin: 0; padding-left: 18px; color: #4E5969;'>{}</ul>",
                    next_steps
                        .iter()
                        .map(|s| format!("<li style='margin-bottom: 10px;'>{}</li>", s))
                        .collect::<String>()
                )
            };

            let content = format!(
                r#"
                <div class="card" style="background: linear-gradient(135deg, #0052D9 0%, #003EB3 100%); color: white; border: none; padding: 24px;">
                    <div style="opacity: 0.8; font-size: 0.95rem; margin-bottom: 8px; font-weight: 500;">总资产市值 (Portfolio Value)</div>
                    <div style="font-size: 2.5rem; font-weight: 900; letter-spacing: -1px; margin-bottom: 16px;">{:.2} <small style="font-size: 1rem; font-weight: 500; opacity: 0.8;">CNY</small></div>
                    <div style="display: flex; gap: 24px; font-size: 0.95rem; opacity: 0.95; border-top: 1px solid rgba(255,255,255,0.15); padding-top: 16px;">
                        <div>可用现金: <strong style="font-size: 1.1rem;">{:.2}</strong></div>
                        <div>权益仓位: <strong style="font-size: 1.1rem;">{:.2}%</strong></div>
                        <div>权益缺口: <strong style="font-size: 1.1rem;">{:.2}</strong></div>
                    </div>
                </div>

                <div class="dashboard-grid">
                    <div class="card">
                        <div class="card-header">
                            <span class="card-title">今日建议买入</span>
                            <a href="/daily" style="font-size: 0.8rem; text-decoration: none; color: var(--primary-color); font-weight: 600;">去执行 &rarr;</a>
                        </div>
                        <div class="card-value text-up">{:.2}</div>
                        <div class="card-sub">包含定投应投 {:.2}</div>
                    </div>
                    <div class="card">
                        <div class="card-header">
                            <span class="card-title">待处理事项</span>
                            <span class="badge {}">{} 项</span>
                        </div>
                        <div class="card-value">{}</div>
                        <div class="card-sub">定投生命周期状态</div>
                    </div>
                    <div class="card">
                        <div class="card-header">
                            <span class="card-title">全局风险状态</span>
                        </div>
                        <div class="card-value">{}</div>
                        <div class="card-sub">风险分数: {:.1} / 100</div>
                    </div>
                    <div class="card">
                        <div class="card-header">
                            <span class="card-title">数据刷新状态</span>
                        </div>
                        <div style="margin-top: 8px; display: flex; flex-wrap: wrap;">{}</div>
                    </div>
                </div>

                <div style="display: grid; grid-template-columns: 1.6fr 1fr; gap: 20px;">
                    <div>
                        <h2>今日执行摘要 (Daily Summary)</h2>
                        <div class="card" style="padding: 0; overflow: hidden;">
                            <table style="min-width: unset;">
                                <tbody>
                                    <tr>
                                        <td style="color: var(--text-muted); font-weight: 500;">今日应定投笔数</td>
                                        <td style="text-align: right; font-weight: 700; font-size: 1.1rem;">{} 笔</td>
                                    </tr>
                                    <tr>
                                        <td style="color: var(--text-muted); font-weight: 500;">权益资产目标</td>
                                        <td style="text-align: right; font-weight: 700; font-size: 1.1rem;">{:.2} CNY</td>
                                    </tr>
                                    <tr>
                                        <td style="color: var(--text-muted); font-weight: 500;">预留现金储备</td>
                                        <td style="text-align: right; font-weight: 700; font-size: 1.1rem;">{:.2} CNY</td>
                                    </tr>
                                    <tr>
                                        <td style="color: var(--text-muted); font-weight: 500;">现金可用余额</td>
                                        <td style="text-align: right; font-weight: 700; font-size: 1.1rem;">{:.2} CNY</td>
                                    </tr>
                                </tbody>
                            </table>
                        </div>

                        <h2>待处理事项 (Action Items)</h2>
                        <div class="table-container">
                            <table style="min-width: unset;">
                                <thead>
                                    <tr>
                                        <th>资产/标的</th>
                                        <th>当前生命周期</th>
                                        <th>建议动作</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {}
                                </tbody>
                            </table>
                        </div>
                    </div>

                    <div>
                        <h2>下一步建议 (Next Steps)</h2>
                        <div class="card" style="background-color: #FFF7E8; border: 1px solid #FFE4BA; padding: 24px;">
                            {}
                        </div>

                        <h2>快捷入口 (Quick Access)</h2>
                        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
                            <a href="/admin/reconcile" class="btn btn-outline" style="padding: 16px 8px; font-size: 0.9rem;">
                                ⚖ 录入持仓快照
                            </a>
                            <a href="/admin/dca-settlements" class="btn btn-outline" style="padding: 16px 8px; font-size: 0.9rem;">
                                📝 录入定投确认
                            </a>
                            <a href="/reports" class="btn btn-outline" style="padding: 16px 8px; font-size: 0.9rem;">
                                📊 查看复盘报告
                            </a>
                            <a href="/holdings" class="btn btn-outline" style="padding: 16px 8px; font-size: 0.9rem;">
                                💰 持仓明细管理
                            </a>
                        </div>
                    </div>
                </div>
                "#,
                summary.total_asset_value,
                summary.available_cash,
                summary.equity_value / summary.total_asset_value * 100.0,
                summary.equity_gap,
                decision.suggested_total_buy,
                lifecycle.total_planned_amount,
                if pending_items.is_empty() {
                    "badge-green"
                } else {
                    "badge-orange"
                },
                pending_items.len(),
                if pending_items.is_empty() {
                    "待办已清"
                } else {
                    "需处理"
                },
                badge_risk(&risk_overlay.risk_label),
                risk_overlay.risk_score,
                cache_html,
                lifecycle.count_due,
                summary.target_equity_value,
                summary.reserve_cash,
                summary.available_cash,
                if pending_items.is_empty() {
                    "<tr><td colspan='3' style='text-align: center; color: var(--text-muted); padding: 32px; font-weight: 500;'>[✓] 所有定投项已闭环</td></tr>".to_string()
                } else {
                    pending_items.iter().map(|i| {
                        format!("<tr><td style='font-weight: 600;'>{}</td><td>{}</td><td><span class='text-warn'>{}</span></td></tr>", 
                            i.asset_id, badge_status(&i.lifecycle_status), i.suggested_next_action)
                    }).collect::<String>()
                },
                next_steps_html
            );

            layout("操作台", content)
        }
        Err(e) => layout(
            "操作台",
            format!(
                "<div class='message-banner message-error'>加载操作台失败: {}</div>",
                e
            ),
        ),
    }
}

#[derive(Deserialize)]
struct AdminQuery {
    success: Option<String>,
    error: Option<String>,
    filter: Option<String>,
}

async fn admin_handler(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<AdminQuery>,
) -> Html<String> {
    let content = r#"
        <div class="message-banner message-error" style="background: #FFF7E8; color: #996000; border-color: #FFE4BA; text-align: center; font-weight: 700;">
            ⚠️ 安全警告：Web 管理功能仅建议在本机 127.0.0.1 使用，请不要暴露到公网。
        </div>

        <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 24px; background: #FFF; padding: 20px; border-radius: 12px; border: 1px solid var(--border-color); box-shadow: var(--shadow);">
            <div>
                <h1 style="margin-bottom: 4px;">系统管理后台 (Admin Console)</h1>
                <p style="color: var(--text-muted); font-size: 0.9rem; margin: 0;">维护组合配置、录入成交数据与执行对账校准</p>
            </div>
            <div>
                <span class="badge badge-outline" style="color: var(--warn-color); border-color: var(--warn-color); font-weight: 700; padding: 4px 12px;">LOCAL ONLY</span>
            </div>
        </div>
        
        <div class="dashboard-grid">
            <div class="card" style="display: flex; flex-direction: column; justify-content: space-between;">
                <div>
                    <div class="card-header"><span class="card-title" style="font-size: 1.1rem;">⚖ 支付宝对账录入</span></div>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.5;">录入支付宝持仓快照，与系统份额进行比对。发现差异后可一键校准持仓数据。</p>
                </div>
                <div style="margin-top: 16px;">
                    <a href="/admin/reconcile" class="btn" style="width: 100%;">进入对账录入 &rarr;</a>
                </div>
            </div>
            <div class="card" style="display: flex; flex-direction: column; justify-content: space-between;">
                <div>
                    <div class="card-header"><span class="card-title" style="font-size: 1.1rem;">📝 定投确认单录入</span></div>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.5;">录入定投真实扣款与确认份额记录。录入后份额将正式计入系统资产。</p>
                </div>
                <div style="margin-top: 16px;">
                    <a href="/admin/dca-settlements" class="btn" style="width: 100%;">进入确认录入 &rarr;</a>
                </div>
            </div>
            <div class="card" style="display: flex; flex-direction: column; justify-content: space-between;">
                <div>
                    <div class="card-header"><span class="card-title" style="font-size: 1.1rem;">🔄 定投计划管理</span></div>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.5;">新增或修改自动定投计划，设置定投金额、频率（日/周/月）及启用状态。</p>
                </div>
                <div style="margin-top: 16px;">
                    <a href="/admin/dca" class="btn" style="width: 100%;">管理定投计划 &rarr;</a>
                </div>
            </div>
            <div class="card" style="display: flex; flex-direction: column; justify-content: space-between;">
                <div>
                    <div class="card-header"><span class="card-title" style="font-size: 1.1rem;">💰 组合资产配置</span></div>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.5;">管理您的资产池，修改基金代码、显示名称、所属赛道及是否启用等。</p>
                </div>
                <div style="margin-top: 16px;">
                    <a href="/admin/assets" class="btn" style="width: 100%;">管理资产配置 &rarr;</a>
                </div>
            </div>
            <div class="card" style="display: flex; flex-direction: column; justify-content: space-between;">
                <div>
                    <div class="card-header"><span class="card-title" style="font-size: 1.1rem;">📈 证券标的数据</span></div>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.5;">配置行情源标的元数据，包括中文名、资产类别标签及行情刷新状态。</p>
                </div>
                <div style="margin-top: 16px;">
                    <a href="/admin/instruments" class="btn" style="width: 100%;">管理行情标的 &rarr;</a>
                </div>
            </div>
            <div class="card" style="display: flex; flex-direction: column; justify-content: space-between;">
                <div>
                    <div class="card-header"><span class="card-title" style="font-size: 1.1rem;">📊 报告查阅</span></div>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.5;">查阅系统生成的各类报告，包括每日总结、每周分析及月度报告。</p>
                </div>
                <div style="margin-top: 16px; display: flex; gap: 8px;">
                    <a href="/reports/daily" class="btn btn-sm" style="flex: 1;">日报</a>
                    <a href="/reports/weekly" class="btn btn-sm" style="flex: 1;">周报</a>
                    <a href="/reports/monthly" class="btn btn-sm" style="flex: 1;">月报</a>
                </div>
            </div>
        </div>
    "#
    .to_string();
    layout_with_msg("管理面板", content, query.success, query.error)
}

async fn admin_audit_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = state.repo.load_web_admin_audit(&ctx).await;

    match result {
        Ok(log) => {
            let mut rows = String::new();
            for r in log.records.iter().rev().take(100) {
                // Show last 100
                rows.push_str(&format!(
                    "<tr>
                        <td><small>{}</small></td>
                        <td><span class='badge badge-blue'>{}</span></td>
                        <td><code>{}</code></td>
                        <td>{}</td>
                        <td><small class='text-muted'>{}</small></td>
                        <td><small class='text-muted'>{}</small></td>
                        <td>{}</td>
                    </tr>",
                    r.timestamp,
                    r.action,
                    r.target_id.as_deref().unwrap_or("-"),
                    r.status,
                    r.old_value_summary,
                    r.new_value_summary,
                    r.note.as_deref().unwrap_or("-")
                ));
            }

            let content = format!(
                r#"
                <div class="message-banner message-error" style="background: #FFF7E8; color: #996000; border-color: #FFE4BA; text-align: center; font-weight: 700; margin-bottom: 24px;">
                    ⚠️ 安全警告：Web 管理功能仅建议在本机 127.0.0.1 使用，请不要暴露到公网。
                </div>

                <div style="margin-bottom: 16px;">
                    <a href="/admin" class="btn btn-outline" style="padding: 8px 16px;">&larr; 返回管理面板</a>
                </div>

                <div style="display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 16px;">
                    <h1>操作审计记录 (Admin Audit Log)</h1>
                    <p style="color: var(--text-muted); font-size: 0.85rem;">展示最近 100 条通过 Web 界面进行的操作记录</p>
                </div>

                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>操作时间</th>
                                <th>动作类别</th>
                                <th>目标 ID</th>
                                <th>状态</th>
                                <th>修改前摘要</th>
                                <th>修改后摘要</th>
                                <th>备注说明</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>
                "#,
                rows
            );
            layout("审计记录", content)
        }
        Err(e) => layout(
            "审计记录",
            format!(
                "<div class='message-banner message-error'>加载审计记录失败: {}</div>",
                e
            ),
        ),
    }
}

async fn admin_reconcile_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminQuery>,
) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let snapshots = state.repo.load_alipay_snapshots(&ctx).await?;
        Ok::<
            (
                models::ConfigRoot,
                models::PortfolioState,
                Vec<models::AlipaySnapshot>,
            ),
            anyhow::Error,
        >((config, portfolio_state, snapshots))
    }
    .await;

    match result {
        Ok((config, portfolio_state, mut snapshots)) => {
            // Asset dropdown options
            let mut asset_options = String::new();
            for asset in &config.assets {
                if asset.enabled {
                    asset_options.push_str(&format!(
                        "<option value='{}'>{} ({})</option>",
                        asset.asset_id, asset.fund_name, asset.asset_id
                    ));
                }
            }

            // Compare result logic (similar to reconcile_handler)
            let mut compare_rows = String::new();
            snapshots.sort_by(|a, b| b.snapshot_date.cmp(&a.snapshot_date));

            // Only take the latest snapshot for each asset
            let mut latest_snapshots = std::collections::HashMap::new();
            for s in &snapshots {
                if !latest_snapshots.contains_key(&s.asset_id) {
                    latest_snapshots.insert(s.asset_id.clone(), s.clone());
                }
            }

            for asset in &config.assets {
                if !asset.enabled {
                    continue;
                }

                if let Some(snapshot) = latest_snapshots.get(&asset.asset_id) {
                    let res = engine::reconciliation::reconcile_asset(
                        &config,
                        &portfolio_state,
                        snapshot,
                    );

                    let status_badge = match res.status.as_str() {
                        "一致" => badge_status("一致"),
                        "小幅差异" => {
                            "<span class='badge badge-blue'>小幅差异</span>".to_string()
                        }
                        "明显差异" => {
                            "<span class='badge badge-orange'>明显差异</span>".to_string()
                        }
                        "需要校准" => {
                            "<span class='badge badge-orange'>需要校准</span>".to_string()
                        }
                        "份额不一致" => {
                            "<span class='badge badge-orange'>份额不一致</span>".to_string()
                        }
                        "成本不一致" => {
                            "<span class='badge badge-orange'>成本不一致</span>".to_string()
                        }
                        "净值日期不一致" => {
                            "<span class='badge badge-blue'>净值日期不一致</span>".to_string()
                        }
                        "缺少系统持仓" => {
                            "<span class='badge badge-orange'>缺少系统持仓</span>".to_string()
                        }
                        _ => format!("<span class='badge badge-gray'>{}</span>", res.status),
                    };

                    let suggest = engine::reconciliation::generate_calibration_suggestion(&res);
                    let action_html = if let Some(s) = suggest {
                        format!(
                            r#"<form action="/admin/reconcile/apply-confirm" method="POST" onsubmit="return confirm('确定要按此快照校准吗？');">
                                <input type="hidden" name="snapshot_id" value="{}">
                                <input type="hidden" name="confirm" value="true">
                                <button type="submit" class="btn btn-outline btn-success" style="padding: 4px 8px; font-size: 0.75rem;">执行校准</button>
                            </form>
                            <small style="display:block; color:var(--text-muted); margin-top:2px;">{}</small>"#,
                            s.snapshot_id, s.reason
                        )
                    } else {
                        "<span style='color: var(--text-muted); font-size: 0.8rem;'>无需操作</span>"
                            .to_string()
                    };

                    let diff_class = if res.market_value_diff.abs() > 1.0 {
                        if res.market_value_diff > 0.0 {
                            "text-up"
                        } else {
                            "text-down"
                        }
                    } else {
                        ""
                    };

                    compare_rows.push_str(&format!(
                        "<tr>
                            <td>
                                <div style='font-weight: 700; color: var(--text-main);'>{}</div>
                                <div style='font-size: 0.75rem; color: var(--text-muted);'><code>{}</code></div>
                            </td>
                            <td style='font-size: 0.85rem;'>{}</td>
                            <td>
                                <div style='font-size: 0.85rem;'>系统: {:.2}</div>
                                <div style='font-size: 0.85rem; color: var(--text-muted);'>支付: {:.2}</div>
                            </td>
                            <td class='{}'>
                                <div style='font-weight: 700;'>{:.2}</div>
                                <div style='font-size: 0.75rem;'>{:.2}%</div>
                            </td>
                            <td>{}</td>
                            <td>{}</td>
                        </tr>",
                        asset.fund_name,
                        asset.asset_id,
                        snapshot.snapshot_date,
                        res.system_market_value,
                        res.alipay_market_value,
                        diff_class,
                        res.market_value_diff,
                        res.market_value_diff_pct * 100.0,
                        status_badge,
                        action_html
                    ));
                } else {
                    compare_rows.push_str(&format!(
                        "<tr>
                            <td><code>{}</code></td>
                            <td>{}</td>
                            <td colspan='5' style='color: var(--text-muted); text-align: center;'>无最新快照</td>
                            <td><span style='color: var(--text-muted);'>-</span></td>
                        </tr>",
                        asset.asset_id, asset.fund_name
                    ));
                }
            }

            let content = format!(
                r#"
                <div class="message-banner message-error" style="background: #FFF7E8; color: #996000; border-color: #FFE4BA; text-align: center; font-weight: 700; margin-bottom: 24px;">
                    ⚠️ 安全警告：Web 管理功能仅建议在本机 127.0.0.1 使用，请不要暴露到公网。
                </div>

                <div style="margin-bottom: 16px;">
                    <a href="/admin" class="btn btn-outline" style="padding: 8px 16px;">&larr; 返回管理面板</a>
                </div>

                <h1>支付宝对账管理</h1>
                
                <div class="card" style="margin-bottom: 2rem;">
                    <h3>录入支付宝快照 (Add Snapshot)</h3>
                    <form action="/admin/reconcile/alipay/add" method="POST" style="display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 1rem; align-items: end;">
                        <div>
                            <label style="display: block; font-size: 0.8rem; margin-bottom: 0.25rem;">资产</label>
                            <select name="asset_id" style="width: 100%;" required>
                                {}
                            </select>
                        </div>
                        <div>
                            <label style="display: block; font-size: 0.8rem; margin-bottom: 0.25rem;">快照日期</label>
                            <input type="text" name="snapshot_date" value="{}" placeholder="YYYY-MM-DD" style="width: 100%;" required>
                        </div>
                        <div>
                            <label style="display: block; font-size: 0.8rem; margin-bottom: 0.25rem;">持仓金额 (CNY)*</label>
                            <input type="number" name="market_value" step="0.01" style="width: 100%;" required>
                        </div>
                        <div>
                            <label style="display: block; font-size: 0.8rem; margin-bottom: 0.25rem;">持有份额 (可选)</label>
                            <input type="number" name="units" step="0.0001" style="width: 100%;">
                        </div>
                        <div>
                            <label style="display: block; font-size: 0.8rem; margin-bottom: 0.25rem;">持仓成本价 (可选)</label>
                            <input type="number" name="cost_basis" step="0.0001" style="width: 100%;">
                        </div>
                        <div>
                            <label style="display: block; font-size: 0.8rem; margin-bottom: 0.25rem;">当前净值 (可选)</label>
                            <input type="number" name="nav" step="0.0001" style="width: 100%;">
                        </div>
                        <div>
                            <label style="display: block; font-size: 0.8rem; margin-bottom: 0.25rem;">净值日期 (可选)</label>
                            <input type="text" name="nav_date" placeholder="YYYY-MM-DD" style="width: 100%;">
                        </div>
                        <div>
                            <label style="display: block; font-size: 0.8rem; margin-bottom: 0.25rem;">累计收益 (可选)</label>
                            <input type="number" name="total_pnl" step="0.01" style="width: 100%;">
                        </div>
                        <button type="submit" class="btn btn-success" style="height: 38px;">保存快照</button>
                    </form>
                </div>

                <h3>最新对账与校准建议</h3>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>资产ID</th>
                                <th>基金名称</th>
                                <th>快照日期</th>
                                <th>系统金额</th>
                                <th>支付宝金额</th>
                                <th>差异金额</th>
                                <th>状态</th>
                                <th>操作</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>
                "#,
                asset_options,
                chrono::Local::now().format("%Y-%m-%d"),
                compare_rows
            );
            layout_with_msg("对账管理", content, query.success, query.error)
        }
        Err(e) => layout(
            "Error",
            format!("<div class='warning-box'>加载数据失败: {}</div>", e),
        ),
    }
}

#[derive(Deserialize)]
struct AddSnapshotForm {
    asset_id: String,
    snapshot_date: String,
    market_value: f64,
    units: Option<f64>,
    cost_basis: Option<f64>,
    nav: Option<f64>,
    nav_date: Option<String>,
    total_pnl: Option<f64>,
}

async fn admin_add_snapshot_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AddSnapshotForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let mut snapshots = state.repo.load_alipay_snapshots(&ctx).await?;

        let asset = config.assets.iter().find(|a| a.asset_id == form.asset_id);
        if let Some(a) = asset {
            if form.market_value < 0.0 {
                return Err(anyhow::anyhow!("金额不能为负数"));
            }

            let snapshot_id = format!(
                "snap_{}_{}",
                form.asset_id,
                chrono::Local::now().format("%Y%m%d%H%M%S")
            );

            // Handle empty strings from form as None
            let parse_opt_f64 = |opt: Option<f64>| opt.filter(|&v| v > 0.0);

            let parse_opt_string = |opt: Option<String>| {
                if let Some(s) = opt {
                    if s.trim().is_empty() {
                        None
                    } else {
                        Some(s.trim().to_string())
                    }
                } else {
                    None
                }
            };

            let new_snapshot = models::AlipaySnapshot {
                snapshot_id: snapshot_id.clone(),
                asset_id: form.asset_id.clone(),
                fund_code: a.fund_code.clone(),
                fund_name: a.fund_name.clone(),
                snapshot_date: form.snapshot_date.clone(),
                market_value: form.market_value,
                units: parse_opt_f64(form.units),
                cost_basis: parse_opt_f64(form.cost_basis),
                nav: parse_opt_f64(form.nav),
                nav_date: parse_opt_string(form.nav_date),
                daily_pnl: None,
                total_pnl: form.total_pnl,
                source: "alipay".to_string(),
                note: Some("Via Web Admin".to_string()),
                created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            snapshots.push(new_snapshot.clone());
            state.repo.save_alipay_snapshots(&ctx, &snapshots).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "add_alipay_snapshot".to_string(),
                target_file: "alipay_snapshots.json".to_string(),
                target_id: Some(snapshot_id),
                old_value_summary: "none".to_string(),
                new_value_summary: format!("{:?}", new_snapshot),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("未找到资产 {}", form.asset_id))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/reconcile?success=快照录入成功"),
        Err(e) => Redirect::to(&format!("/admin/reconcile?error={}", e)),
    }
}

#[derive(Deserialize)]
struct ReconcileApplyForm {
    snapshot_id: String,
    confirm: Option<String>,
}

async fn admin_reconcile_apply_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ReconcileApplyForm>,
) -> Redirect {
    if form.confirm.as_deref() != Some("true") {
        return Redirect::to("/admin/reconcile?error=未确认校准操作");
    }

    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let mut portfolio_state = state.repo.load_state(&ctx).await?;
        let snapshots = state.repo.load_alipay_snapshots(&ctx).await?;

        let snapshot = snapshots.iter().find(|s| s.snapshot_id == form.snapshot_id);

        if let Some(s) = snapshot {
            let res = engine::reconciliation::reconcile_asset(&config, &portfolio_state, s);
            if let Some(suggest) = engine::reconciliation::generate_calibration_suggestion(&res) {
                let audit_record =
                    engine::reconciliation::apply_calibration(&mut portfolio_state, &suggest);

                // Save updated state
                state.repo.save_state(&ctx, &portfolio_state).await?;

                // Save domain audit
                let mut audits = state
                    .repo
                    .load_reconciliation_audits(&ctx)
                    .await
                    .unwrap_or_default();
                audits.push(audit_record.clone());
                state.repo.save_reconciliation_audits(&ctx, &audits).await?;

                // Save web admin audit
                let web_audit = models::WebAdminAudit {
                    audit_id: format!("audit_web_{}", chrono::Local::now().timestamp_millis()),
                    timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    actor: "local_web".to_string(),
                    actor_user_id: Some(ctx.actor_user_id.clone()),
                    target_user_id: Some(ctx.target_user_id.clone()),
                    portfolio_id: Some(ctx.portfolio_id.clone()),
                    role: Some(ctx.role.clone()),
                    action: "apply_calibration".to_string(),
                    target_file: "portfolio_state.json".to_string(),
                    target_id: Some(s.asset_id.clone()),
                    old_value_summary: format!(
                        "units:{}, cost:{}",
                        audit_record.old_units, audit_record.old_cost_basis
                    ),
                    new_value_summary: format!(
                        "units:{}, cost:{}",
                        audit_record.new_units, audit_record.new_cost_basis
                    ),
                    status: "success".to_string(),
                    note: Some(format!("Based on snapshot {}", s.snapshot_id)),
                };
                state.repo.append_web_admin_audit(&ctx, web_audit).await?;

                Ok::<(), anyhow::Error>(())
            } else {
                Err(anyhow::anyhow!("资产 {} 状态一致，无需校准", s.asset_id))
            }
        } else {
            Err(anyhow::anyhow!("未找到快照 {}", form.snapshot_id))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/reconcile?success=校准执行成功，持仓已更新"),
        Err(e) => Redirect::to(&format!("/admin/reconcile?error={}", e)),
    }
}

#[derive(Deserialize)]
struct AddSettlementForm {
    asset_id: String,
    plan_id: Option<String>,
    deduction_date: String,
    confirmation_date: String,
    amount: f64,
    confirmed_nav: f64,
    confirmed_units: f64,
    fee: Option<f64>,
    note: Option<String>,
}

async fn admin_dca_settlements_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminQuery>,
) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let settlements = state.repo.load_settlements(&ctx).await?;
        Ok::<(models::ConfigRoot, Vec<models::DcaSettlement>), anyhow::Error>((config, settlements))
    }
    .await;

    match result {
        Ok((config, mut settlements)) => {
            let mut asset_options = String::new();
            for asset in &config.assets {
                if asset.enabled {
                    asset_options.push_str(&format!(
                        "<option value='{}'>{} ({})</option>",
                        asset.asset_id, asset.fund_name, asset.asset_id
                    ));
                }
            }

            settlements.sort_by(|a, b| b.deduction_date.cmp(&a.deduction_date));
            let mut rows = String::new();
            for s in settlements.iter().take(50) {
                let (_status_text, status_badge) = if s.applied {
                    ("已入账", badge_status("已应用"))
                } else {
                    (
                        "待处理",
                        "<span class='badge badge-blue'>待入账</span>".to_string(),
                    )
                };

                let mut action_html = String::new();
                if !s.applied {
                    action_html = format!(
                        r#"<form action="/admin/dca-settlements/apply-confirm" method="POST" onsubmit="return confirm('确定要将此笔份额正式计入持仓吗？');">
                            <input type="hidden" name="settlement_id" value="{}">
                            <input type="hidden" name="confirm" value="true">
                            <button type="submit" class="btn btn-outline btn-success" style="padding: 4px 8px; font-size: 0.75rem;">执行入账</button>
                        </form>"#,
                        s.settlement_id
                    );
                }

                rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-weight: 700; color: var(--text-main);'>{}</div>
                            <div style='font-size: 0.75rem; color: var(--text-muted);'><code>{}</code></div>
                        </td>
                        <td style='font-size: 0.85rem;'>{}</td>
                        <td style='font-weight: 800; font-family: DIN Alternate;'>{:.2}</td>
                        <td>
                            <div style='font-size: 0.9rem; font-weight: 600;'>{:.4}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted);'>{:.2} 份</div>
                        </td>
                        <td>{}</td>
                        <td>{}</td>
                    </tr>",
                    s.fund_name,
                    s.asset_id,
                    s.deduction_date,
                    s.amount,
                    s.confirmed_nav,
                    s.confirmed_units,
                    status_badge,
                    action_html
                ));
            }

            let today = chrono::Local::now().format("%Y-%m-%d").to_string();

            let content = format!(
                r#"
                <div class="message-banner message-error" style="background: #FFF7E8; color: #996000; border-color: #FFE4BA; text-align: center; font-weight: 700; margin-bottom: 24px;">
                    ⚠️ 安全警告：Web 管理功能仅建议在本机 127.0.0.1 使用，请不要暴露到公网。
                </div>

                <div style="margin-bottom: 16px;">
                    <a href="/admin" class="btn btn-outline" style="padding: 8px 16px;">&larr; 返回管理面板</a>
                </div>

                <div style="display: grid; grid-template-columns: 1fr 2fr; gap: 24px;">
                    <div>
                        <h1>录入定投确认单</h1>
                        <div class="card">
                            <form action="/admin/dca-settlements/add" method="POST">
                                <div class="form-group">
                                    <label>目标资产</label>
                                    <select name="asset_id" required>
                                        {}
                                    </select>
                                </div>
                                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
                                    <div class="form-group">
                                        <label>扣款日期</label>
                                        <input type="text" name="deduction_date" value="{}" placeholder="YYYY-MM-DD" required>
                                    </div>
                                    <div class="form-group">
                                        <label>确认日期</label>
                                        <input type="text" name="confirmation_date" value="{}" placeholder="YYYY-MM-DD" required>
                                    </div>
                                </div>
                                <div class="form-group">
                                    <label>成交金额 (CNY)*</label>
                                    <input type="number" name="amount" step="0.01" required>
                                </div>
                                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
                                    <div class="form-group">
                                        <label>成交净值 (NAV)*</label>
                                        <input type="number" name="confirmed_nav" step="0.0001" required>
                                    </div>
                                    <div class="form-group">
                                        <label>成交份额 (Units)*</label>
                                        <input type="number" name="confirmed_units" step="0.0001" required>
                                    </div>
                                </div>
                                <button type="submit" class="btn btn-success" style="width: 100%;">保存成交确认</button>
                            </form>
                        </div>
                    </div>

                    <div>
                        <h1>最近结算与入账历史</h1>
                        <div class="table-container">
                            <table style="min-width: unset;">
                                <thead>
                                    <tr>
                                        <th>资产/基金</th>
                                        <th>扣款日期</th>
                                        <th>金额</th>
                                        <th>净值/份额</th>
                                        <th>当前状态</th>
                                        <th>入账操作</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {}
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>
                "#,
                asset_options, today, today, rows
            );
            layout_with_msg("结算管理", content, query.success, query.error)
        }
        Err(e) => layout(
            "结算管理",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
    }
}

async fn admin_add_settlement_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AddSettlementForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let asset = config.assets.iter().find(|a| a.asset_id == form.asset_id);

        if let Some(a) = asset {
            let mut settlements = state.repo.load_settlements(&ctx).await?;
            let settlement_id = format!("settle_{}", chrono::Local::now().timestamp_millis());

            let new_settlement = models::DcaSettlement {
                settlement_id: settlement_id.clone(),
                plan_id: form.plan_id.clone(),
                asset_id: form.asset_id.clone(),
                fund_code: a.fund_code.clone(),
                fund_name: a.fund_name.clone(),
                scheduled_date: None,
                deduction_date: form.deduction_date.clone(),
                confirmation_date: form.confirmation_date.clone(),
                amount: form.amount,
                confirmed_nav: form.confirmed_nav,
                confirmed_units: form.confirmed_units,
                fee: form.fee,
                currency: "CNY".to_string(),
                source: "alipay".to_string(),
                status: models::DcaSettlementStatus::Confirmed,
                applied: false,
                note: form.note.clone(),
                created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            settlements.push(new_settlement.clone());
            state.repo.save_settlements(&ctx, &settlements).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "add_dca_settlement".to_string(),
                target_file: "dca_settlements.json".to_string(),
                target_id: Some(settlement_id),
                old_value_summary: "none".to_string(),
                new_value_summary: format!("{:?}", new_settlement),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("未找到资产 {}", form.asset_id))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/dca-settlements?success=结算录入成功"),
        Err(e) => Redirect::to(&format!("/admin/dca-settlements?error={}", e)),
    }
}

#[derive(Deserialize)]
struct SettlementApplyForm {
    settlement_id: String,
    confirm: String,
}

async fn admin_settlement_apply_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<SettlementApplyForm>,
) -> Redirect {
    if form.confirm != "true" {
        return Redirect::to("/admin/dca-settlements?error=未确认应用操作");
    }

    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let mut portfolio_state = state.repo.load_state(&ctx).await?;
        let mut settlements = state.repo.load_settlements(&ctx).await?;

        let settlement_idx = settlements
            .iter()
            .position(|s| s.settlement_id == form.settlement_id);

        if let Some(idx) = settlement_idx {
            if settlements[idx].applied {
                return Err(anyhow::anyhow!(
                    "结算 {} 已经应用过，请勿重复操作",
                    form.settlement_id
                ));
            }

            let s = &settlements[idx];
            let asset_id = s.asset_id.clone();
            let settlement_id = s.settlement_id.clone();
            let impact =
                engine::dca_settlement::calculate_settlement_impact(&config, &portfolio_state, s);

            let audit_record =
                engine::dca_settlement::apply_settlement(&mut portfolio_state, s, &impact);

            // Mark as applied
            settlements[idx].applied = true;

            // Save updated state and settlements
            state.repo.save_state(&ctx, &portfolio_state).await?;
            state.repo.save_settlements(&ctx, &settlements).await?;

            // Save web admin audit
            let web_audit = models::WebAdminAudit {
                audit_id: format!("audit_web_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "apply_dca_settlement".to_string(),
                target_file: "portfolio_state.json".to_string(),
                target_id: Some(asset_id),
                old_value_summary: format!(
                    "units:{}, cost:{}",
                    audit_record.old_units, audit_record.old_cost_basis
                ),
                new_value_summary: format!(
                    "units:{}, cost:{}",
                    audit_record.new_units, audit_record.new_cost_basis
                ),
                status: "success".to_string(),
                note: Some(format!("Based on settlement {}", settlement_id)),
            };
            state.repo.append_web_admin_audit(&ctx, web_audit).await?;

            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("未找到结算记录 {}", form.settlement_id))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/dca-settlements?success=结算执行成功，持仓已更新"),
        Err(e) => Redirect::to(&format!("/admin/dca-settlements?error={}", e)),
    }
}

async fn admin_dca_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminQuery>,
) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let plans = state.repo.load_plans(&ctx).await?;
        Ok::<(models::ConfigRoot, Vec<models::DcaPlan>), anyhow::Error>((config, plans))
    }
    .await;

    match result {
        Ok((config, plans)) => {
            let mut asset_options = String::new();
            for asset in &config.assets {
                if asset.enabled {
                    asset_options.push_str(&format!(
                        "<option value='{}'>{} ({})</option>",
                        asset.asset_id, asset.fund_name, asset.asset_id
                    ));
                }
            }

            let mut rows = String::new();
            for p in &plans {
                let (_status_text, status_badge) = if p.enabled {
                    ("运行中", badge_status("启用"))
                } else {
                    (
                        "已禁用",
                        "<span class='badge badge-gray'>禁用</span>".to_string(),
                    )
                };

                let freq_label = match p.frequency {
                    models::DcaFrequency::Daily => "每日".to_string(),
                    models::DcaFrequency::Weekly => format!("每周(周{})", p.weekday.unwrap_or(1)),
                    models::DcaFrequency::Monthly => {
                        format!("每月({}日)", p.month_day.unwrap_or(1))
                    }
                };

                let action_btn = if p.enabled {
                    format!(
                        r#"<form action="/admin/dca/disable" method="POST" style="display:inline;">
                            <input type="hidden" name="plan_id" value="{}">
                            <button type="submit" class="btn btn-outline" style="padding: 4px 8px; font-size: 0.75rem; color: var(--warn-color); border-color: var(--warn-color);">暂停</button>
                        </form>"#,
                        p.plan_id
                    )
                } else {
                    format!(
                        r#"<form action="/admin/dca/enable" method="POST" style="display:inline;">
                            <input type="hidden" name="plan_id" value="{}">
                            <button type="submit" class="btn btn-outline" style="padding: 4px 8px; font-size: 0.75rem; color: var(--down-color); border-color: var(--down-color);">开启</button>
                        </form>"#,
                        p.plan_id
                    )
                };

                let remove_btn = format!(
                    r#"<form action="/admin/dca/remove" method="POST" style="display:inline;" onsubmit="return confirm('确定要永久删除此定投计划吗？');">
                        <input type="hidden" name="plan_id" value="{}">
                        <button type="submit" class="btn btn-outline" style="padding: 4px 8px; font-size: 0.75rem; color: var(--up-color); border-color: var(--up-color);">删除</button>
                    </form>"#,
                    p.plan_id
                );

                let update_amount_form = format!(
                    r#"<form action="/admin/dca/update-amount" method="POST" style="display:inline-flex; gap: 4px;">
                        <input type="hidden" name="plan_id" value="{}">
                        <input type="number" name="amount" value="{:.2}" step="0.01" style="width: 80px; padding: 4px; font-size: 0.85rem; border-radius: 4px;">
                        <button type="submit" class="btn btn-outline" style="padding: 4px 8px; font-size: 0.75rem;">修改</button>
                    </form>"#,
                    p.plan_id, p.amount
                );

                rows.push_str(&format!(
                    "<tr>
                        <td><code>{}</code></td>
                        <td style='font-weight: 700;'>{}</td>
                        <td><span class='badge badge-outline' style='font-weight: 600;'>{}</span></td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>
                            <div style='display: flex; gap: 8px;'>
                                {} {}
                            </div>
                        </td>
                    </tr>",
                    p.asset_id,
                    p.fund_name,
                    freq_label,
                    update_amount_form,
                    status_badge,
                    action_btn,
                    remove_btn
                ));
            }

            let content = format!(
                r#"
                <div class="message-banner message-error" style="background: #FFF7E8; color: #996000; border-color: #FFE4BA; text-align: center; font-weight: 700; margin-bottom: 24px;">
                    ⚠️ 安全警告：Web 管理功能仅建议在本机 127.0.0.1 使用，请不要暴露到公网。
                </div>

                <div style="margin-bottom: 16px;">
                    <a href="/admin" class="btn btn-outline" style="padding: 8px 16px;">&larr; 返回管理面板</a>
                </div>

                <div style="display: grid; grid-template-columns: 1fr 2fr; gap: 24px;">
                    <div>
                        <h1>新增定投计划</h1>
                        <div class="card">
                            <form action="/admin/dca/add" method="POST">
                                <div class="form-group">
                                    <label>目标资产</label>
                                    <select name="asset_id" required>
                                        {}
                                    </select>
                                </div>
                                <div class="form-group">
                                    <label>执行频率</label>
                                    <select name="frequency" required>
                                        <option value="daily">每日 (工作日)</option>
                                        <option value="weekly">每周</option>
                                        <option value="monthly">每月</option>
                                    </select>
                                </div>
                                <div class="form-group">
                                    <label>周几/几号 (可选)</label>
                                    <input type="number" name="day" placeholder="1-7 或 1-31">
                                    <small style="color: var(--text-muted);">每周：1为周一；每月：1为1号</small>
                                </div>
                                <div class="form-group">
                                    <label>定投金额 (CNY)*</label>
                                    <input type="number" name="amount" step="0.01" required>
                                </div>
                                <button type="submit" class="btn btn-success" style="width: 100%;">+ 创建定投计划</button>
                            </form>
                        </div>
                    </div>

                    <div>
                        <h1>活跃定投计划列表</h1>
                        <div class="table-container">
                            <table style="min-width: unset;">
                                <thead>
                                    <tr>
                                        <th>资产ID</th>
                                        <th>基金名称</th>
                                        <th>频率</th>
                                        <th>金额 (CNY)</th>
                                        <th>当前状态</th>
                                        <th>管理操作</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {}
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>
                "#,
                asset_options, rows
            );
            layout_with_msg("定投计划", content, query.success, query.error)
        }
        Err(e) => layout("Error", format!("加载定投计划失败: {}", e)),
    }
}

#[derive(Deserialize)]
struct DcaAddForm {
    asset_id: String,
    frequency: String,
    day: Option<u32>,
    amount: f64,
}

async fn admin_dca_add_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<DcaAddForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let asset = config.assets.iter().find(|a| a.asset_id == form.asset_id);

        if let Some(a) = asset {
            let mut plans = state.repo.load_plans(&ctx).await?;
            let freq = match form.frequency.as_str() {
                "daily" => models::DcaFrequency::Daily,
                "weekly" => models::DcaFrequency::Weekly,
                "monthly" => models::DcaFrequency::Monthly,
                _ => return Err(anyhow::anyhow!("无效的频率")),
            };

            let plan_id = format!("plan_{}", chrono::Local::now().timestamp_millis());
            let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let new_plan = models::DcaPlan {
                plan_id: plan_id.clone(),
                asset_id: form.asset_id.clone(),
                fund_code: a.fund_code.clone(),
                fund_name: a.fund_name.clone(),
                amount: form.amount,
                currency: "CNY".to_string(),
                frequency: freq,
                weekday: if form.frequency == "weekly" {
                    form.day
                } else {
                    None
                },
                month_day: if form.frequency == "monthly" {
                    form.day
                } else {
                    None
                },
                start_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                end_date: None,
                enabled: true,
                priority: 0,
                note: Some("Via Web Admin".to_string()),
                created_at: now_str.clone(),
                updated_at: now_str,
            };

            plans.push(new_plan.clone());
            state.repo.save_plans(&ctx, &plans).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "add_dca_plan".to_string(),
                target_file: "dca_plans.json".to_string(),
                target_id: Some(plan_id),
                old_value_summary: "none".to_string(),
                new_value_summary: format!("{:?}", new_plan),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("资产未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/dca?success=定投计划新增成功"),
        Err(e) => Redirect::to(&format!("/admin/dca?error={}", e)),
    }
}

#[derive(Deserialize)]
struct DcaIdForm {
    plan_id: String,
}

#[derive(Deserialize)]
struct DcaUpdateAmountForm {
    plan_id: String,
    amount: f64,
}

async fn admin_dca_update_amount_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<DcaUpdateAmountForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        if let Some(p) = plans.iter_mut().find(|p| p.plan_id == form.plan_id) {
            let old_amount = p.amount;
            p.amount = form.amount;
            state.repo.save_plans(&ctx, &plans).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "update_dca_amount".to_string(),
                target_file: "dca_plans.json".to_string(),
                target_id: Some(form.plan_id.clone()),
                old_value_summary: format!("amount: {}", old_amount),
                new_value_summary: format!("amount: {}", form.amount),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/dca?success=定投金额更新成功"),
        Err(e) => Redirect::to(&format!("/admin/dca?error={}", e)),
    }
}

async fn admin_dca_enable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<DcaIdForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        if let Some(p) = plans.iter_mut().find(|p| p.plan_id == form.plan_id) {
            p.enabled = true;
            state.repo.save_plans(&ctx, &plans).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "enable_dca_plan".to_string(),
                target_file: "dca_plans.json".to_string(),
                target_id: Some(form.plan_id.clone()),
                old_value_summary: "enabled: false".to_string(),
                new_value_summary: "enabled: true".to_string(),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/dca?success=计划已启用"),
        Err(e) => Redirect::to(&format!("/admin/dca?error={}", e)),
    }
}

async fn admin_dca_disable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<DcaIdForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        if let Some(p) = plans.iter_mut().find(|p| p.plan_id == form.plan_id) {
            p.enabled = false;
            state.repo.save_plans(&ctx, &plans).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "disable_dca_plan".to_string(),
                target_file: "dca_plans.json".to_string(),
                target_id: Some(form.plan_id.clone()),
                old_value_summary: "enabled: true".to_string(),
                new_value_summary: "enabled: false".to_string(),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/dca?success=计划已禁用"),
        Err(e) => Redirect::to(&format!("/admin/dca?error={}", e)),
    }
}

async fn admin_dca_remove_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<DcaIdForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        if let Some(idx) = plans.iter().position(|p| p.plan_id == form.plan_id) {
            let removed = plans.remove(idx);
            state.repo.save_plans(&ctx, &plans).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "remove_dca_plan".to_string(),
                target_file: "dca_plans.json".to_string(),
                target_id: Some(form.plan_id.clone()),
                old_value_summary: format!("{:?}", removed),
                new_value_summary: "removed".to_string(),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/dca?success=计划已删除"),
        Err(e) => Redirect::to(&format!("/admin/dca?error={}", e)),
    }
}

async fn admin_assets_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminQuery>,
) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = state.repo.load_config(&ctx).await;

    match result {
        Ok(config) => {
            let mut rows = String::new();
            let filter = query.filter.as_deref().unwrap_or("all");

            for a in &config.assets {
                // Apply filter
                let matches_filter = match filter {
                    "unclassified" => {
                        a.sector.is_empty() || a.sector == "未分类" || a.sector == "待确认"
                    }
                    "disabled" => !a.enabled,
                    "enabled" => a.enabled,
                    _ => true,
                };
                if !matches_filter {
                    continue;
                }

                let status_badge = if a.enabled {
                    "<span class='badge badge-blue'>启用中</span>"
                } else {
                    "<span class='badge badge-gray'>已禁用</span>"
                };

                let action_btn = if a.enabled {
                    format!(
                        r#"<form action="/admin/assets/disable" method="POST" style="display:inline;"><input type="hidden" name="asset_id" value="{}"><button type="submit" class="btn btn-sm btn-outline">禁用</button></form>"#,
                        a.asset_id
                    )
                } else {
                    format!(
                        r#"<form action="/admin/assets/enable" method="POST" style="display:inline;"><input type="hidden" name="asset_id" value="{}"><button type="submit" class="btn btn-sm btn-outline">启用</button></form>"#,
                        a.asset_id
                    )
                };

                rows.push_str(&format!(
                    r#"<tr>
                        <td>
                            <div style="font-weight: 700;">{}</div>
                            <div style="font-size: 0.75rem; color: var(--text-muted);">ID: <code>{}</code></div>
                        </td>
                        <td>
                            <form action="/admin/assets/set-fund-code" method="POST" style="display:flex; gap: 4px;">
                                <input type="hidden" name="asset_id" value="{0}">
                                <input type="text" name="fund_code" value="{1}" style="width: 100px; padding: 6px 8px; font-size: 0.85rem;">
                                <button type="submit" class="btn btn-sm btn-outline">💾</button>
                            </form>
                        </td>
                        <td>
                            <form action="/admin/assets/set-sector" method="POST" style="display:flex; gap: 4px;">
                                <input type="hidden" name="asset_id" value="{0}">
                                <input type="text" name="sector" value="{2}" style="width: 120px; padding: 6px 8px; font-size: 0.85rem;" placeholder="赛道名称">
                                <button type="submit" class="btn btn-sm btn-outline">💾</button>
                            </form>
                        </td>
                        <td>{3}</td>
                        <td class="text-right">
                            <div style="display: flex; gap: 8px; justify-content: flex-end;">
                                {4}
                                <form action="/admin/assets/remove" method="POST" style="display:inline;" onsubmit="return confirm('确定要归档此资产吗？历史数据将保留。');">
                                    <input type="hidden" name="asset_id" value="{0}">
                                    <button type="submit" class="btn btn-sm btn-outline" style="color: var(--up-color);">归档</button>
                                </form>
                            </div>
                        </td>
                    </tr>"#,
                    a.fund_name, a.asset_id, a.sector, status_badge, action_btn
                ));
            }

            let filter_chips = format!(
                r#"<div style="display: flex; gap: 8px; margin-bottom: 24px;">
                    <a href="/admin/assets" class="badge badge-outline {}" style="text-decoration: none; padding: 6px 16px;">全部</a>
                    <a href="/admin/assets?filter=unclassified" class="badge badge-outline {}" style="text-decoration: none; padding: 6px 16px;">未分类</a>
                    <a href="/admin/assets?filter=enabled" class="badge badge-outline {}" style="text-decoration: none; padding: 6px 16px;">已启用</a>
                    <a href="/admin/assets?filter=disabled" class="badge badge-outline {}" style="text-decoration: none; padding: 6px 16px;">已禁用</a>
                </div>"#,
                if filter == "all" { "active" } else { "" },
                if filter == "unclassified" { "active" } else { "" },
                if filter == "enabled" { "active" } else { "" },
                if filter == "disabled" { "active" } else { "" },
            ).replace("active", "badge-blue");

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 32px;">
                    <div>
                        <h1 style="margin-bottom: 4px;">资产配置管理 (Asset Config)</h1>
                        <p style="color: var(--text-muted); font-size: 0.95rem; margin: 0;">维护资产ID、基金代码与赛道分类的映射关系</p>
                    </div>
                    <div class="action-group" style="margin-top: 0;">
                        <button onclick="autoClassify(this)" class="btn">🤖 自动分类全部</button>
                        <button onclick="document.getElementById('addAssetModal').style.display='block'" class="btn btn-outline">➕ 新增资产</button>
                    </div>
                </div>

                {}

                <div class="table-container">
                    <div class="table-wrap">
                        <table>
                            <thead>
                                <tr>
                                    <th>资产名称 / ID</th>
                                    <th>基金代码 (Fund Code)</th>
                                    <th>所属赛道 (Sector)</th>
                                    <th>状态</th>
                                    <th class="text-right">操作</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>
                </div>

                <div id="addAssetModal" style="display:none; position:fixed; z-index:2000; left:0; top:0; width:100%; height:100%; background:rgba(0,0,0,0.5); backdrop-filter:blur(4px);">
                    <div class="card" style="max-width:500px; margin: 100px auto; position:relative;">
                        <div class="card-header">
                            <span class="card-title">新增资产</span>
                            <button onclick="document.getElementById('addAssetModal').style.display='none'" class="btn-ghost">关闭</button>
                        </div>
                        <form action="/admin/assets/add" method="POST">
                            <div class="form-group">
                                <label>资产名称</label>
                                <input type="text" name="fund_name" required placeholder="如：易方达标普500">
                            </div>
                            <div class="form-group">
                                <label>基金代码</label>
                                <input type="text" name="fund_code" required placeholder="如：050025">
                            </div>
                            <div class="form-group">
                                <label>所属赛道</label>
                                <input type="text" name="sector" placeholder="如：美国大盘">
                            </div>
                            <button type="submit" class="btn btn-block">创建资产</button>
                        </form>
                    </div>
                </div>
                "#,
                filter_chips, rows
            );
            layout_with_msg("资产管理", content, query.success, query.error)
        }
        Err(e) => layout(
            "资产管理",
            format!(
                "<div class='message-banner message-error'>加载配置失败: {}</div>",
                e
            ),
        ),
    }
}

#[derive(Deserialize)]
struct AssetFundCodeForm {
    asset_id: String,
    fund_code: String,
}

async fn admin_asset_set_fund_code_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetFundCodeForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut config = state.repo.load_config(&ctx).await?;
        if let Some(a) = config
            .assets
            .iter_mut()
            .find(|a| a.asset_id == form.asset_id)
        {
            let old_code = a.fund_code.clone();
            a.fund_code = form.fund_code.clone();
            state.repo.save_config(&ctx, &config).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "set_asset_fund_code".to_string(),
                target_file: "config.toml".to_string(),
                target_id: Some(form.asset_id.clone()),
                old_value_summary: format!("fund_code: {}", old_code),
                new_value_summary: format!("fund_code: {}", form.fund_code),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("资产未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/assets?success=基金代码设置成功"),
        Err(e) => Redirect::to(&format!("/admin/assets?error={}", e)),
    }
}

#[derive(Deserialize)]
struct AssetRenameForm {
    asset_id: String,
    fund_name: String,
}

async fn admin_asset_rename_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetRenameForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut config = state.repo.load_config(&ctx).await?;
        if let Some(a) = config
            .assets
            .iter_mut()
            .find(|a| a.asset_id == form.asset_id)
        {
            let old_name = a.fund_name.clone();
            a.fund_name = form.fund_name.clone();
            state.repo.save_config(&ctx, &config).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "rename_asset".to_string(),
                target_file: "config.toml".to_string(),
                target_id: Some(form.asset_id.clone()),
                old_value_summary: format!("fund_name: {}", old_name),
                new_value_summary: format!("fund_name: {}", form.fund_name),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("资产未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/assets?success=资产更名成功"),
        Err(e) => Redirect::to(&format!("/admin/assets?error={}", e)),
    }
}

#[derive(Deserialize)]
struct AssetSectorForm {
    asset_id: String,
    sector: String,
}

async fn admin_asset_set_sector_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetSectorForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut config = state.repo.load_config(&ctx).await?;
        if let Some(a) = config
            .assets
            .iter_mut()
            .find(|a| a.asset_id == form.asset_id)
        {
            let old_sector = a.sector.clone();
            a.sector = form.sector.clone();
            state.repo.save_config(&ctx, &config).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "set_asset_sector".to_string(),
                target_file: "config.toml".to_string(),
                target_id: Some(form.asset_id.clone()),
                old_value_summary: format!("sector: {}", old_sector),
                new_value_summary: format!("sector: {}", form.sector),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("资产未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/assets?success=资产板块设置成功"),
        Err(e) => Redirect::to(&format!("/admin/assets?error={}", e)),
    }
}

async fn admin_instruments_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminQuery>,
) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = state.repo.load_instruments(&ctx).await;

    match result {
        Ok(instruments_list) => {
            let mut rows = String::new();
            let mut instruments = instruments_list.clone();
            instruments.sort_by(|a, b| a.instrument_id.cmp(&b.instrument_id));

            for inst in &instruments {
                let status_badge = if inst.enabled {
                    "<span class='badge badge-blue'>启用中</span>"
                } else {
                    "<span class='badge badge-gray'>已禁用</span>"
                };

                let type_badge = match inst.asset_class {
                    models::AssetClass::Etf => "<span class='badge badge-outline' style='color:#165DFF;'>股票/ETF</span>",
                    models::AssetClass::Index => "<span class='badge badge-outline' style='color:#722ED1;'>指数</span>",
                    models::AssetClass::Fx => "<span class='badge badge-outline' style='color:#2F54EB;'>外汇</span>",
                    models::AssetClass::SpotCommodity => "<span class='badge badge-outline' style='color:#FA8C16;'>大宗商品</span>",
                    models::AssetClass::Crypto => "<span class='badge badge-outline' style='color:#FADB14;'>加密货币</span>",
                    _ => "<span class='badge badge-outline'>其他</span>",
                };

                let action_btn = if inst.enabled {
                    format!(r#"<form action="/admin/instruments/disable" method="POST" style="display:inline;"><input type="hidden" name="instrument_id" value="{}"><button type="submit" class="btn btn-sm btn-outline">禁用</button></form>"#, inst.instrument_id)
                } else {
                    format!(r#"<form action="/admin/instruments/enable" method="POST" style="display:inline;"><input type="hidden" name="instrument_id" value="{}"><button type="submit" class="btn btn-sm btn-outline">启用</button></form>"#, inst.instrument_id)
                };

                rows.push_str(&format!(
                    r#"<tr>
                        <td>
                            <div style="font-weight: 700;">{}</div>
                            <div style="font-size: 0.75rem; color: var(--text-muted);">Symbol: <code>{}</code></div>
                        </td>
                        <td>{}</td>
                        <td>
                            <form action="/admin/instruments/update-metadata" method="POST" style="display:flex; flex-direction:column; gap: 4px;">
                                <input type="hidden" name="instrument_id" value="{}">
                                <input type="text" name="name_zh" value="{}" placeholder="中文显示名称" style="width: 140px; padding: 4px 8px; font-size: 0.8rem;">
                                <button type="submit" class="btn btn-sm btn-outline" style="padding: 2px 8px; font-size: 0.7rem;">💾 保存名称</button>
                            </form>
                        </td>
                        <td><div style="font-size: 0.85rem; font-weight: 600;">{}</div><div style="font-size: 0.7rem; color: var(--text-muted);">{}</div></td>
                        <td>{}</td>
                        <td class="text-right">{}</td>
                    </tr>"#,
                    inst.instrument_id,
                    inst.symbol,
                    type_badge,
                    inst.instrument_id,
                    inst.name_zh.as_deref().unwrap_or(""),
                    inst.provider,
                    inst.currency,
                    status_badge,
                    action_btn
                ));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 32px;">
                    <div>
                        <h1 style="margin-bottom: 4px;">市场标的管理 (Instrument Registry)</h1>
                        <p style="color: var(--text-muted); font-size: 0.95rem; margin: 0;">用于行情刷新、指数锚定、Kelly 建议及钟摆逻辑的全局标的注册表</p>
                    </div>
                </div>

                <div class="table-container">
                    <div class="table-wrap">
                        <table>
                            <thead>
                                <tr>
                                    <th>标的 ID / 代码</th>
                                    <th>资产类型</th>
                                    <th>中文显示名</th>
                                    <th>行情源 / 币种</th>
                                    <th>状态</th>
                                    <th class="text-right">操作</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>
                </div>

                <div class="card" style="background: #FFFBE6; border: 1px solid #FFE58F; padding: 20px;">
                    <h3 style="margin-top: 0; color: #856404;">📝 注册表说明</h3>
                    <p style="font-size: 0.9rem; color: #856404; line-height: 1.6; margin-bottom: 0;">
                        1. <strong>标的 ID</strong> 是内部引用键，<strong>Symbol</strong> 是第三方行情接口的查询代码（如 QQQ, BTC-USD）。<br>
                        2. 只有<strong>已启用</strong>的标的才会参与后台定期行情刷新。<br>
                        3. 如需新增自定义指数或商品，请联系系统管理员或通过配置文件手动注入。
                    </p>
                </div>
                "#,
                rows
            );
            layout_with_msg("标的管理", content, query.success, query.error)
        }
        Err(e) => layout("标的管理", format!("<div class='message-banner message-error'>标的数据加载失败: {}</div>", e)),
    }
}

#[derive(Deserialize)]
struct InstrumentIdForm {
    instrument_id: String,
}

async fn admin_instrument_enable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentIdForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        if let Some(inst) = instruments
            .iter_mut()
            .find(|i| i.instrument_id == form.instrument_id)
        {
            inst.enabled = true;
            state.repo.save_instruments(&ctx, &instruments).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "enable_instrument".to_string(),
                target_file: "instruments.json".to_string(),
                target_id: Some(form.instrument_id.clone()),
                old_value_summary: "enabled: false".to_string(),
                new_value_summary: "enabled: true".to_string(),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("证券未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/instruments?success=证券已启用"),
        Err(e) => Redirect::to(&format!("/admin/instruments?error={}", e)),
    }
}

async fn admin_instrument_disable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentIdForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        if let Some(inst) = instruments
            .iter_mut()
            .find(|i| i.instrument_id == form.instrument_id)
        {
            inst.enabled = false;
            state.repo.save_instruments(&ctx, &instruments).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "disable_instrument".to_string(),
                target_file: "instruments.json".to_string(),
                target_id: Some(form.instrument_id.clone()),
                old_value_summary: "enabled: true".to_string(),
                new_value_summary: "enabled: false".to_string(),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("证券未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/instruments?success=证券已禁用"),
        Err(e) => Redirect::to(&format!("/admin/instruments?error={}", e)),
    }
}

#[derive(Deserialize)]
struct InstrumentMetadataForm {
    instrument_id: String,
    name_zh: Option<String>,
    display_label: Option<String>,
}

async fn admin_instrument_update_metadata_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentMetadataForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        let instrument_id = form.instrument_id.clone();

        let (old_meta, new_meta) = {
            if let Some(inst) = instruments
                .iter_mut()
                .find(|i| i.instrument_id == instrument_id)
            {
                let old_meta = format!(
                    "name_zh: {:?}, label: {:?}",
                    inst.name_zh, inst.display_label
                );

                if let Some(n) = form.name_zh.filter(|n| !n.trim().is_empty()) {
                    inst.name_zh = Some(n.trim().to_string());
                }
                if let Some(l) = form.display_label.filter(|l| !l.trim().is_empty()) {
                    inst.display_label = Some(l.trim().to_string());
                }

                let new_meta = format!(
                    "name_zh: {:?}, label: {:?}",
                    inst.name_zh, inst.display_label
                );
                (old_meta, new_meta)
            } else {
                return Err(anyhow::anyhow!("证券未找到"));
            }
        };

        state.repo.save_instruments(&ctx, &instruments).await?;

        let audit = models::WebAdminAudit {
            audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            actor: "local_web".to_string(),
            actor_user_id: Some(ctx.actor_user_id.clone()),
            target_user_id: Some(ctx.target_user_id.clone()),
            portfolio_id: Some(ctx.portfolio_id.clone()),
            role: Some(ctx.role.clone()),
            action: "update_instrument_metadata".to_string(),
            target_file: "instruments.json".to_string(),
            target_id: Some(instrument_id),
            old_value_summary: old_meta,
            new_value_summary: new_meta,
            status: "success".to_string(),
            note: None,
        };
        state.repo.append_web_admin_audit(&ctx, audit).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/instruments?success=证券元数据更新成功"),
        Err(e) => Redirect::to(&format!("/admin/instruments?error={}", e)),
    }
}

// --- Autonomous Operation Handlers ---

async fn operation_page_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let status_res = state.repo.load_operation_status(&ctx).await;

    match status_res {
        Ok(status) => {
            let report_html = if let Some(report) = &status.last_report {
                let mut rows = String::new();
                for sug in &report.suggestions {
                    let (status_badge, row_bg) = match sug.status.as_str() {
                        "execute" => ("<span class='badge badge-red'>执行买入</span>", "rgba(245, 63, 63, 0.02)"),
                        "skip" => ("<span class='badge badge-gray'>跳过</span>", "transparent"),
                        "pause" => ("<span class='badge badge-orange'>暂停定投</span>", "rgba(255, 125, 0, 0.02)"),
                        "resume" => ("<span class='badge badge-green'>恢复定投</span>", "rgba(0, 180, 42, 0.02)"),
                        _ => ("<span class='badge'>未知</span>", "transparent"),
                    };

                    rows.push_str(&format!(
                        r#"<tr style="background: {};">
                            <td>
                                <div style="font-weight: 700;">{}</div>
                                <div style="font-size: 0.75rem; color: var(--text-muted);"><code>{}</code></div>
                            </td>
                            <td>
                                <div style="font-size: 0.85rem;">{} ({:+.2}%)</div>
                                <div style="font-size: 0.7rem; color: var(--text-muted);">得分: {:.1} · {}</div>
                            </td>
                            <td class="text-right">
                                <div class="text-up tabular" style="font-weight: 800;">{:.2}</div>
                                <div style="font-size: 0.7rem; color: var(--text-muted);">Kelly x{:.2}</div>
                            </td>
                            <td class="text-right">
                                <div class="tabular">当前: {:.1}%</div>
                                <div style="font-size: 0.7rem; color: var(--text-muted);">缺口: {:+.1}%</div>
                            </td>
                            <td>{}</td>
                            <td>
                                <div style="font-size: 0.85rem; font-weight: 500;">{}</div>
                                <div style="font-size: 0.75rem; color: var(--text-muted); line-height: 1.4;">{}</div>
                            </td>
                        </tr>"#,
                        row_bg, sug.fund_name, sug.fund_code,
                        sug.benchmark_symbol.as_deref().unwrap_or("-"), sug.benchmark_return * 100.0,
                        sug.pendulum_score, sug.regime_label,
                        sug.suggested_amount, sug.kelly_multiplier,
                        sug.current_weight * 100.0, sug.allocation_gap * 100.0,
                        status_badge, sug.reason, sug.explanation
                    ));
                }

                format!(
                    r#"
                    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;">
                        <h2 style="margin: 0;">📊 最近运行报告 ({})</h2>
                        <span style="font-size: 0.85rem; color: var(--text-muted); font-weight: 600;">生成时间: {}</span>
                    </div>

                    <div class="dashboard-grid">
                        <div class="card">
                            <div class="card-header"><span class="card-title">权益仓位</span></div>
                            <div class="card-value tabular">{:.1}% <small style="font-size: 1rem; color: var(--text-muted); font-weight: 400;">/ {:.1}%</small></div>
                            <div class="card-sub">当前实际 vs 目标策略</div>
                        </div>
                        <div class="card">
                            <div class="card-header"><span class="card-title">定投执行情况</span></div>
                            <div class="card-value tabular">{} <small style="font-size: 1rem; color: var(--text-muted); font-weight: 400;">项执行</small></div>
                            <div class="card-sub">{} 项定投被策略跳过</div>
                        </div>
                    </div>

                    <div class="table-container">
                        <div class="table-wrap">
                            <table>
                                <thead>
                                    <tr>
                                        <th>基金资产</th>
                                        <th>基准行情</th>
                                        <th class="text-right">建议买入</th>
                                        <th class="text-right">仓位/缺口</th>
                                        <th>建议动作</th>
                                        <th>原因说明</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {}
                                </tbody>
                            </table>
                        </div>
                    </div>
                    "#,
                    report.date, report.timestamp,
                    report.current_equity_weight * 100.0, report.target_equity_weight * 100.0,
                    report.dca_execution_result.executed_count, report.dca_execution_result.skipped_count,
                    rows
                )
            } else {
                "<div class='empty-state'><span class='empty-state-icon'>🤖</span><div class='empty-state-text'>尚未运行过自主运作。点击右上角按钮开始评估组合。</div></div>".to_string()
            };

            let policy = &status.policy;
            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 32px;">
                    <div>
                        <h1 style="margin-bottom: 4px;">自主运作控制台 (Operations Policy)</h1>
                        <p style="color: var(--text-muted); font-size: 1rem; margin: 0;">基于风险模型与仓位目标自动生成每日调仓建议</p>
                    </div>
                    <div style="display: flex; gap: 12px;">
                        <button id="runOpBtn" class="btn" onclick="runOperation(this)">⚡ 立即运行评估</button>
                    </div>
                </div>

                {}

                <h2 style="margin-top: 48px;">⚙️ 策略参数配置 (Policy Config)</h2>
                <form id="policy-form">
                    <div class="dashboard-grid">
                        <div class="card">
                            <h3 style="margin-top: 0;">🎯 仓位目标</h3>
                            <div class="form-group">
                                <label>目标总权益权重</label>
                                <input type="number" name="target_equity_weight" value="{}" step="0.01" min="0" max="1">
                                <p style="font-size: 0.75rem; color: var(--text-muted); margin-top: 4px;">组合中权益类资产占总市值的目标比例 (0.0-1.0)。</p>
                            </div>
                            <div class="form-group">
                                <label>最小现金储备</label>
                                <input type="number" name="min_cash_reserve" value="{:.2}" min="0">
                                <p style="font-size: 0.75rem; color: var(--text-muted); margin-top: 4px;">必须保留的最低现金金额，低于此值将停止所有买入。</p>
                            </div>
                        </div>

                        <div class="card">
                            <h3 style="margin-top: 0;">🚫 买入限制</h3>
                            <div class="form-group">
                                <label>单日总买入上限</label>
                                <input type="number" name="max_daily_buy_amount" value="{:.2}" min="0">
                            </div>
                            <div class="form-group">
                                <label>单资产买入上限</label>
                                <input type="number" name="max_single_asset_buy_amount" value="{:.2}" min="0">
                            </div>
                        </div>

                        <div class="card">
                            <h3 style="margin-top: 0;">⚖️ 风险控制</h3>
                            <div class="form-group">
                                <label>单资产权重上限</label>
                                <input type="number" name="max_single_asset_weight" value="{}" step="0.01" min="0" max="1">
                            </div>
                            <div class="form-group">
                                <label>单板块权重上限</label>
                                <input type="number" name="max_sector_weight" value="{}" step="0.01" min="0" max="1">
                            </div>
                        </div>

                        <div class="card">
                            <h3 style="margin-top: 0;">🤖 自动化增强</h3>
                            <div class="form-group">
                                <label>启用 Kelly 仓位管理</label>
                                <select name="kelly_enabled">
                                    <option value="true" {} >启用 (动态调整买入量)</option>
                                    <option value="false" {} >禁用 (固定金额)</option>
                                </select>
                            </div>
                            <div class="form-group">
                                <label>启用钟摆周期管理</label>
                                <select name="pendulum_enabled">
                                    <option value="true" {} >启用 (基于估值热度调控)</option>
                                    <option value="false" {} >禁用</option>
                                </select>
                            </div>
                        </div>
                    </div>
                    
                    <div style="background: #FFF; padding: 24px; border-radius: var(--radius); border: 1px solid var(--border-color); display: flex; justify-content: space-between; align-items: center;">
                        <div style="font-size: 0.9rem; color: var(--text-muted);">
                            ⚠️ 修改参数将直接影响每日流水线生成的买入建议。
                        </div>
                        <button type="button" class="btn" style="padding: 12px 48px;" onclick="savePolicies(this)">💾 保存策略配置</button>
                    </div>
                </form>

                <script>
                    async function runOperation(btn) {{
                        if (btn) {{
                            btn.disabled = true;
                            btn.innerText = '⏳ 正在评估中...';
                        }}
                        try {{
                            const res = await fetch('/api/operation/run', {{ method: 'POST' }});
                            if (res.ok) {{
                                location.reload();
                            }} else {{
                                alert('评估任务启动失败');
                                if (btn) {{
                                    btn.disabled = false;
                                    btn.innerText = '⚡ 立即运行评估';
                                }}
                            }}
                        }} catch (e) {{
                            alert('网络请求异常');
                            if (btn) {{
                                btn.disabled = false;
                                btn.innerText = '⚡ 立即运行评估';
                            }}
                        }}
                    }}

                    async function savePolicies(btn) {{
                        const form = document.getElementById('policy-form');
                        const formData = new FormData(form);
                        const data = {{}};
                        formData.forEach((value, key) => {{
                            if (key === 'kelly_enabled' || key === 'pendulum_enabled') {{
                                data[key] = value === 'true';
                            }} else {{
                                data[key] = parseFloat(value);
                            }}
                        }});

                        if (btn) {{
                            btn.disabled = true;
                            btn.innerText = '⏳ 正在保存...';
                        }}

                        try {{
                            const res = await fetch('/api/operation/policies', {{
                                method: 'POST',
                                headers: {{ 'Content-Type': 'application/json' }},
                                body: JSON.stringify(data)
                            }});
                            if (res.ok) {{
                                alert('策略配置已更新');
                                location.reload();
                            }} else {{
                                alert('保存失败');
                                if (btn) {{
                                    btn.disabled = false;
                                    btn.innerText = '💾 保存策略配置';
                                }}
                            }}
                        }} catch (e) {{
                            alert('网络请求异常');
                            if (btn) {{
                                btn.disabled = false;
                                btn.innerText = '💾 保存策略配置';
                            }}
                        }}
                    }}
                </script>
                "#,
                report_html,
                policy.target_equity_weight, policy.min_cash_reserve,
                policy.max_daily_buy_amount, policy.max_single_asset_buy_amount,
                policy.max_single_asset_weight, policy.max_sector_weight,
                if policy.kelly_enabled { "selected" } else { "" },
                if !policy.kelly_enabled { "selected" } else { "" },
                if policy.pendulum_enabled { "selected" } else { "" },
                if !policy.pendulum_enabled { "selected" } else { "" }
            );

            layout("操作策略", content)
        }
        Err(e) => layout("操作策略", format!("<div class='message-banner message-error'>数据加载失败: {}</div>", e)),
    }
}

async fn api_operation_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::OperationStatus> {
    let ctx = RepositoryContext::default();
    let status = state
        .repo
        .load_operation_status(&ctx)
        .await
        .unwrap_or_else(|_| models::OperationStatus::default());
    Json(status)
}

async fn api_operation_report_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ctx = RepositoryContext::default();
    let status = state
        .repo
        .load_operation_status(&ctx)
        .await
        .unwrap_or_else(|_| models::OperationStatus::default());

    if let Some(report) = status.last_report {
        Json(serde_json::to_value(report).unwrap())
    } else {
        Json(serde_json::json!({ "error": "No report available" }))
    }
}

async fn api_operation_run_handler(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let ctx = RepositoryContext::default();
    let config_res = state.repo.load_config(&ctx).await;

    match config_res {
        Ok(config) => {
            // run_autonomous_operation now handles internal refresh if needed via evaluate_operation_state
            match engine::run_autonomous_operation(state.repo.as_ref(), &ctx, &config).await {
                Ok(report) => Json(serde_json::json!({ "success": true, "report": report })),
                Err(e) => Json(
                    serde_json::json!({ "success": false, "message": e.to_string() as String }),
                ),
            }
        }
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() as String })),
    }
}

async fn api_get_operation_policies_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::OperationPolicy> {
    let ctx = RepositoryContext::default();
    let policy = state
        .repo
        .load_operation_policy(&ctx)
        .await
        .unwrap_or_else(|_| models::OperationPolicy::default());
    Json(policy)
}

async fn api_save_operation_policies_handler(
    State(state): State<Arc<AppState>>,
    Json(policy): Json<models::OperationPolicy>,
) -> Json<serde_json::Value> {
    let ctx = RepositoryContext::default();
    match state.repo.save_operation_policy(&ctx, &policy).await {
        Ok(_) => Json(serde_json::json!({ "success": true })),
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    }
}

// --- Backtest Handlers ---

#[derive(Deserialize)]
struct BacktestRunForm {
    start_date: String,
    end_date: String,
    initial_cash: f64,
    include_baseline: bool,
}

async fn backtest_page_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let report_opt = state.last_backtest_report.read().await;

    let report_html = if let Some(report) = report_opt.as_ref() {
        let mut daily_rows = String::new();
        for day in report.daily_results.iter().rev().take(100) {
            let trades_html = if day.trades.is_empty() {
                "无成交".to_string()
            } else {
                day.trades
                    .iter()
                    .map(|t| format!("{} 买入 {:.2}", t.fund_name, t.amount))
                    .collect::<Vec<_>>()
                    .join("<br>")
            };

            daily_rows.push_str(&format!(
                r#"<tr>
                    <td>{}</td>
                    <td>{:.2}</td>
                    <td>{:.2}%</td>
                    <td>{}</td>
                </tr>"#,
                day.date,
                day.total_value,
                day.equity_weight * 100.0,
                trades_html
            ));
        }

        let baseline_info = if let Some(baseline) = &report.baseline_metrics {
            format!(
                r#"<div class="stat-item">
                    <span class="stat-label">基准最终值 (Fixed DCA)</span>
                    <span class="stat-value">{:.2}</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">策略收益差</span>
                    <span class="stat-value {}">{:+.2}</span>
                </div>"#,
                baseline.final_value,
                if report.main_metrics.final_value >= baseline.final_value {
                    "text-up"
                } else {
                    "text-down"
                },
                report.main_metrics.final_value - baseline.final_value
            )
        } else {
            "".to_string()
        };

        format!(
            r#"<div class="card">
                <div class="card-header"><h3>回测报告 (最近运行)</h3></div>
                <div class="operation-stats">
                    <div class="stat-item"><span class="stat-label">周期</span><span class="stat-value">{} 至 {}</span></div>
                    <div class="stat-item"><span class="stat-label">最终估值</span><span class="stat-value">{:.2}</span></div>
                    <div class="stat-item"><span class="stat-label">总投入</span><span class="stat-value">{:.2}</span></div>
                    <div class="stat-item"><span class="stat-label">买入天数</span><span class="stat-value">{} 天</span></div>
                    <div class="stat-item"><span class="stat-label">最大回撤</span><span class="stat-value">{:.2}%</span></div>
                    {}
                </div>
                
                <div class="table-container" style="margin-top: 20px;">
                    <h4>每日仿真明细 (展示最近100条)</h4>
                    <table>
                        <thead>
                            <tr>
                                <th>日期</th>
                                <th>组合市值</th>
                                <th>权益仓位</th>
                                <th>交易仿真</th>
                            </tr>
                        </thead>
                        <tbody>{}</tbody>
                    </table>
                </div>
            </div>"#,
            report.request.start_date,
            report.request.end_date,
            report.main_metrics.final_value,
            report.main_metrics.total_invested,
            report.main_metrics.total_buy_days,
            report.main_metrics.max_drawdown * 100.0,
            baseline_info,
            daily_rows
        )
    } else {
        "<p class='text-muted'>暂无回测报告，请配置参数并运行。</p>".to_string()
    };

    Html(format!(
        r#"<!DOCTYPE html>
        <html>
        <head>
            <title>策略回测 - JDI</title>
            <meta charset="UTF-8">
            {}
            <style>
                .backtest-form {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px; margin-bottom: 20px; }}
                .stat-item {{ padding: 10px; }}
                .text-up {{ color: var(--up-color); }}
                .text-down {{ color: var(--down-color); }}
            </style>
        </head>
        <body>
            <div class="container">
                <header class="header">
                    <div class="header-content">
                        <div class="logo">🤖 策略回测仿真</div>
                        <nav class="nav">
                            <a href="/dashboard">仪表盘</a>
                            <a href="/operation">自主运作</a>
                            <a href="/backtest" class="active">历史回测</a>
                        </nav>
                    </div>
                </header>

                <div class="card" style="margin-bottom: 20px;">
                    <div class="card-header"><h3>仿真参数配置</h3></div>
                    <form id="backtest-form" class="backtest-form">
                        <div class="form-group">
                            <label>开始日期</label>
                            <input type="date" name="start_date" value="2024-01-01" required>
                        </div>
                        <div class="form-group">
                            <label>结束日期</label>
                            <input type="date" name="end_date" value="{}" required>
                        </div>
                        <div class="form-group">
                            <label>初始现金</label>
                            <input type="number" name="initial_cash" value="100000" step="1000">
                        </div>
                        <div class="form-group" style="display:flex; align-items: center; gap: 8px; padding-top: 25px;">
                            <input type="checkbox" name="include_baseline" id="include_baseline" checked>
                            <label for="include_baseline" style="margin:0">包含基准对比</label>
                        </div>
                        <div class="form-group" style="padding-top: 15px;">
                            <button type="button" onclick="runBacktest()" class="btn btn-primary" style="width:100%">开始仿真</button>
                        </div>
                    </form>
                </div>

                <div id="loading" style="display:none; text-align:center; padding: 40px;">
                    <div class="spinner"></div>
                    <p style="margin-top: 15px; color: var(--text-muted);">正在获取历史数据并执行逐日仿真，请稍候...</p>
                </div>

                <div id="report-container">
                    {}
                </div>
            </div>

            <script>
                async function runBacktest() {{
                    const form = document.getElementById("backtest-form");
                    const loading = document.getElementById("loading");
                    const container = document.getElementById("report-container");
                    
                    const formData = new FormData(form);
                    const data = {{
                        start_date: formData.get("start_date"),
                        end_date: formData.get("end_date"),
                        initial_cash: parseFloat(formData.get("initial_cash")),
                        include_baseline: document.getElementById("include_baseline").checked
                    }};

                    loading.style.display = "block";
                    container.style.opacity = "0.5";

                    try {{
                        const resp = await fetch("/api/backtest/run", {{
                            method: "POST",
                            headers: {{ "Content-Type": "application/json" }},
                            body: JSON.stringify(data)
                        }});
                        const res = await resp.json();
                        if (res.success) {{
                            location.reload();
                        }} else {{
                            alert("回测失败: " + res.message);
                        }}
                    }} catch (e) {{
                        alert("请求异常: " + e);
                    }} finally {{
                        loading.style.display = "none";
                        container.style.opacity = "1";
                    }}
                }}
            </script>
        </body>
        </html>"#,
        BACKTEST_UI_CSS,
        Local::now().format("%Y-%m-%d"),
        report_html
    ))
}

const BACKTEST_UI_CSS: &str = r#"
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
<style>
    :root {
        --primary-color: #0052D9;
        --bg-color: #F3F5F8;
        --card-bg: #FFFFFF;
        --text-main: #1D2129;
        --text-muted: #86909C;
        --up-color: #F53F3F;
        --down-color: #00B42A;
        --border-color: #E5E6EB;
    }
    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: var(--bg-color); color: var(--text-main); margin: 0; line-height: 1.5; }
    .container { max-width: 1100px; margin: 0 auto; padding: 20px; }
    .card { background: var(--card-bg); border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); padding: 24px; border: 1px solid var(--border-color); }
    .card-header { border-bottom: 1px solid var(--border-color); margin: -24px -24px 20px -24px; padding: 16px 24px; display: flex; justify-content: space-between; align-items: center; }
    .card-header h3 { margin: 0; font-size: 1.1rem; }
    .btn { padding: 8px 16px; border-radius: 4px; border: none; cursor: pointer; font-weight: 500; font-size: 0.9rem; transition: all 0.2s; }
    .btn-primary { background: var(--primary-color); color: white; }
    .btn-primary:hover { background: #0045B5; }
    .operation-stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 16px; }
    .stat-label { display: block; font-size: 0.8rem; color: var(--text-muted); margin-bottom: 4px; }
    .stat-value { font-size: 1.2rem; font-weight: 700; color: var(--primary-color); }
    .table-container { width: 100%; overflow-x: auto; }
    table { width: 100%; border-collapse: collapse; margin-top: 10px; font-size: 0.9rem; }
    th { text-align: left; background: #F7F8FA; padding: 12px; border-bottom: 1px solid var(--border-color); color: var(--text-muted); font-weight: 500; }
    td { padding: 12px; border-bottom: 1px solid var(--border-color); }
    .header { background: white; border-bottom: 1px solid var(--border-color); margin-bottom: 24px; position: sticky; top: 0; z-index: 100; }
    .header-content { max-width: 1100px; margin: 0 auto; padding: 12px 20px; display: flex; justify-content: space-between; align-items: center; }
    .logo { font-size: 1.2rem; font-weight: 800; color: var(--primary-color); }
    .nav { display: flex; gap: 24px; }
    .nav a { text-decoration: none; color: var(--text-muted); font-weight: 500; font-size: 0.95rem; }
    .nav a.active { color: var(--primary-color); }
    .form-group label { display: block; font-size: 0.85rem; margin-bottom: 6px; font-weight: 500; }
    .form-group input { width: 100%; padding: 8px 12px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box; }
    .spinner { width: 40px; height: 40px; border: 4px solid rgba(0,82,217,0.1); border-top-color: var(--primary-color); border-radius: 50%; animation: spin 1s linear infinite; margin: 0 auto; }
    @keyframes spin { to { transform: rotate(360deg); } }
</style>
"#;

async fn api_backtest_run_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BacktestRunForm>,
) -> Json<serde_json::Value> {
    let ctx = RepositoryContext::default();
    let config = match state.repo.load_config(&ctx).await {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    };

    let req = models::BacktestRequest {
        start_date: payload.start_date,
        end_date: payload.end_date,
        initial_cash: payload.initial_cash,
        portfolio_id: ctx.portfolio_id.clone(),
        policy_override: None,
        include_baseline: payload.include_baseline,
    };

    match engine::backtest::run_backtest(state.repo.as_ref(), &ctx, &config, req).await {
        Ok(report) => {
            let mut last_report = state.last_backtest_report.write().await;
            *last_report = Some(report.clone());
            Json(serde_json::json!({ "success": true, "report": report }))
        }
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    }
}

async fn api_backtest_latest_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let report_opt = state.last_backtest_report.read().await;
    if let Some(report) = report_opt.as_ref() {
        Json(serde_json::json!({ "success": true, "report": report }))
    } else {
        Json(serde_json::json!({ "success": false, "message": "No backtest report found" }))
    }
}

// Cash structs
#[derive(Deserialize)]
struct AssetIdForm {
    asset_id: String,
}

#[derive(Deserialize)]
struct CashSetForm {
    amount: f64,
}

#[derive(Deserialize)]
struct CashAdjustForm {
    amount: f64, // positive for cash_in, negative for cash_out
}

async fn cash_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let portfolio_state = state.repo.load_state(&ctx).await.unwrap_or_default();

    let content = format!(
        r#"
        <div style="max-width: 600px; margin: 0 auto;">
            <h1>现金管理</h1>
            <div class="card">
                <div class="card-header"><span class="card-title">当前可用现金</span></div>
                <div class="card-value">{:.2}</div>
            </div>
            
            <div class="card" style="margin-top: 24px;">
                <div class="card-header"><span class="card-title">调整现金</span></div>
                <form action="/api/cash/adjust" method="post" style="margin-bottom: 24px;">
                    <div class="form-group">
                        <label>金额 (正数为转入，负数为转出)</label>
                        <input type="number" name="amount" step="0.01" required placeholder="例如: 10000.00">
                    </div>
                    <button type="submit" class="btn">提交调整</button>
                </form>
                
                <hr style="border: none; border-top: 1px solid var(--border-color); margin: 24px 0;" />
                
                <div class="card-header"><span class="card-title">设置初始现金 (覆盖)</span></div>
                <form action="/api/cash/set-initial" method="post">
                    <div class="form-group">
                        <label>直接设置现金余额</label>
                        <input type="number" name="amount" step="0.01" required placeholder="例如: 100000.00">
                    </div>
                    <button type="submit" class="btn btn-outline" onclick="return confirm('警告：这会直接覆盖当前现金余额并生成流水，确定吗？')">强行设置</button>
                </form>
            </div>
        </div>
        "#,
        portfolio_state.cash
    );
    layout("现金管理", content)
}

async fn api_cash_set_initial_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CashSetForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let tx = crate::models::Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        date: Local::now().format("%Y-%m-%d").to_string(),
        transaction_type: "cash_set".to_string(),
        asset_id: None,
        amount: form.amount,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "Web端初始现金设定".to_string(),
        source: "manual".to_string(),
        raw_description: "Initial cash set".to_string(),
    };
    let mut transactions = state.repo.load_transactions(&ctx).await.unwrap_or_default();
    transactions.push(tx);
    let _ = state.repo.save_transactions(&ctx, &transactions).await;
    if let Ok(new_state) =
        crate::engine::holdings::rebuild_holdings_from_transactions(&transactions)
    {
        let _ = state.repo.save_state(&ctx, &new_state).await;
    }
    Redirect::to("/dashboard")
}

async fn api_cash_adjust_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CashAdjustForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let tx_type = if form.amount >= 0.0 {
        "cash_in"
    } else {
        "cash_out"
    };
    let amount = form.amount.abs();
    let tx = crate::models::Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        date: Local::now().format("%Y-%m-%d").to_string(),
        transaction_type: tx_type.to_string(),
        asset_id: None,
        amount,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "Web端现金调整".to_string(),
        source: "manual".to_string(),
        raw_description: format!("Cash {}", tx_type),
    };
    let mut transactions = state.repo.load_transactions(&ctx).await.unwrap_or_default();
    transactions.push(tx);
    let _ = state.repo.save_transactions(&ctx, &transactions).await;
    if let Ok(new_state) =
        crate::engine::holdings::rebuild_holdings_from_transactions(&transactions)
    {
        let _ = state.repo.save_state(&ctx, &new_state).await;
    }
    Redirect::to("/dashboard")
}

async fn api_assets_auto_classify_handler(State(state): State<Arc<AppState>>) -> Redirect {
    let ctx = RepositoryContext::default();
    if let Ok(mut config) = state.repo.load_config(&ctx).await {
        let mut changed = 0;
        for asset in &mut config.assets {
            if asset.sector.is_empty() || asset.sector == "未分类" || asset.sector == "待确认"
            {
                let name = asset.fund_name.to_lowercase();

                let mut new_sector = None;

                if name.contains("纳斯达克科技")
                    || name.contains("nasdaq tech")
                    || name.contains("nasdaq100")
                    || name.contains("纳斯达克100")
                    || name.contains("nasdaq")
                    || name.contains("qqq")
                {
                    new_sector = Some("美国科技".to_string());
                } else if name.contains("标普500")
                    || name.contains("s&p 500")
                    || name.contains("s&p500")
                    || name.contains("spy")
                    || name.contains("ivv")
                    || name.contains("voo")
                {
                    new_sector = Some("美国大盘".to_string());
                } else if name.contains("生物科技")
                    || name.contains("创新药")
                    || name.contains("医疗")
                    || name.contains("biotech")
                    || name.contains("医药")
                {
                    new_sector = Some("生物科技".to_string());
                } else if name.contains("日经") || name.contains("日本") || name.contains("nikkei")
                {
                    new_sector = Some("日本".to_string());
                } else if name.contains("越南") || name.contains("vietnam") {
                    new_sector = Some("越南".to_string());
                } else if name.contains("印度") || name.contains("india") {
                    new_sector = Some("印度".to_string());
                } else if name.contains("黄金") || name.contains("gold") {
                    new_sector = Some("黄金".to_string());
                } else if name.contains("债")
                    || name.contains("国开")
                    || name.contains("同业存单")
                    || name.contains("中短债")
                    || name.contains("美元债")
                    || name.contains("bond")
                {
                    new_sector = Some("债券".to_string());
                } else if name.contains("dax")
                    || name.contains("德国")
                    || name.contains("cac40")
                    || name.contains("法国")
                    || name.contains("欧洲")
                    || name.contains("euro")
                {
                    new_sector = Some("欧洲".to_string());
                } else if name.contains("商品")
                    || name.contains("抗通胀")
                    || name.contains("commodity")
                {
                    new_sector = Some("商品".to_string());
                } else if name.contains("富时100") || name.contains("英国") || name.contains("ftse")
                {
                    new_sector = Some("欧洲".to_string());
                }

                if let Some(s) = new_sector {
                    if asset.sector != s {
                        asset.sector = s;
                        changed += 1;
                    }
                } else if asset.sector.is_empty() || asset.sector == "未分类" {
                    asset.sector = "待确认".to_string();
                    changed += 1;
                }
            }
        }
        if changed > 0 {
            let _ = state.repo.save_config(&ctx, &config).await;
        }
    }
    Redirect::to("/dashboard")
}

async fn template_transactions_handler() -> (axum::http::HeaderMap, String) {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_static("attachment; filename=transactions_template.csv"),
    );

    let content = "交易日期,交易类型,资产代码,资产名称,金额,份额,价格,手续费,币种,来源,备注\n\
        2024-01-01,buy,000216,华安黄金ETF联接A,1000.0,2.5,400.0,1.2,CNY,manual,示例买入\n\
        2024-01-02,sell,000216,华安黄金ETF联接A,500.0,1.25,400.0,0.6,CNY,manual,示例卖出"
        .to_string();

    (headers, content)
}

async fn template_alipay_holdings_handler() -> (axum::http::HeaderMap, String) {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_static("attachment; filename=alipay_holdings_template.csv"),
    );

    let content = "基金代码,基金名称,持有份额,持有金额,最新净值,净值日期,投入本金,持有收益,持有收益率,来源\n\
        000216,华安黄金ETF联接A,124.45,49782.36,1.23,2024-06-02,45000.0,4782.36,10.6,alipay_screenshot\n\
        000042,财通资管积极配置,5678.9,10234.56,1.80,2024-06-02,10000.0,234.56,2.3,alipay_screenshot"
        .to_string();

    (headers, content)
}

#[derive(Deserialize)]
struct AssetAddForm {
    fund_name: String,
    fund_code: String,
    sector: Option<String>,
}

async fn admin_asset_add_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetAddForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut config = state.repo.load_config(&ctx).await?;

        // Generate a new asset_id if it doesn't exist
        let asset_id = form.fund_code.clone();
        if config.assets.iter().any(|a| a.asset_id == asset_id) {
            anyhow::bail!("资产 ID {} 已存在", asset_id);
        }

        let new_asset = models::AssetConfig {
            asset_id: asset_id.clone(),
            fund_code: form.fund_code.clone(),
            fund_name: form.fund_name.clone(),
            sector: form.sector.unwrap_or_default(),
            currency: "CNY".to_string(),
            market_data_provider: Some("eastmoney".to_string()),
            enabled: true,
            ..Default::default()
        };

        config.assets.push(new_asset);
        state.repo.save_config(&ctx, &config).await?;

        let audit = models::WebAdminAudit {
            audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            actor: "local_web".to_string(),
            actor_user_id: Some(ctx.actor_user_id.clone()),
            target_user_id: Some(ctx.target_user_id.clone()),
            portfolio_id: Some(ctx.portfolio_id.clone()),
            role: Some(ctx.role.clone()),
            action: "add_asset".to_string(),
            target_file: "config.json".to_string(),
            target_id: Some(asset_id),
            old_value_summary: "".to_string(),
            new_value_summary: format!("{:?}", form.fund_name),
            status: "success".to_string(),
            note: None,
        };
        state.repo.append_web_admin_audit(&ctx, audit).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/assets?success=资产已添加"),
        Err(e) => Redirect::to(&format!("/admin/assets?error={}", e)),
    }
}

async fn admin_asset_remove_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetIdForm>,
) -> Redirect {
    let ctx = RepositoryContext::default();
    let result = async {
        let mut config = state.repo.load_config(&ctx).await?;
        if let Some(idx) = config
            .assets
            .iter()
            .position(|a| a.asset_id == form.asset_id)
        {
            config.assets.remove(idx);
            state.repo.save_config(&ctx, &config).await?;

            // Also delete related transactions
            let mut txs = state.repo.load_transactions(&ctx).await.unwrap_or_default();
            let initial_len = txs.len();
            txs.retain(|t| t.asset_id.as_deref() != Some(&form.asset_id));
            if txs.len() < initial_len {
                state.repo.save_transactions(&ctx, &txs).await?;
                if let Ok(new_state) =
                    crate::engine::holdings::rebuild_holdings_from_transactions(&txs)
                {
                    let _ = state.repo.save_state(&ctx, &new_state).await;
                }
            }
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("资产未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Redirect::to("/admin/assets?success=资产已删除"),
        Err(e) => Redirect::to(&format!("/admin/assets?error={}", e)),
    }
}

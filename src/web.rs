use crate::repository::{Repository, RepositoryContext};
use crate::{engine, models};
use anyhow::Result;
use axum::{
    Router,
    extract::{Form, Query, State},
    response::{Html, Redirect},
    routing::{get, post},
};
use chrono::Local;
use std::net::SocketAddr;
use std::sync::Arc;

use serde::Deserialize;

struct AppState {
    repo: Arc<dyn Repository>,
}

pub async fn start_server(port: u16, repo: Arc<dyn Repository>) -> Result<()> {
    let app_state = Arc::new(AppState { repo });

    let app = Router::new()
        .route("/", get(dashboard_handler))
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
        .route(
            "/admin/assets/set-fund-code",
            post(admin_asset_set_fund_code_handler),
        )
        .route("/admin/assets/rename", post(admin_asset_rename_handler))
        .route(
            "/admin/assets/set-sector",
            post(admin_asset_set_sector_handler),
        )
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
        .route("/daily", get(daily_handler))
        .route("/reports", get(reports_handler))
        .route("/instruments", get(instruments_handler))
        .route("/dca", get(dca_handler))
        .route("/dca/settlements", get(dca_settlements_handler))
        .route("/dca/lifecycle", get(dca_lifecycle_handler))
        .route("/reconcile", get(reconcile_handler))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Starting web server at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn layout(title: &str, content: String) -> Html<String> {
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
            r#"<div class="message-banner message-success">{}</div>"#,
            s
        ));
    }
    if let Some(e) = error {
        msg_html.push_str(&format!(
            r#"<div class="message-banner message-error">{}</div>"#,
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
            --bg-color: #F3F5F8;
            --card-bg: #FFFFFF;
            --text-main: #1D2129;
            --text-muted: #86909C;
            --border-color: #E5E6EB;
            --up-color: #F53F3F; /* Red for profit/up in China */
            --down-color: #00B42A; /* Green for loss/down in China */
            --warn-color: #FF7D00;
            --info-color: #165DFF;
            --nav-bg: #FFFFFF;
            --nav-active: #0052D9;
            --shadow: 0 2px 8px rgba(0,0,0,0.06);
        }}
        * {{ box-sizing: border-box; -webkit-tap-highlight-color: transparent; }}
        body {{ 
            font-family: -apple-system, BlinkMacSystemFont, "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif; 
            line-height: 1.5; 
            color: var(--text-main); 
            background-color: var(--bg-color);
            margin: 0;
            padding: 0;
            padding-bottom: 70px; /* Space for bottom nav */
        }}
        
        /* Layout */
        .container {{ max-width: 1200px; margin: 0 auto; padding: 20px; }}
        header {{ background: var(--nav-bg); border-bottom: 1px solid var(--border-color); position: sticky; top: 0; z-index: 100; box-shadow: 0 1px 3px rgba(0,0,0,0.02); }}
        .header-wrap {{ display: flex; align-items: center; justify-content: space-between; padding: 0 20px; height: 60px; }}
        .logo {{ font-weight: 800; font-size: 1.3rem; color: var(--primary-color); text-decoration: none; letter-spacing: -0.5px; }}
        
        /* Desktop Nav */
        .nav-desktop {{ display: flex; gap: 4px; }}
        .nav-desktop a {{ 
            color: var(--text-main); 
            text-decoration: none; 
            padding: 8px 14px; 
            font-size: 0.95rem; 
            font-weight: 600;
            border-radius: 6px;
            transition: all 0.2s;
        }}
        .nav-desktop a:hover {{ background: #F2F3F5; color: var(--primary-color); }}
        .nav-desktop a.active {{ color: var(--nav-active); background: #E8F3FF; }}
        
        /* Mobile Bottom Nav */
        .nav-bottom {{ 
            display: none; 
            position: fixed; 
            bottom: 0; 
            left: 0; 
            right: 0; 
            height: 64px; 
            background: var(--nav-bg); 
            border-top: 1px solid var(--border-color); 
            z-index: 1000;
            justify-content: space-around;
            align-items: center;
            padding-bottom: env(safe-area-inset-bottom);
            box-shadow: 0 -2px 10px rgba(0,0,0,0.05);
        }}
        .nav-item {{ 
            display: flex; 
            flex-direction: column; 
            align-items: center; 
            text-decoration: none; 
            color: var(--text-muted); 
            font-size: 0.7rem;
            flex: 1;
            padding-top: 8px;
            font-weight: 500;
        }}
        .nav-item.active {{ color: var(--nav-active); }}
        .nav-icon {{ font-size: 1.5rem; margin-bottom: 2px; }}

        /* UI Elements */
        .card {{ 
            background: var(--card-bg); 
            border-radius: 12px; 
            padding: 20px; 
            margin-bottom: 20px; 
            box-shadow: var(--shadow); 
            border: 1px solid var(--border-color); 
            transition: transform 0.2s;
        }}
        .card-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; border-bottom: 1px solid #F7F8FA; padding-bottom: 10px; }}
        .card-title {{ font-size: 0.95rem; font-weight: 700; color: var(--text-main); }}
        .card-value {{ font-size: 1.8rem; font-weight: 800; font-family: "DIN Alternate", "Helvetica Neue", Helvetica, Arial, sans-serif; line-height: 1.2; }}
        .card-sub {{ font-size: 0.85rem; color: var(--text-muted); margin-top: 6px; }}
        
        .dashboard-grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 20px; margin-bottom: 20px; }}
        
        h1 {{ font-size: 1.5rem; font-weight: 800; margin-bottom: 20px; color: var(--text-main); }}
        h2 {{ font-size: 1.2rem; font-weight: 700; margin-top: 32px; margin-bottom: 16px; }}
        h3 {{ font-size: 1.05rem; font-weight: 700; margin-top: 24px; margin-bottom: 12px; }}

        /* Tables */
        .table-container {{ background: var(--card-bg); border-radius: 12px; overflow-x: auto; border: 1px solid var(--border-color); margin-bottom: 24px; box-shadow: var(--shadow); }}
        table {{ width: 100%; border-collapse: collapse; font-size: 0.9rem; min-width: 600px; }}
        th {{ background: #F7F8FA; color: var(--text-muted); font-weight: 600; text-align: left; padding: 14px 16px; border-bottom: 1px solid var(--border-color); font-size: 0.85rem; }}
        td {{ padding: 14px 16px; border-bottom: 1px solid #F7F8FA; vertical-align: middle; }}
        tr:hover td {{ background-color: #FBFCFE; }}
        tr:last-child td {{ border-bottom: none; }}
        
        /* Badges & Text */
        .badge {{ display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 0.75rem; font-weight: 700; color: #fff; background: var(--text-muted); white-space: nowrap; }}
        .badge-red {{ background: var(--up-color); }}
        .badge-green {{ background: var(--down-color); }}
        .badge-blue {{ background: var(--info-color); }}
        .badge-orange {{ background: var(--warn-color); }}
        .badge-gray {{ background: var(--text-muted); }}
        .badge-outline {{ background: transparent; border: 1.5px solid currentColor; }}
        
        .text-up {{ color: var(--up-color); font-weight: 700; }}
        .text-down {{ color: var(--down-color); font-weight: 700; }}
        .text-warn {{ color: var(--warn-color); font-weight: 600; }}
        .text-muted {{ color: var(--text-muted); }}
        
        /* Messages */
        .message-banner {{ padding: 14px 20px; margin-bottom: 20px; border-radius: 10px; font-size: 0.95rem; border: 1px solid transparent; font-weight: 500; }}
        .message-success {{ background: #E8FFEA; color: #008026; border-color: #AFF0B5; }}
        .message-error {{ background: #FFECE8; color: #AD352F; border-color: #FFD2CC; }}
        
        .admin-warning {{ background: #FFF7E8; color: #996000; padding: 10px 20px; font-size: 0.85rem; text-align: center; border-bottom: 1px solid #FFE4BA; font-weight: 600; }}
        
        /* Future Leaderboard / Ranking */
        .ranking-card {{ display: flex; align-items: center; padding: 12px; gap: 16px; border-bottom: 1px solid #F7F8FA; }}
        .ranking-row {{ display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; background: var(--card-bg); border-radius: 8px; margin-bottom: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.02); transition: transform 0.1s; cursor: pointer; }}
        .ranking-row:hover {{ transform: scale(1.01); box-shadow: 0 4px 12px rgba(0,0,0,0.05); }}
        .ranking-pos {{ width: 28px; font-weight: 900; color: var(--text-muted); text-align: center; font-size: 1.2rem; }}
        .ranking-pos-1 {{ color: #FFD700; font-size: 1.4rem; }}
        .ranking-pos-2 {{ color: #C0C0C0; }}
        .ranking-pos-3 {{ color: #CD7F32; }}
        .metric-pill {{ background: #F2F3F5; padding: 3px 12px; border-radius: 14px; font-size: 0.8rem; font-weight: 700; color: var(--text-main); }}
        .performance-badge {{ padding: 4px 8px; border-radius: 6px; font-weight: 800; font-size: 0.9rem; }}
        .public-profile-card {{ display: flex; align-items: center; gap: 16px; padding: 16px; background: linear-gradient(135deg, #1D2129 0%, #4E5969 100%); color: white; border-radius: 12px; margin-bottom: 20px; }}
        .profile-avatar {{ width: 48px; height: 48px; background: rgba(255,255,255,0.2); border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 1.5rem; border: 2px solid rgba(255,255,255,0.3); }}

        /* Forms */
        .form-group {{ margin-bottom: 20px; }}
        .form-group label {{ display: block; margin-bottom: 8px; font-size: 0.9rem; font-weight: 700; color: var(--text-main); }}
        input[type="text"], input[type="number"], select, textarea {{ 
            width: 100%; padding: 12px 14px; border: 1.5px solid var(--border-color); border-radius: 10px; font-size: 1rem; outline: none; transition: border-color 0.2s; background: #FFF;
        }}
        input:focus, select:focus, textarea:focus {{ border-color: var(--primary-color); box-shadow: 0 0 0 3px rgba(0, 82, 217, 0.1); }}
        .btn {{ 
            display: inline-block; padding: 12px 24px; background: var(--primary-color); color: #fff; text-decoration: none; border-radius: 10px; 
            font-size: 1rem; font-weight: 700; border: none; cursor: pointer; text-align: center; transition: all 0.2s; box-shadow: 0 2px 4px rgba(0, 82, 217, 0.2);
        }}
        .btn:hover {{ opacity: 0.9; transform: translateY(-1px); box-shadow: 0 4px 8px rgba(0, 82, 217, 0.3); }}
        .btn-danger {{ background: var(--up-color); box-shadow: 0 2px 4px rgba(245, 63, 63, 0.2); }}
        .btn-success {{ background: var(--down-color); box-shadow: 0 2px 4px rgba(0, 180, 42, 0.2); }}
        .btn-outline {{ background: transparent; border: 1.5px solid var(--border-color); color: var(--text-main); box-shadow: none; }}
        .btn-outline:hover {{ background: #F7F8FA; border-color: var(--text-muted); transform: none; box-shadow: none; }}

        .score-meter-wrap {{ width: 100px; }}
        .score-meter {{ height: 8px; width: 100%; background: linear-gradient(to right, var(--down-color), #f1c40f, var(--up-color)); border-radius: 4px; position: relative; }}
        .score-pointer {{ position: absolute; top: -4px; width: 4px; height: 16px; background: var(--text-main); border-radius: 2px; transform: translateX(-50%); border: 1px solid #FFF; }}

        @media (max-width: 768px) {{
            body {{ padding-bottom: 80px; }}
            .container {{ padding: 16px; }}
            .nav-desktop {{ display: none; }}
            .nav-bottom {{ display: flex; }}
            .dashboard-grid {{ grid-template-columns: 1fr; }}
            table {{ min-width: unset; }}
            .card-value {{ font-size: 1.6rem; }}
            h1 {{ font-size: 1.3rem; }}
            th, td {{ padding: 12px 8px; font-size: 0.85rem; }}
            .mobile-hide {{ display: none; }}
        }}
    </style>
</head>
<body>
    <header>
        <div class="header-wrap">
            <a href="/" class="logo">JDI PORTFOLIO</a>
            <nav class="nav-desktop">
                <a href="/ops">操作台</a>
                <a href="/daily">今日</a>
                <a href="/holdings">持仓</a>
                <a href="/dca">定投</a>
                <a href="/reconcile">对账</a>
                <a href="/instruments">市场</a>
                <a href="/reports">报告</a>
                <a href="/admin">管理</a>
            </nav>
        </div>
    </header>

    <main class="container">
        {}
        {}
    </main>

    <nav class="nav-bottom">
        <a href="/ops" class="nav-item">
            <span class="nav-icon">📊</span>
            <span>操作台</span>
        </a>
        <a href="/daily" class="nav-item">
            <span class="nav-icon">📅</span>
            <span>今日</span>
        </a>
        <a href="/holdings" class="nav-item">
            <span class="nav-icon">💰</span>
            <span>持仓</span>
        </a>
        <a href="/dca" class="nav-item">
            <span class="nav-icon">🔄</span>
            <span>定投</span>
        </a>
        <a href="/reconcile" class="nav-item">
            <span class="nav-icon">⚖</span>
            <span>对账</span>
        </a>
        <a href="/admin" class="nav-item">
            <span class="nav-icon">⚙</span>
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
</body>
</html>
"#,
        title, msg_html, content
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

fn badge_status(status: &str) -> String {
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

async fn dashboard_handler() -> Redirect {
    Redirect::to("/ops")
}

async fn holdings_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
        Ok::<
            (
                models::ConfigRoot,
                models::PortfolioState,
                models::PortfolioSummary,
            ),
            anyhow::Error,
        >((config, portfolio_state, summary))
    }
    .await;

    match result {
        Ok((config, portfolio_state, summary)) => {
            let mut rows = String::new();
            for holding in &portfolio_state.asset_holdings {
                let asset_config = config
                    .assets
                    .iter()
                    .find(|a| a.asset_id == holding.asset_id);
                if !asset_config.map(|a| a.enabled).unwrap_or(false) {
                    continue;
                }

                let fund_name = asset_config
                    .map(|a| a.fund_name.as_str())
                    .unwrap_or("Unknown");
                let sector = asset_config.map(|a| a.sector.as_str()).unwrap_or("Unknown");
                let nav_str = holding
                    .latest_nav
                    .map(|n| format!("{:.4}", n))
                    .unwrap_or_else(|| "0.0000".to_string());
                let nav_date = holding.latest_nav_date.as_deref().unwrap_or("-");
                let status = holding.latest_nav_status.as_deref().unwrap_or("正常");

                let market_value = holding.last_market_value;
                let cost = holding.cost_basis;
                let pnl = market_value - cost;

                let weight_total = market_value / summary.total_asset_value;
                let weight_equity = market_value / summary.equity_value;

                let pnl_pct_val = if cost.abs() > 0.001 { pnl / cost } else { 0.0 };
                let pnl_class = color_class(pnl);
                let pnl_sign = if pnl > 0.001 { "+" } else { "" };

                rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-weight: 700; color: var(--text-main); font-size: 1.05rem;'>{}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted); margin-top: 2px;'>
                                <code>{}</code> · <span class='badge badge-outline' style='color: var(--text-muted); font-weight: 400; padding: 0 4px;'>{}</span>
                            </div>
                        </td>
                        <td>
                            <div style='font-weight: 700; font-size: 1.05rem;'>{:.2}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted);'>{:.2} 份</div>
                        </td>
                        <td class='{}'>
                            <div style='font-weight: 700; font-size: 1.05rem;'>{}{:.2}</div>
                            <div style='font-size: 0.85rem;'>{}{:.2}%</div>
                        </td>
                        <td>
                            <div style='font-weight: 600;'>{}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted);'>{}</div>
                        </td>
                        <td>
                            <div style='font-size: 0.9rem; font-weight: 600;'>占比: {:.2}%</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted);'>权益: {:.2}%</div>
                        </td>
                        <td>{}</td>
                    </tr>",
                    fund_name,
                    holding.fund_code,
                    sector,
                    market_value,
                    holding.units,
                    pnl_class,
                    pnl_sign,
                    pnl,
                    pnl_sign,
                    pnl_pct_val * 100.0,
                    nav_str,
                    nav_date,
                    weight_total * 100.0,
                    weight_equity * 100.0,
                    badge_status(status)
                ));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 24px; background: #FFF; padding: 20px; border-radius: 12px; border: 1px solid var(--border-color); box-shadow: var(--shadow);">
                    <div>
                        <h1 style="margin-bottom: 4px;">我的持仓 (Portfolio Holdings)</h1>
                        <p style="color: var(--text-muted); font-size: 0.9rem; margin: 0;">实时追踪您的资产市值、盈亏与配置比例</p>
                    </div>
                    <div style="text-align: right;">
                        <div style="font-size: 0.85rem; color: var(--text-muted); font-weight: 600;">权益资产总市值</div>
                        <div style="font-size: 1.8rem; font-weight: 900; color: var(--text-main); letter-spacing: -1px;">{:.2} <small style="font-size: 0.9rem; font-weight: 500;">CNY</small></div>
                    </div>
                </div>

                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>基金名称 / 赛道</th>
                                <th>市值 / 份额</th>
                                <th>持仓盈亏 / 收益率</th>
                                <th>最新净值 / 日期</th>
                                <th>仓位权重</th>
                                <th>数据状态</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>
                "#,
                summary.equity_value, rows
            );

            layout("当前持仓", content)
        }
        Err(e) => layout(
            "当前持仓",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
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
        Ok(transactions) => {
            let mut rows = String::new();
            for tx in transactions {
                let type_cn = match tx.transaction_type.as_str() {
                    "buy" => "买入",
                    "sell" => "卖出",
                    "cash_in" => "现金转入",
                    "cash_out" => "现金转出",
                    "expense" => "支出",
                    "manual_cash_adjustment" | "cash_set" => "手动现金调整",
                    other => other,
                };

                let type_class = match tx.transaction_type.as_str() {
                    "buy" | "cash_in" => "text-up",
                    "sell" | "cash_out" | "expense" => "text-down",
                    _ => "",
                };

                rows.push_str(&format!(
                    "<tr>
                        <td><small><code>{}</code></small></td>
                        <td>{}</td>
                        <td class='{}'><strong>{}</strong></td>
                        <td><code>{}</code></td>
                        <td><strong>{:.2}</strong></td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{:.2}</td>
                        <td><small>{}</small></td>
                        <td><small>{}</small></td>
                    </tr>",
                    tx.id,
                    tx.date,
                    type_class,
                    type_cn,
                    tx.asset_id.as_deref().unwrap_or("-"),
                    tx.amount,
                    tx.units
                        .map(|u| format!("{:.2}", u))
                        .unwrap_or_else(|| "-".to_string()),
                    tx.price
                        .map(|p| format!("{:.2}", p))
                        .unwrap_or_else(|| "-".to_string()),
                    tx.fee,
                    tx.currency,
                    tx.note
                ));
            }

            let content = format!(
                r#"
                <h1>交易记录</h1>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>交易ID</th>
                                <th>日期</th>
                                <th>类型</th>
                                <th>资产ID</th>
                                <th>金额</th>
                                <th>份额</th>
                                <th>价格</th>
                                <th>费用</th>
                                <th>币种</th>
                                <th>备注</th>
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

            layout("交易记录", content)
        }
        Err(e) => layout(
            "交易记录",
            format!("<div class='warning-box'>数据加载失败: {}</div>", e),
        ),
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
                    badge_status("正常")
                } else {
                    badge_status("已禁用")
                };

                rows.push_str(&format!(
                    "<tr>
                        <td><code>{}</code></td>
                        <td>{}</td>
                        <td><strong>{}</strong></td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td><code>{}</code></td>
                        <td><code>{}</code></td>
                        <td><code>{}</code></td>
                        <td>{}</td>
                    </tr>",
                    asset.asset_id,
                    asset.fund_code,
                    asset.fund_name,
                    asset.sector,
                    asset.currency,
                    asset.valuation_method,
                    status_badge,
                    asset.reference_index_name.as_deref().unwrap_or("-"),
                    asset.reference_index_symbol.as_deref().unwrap_or("-"),
                    asset.reference_instrument_id.as_deref().unwrap_or("-"),
                    asset.reference_instrument_symbol.as_deref().unwrap_or("-"),
                    asset.market_data_provider.as_deref().unwrap_or("-"),
                ));
            }

            let content = format!(
                r#"
                <h1>资产列表</h1>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>资产ID</th>
                                <th>基金代码</th>
                                <th>基金名称</th>
                                <th>赛道</th>
                                <th>币种</th>
                                <th>估值方法</th>
                                <th>状态</th>
                                <th>参考指数</th>
                                <th>指数代码</th>
                                <th>标的ID</th>
                                <th>标的代码</th>
                                <th>行情来源</th>
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

            layout("资产列表", content)
        }
        Err(e) => layout(
            "资产列表",
            format!("<div class='warning-box'>数据加载失败: {}</div>", e),
        ),
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
                        || res.warning.as_ref().map_or(false, |w| w.contains("汇率")))
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
                    let z_class = z_val.map(|z| color_class(z)).unwrap_or("");
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
                let z_class = f.z_score.map(|z| color_class(z)).unwrap_or("");

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
            format!(
                "<div class='warning-box'>暂无风险缓存数据，请先在 CLI 运行 <code>cargo run -- data refresh --risk</code></div>"
            ),
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
                if let Some(s) = symbol_opt {
                    if s == entry.symbol {
                        regimes.insert(asset.asset_id.clone(), entry.result.clone());
                    }
                }
            }
        }

        let preview =
            engine::kelly::calculate_kelly_preview(&config, &decision, &risk_overlay, &regimes);

        Ok::<models::KellyPortfolioPreview, anyhow::Error>(preview)
    }
    .await;

    match result {
        Ok(preview) => {
            let mut result_rows = String::new();
            for res in &preview.results {
                let pnl_class = if res.kelly_multiplier > 1.0 {
                    "text-up"
                } else if res.kelly_multiplier < 1.0 {
                    "text-down"
                } else {
                    ""
                };

                result_rows.push_str(&format!(
                    "<tr>
                        <td>{}</td>
                        <td><code>{}</code><br><small>{}</small></td>
                        <td>{:.2}</td>
                        <td>{:.1}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td class='{}'><strong>{:.2}x</strong></td>
                        <td><strong>{:.2}</strong></td>
                        <td>{}</td>
                    </tr>",
                    res.sector,
                    res.asset_id,
                    res.fund_code,
                    res.base_suggested_buy,
                    res.pendulum_score,
                    badge_regime(&res.market_regime_label),
                    badge_risk(&res.global_risk_label),
                    pnl_class,
                    res.kelly_multiplier,
                    res.capped_preview_buy_amount,
                    badge_status(&res.status)
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
                <h1>Kelly 仓位预览</h1>
                
                <div class="dashboard-grid">
                    <div class="card">
                        <h3>组合总倍率</h3>
                        <div class="value">{:.2}x</div>
                        <div class="sub-value">相对于基础建议</div>
                    </div>
                    <div class="card">
                        <h3>基础总买入</h3>
                        <div class="value">{:.2}</div>
                        <div class="sub-value">未调节金额</div>
                    </div>
                    <div class="card">
                        <h3>Kelly 预览总买入</h3>
                        <div class="value">{:.2}</div>
                        <div class="sub-value">调节后金额</div>
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
                                <th>钟摆分数</th>
                                <th>市场状态</th>
                                <th>全局风险</th>
                                <th>Kelly 倍率</th>
                                <th>预览买入</th>
                                <th>状态</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="warning-box" style="background-color: #e8f4fd; border-left-color: #3498db; color: #2c3e50;">
                    <strong>模型说明:</strong><br>
                    1. 该结果仅为预览，<strong>不会</strong>自动执行买入，也<strong>不会</strong>修改组合状态。<br>
                    2. 胜率 p 和 赔率 b 是基于当前市场周期和全局风险指标的估算值。<br>
                    3. Kelly 倍率 = 基础倍率 * (1 + 2 * 分段 Kelly 分数)。<br>
                    4. 极高风险或市场过热时，倍率会自动大幅降低甚至归零。<br>
                    <br>
                    <strong>中文警告:</strong> Kelly 参数基于模型估计，并非真实胜率。该结果仅用于仓位参考，不应被视为确定性预测。
                </div>
                "#,
                preview.total_multiplier,
                preview.base_total_buy,
                preview.preview_total_buy,
                badge_risk(&preview.global_risk_label),
                preview.global_risk_score,
                warnings_html,
                result_rows
            );

            layout("Kelly 预览", content)
        }
        Err(e) => layout(
            "Kelly 预览",
            format!("<div class='warning-box'>Kelly 数据加载失败: {}</div>", e),
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
                if let Some(s) = symbol_opt {
                    if s == entry.symbol {
                        regimes.insert(asset.asset_id.clone(), entry.result.clone());
                    }
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
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let plans = state.repo.load_plans(&ctx).await?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();

        let dca_preview = engine::dca::calculate_dca_preview(&config, &plans, &date);
        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date.clone());

        Ok::<
            (
                models::DcaPreviewSummary,
                Vec<models::DcaPlan>,
                models::ConfigRoot,
                f64,
            ),
            anyhow::Error,
        >((dca_preview, plans, config, decision.suggested_total_buy))
    }
    .await;

    match result {
        Ok((summary, all_plans, config, base_buy)) => {
            let mut plan_rows = String::new();
            for p in all_plans {
                let asset = config.assets.iter().find(|a| a.asset_id == p.asset_id);
                let fund_name = asset.map(|a| a.fund_name.as_str()).unwrap_or("Unknown");

                let freq_str = match p.frequency {
                    models::DcaFrequency::Daily => "每日".to_string(),
                    models::DcaFrequency::Weekly => format!("每周(周{})", p.weekday.unwrap_or(1)),
                    models::DcaFrequency::Monthly => {
                        format!("每月({}日)", p.month_day.unwrap_or(1))
                    }
                };

                let (status_text, status_badge) = if p.enabled {
                    ("启用中", "badge-blue")
                } else {
                    ("已禁用", "badge-gray")
                };

                plan_rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-weight: 700; color: var(--text-main); font-size: 1.05rem;'>{}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted); margin-top: 2px;'><code>{}</code></div>
                        </td>
                        <td style='font-weight: 800; font-size: 1.1rem; font-family: DIN Alternate, Helvetica Neue;'>{:.2}</td>
                        <td><span class='badge badge-outline' style='color: var(--info-color); border-color: var(--info-color); font-weight: 600;'>{}</span></td>
                        <td><span class='badge {}'>{}</span></td>
                        <td style='font-size: 0.9rem;'>{}</td>
                        <td><div style='font-size: 0.85rem; color: var(--text-muted);'>{}</div></td>
                    </tr>",
                    fund_name, p.asset_id, p.amount, freq_str, status_badge, status_text, p.start_date, p.note.as_deref().unwrap_or("-")
                ));
            }

            let mut due_rows = String::new();
            for item in &summary.items {
                if item.status == "今日应投" {
                    let asset = config.assets.iter().find(|a| a.asset_id == item.asset_id);
                    let fund_name = asset.map(|a| a.fund_name.as_str()).unwrap_or("Unknown");

                    due_rows.push_str(&format!(
                        "<tr>
                            <td>
                                <div style='font-weight: 700; color: var(--text-main); font-size: 1.05rem;'>{}</div>
                                <div style='font-size: 0.8rem; color: var(--text-muted);'><code>{}</code></div>
                            </td>
                            <td style='font-weight: 800; font-size: 1.1rem;' class='text-up'>{:.2}</td>
                            <td><span class='badge badge-red'>今日应投</span></td>
                            <td><div style='font-size: 0.85rem; color: var(--warn-color); font-weight: 600;'>{}</div></td>
                        </tr>",
                        fund_name, item.asset_id, item.amount, item.warnings.join(", ")
                    ));
                }
            }

            if due_rows.is_empty() {
                due_rows = "<tr><td colspan='4' style='text-align:center; padding: 48px; color: var(--text-muted); font-weight: 500;'>今日无应投项目</td></tr>".to_string();
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 24px; background: #FFF; padding: 20px; border-radius: 12px; border: 1px solid var(--border-color); box-shadow: var(--shadow);">
                    <div>
                        <h1 style="margin-bottom: 4px;">自动定投计划 (DCA Strategy)</h1>
                        <p style="color: var(--text-muted); font-size: 0.9rem; margin: 0;">设定长期定投规则，系统每日自动计算应投额度</p>
                    </div>
                    <div style="text-align: right;">
                        <a href="/admin/dca" class="btn">管理定投计划 &rarr;</a>
                    </div>
                </div>

                <div class="dashboard-grid">
                    <div class="card">
                        <div class="card-header"><span class="card-title">今日定投应投总额</span></div>
                        <div class="card-value text-up">{:.2} <small style="font-size: 0.9rem; font-weight: 500; opacity: 0.8;">CNY</small></div>
                        <div class="card-sub">日期: {}</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">权益补足建议买入</span></div>
                        <div class="card-value">{:.2} <small style="font-size: 0.9rem; font-weight: 500; opacity: 0.8;">CNY</small></div>
                        <div class="card-sub">基于当前目标仓位缺口</div>
                    </div>
                </div>

                <h2 style="margin-bottom: 16px;">今日待扣款定投 (Today Due)</h2>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>基金名称 / 资产ID</th>
                                <th>应投金额</th>
                                <th>当前状态</th>
                                <th>风险/异常说明</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <h2 style="margin-bottom: 16px;">全部定投计划列表 (All Plans)</h2>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>基金名称 / 资产ID</th>
                                <th>单次金额</th>
                                <th>执行频率</th>
                                <th>计划状态</th>
                                <th>开始日期</th>
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
                        💡 <strong>定投说明:</strong><br>
                        • <strong>今日应投</strong> 是基于您设定的频率（每日/每周/每月）计算出的今日应扣款金额。<br>
                        • 最终实际建议买入额会综合考虑 <strong>权益仓位缺口</strong> 与 <strong>全局风险调整</strong>。<br>
                        • 建议在 <strong>“操作台”</strong> 查看综合后的最终执行方案。
                    </p>
                </div>
                "#,
                summary.total_due_amount, summary.date, base_buy, due_rows, plan_rows
            );

            layout("定投计划", content)
        }
        Err(e) => layout(
            "定投计划",
            format!(
                "<div class='message-banner message-error'>定投数据加载失败: {}</div>",
                e
            ),
        ),
    }
}

async fn daily_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let date = Local::now().format("%Y-%m-%d").to_string();

        let dca_plans = state.repo.load_plans(&ctx).await?;
        let dca_preview = engine::dca::calculate_dca_preview(&config, &dca_plans, &date);

        let decision =
            engine::decision::generate_buy_suggestions(&config, &portfolio_state, date.clone());

        // Load caches for risk and regime
        let risk_cache = state.repo.load_risk_cache(&ctx).await?;
        let regime_cache = state.repo.load_regime_cache(&ctx).await?.clone();

        // Default to low/safe values if no cache
        let risk_overlay = if let Some(rc) = risk_cache {
            rc.overlay
        } else {
            models::GlobalRiskOverlay {
                risk_score: 0.0,
                risk_label: "未知(未刷新)".to_string(),
                factor_results: vec![],
                warnings: vec!["请运行 data refresh --risk".to_string()],
                explanation: "请运行 data refresh --risk 以获取准确风险评估。".to_string(),
            }
        };

        let mut regimes = std::collections::HashMap::new();
        for entry in regime_cache.entries {
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

        let adjusted = engine::adjusted_decision::calculate_adjusted_decision(
            &config,
            &portfolio_state,
            &decision,
            &risk_overlay,
            &regimes,
        );
        let kelly =
            engine::kelly::calculate_kelly_preview(&config, &decision, &risk_overlay, &regimes);

        let snapshots = state.repo.load_alipay_snapshots(&ctx).await?;
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
                    &config,
                    &portfolio_state,
                    s,
                ));
            }
        }

        let plan = engine::daily_plan::generate_daily_execution_plan(
            &config,
            &portfolio_state,
            date.clone(),
            &dca_preview,
            &adjusted,
            &kelly,
            &reconciliation_results,
        );

        let settlements = state.repo.load_settlements(&ctx).await?;
        let lifecycle = engine::dca_lifecycle::calculate_dca_lifecycle(
            &config,
            &dca_plans,
            &settlements,
            &snapshots,
            &portfolio_state,
            &date,
        );

        Ok::<(models::DailyExecutionPlan, models::DcaLifecycleSummary), anyhow::Error>((
            plan, lifecycle,
        ))
    }
    .await;

    match result {
        Ok((plan, lifecycle)) => {
            let mut rows = String::new();
            for item in plan.items {
                let badge_class = match item.status.as_str() {
                    "今日应执行" => "badge-red",
                    "暂停执行" | "等待对账" => "badge-gray",
                    "建议观察" | "数据不足" => "badge-orange",
                    _ => "badge-gray",
                };

                rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-weight: 700; color: var(--text-main); font-size: 1.05rem;'>{}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted); margin-top: 2px;'>{}</div>
                        </td>
                        <td>
                            <div style='font-size: 0.85rem; color: var(--text-muted);'>定投: {:.2}</div>
                            <div style='font-size: 0.85rem; color: var(--text-muted);'>加仓: {:.2}</div>
                        </td>
                        <td>
                            <div class='text-up' style='font-size: 1.2rem; font-weight: 900; font-family: DIN Alternate, Helvetica Neue;'>{:.2}</div>
                        </td>
                        <td>{}</td>
                        <td><span class='badge {}'>{}</span></td>
                        <td><div style='font-size: 0.85rem; color: var(--text-muted); max-width: 250px;'>{}</div></td>
                    </tr>",
                    item.fund_name,
                    item.sector,
                    item.dca_due_amount,
                    item.adjusted_decision_amount,
                    item.recommended_amount,
                    badge_status(&item.reconciliation_status),
                    badge_class,
                    item.status,
                    item.explanation
                ));
                if !item.warnings.is_empty() {
                    rows.push_str(&format!(
                        "<tr><td colspan='10' style='font-size: 0.8rem; color: var(--up-color); background-color: #FFF2F0; padding: 4px 16px;'>⚠ {}</td></tr>",
                        item.warnings.join(" | ")
                    ));
                }
            }

            let mut global_warnings_html = String::new();
            if !plan.warnings.is_empty() {
                global_warnings_html = format!(
                    r#"<div class="message-banner message-error" style="margin-bottom: 20px;">
                        <strong>全局警告:</strong> {}
                    </div>"#,
                    plan.warnings.join(" | ")
                );
            }

            let mut lifecycle_reminder = String::new();
            if lifecycle.count_waiting_confirmation > 0 || lifecycle.count_unapplied > 0 {
                lifecycle_reminder = format!(
                    r#"<div class="message-banner message-success" style="background: #E8F3FF; color: #0052D9; border-color: #B2D3FF; margin-bottom: 24px;">
                        📢 您有 <strong>{}</strong> 笔定投待确认，<strong>{}</strong> 笔确认单待入账。建议先处理以保证数据准确。 <a href='/dca/lifecycle' style='color: inherit; font-weight: 700;'>去处理 &rarr;</a>
                    </div>"#,
                    lifecycle.count_waiting_confirmation, lifecycle.count_unapplied
                );
            }

            let content = format!(
                r#"
                {}
                {}
                
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 24px; background: #FFF; padding: 20px; border-radius: 12px; border: 1px solid var(--border-color); box-shadow: var(--shadow);">
                    <div>
                        <h1 style="margin-bottom: 4px;">今日操作建议 (Daily Plan)</h1>
                        <p style="color: var(--text-muted); font-size: 0.9rem; margin: 0;">日期: {} · 建议您根据下表金额执行手动买入</p>
                    </div>
                    <div style="text-align: right;">
                        <div style="font-size: 0.85rem; color: var(--text-muted); font-weight: 600;">建议买入总额</div>
                        <div style="font-size: 1.8rem; font-weight: 900; color: var(--up-color); letter-spacing: -1px;">{:.2} <small style="font-size: 0.9rem; font-weight: 500;">CNY</small></div>
                    </div>
                </div>

                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>基金名称 / 赛道</th>
                                <th>计划详情 (定投/加仓)</th>
                                <th>最终执行建议金额</th>
                                <th>数据状态</th>
                                <th>执行建议</th>
                                <th>决策逻辑说明</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="card" style="background-color: #F7F8FA; border: 1px dashed var(--border-color); padding: 20px;">
                    <h3 style="margin-top: 0;">💡 交易执行建议</h3>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.6; margin-bottom: 0;">
                        1. <strong>优先执行:</strong> 请优先执行状态为 <span class="badge badge-red">今日应执行</span> 的项目。<br>
                        2. <strong>对账先行:</strong> 若项目状态为 <span class="badge badge-orange">等待对账</span>，建议先录入最新快照，确认持仓准确后再执行。<br>
                        3. <strong>风险控制:</strong> 建议金额已根据全局风险因子（VIX、美债等）自动进行了动态调整。
                    </p>
                </div>
                "#,
                global_warnings_html,
                lifecycle_reminder,
                plan.date,
                plan.total_recommended_amount,
                rows
            );

            layout("今日执行", content)
        }
        Err(e) => layout(
            "今日执行",
            format!(
                "<div class='message-banner message-error'>执行计划加载失败: {}</div>",
                e
            ),
        ),
    }
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
async fn reconcile_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let snapshots = state.repo.load_alipay_snapshots(&ctx).await?;

        let mut latest_snaps = std::collections::HashMap::new();
        for s in &snapshots {
            let entry = latest_snaps.entry(s.asset_id.clone()).or_insert(s.clone());
            if s.snapshot_date >= entry.snapshot_date {
                *entry = s.clone();
            }
        }

        let mut results = Vec::new();
        for asset in &config.assets {
            if let Some(s) = latest_snaps.get(&asset.asset_id) {
                let res = engine::reconciliation::reconcile_asset(&config, &portfolio_state, s);
                results.push(Some(res));
            } else {
                results.push(None);
            }
        }

        Ok::<Vec<Option<models::ReconciliationResult>>, anyhow::Error>(results)
    }
    .await;

    match result {
        Ok(results) => {
            let ctx = RepositoryContext::default();
            let config = state.repo.load_config(&ctx).await.unwrap_or_default();
            let mut result_rows = String::new();

            let mut total_diff = 0.0;
            let mut count_diff = 0;

            for (i, res_opt) in results.into_iter().enumerate() {
                let asset = &config.assets[i];
                if let Some(res) = res_opt {
                    let diff_class = if res.market_value_diff.abs() > 1.0 {
                        if res.market_value_diff > 0.0 {
                            "text-up"
                        } else {
                            "text-down"
                        }
                    } else {
                        ""
                    };

                    if res.market_value_diff.abs() > 1.0 {
                        total_diff += res.market_value_diff;
                        count_diff += 1;
                    }

                    let status_badge = match res.status.as_str() {
                        "一致" => badge_status("一致"),
                        "需要校准" | "份额不一致" => {
                            "<span class='badge badge-red'>份额不一</span>".to_string()
                        }
                        "明显差异" => {
                            "<span class='badge badge-red'>明显差异</span>".to_string()
                        }
                        "小幅差异" => {
                            "<span class='badge badge-orange'>小幅差异</span>".to_string()
                        }
                        _ => format!("<span class='badge badge-gray'>{}</span>", res.status),
                    };

                    result_rows.push_str(&format!(
                        "<tr>
                            <td>
                                <div style='font-weight: 700; color: var(--text-main); font-size: 1.05rem;'>{}</div>
                                <div style='font-size: 0.8rem; color: var(--text-muted); margin-top: 2px;'><code>{}</code></div>
                            </td>
                            <td style='font-size: 0.9rem;'>{}</td>
                            <td>
                                <div style='font-size: 0.9rem; font-weight: 600;'>系统: {:.2}</div>
                                <div style='font-size: 0.9rem; color: var(--text-muted);'>支付: {:.2}</div>
                            </td>
                            <td class='{}'>
                                <div style='font-weight: 800; font-size: 1.05rem;'>{:.2}</div>
                                <div style='font-size: 0.8rem;'>{:.2}%</div>
                            </td>
                            <td>
                                <div style='font-size: 0.9rem; font-weight: 600;'>系统: {:.4}</div>
                                <div style='font-size: 0.9rem; color: var(--text-muted);'>支付: {:.4}</div>
                            </td>
                            <td>{}</td>
                            <td><div style='font-size: 0.85rem; color: var(--text-warn); font-weight: 600;'>{}</div></td>
                        </tr>",
                        asset.fund_name,
                        asset.asset_id,
                        res.snapshot_date,
                        res.system_market_value,
                        res.alipay_market_value,
                        diff_class,
                        res.market_value_diff,
                        res.market_value_diff_pct * 100.0,
                        res.system_units.unwrap_or(0.0),
                        res.alipay_units.unwrap_or(0.0),
                        status_badge,
                        res.suggested_action
                    ));
                } else {
                    result_rows.push_str(&format!(
                        "<tr>
                            <td>
                                <div style='font-weight: 700; color: var(--text-main); font-size: 1.05rem;'>{}</div>
                                <div style='font-size: 0.8rem; color: var(--text-muted); margin-top: 2px;'><code>{}</code></div>
                            </td>
                            <td colspan='5' style='text-align: center; color: var(--text-muted); padding: 24px; font-weight: 500;'>未录入支付宝持仓快照</td>
                            <td>{}</td>
                        </tr>",
                        asset.fund_name,
                        asset.asset_id,
                        badge_status("缺失快照")
                    ));
                }
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 24px; background: #FFF; padding: 20px; border-radius: 12px; border: 1px solid var(--border-color); box-shadow: var(--shadow);">
                    <div>
                        <h1 style="margin-bottom: 4px;">支付宝对账与校准 (Reconciliation)</h1>
                        <p style="color: var(--text-muted); font-size: 0.9rem; margin: 0;">对比系统账面价值与支付宝持仓实测值，发现并修正数据差异</p>
                    </div>
                    <div style="text-align: right;">
                        <a href="/admin/reconcile" class="btn">进入录入管理 &rarr;</a>
                    </div>
                </div>

                <div class="dashboard-grid">
                    <div class="card">
                        <div class="card-header"><span class="card-title">累计市值差异</span></div>
                        <div class="card-value {}">{:.2} <small style="font-size: 0.9rem; font-weight: 500; opacity: 0.8;">CNY</small></div>
                        <div class="card-sub">共 {} 项存在显著差异</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">数据同步建议</span></div>
                        <div class="card-value">{}</div>
                        <div class="card-sub">建议操作动作</div>
                    </div>
                </div>

                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>基金名称 / 资产ID</th>
                                <th>快照日期</th>
                                <th>市值对比 (系统/支付)</th>
                                <th>市值绝对差异 (百分比)</th>
                                <th>份额对比 (系统/支付)</th>
                                <th>比对状态</th>
                                <th>建议校准动作</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="card" style="background-color: #F7F8FA; border: 1px dashed var(--border-color); padding: 20px;">
                    <h3 style="margin-top: 0;">ℹ 对账逻辑深度说明</h3>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.6; margin-bottom: 0;">
                        1. <strong>系统市值:</strong> 基于系统记录的份额 × 最后获取的基金净值（NAV）计算得出。<br>
                        2. <strong>支付宝市值:</strong> 您在支付宝 App 中直接看到的当前持有市值快照。<br>
                        3. <strong>差异原因:</strong> 通常由净值更新延迟、未录入的确认单、或红利发放导致。<br>
                        4. <strong>校准操作:</strong> 如果 <strong>份额不一致</strong>，且确信支付宝数据更准，请使用管理后台的“校准”功能同步份额。
                    </p>
                </div>
                "#,
                if total_diff.abs() > 1.0 {
                    if total_diff > 0.0 {
                        "text-up"
                    } else {
                        "text-down"
                    }
                } else {
                    ""
                },
                total_diff,
                count_diff,
                if count_diff > 0 {
                    "需校准"
                } else {
                    "已同步"
                },
                result_rows
            );

            layout("支付宝对账", content)
        }
        Err(e) => layout(
            "支付宝对账",
            format!(
                "<div class='message-banner message-error'>对账数据加载失败: {}</div>",
                e
            ),
        ),
    }
}
async fn instruments_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let cache = state.repo.load_instrument_cache(&ctx).await?;
        let registry = state.repo.load_instruments(&ctx).await.unwrap_or_default();
        Ok::<(models::InstrumentQuoteCache, Vec<models::InstrumentConfig>), anyhow::Error>((
            cache, registry,
        ))
    }
    .await;

    match result {
        Ok((cache, registry)) => {
            let mut rows = String::new();
            if cache.entries.is_empty() {
                rows.push_str("<tr><td colspan='6' style='text-align: center; padding: 64px; color: var(--text-muted); font-weight: 500;'>暂无行情缓存，请先运行 <code>cargo run -- data refresh --instrument</code></td></tr>");
            }

            let mut sorted_entries = cache.entries.clone();
            sorted_entries.sort_by(|a, b| a.symbol.cmp(&b.symbol));

            for q in sorted_entries {
                let inst_config = registry.iter().find(|i| i.symbol == q.symbol);
                let display_name = q
                    .name_zh
                    .as_deref()
                    .or(inst_config.and_then(|c| c.name_zh.as_deref()))
                    .unwrap_or(&q.symbol);
                let asset_class = inst_config
                    .map(|c| c.asset_class.clone())
                    .unwrap_or(models::AssetClass::Custom);
                let category = inst_config
                    .and_then(|c| c.category_zh.as_deref())
                    .unwrap_or("-");

                let asset_class_label = match asset_class {
                    models::AssetClass::Etf => "ETF",
                    models::AssetClass::Index => "指数",
                    models::AssetClass::SpotCommodity => "现货",
                    models::AssetClass::Futures => "期货",
                    models::AssetClass::Fx => "外汇",
                    models::AssetClass::Crypto => "加密",
                    _ => "其它",
                };

                let price_class = if q.price > 0.0 {
                    "text-up"
                } else {
                    "text-muted"
                };

                rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-size: 1.05rem; font-weight: 700; color: var(--text-main);'>{}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted); margin-top: 2px;'>
                                <code>{}</code> · <span class='badge badge-outline' style='color: var(--text-muted); font-weight: 400; padding: 0 4px;'>{}</span>
                            </div>
                        </td>
                        <td class='{}' style='font-size: 1.1rem; font-weight: 800; font-family: DIN Alternate, Helvetica Neue, Helvetica;'>
                            {:.4} <small style='font-size: 0.75rem; font-weight: 500; opacity: 0.8;'>{}</small>
                        </td>
                        <td>
                            <div style='font-size: 0.9rem; font-weight: 500;'>{}</div>
                            <div style='font-size: 0.75rem; color: var(--text-muted);'>{}</div>
                        </td>
                        <td>
                            <div style='font-size: 0.85rem; font-weight: 600;'>{}</div>
                            <div style='font-size: 0.75rem; color: var(--text-muted);'>{}</div>
                        </td>
                        <td>
                            <span class='badge badge-blue badge-outline' style='font-weight: 600;'>{}</span>
                        </td>
                        <td>{}</td>
                    </tr>",
                    display_name,
                    q.symbol,
                    asset_class_label,
                    price_class,
                    q.price,
                    q.currency,
                    q.provider,
                    q.source,
                    category,
                    q.quote_unit,
                    q.status,
                    badge_status("正常")
                ));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 24px; background: #FFF; padding: 20px; border-radius: 12px; border: 1px solid var(--border-color); box-shadow: var(--shadow);">
                    <div>
                        <h1 style="margin-bottom: 4px;">市场自选行情 (Market Watchlist)</h1>
                        <p style="color: var(--text-muted); font-size: 0.9rem; margin: 0;">全球多资产行情实时监控（缓存模式）</p>
                    </div>
                    <div style="text-align: right;">
                        <div style="font-size: 0.85rem; color: var(--text-muted); font-weight: 600;">缓存数据日期</div>
                        <div style="font-size: 1.2rem; font-weight: 700; color: var(--text-main);">{}</div>
                    </div>
                </div>

                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>标的名称 / 代码</th>
                                <th>最新价格 / 货币</th>
                                <th>数据源 / 提供商</th>
                                <th>板块 / 单位</th>
                                <th>行情状态</th>
                                <th>操作建议</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="card" style="background-color: #F7F8FA; border: 1px dashed var(--border-color);">
                    <p style="font-size: 0.85rem; color: var(--text-muted); margin: 0;">
                        💡 <strong>行情说明:</strong> 此处显示的是本地缓存数据。如果价格长期未变，请在终端执行 <code>cargo run -- data refresh --instrument</code> 强制刷新。
                    </p>
                </div>
                "#,
                cache.fetched_at, rows
            );

            layout("市场行情", content)
        }
        Err(e) => layout(
            "市场行情",
            format!(
                "<div class='message-banner message-error'>行情数据加载失败: {}</div>",
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

        let summary = engine::dca_lifecycle::calculate_dca_lifecycle(
            &config,
            &dca_plans,
            &settlements,
            &snapshots,
            &portfolio_state,
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

        let lifecycle = engine::dca_lifecycle::calculate_dca_lifecycle(
            &config,
            &dca_plans,
            &settlements,
            &snapshots,
            &portfolio_state,
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

async fn reports_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = RepositoryContext::default();
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();

        let plans = state.repo.load_plans(&ctx).await?;
        let settlements = state.repo.load_settlements(&ctx).await?;
        let snapshots = state.repo.load_alipay_snapshots(&ctx).await?;

        let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
        let dca_lifecycle = engine::dca_lifecycle::calculate_dca_lifecycle(
            &config,
            &plans,
            &settlements,
            &snapshots,
            &portfolio_state,
            &date,
        );

        let risk_cache = state.repo.load_risk_cache(&ctx).await?;
        let risk_overlay = risk_cache.map(|rc| rc.overlay);

        let report = engine::report::generate_investment_report(
            models::ReportPeriod::Daily,
            &format!("每日复盘报告 - {}", date),
            &date,
            &date,
            Some(summary),
            Some(dca_lifecycle),
            risk_overlay,
            None,
            &[],
        );

        Ok::<models::InvestmentReport, anyhow::Error>(report)
    }
    .await;

    match result {
        Ok(report) => {
            let mut sections_html = String::new();
            for section in report.sections {
                let (badge_text, badge_class) = match section.status.as_str() {
                    "正常" | "一致" | "良" => ("正常", "badge-green"),
                    "警告" | "需要关注" | "注意" => {
                        (section.status.as_str(), "badge-orange")
                    }
                    "错误" | "不一致" | "异常" => (section.status.as_str(), "badge-red"),
                    _ => (section.status.as_str(), "badge-blue"),
                };

                let mut details_html = String::new();
                for detail in section.details {
                    details_html.push_str(&format!(
                        r#"<div style="font-size: 0.85rem; color: #4E5969; margin-bottom: 6px; padding-left: 12px; position: relative;">
                            <span style="position: absolute; left: 0; color: var(--primary-color);">•</span> {}
                        </div>"#,
                        detail
                    ));
                }

                sections_html.push_str(&format!(
                    r#"<div class="card" style="display: flex; flex-direction: column;">
                        <div class="card-header">
                            <span class="card-title" style="font-size: 1.05rem;">{}</span>
                            <span class="badge {}">{}</span>
                        </div>
                        <div style="flex: 1;">
                            <div style="font-size: 0.95rem; font-weight: 700; margin-bottom: 12px; color: var(--text-main);">{}</div>
                            <div style="background: #F7F8FA; padding: 12px; border-radius: 8px;">{}</div>
                        </div>
                    </div>"#,
                    section.title, badge_class, badge_text, section.summary, details_html
                ));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 24px; background: #FFF; padding: 20px; border-radius: 12px; border: 1px solid var(--border-color); box-shadow: var(--shadow);">
                    <div>
                        <h1 style="margin-bottom: 4px;">投资复盘中心 (Review Center)</h1>
                        <p style="color: var(--text-muted); font-size: 0.9rem; margin: 0;">基于多维度数据的智能化投资分析报告</p>
                    </div>
                    <div style="text-align: right;">
                        <div style="font-size: 0.85rem; color: var(--text-muted); font-weight: 600;">报告生成时间</div>
                        <div style="font-size: 1.2rem; font-weight: 700; color: var(--text-main);">{}</div>
                    </div>
                </div>

                <div class="dashboard-grid" style="grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));">
                    {}
                </div>

                <div class="card" style="background-color: #F7F8FA; border: 1px dashed var(--border-color); padding: 20px;">
                    <h3 style="margin-top: 0;">💡 投资复盘建议 (Analysis Strategy)</h3>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.6; margin-bottom: 0;">
                        • <strong>查看时机:</strong> 建议每日收盘后（约 15:30 以后）运行 <code>data refresh --all</code> 后查看此报告。<br>
                        • <strong>关注异常:</strong> 优先处理标记为 <span class="badge badge-orange">需要关注</span> 或 <span class="badge badge-red">不一致</span> 的项。<br>
                        • <strong>定期回顾:</strong> 建议每周、每月进行一次深度复盘，调整大类资产配置比例。
                    </p>
                </div>
                "#,
                report.generated_at, sections_html
            );

            layout("复盘报告", content)
        }
        Err(e) => layout(
            "复盘报告",
            format!(
                "<div class='message-banner message-error'>报告生成失败: {}</div>",
                e
            ),
        ),
    }
}

#[derive(Deserialize)]
struct AdminQuery {
    success: Option<String>,
    error: Option<String>,
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
                    <div class="card-header"><span class="card-title" style="font-size: 1.1rem;">📑 系统审计记录</span></div>
                    <p style="font-size: 0.9rem; color: var(--text-muted); line-height: 1.5;">查看通过 Web 界面进行的各种修改记录。确保每一笔操作均有迹可循。</p>
                </div>
                <div style="margin-top: 16px;">
                    <a href="/admin/audit" class="btn" style="width: 100%;">查看操作记录 &rarr;</a>
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
                            "<span class='badge badge-red'>需要校准</span>".to_string()
                        }
                        "份额不一致" => {
                            "<span class='badge badge-red'>份额不一致</span>".to_string()
                        }
                        "成本不一致" => {
                            "<span class='badge badge-orange'>成本不一致</span>".to_string()
                        }
                        "净值日期不一致" => {
                            "<span class='badge badge-blue'>净值日期不一致</span>".to_string()
                        }
                        "缺少系统持仓" => {
                            "<span class='badge badge-red'>缺少系统持仓</span>".to_string()
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
            let parse_opt_f64 = |opt: Option<f64>| {
                if let Some(v) = opt {
                    if v > 0.0 { Some(v) } else { None }
                } else {
                    None
                }
            };

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
            for a in &config.assets {
                let status_badge = if a.enabled {
                    badge_status("启用")
                } else {
                    "<span class='badge badge-gray'>禁用</span>".to_string()
                };

                let fund_code_form = format!(
                    r#"<form action="/admin/assets/set-fund-code" method="POST" style="display:inline-flex; gap: 4px;">
                        <input type="hidden" name="asset_id" value="{}">
                        <input type="text" name="fund_code" value="{}" style="width: 80px; padding: 4px; font-size: 0.85rem;">
                        <button type="submit" class="btn btn-outline" style="padding: 4px 8px; font-size: 0.75rem;">设置</button>
                    </form>"#,
                    a.asset_id, a.fund_code
                );

                let rename_form = format!(
                    r#"<form action="/admin/assets/rename" method="POST" style="display:inline-flex; gap: 4px;">
                        <input type="hidden" name="asset_id" value="{}">
                        <input type="text" name="fund_name" value="{}" style="width: 140px; padding: 4px; font-size: 0.85rem;">
                        <button type="submit" class="btn btn-outline" style="padding: 4px 8px; font-size: 0.75rem;">更名</button>
                    </form>"#,
                    a.asset_id, a.fund_name
                );

                let sector_form = format!(
                    r#"<form action="/admin/assets/set-sector" method="POST" style="display:inline-flex; gap: 4px;">
                        <input type="hidden" name="asset_id" value="{}">
                        <input type="text" name="sector" value="{}" style="width: 100px; padding: 4px; font-size: 0.85rem;">
                        <button type="submit" class="btn btn-outline" style="padding: 4px 8px; font-size: 0.75rem;">设置</button>
                    </form>"#,
                    a.asset_id, a.sector
                );

                rows.push_str(&format!(
                    "<tr>
                        <td><code>{}</code></td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                    </tr>",
                    a.asset_id, rename_form, fund_code_form, sector_form, status_badge
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
                    <h1>资产配置管理 (Asset Config)</h1>
                    <p style="color: var(--text-muted); font-size: 0.85rem;">维护资产名称、代码及赛道分类，修改后立即生效并同步到所有页面</p>
                </div>

                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>资产 ID</th>
                                <th>显示名称 (Rename)</th>
                                <th>基金代码 (Fund Code)</th>
                                <th>所属板块 (Sector)</th>
                                <th>当前状态</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="card" style="background-color: #F7F8FA; border: 1px dashed var(--border-color); padding: 20px;">
                    <p style="font-size: 0.9rem; color: var(--text-muted); margin: 0; line-height: 1.6;">
                        💡 <strong>配置说明:</strong><br>
                        • <strong>资产 ID:</strong> 系统的唯一标识符，通常不可更改。<br>
                        • <strong>基金代码:</strong> 用于从行情提供商（如天天基金、雅虎财经）抓取数据。<br>
                        • <strong>更名:</strong> 仅修改在 UI 上的显示名称。
                    </p>
                </div>
                "#,
                rows
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
                    badge_status("启用")
                } else {
                    "<span class='badge badge-gray'>禁用</span>".to_string()
                };

                let action_btn = if inst.enabled {
                    format!(
                        r#"<form action="/admin/instruments/disable" method="POST" style="display:inline;">
                            <input type="hidden" name="instrument_id" value="{}">
                            <button type="submit" class="btn btn-outline" style="padding: 4px 8px; font-size: 0.75rem; color: var(--warn-color); border-color: var(--warn-color);">禁用</button>
                        </form>"#,
                        inst.instrument_id
                    )
                } else {
                    format!(
                        r#"<form action="/admin/instruments/enable" method="POST" style="display:inline;">
                            <input type="hidden" name="instrument_id" value="{}">
                            <button type="submit" class="btn btn-outline" style="padding: 4px 8px; font-size: 0.75rem; color: var(--down-color); border-color: var(--down-color);">启用</button>
                        </form>"#,
                        inst.instrument_id
                    )
                };

                let metadata_form = format!(
                    r#"<form action="/admin/instruments/update-metadata" method="POST" style="display:grid; gap: 4px;">
                        <input type="hidden" name="instrument_id" value="{}">
                        <input type="text" name="name_zh" value="{}" placeholder="中文名" style="font-size: 0.85rem; padding: 4px;">
                        <input type="text" name="display_label" value="{}" placeholder="显示标签" style="font-size: 0.85rem; padding: 4px;">
                        <button type="submit" class="btn btn-outline" style="padding: 4px; font-size: 0.75rem;">保存元数据</button>
                    </form>"#,
                    inst.instrument_id,
                    inst.name_zh.as_deref().unwrap_or(""),
                    inst.display_label.as_deref().unwrap_or("")
                );

                rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-weight: 700; color: var(--text-main);'>{}</div>
                            <div style='font-size: 0.75rem; color: var(--text-muted);'><code>{}</code></div>
                        </td>
                        <td>
                            <div style='font-size: 0.85rem;'>{}</div>
                            <div style='font-size: 0.75rem; color: var(--text-muted);'>{}</div>
                        </td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                    </tr>",
                    inst.instrument_id,
                    inst.symbol,
                    inst.name_en.as_deref().unwrap_or("-"),
                    inst.name.as_str(),
                    metadata_form,
                    status_badge,
                    action_btn
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
                    <h1>证券标的主数据管理 (Instrument Registry)</h1>
                    <p style="color: var(--text-muted); font-size: 0.85rem;">维护市场标的的显示名称、本地化标签及行情源启用状态</p>
                </div>

                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>标的代码 / ID</th>
                                <th>系统名称 / 英文名</th>
                                <th style='width: 200px;'>中文显示元数据</th>
                                <th>当前状态</th>
                                <th>管理操作</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="card" style="background-color: #F7F8FA; border: 1px dashed var(--border-color); padding: 20px;">
                    <p style="font-size: 0.9rem; color: var(--text-muted); margin: 0; line-height: 1.6;">
                        💡 <strong>数据说明:</strong><br>
                        • <strong>中文名:</strong> 修改后将在“市场行情”页面优先显示。<br>
                        • <strong>显示标签:</strong> 额外的分类信息，用于 UI 辅助展示。<br>
                        • <strong>禁用:</strong> 禁用后该标的将不再参与自动行情刷新。
                    </p>
                </div>
                "#,
                rows
            );
            layout_with_msg("证券管理", content, query.success, query.error)
        }
        Err(e) => layout(
            "证券管理",
            format!(
                "<div class='message-banner message-error'>加载证券数据失败: {}</div>",
                e
            ),
        ),
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

                if let Some(n) = form.name_zh {
                    if !n.trim().is_empty() {
                        inst.name_zh = Some(n.trim().to_string());
                    }
                }
                if let Some(l) = form.display_label {
                    if !l.trim().is_empty() {
                        inst.display_label = Some(l.trim().to_string());
                    }
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

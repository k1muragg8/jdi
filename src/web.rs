use crate::models::{FxRate, MarketPrice};
use crate::{api, engine, models, storage};
use anyhow::Result;
use axum::{Router, extract::State, response::Html, routing::get};
use chrono::Local;
use std::net::SocketAddr;
use std::sync::Arc;

struct AppState {
    config_path: String,
    state_path: String,
    transactions_path: String,
    dca_plans_path: String,
    alipay_snapshots_path: String,
    instruments_path: String,
}

pub async fn start_server(
    port: u16,
    config_path: String,
    state_path: String,
    transactions_path: String,
    dca_plans_path: String,
    alipay_snapshots_path: String,
    instruments_path: String,
) -> Result<()> {
    let app_state = Arc::new(AppState {
        config_path,
        state_path,
        transactions_path,
        dca_plans_path,
        alipay_snapshots_path,
        instruments_path,
    });

    let app = Router::new()
        .route("/", get(dashboard_handler))
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
        .route("/instruments", get(instruments_handler))
        .route("/dca", get(dca_handler))
        .route("/dca/settlements", get(dca_settlements_handler))
        .route("/reconcile", get(reconcile_handler))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Starting web server at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn layout(title: &str, content: String) -> Html<String> {
    Html(format!(
        r#"
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - JDI Portfolio</title>
    <style>
        :root {{
            --primary-color: #2c3e50;
            --up-color: #e74c3c; /* Red */
            --down-color: #27ae60; /* Green */
            --bg-color: #f4f7f6;
            --card-bg: #ffffff;
            --text-main: #333;
            --text-muted: #7f8c8d;
            --border-color: #eee;
        }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif; line-height: 1.6; color: var(--text-main); max-width: 1400px; margin: 0 auto; padding: 0; background-color: var(--bg-color); }}
        nav {{ background-color: var(--primary-color); padding: 0.5rem 1rem; position: sticky; top: 0; z-index: 1000; display: flex; flex-wrap: wrap; gap: 0.25rem; box-shadow: 0 2px 5px rgba(0,0,0,0.1); }}
        nav a {{ color: rgba(255,255,255,0.8); text-decoration: none; padding: 0.6rem 0.8rem; border-radius: 4px; font-weight: 500; font-size: 0.9rem; transition: all 0.2s; }}
        nav a:hover {{ color: white; background-color: rgba(255,255,255,0.1); }}
        main {{ padding: 20px; }}
        h1, h2, h3 {{ color: var(--primary-color); margin-top: 1.5rem; }}
        .dashboard-grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 1.25rem; margin-bottom: 2rem; }}
        .card {{ background-color: var(--card-bg); padding: 1.25rem; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); border: 1px solid var(--border-color); display: flex; flex-direction: column; }}
        .card h3 {{ margin: 0 0 0.75rem 0; font-size: 0.85rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-muted); border-bottom: 1px solid var(--border-color); padding-bottom: 0.5rem; }}
        .card .value {{ font-size: 1.5rem; font-weight: 700; color: var(--primary-color); }}
        .card .sub-value {{ font-size: 0.85rem; color: var(--text-muted); margin-top: 0.4rem; }}
        .table-container {{ overflow-x: auto; background-color: var(--card-bg); border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); margin-bottom: 2rem; border: 1px solid var(--border-color); }}
        table {{ width: 100%; border-collapse: collapse; font-size: 0.9rem; min-width: 800px; }}
        th, td {{ border: none; border-bottom: 1px solid var(--border-color); padding: 12px 15px; text-align: left; }}
        th {{ background-color: #f8f9fa; color: var(--primary-color); font-weight: 600; position: sticky; top: 0; }}
        tr:last-child td {{ border-bottom: none; }}
        tr:hover {{ background-color: #f9fbfd; }}
        .badge {{ display: inline-block; padding: 0.25em 0.5em; font-size: 0.75rem; font-weight: 600; border-radius: 4px; color: white; background-color: var(--text-muted); }}
        .badge-red {{ background-color: var(--up-color); }}
        .badge-green {{ background-color: var(--down-color); }}
        .badge-blue {{ background-color: #3498db; }}
        .badge-orange {{ background-color: #f39c12; }}
        .badge-gray {{ background-color: #95a5a6; }}
        .text-up {{ color: var(--up-color); font-weight: 600; }}
        .text-down {{ color: var(--down-color); font-weight: 600; }}
        .progress-container {{ width: 100%; background-color: #eee; border-radius: 10px; height: 6px; margin-top: 8px; }}
        .progress-bar {{ height: 100%; border-radius: 10px; }}
        .score-meter-wrap {{ margin: 1rem 0; }}
        .score-meter {{ height: 12px; width: 100%; background: linear-gradient(to right, var(--down-color), #ddd, var(--up-color)); border-radius: 6px; position: relative; }}
        .score-pointer {{ position: absolute; top: -4px; width: 3px; height: 20px; background-color: var(--primary-color); border: 1px solid white; transform: translateX(-50%); }}
        .warning-box {{ background-color: #fff3cd; border-left: 4px solid #ffeeba; color: #856404; padding: 1rem; border-radius: 4px; margin-bottom: 1.5rem; font-size: 0.9rem; }}
        @media (max-width: 768px) {{
            nav {{ justify-content: space-around; }}
            .dashboard-grid {{ grid-template-columns: 1fr; }}
            main {{ padding: 12px; }}
            h1 {{ font-size: 1.5rem; }}
        }}
    </style>
</head>
<body>
    <nav>
        <a href="/">首页</a>
        <a href="/daily">今日执行</a>
        <a href="/holdings">当前持仓</a>
        <a href="/sectors">赛道概览</a>
        <a href="/decisions">今日建议</a>
        <a href="/decision/adjusted">风险调整建议</a>
        <a href="/regime">市场冷热</a>
        <a href="/risk">全局风险</a>
        <a href="/kelly">Kelly预览</a>
        <a href="/dca">定投计划</a>
        <a href="/dca/settlements">定投确认</a>
        <a href="/reconcile">支付宝对账</a>
        <a href="/valuation/proxy">估算净值</a>
        <a href="/transactions">交易记录</a>
        <a href="/assets">资产列表</a>
    </nav>
    <main>
        {}
    </main>
</body>
</html>
"#,
        title, content
    ))
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

async fn dashboard_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state.config_path)?;
        let portfolio_state = storage::load_state(&state.state_path)?;
        let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date);

        let fx_provider = crate::api::create_fx_provider(&config.fx, None);
        let usd_cnh = fx_provider
            .fetch_latest_rate(&config.fx.usd_cnh_symbol)
            .ok();

        let market_provider = crate::api::create_market_provider(&config.market, Some("yahoo"));
        let btc = market_provider.fetch_latest_price("BTC-USD").ok();
        let eth = market_provider.fetch_latest_price("ETH-USD").ok();
        let sol = market_provider.fetch_latest_price("SOL-USD").ok();

        let qqq_regime = market_provider
            .fetch_daily_candles("QQQ", config.regime.default_lookback_days)
            .ok()
            .map(|candles| {
                engine::regime::calculate_market_regime("QQQ", &candles, &config.regime)
            });

        let global_risk = engine::risk_overlay::calculate_risk_overlay(
            &config.risk,
            &config.regime,
            market_provider.as_ref(),
            fx_provider.as_ref(),
        );

        let ret: Result<(
            models::ConfigRoot,
            engine::PortfolioSummary,
            engine::decision::DecisionResult,
            Option<FxRate>,
            Option<MarketPrice>,
            Option<MarketPrice>,
            Option<MarketPrice>,
            Option<models::MarketRegimeResult>,
            models::GlobalRiskOverlay,
        )> = Ok((
            config,
            summary,
            decision,
            usd_cnh,
            btc,
            eth,
            sol,
            qqq_regime,
            global_risk,
        ));
        ret
    })
    .await
    .unwrap();

    match result {
        Ok((config, summary, decision, usd_cnh, btc, eth, sol, qqq_regime, global_risk)) => {
            let base_cur = &config.portfolio.base_currency;

            let mut risk_cards = format!(
                r#"
                <div class="card">
                    <h3>全局风险指数</h3>
                    <div class="value">{}</div>
                    <div class="sub-value">分数: {:.1} / 100</div>
                </div>
                "#,
                badge_risk(&global_risk.risk_label),
                global_risk.risk_score
            );

            if let Some(regime) = qqq_regime {
                risk_cards.push_str(&format!(
                    r#"
                    <div class="card">
                        <h3>QQQ 市场状态</h3>
                        <div class="value">{}</div>
                        <div class="sub-value">钟摆分数: {:.1}</div>
                    </div>
                    "#,
                    badge_regime(&regime.regime_label),
                    regime.pendulum_score
                ));
            }

            if let Some(fx) = usd_cnh {
                risk_cards.push_str(&format!(
                    r#"
                    <div class="card">
                        <h3>USD/CNH 汇率</h3>
                        <div class="value">{:.4}</div>
                        <div class="sub-value">{} | {}</div>
                    </div>
                    "#,
                    fx.rate, fx.source, fx.date
                ));
            }

            for crypto in vec![btc, eth, sol] {
                if let Some(c) = crypto {
                    risk_cards.push_str(&format!(
                        r#"
                        <div class="card">
                            <h3>{} 价格</h3>
                            <div class="value">{:.2}</div>
                            <div class="sub-value">{} | {}</div>
                        </div>
                        "#,
                        c.symbol, c.price, c.source, c.date
                    ));
                }
            }

            let content = format!(
                r#"
                <h1>组合概览</h1>
                <div class="dashboard-grid">
                    <div class="card">
                        <h3>总资产</h3>
                        <div class="value">{:.2} {}</div>
                    </div>
                    <div class="card">
                        <h3>当前现金</h3>
                        <div class="value">{:.2} {}</div>
                        <div class="sub-value">可用现金: {:.2} {}</div>
                    </div>
                    <div class="card">
                        <h3>可用现金占比</h3>
                        <div class="value">{}</div>
                        <div class="sub-value">占总资产比例</div>
                    </div>
                    <div class="card">
                        <h3>现金安全垫</h3>
                        <div class="value">{:.2} {}</div>
                        <div class="sub-value">占比: {}</div>
                    </div>
                    <div class="card">
                        <h3>目标权益仓</h3>
                        <div class="value">{:.2} {}</div>
                    </div>
                    <div class="card">
                        <h3>当前权益仓</h3>
                        <div class="value">{:.2} {}</div>
                        <div class="sub-value">达成率: {}</div>
                    </div>
                    <div class="card">
                        <h3>权益仓占比</h3>
                        <div class="value">{}</div>
                        <div class="sub-value">占总资产比例</div>
                    </div>
                    <div class="card">
                        <h3>权益缺口</h3>
                        <div class="value">{:.2} {}</div>
                        <div class="sub-value">缺口率: {}</div>
                    </div>
                </div>

                <h1>今日买入建议</h1>
                <div class="dashboard-grid">
                    <div class="card">
                        <h3>建议总买入</h3>
                        <div class="value">{:.2} {}</div>
                        <div class="sub-value">单日上限: {:.2} {}</div>
                    </div>
                    <div class="card">
                        <h3>买入上限使用率</h3>
                        <div class="value">{}</div>
                    </div>
                    <div class="card">
                        <h3>数据状态</h3>
                        <div class="value">{}</div>
                    </div>
                </div>

                <h1>风险与行情</h1>
                <div class="dashboard-grid">
                    {}
                </div>
                "#,
                summary.total_asset_value,
                base_cur,
                summary.cash,
                base_cur,
                summary.available_cash,
                base_cur,
                safe_div(summary.available_cash, summary.total_asset_value),
                summary.reserve_cash,
                base_cur,
                safe_div(summary.reserve_cash, summary.total_asset_value),
                summary.target_equity_value,
                base_cur,
                summary.equity_value,
                base_cur,
                safe_div(summary.equity_value, summary.target_equity_value),
                safe_div(summary.equity_value, summary.total_asset_value),
                summary.equity_gap,
                base_cur,
                safe_div(summary.equity_gap, summary.target_equity_value),
                decision.suggested_total_buy,
                base_cur,
                decision.max_daily_buy_total,
                base_cur,
                safe_div(decision.suggested_total_buy, decision.max_daily_buy_total),
                badge_status("正常"),
                risk_cards
            );

            layout("首页", content)
        }
        Err(e) => layout(
            "首页",
            format!("<div class='warning-box'>数据加载失败: {}</div>", e),
        ),
    }
}

async fn holdings_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state.config_path)?;
        let portfolio_state = storage::load_state(&state.state_path)?;
        let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
        Ok::<
            (
                models::ConfigRoot,
                models::PortfolioState,
                engine::PortfolioSummary,
            ),
            anyhow::Error,
        >((config, portfolio_state, summary))
    })
    .await
    .unwrap();

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
                    .unwrap_or_else(|| "N/A".to_string());
                let nav_date = holding.latest_nav_date.as_deref().unwrap_or("N/A");
                let source = holding.latest_nav_source.as_deref().unwrap_or("N/A");
                let status = holding.latest_nav_status.as_deref().unwrap_or("N/A");

                let market_value = holding.last_market_value;
                let cost = holding.cost_basis;
                let pnl = market_value - cost;

                let weight_total = safe_div(market_value, summary.total_asset_value);
                let weight_equity = safe_div(market_value, summary.equity_value);

                let pnl_pct_val = if cost.abs() > 0.001 { pnl / cost } else { 0.0 };
                let pnl_pct_str = fmt_pct(pnl_pct_val);
                let pnl_class = color_class(pnl);

                rows.push_str(&format!(
                    "<tr>
                        <td><code>{}</code></td>
                        <td>{}</td>
                        <td><strong>{}</strong></td>
                        <td>{}</td>
                        <td>{:.2}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td><small>{}</small></td>
                        <td>{}</td>
                        <td>{:.2}</td>
                        <td>{:.2}</td>
                        <td class='{}'><strong>{:.2}</strong><br><small>{}</small></td>
                        <td>{}</td>
                        <td>{}</td>
                    </tr>",
                    holding.asset_id,
                    holding.fund_code,
                    fund_name,
                    sector,
                    holding.units,
                    nav_str,
                    nav_date,
                    source,
                    badge_status(status),
                    market_value,
                    cost,
                    pnl_class,
                    pnl,
                    pnl_pct_str,
                    weight_total,
                    weight_equity
                ));
            }

            let content = format!(
                r#"
                <h1>当前持仓</h1>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>资产ID</th>
                                <th>代码</th>
                                <th>基金名称</th>
                                <th>赛道</th>
                                <th>持有份额</th>
                                <th>最新净值</th>
                                <th>净值日期</th>
                                <th>来源</th>
                                <th>状态</th>
                                <th>当前市值</th>
                                <th>持仓成本</th>
                                <th>浮动盈亏</th>
                                <th>总资产占比</th>
                                <th>权益仓占比</th>
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

            layout("当前持仓", content)
        }
        Err(e) => layout(
            "当前持仓",
            format!("<div class='warning-box'>行情数据获取失败: {}</div>", e),
        ),
    }
}

async fn sectors_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state.config_path)?;
        let portfolio_state = storage::load_state(&state.state_path)?;
        let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
        Ok::<engine::PortfolioSummary, anyhow::Error>(summary)
    })
    .await
    .unwrap();

    match result {
        Ok(summary) => {
            let mut rows = String::new();
            for s in summary.sector_summaries {
                let target_pct_val = s.target_weight;
                let current_pct_val = s.current_weight;

                let target_pct_str = fmt_pct(target_pct_val);
                let current_pct_str = fmt_pct(current_pct_val);
                let gap_ratio_str = if target_pct_val > 0.001 {
                    fmt_pct(s.gap_ratio)
                } else {
                    "N/A".to_string()
                };

                let (_status_cn, status_code) = match s.status.as_str() {
                    "underweight" => ("低配", "低配"),
                    "neutral" => ("均衡", "均衡"),
                    "overweight" => ("超配", "超配"),
                    "disabled" => ("已禁用", "已禁用"),
                    other => (other, other),
                };

                let gap_class = if s.gap_value > 1.0 {
                    "text-down"
                } else if s.gap_value < -1.0 {
                    "text-up"
                } else {
                    ""
                };

                // Progress bar
                let progress_width = (current_pct_val * 100.0).clamp(0.0, 100.0);
                let progress_color = if s.status == "overweight" {
                    "var(--up-color)"
                } else if s.status == "underweight" {
                    "var(--down-color)"
                } else {
                    "var(--primary-color)"
                };

                rows.push_str(&format!(
                    "<tr>
                        <td><strong>{}</strong></td>
                        <td>{}</td>
                        <td>
                            <div>{}</div>
                            <div class='progress-container'>
                                <div class='progress-bar' style='width: {:.1}%; background-color: {};'></div>
                            </div>
                        </td>
                        <td class='{}'>{}</td>
                        <td>{:.2}</td>
                        <td>{:.2}</td>
                        <td class='{}'><strong>{:.2}</strong></td>
                        <td>{}</td>
                    </tr>",
                    s.sector_name, target_pct_str, current_pct_str, progress_width, progress_color, gap_class, gap_ratio_str, s.target_value, s.current_value, gap_class, s.gap_value, badge_status(status_code)
                ));
            }

            let content = format!(
                r#"
                <h1>赛道概览</h1>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>赛道</th>
                                <th>目标占比</th>
                                <th>当前占比</th>
                                <th>缺口比例</th>
                                <th>目标市值</th>
                                <th>当前市值</th>
                                <th>缺口金额</th>
                                <th>状态</th>
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

            layout("赛道概览", content)
        }
        Err(e) => layout(
            "赛道概览",
            format!("<div class='warning-box'>行情数据获取失败: {}</div>", e),
        ),
    }
}

async fn decisions_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state.config_path)?;
        let portfolio_state = storage::load_state(&state.state_path)?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let result = engine::generate_buy_suggestions(&config, &portfolio_state, date);
        Ok::<(models::ConfigRoot, engine::decision::DecisionResult), anyhow::Error>((
            config, result,
        ))
    })
    .await
    .unwrap();

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
    let result =
        tokio::task::spawn_blocking(move || storage::load_transactions(&state.transactions_path))
            .await
            .unwrap();

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
    let result = tokio::task::spawn_blocking(move || storage::load_config(&state.config_path))
        .await
        .unwrap();

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
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state_clone.config_path)?;
        let portfolio_state = storage::load_state(&state_clone.state_path)?;
        let market_provider = crate::api::create_market_provider(&config.market, None);
        let fx_provider = crate::api::create_fx_provider(&config.fx, None);
        let instruments =
            storage::instrument_store::load_instruments(&state_clone.instruments_path)
                .unwrap_or_default();

        let mut adjusted_config = config.clone();
        for asset in &mut adjusted_config.assets {
            if let Some(rid) = &asset.reference_instrument_id {
                if let Some(i) = instruments.iter().find(|i| i.instrument_id == *rid) {
                    asset.reference_instrument_symbol = Some(i.provider_symbol.clone());
                }
            }
        }

        let results = engine::calculate_proxy_valuations(
            &adjusted_config,
            &portfolio_state,
            market_provider.as_ref(),
            fx_provider.as_ref(),
        );
        Ok::<Vec<models::ProxyValuationResult>, anyhow::Error>(results)
    })
    .await
    .unwrap();

    match result {
        Ok(results) => {
            let mut rows = String::new();
            for res in results {
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
                <h1>估值预览 (指数代理估算)</h1>
                <div class="warning-box">
                    <strong>提示:</strong> 估算净值仅用于当日实时参考，不覆盖官方净值，亦不参与当前建议买入金额的计算。
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
                rows
            );

            layout("估算净值", content)
        }
        Err(e) => layout(
            "估算净值",
            format!("<div class='warning-box'>行情数据获取失败: {}</div>", e),
        ),
    }
}

async fn regime_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state_clone.config_path)?;
        let instruments =
            storage::instrument_store::load_instruments(&state_clone.instruments_path)
                .unwrap_or_default();
        let market_provider = crate::api::create_market_provider(&config.market, None);

        let mut target_symbols = Vec::new();
        for asset in &config.assets {
            let symbol_opt = if let Some(rid) = &asset.reference_instrument_id {
                instruments
                    .iter()
                    .find(|i| i.instrument_id == *rid)
                    .map(|i| i.provider_symbol.clone())
            } else {
                asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone())
            };

            if let Some(s) = symbol_opt {
                if !target_symbols.contains(&s) {
                    target_symbols.push(s);
                }
            }
        }

        let mut results = Vec::new();
        for sym in target_symbols {
            if let Ok(candles) =
                market_provider.fetch_daily_candles(&sym, config.regime.default_lookback_days)
            {
                let regime =
                    engine::regime::calculate_market_regime(&sym, &candles, &config.regime);
                results.push(regime);
            }
        }

        Ok::<Vec<models::MarketRegimeResult>, anyhow::Error>(results)
    })
    .await
    .unwrap();

    match result {
        Ok(results) => {
            let mut rows = String::new();
            for res in results {
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

                // Visual score meter
                // score is -100 to 100, pointer position is 0% to 100%
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
                <h1>市场冷热分析</h1>
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
                    <strong>风险提示:</strong> 金融市场收益并不严格服从正态分布，Z-score 仅用于衡量相对偏离程度，不应被理解为确定性预测。
                </div>
                "#,
                rows
            );

            layout("市场冷热", content)
        }
        Err(e) => layout(
            "市场冷热",
            format!("<div class='warning-box'>行情数据获取失败: {}</div>", e),
        ),
    }
}

async fn risk_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state.config_path)?;
        let market_provider = crate::api::create_market_provider(&config.market, None);
        let fx_provider = crate::api::create_fx_provider(&config.fx, None);

        let overlay = engine::risk_overlay::calculate_risk_overlay(
            &config.risk,
            &config.regime,
            market_provider.as_ref(),
            fx_provider.as_ref(),
        );
        Ok::<models::GlobalRiskOverlay, anyhow::Error>(overlay)
    })
    .await
    .unwrap();

    match result {
        Ok(overlay) => {
            let mut factor_rows = String::new();
            for f in overlay.factor_results {
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
                for w in overlay.warnings {
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
                <h1>全局风险覆盖分析</h1>
                <div class="dashboard-grid">
                    <div class="card">
                        <h3>全局风险分数</h3>
                        <div class="value">{:.2} / 100</div>
                        <div class="sub-value">综合各项因子计算</div>
                    </div>
                    <div class="card">
                        <h3>风险等级</h3>
                        <div class="value">{}</div>
                        <div class="sub-value">当前市场总体评估</div>
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
                    <strong>风险提示:</strong> 该评分目前仅用于分析，不作为投资建议。金融数据存在滞后性，请以实际行情为准。
                </div>
                "#,
                overlay.risk_score,
                badge_risk(&overlay.risk_label),
                warning_html,
                explain_list,
                factor_rows
            );

            layout("全局风险", content)
        }
        Err(e) => layout(
            "全局风险",
            format!("<div class='warning-box'>风险数据加载失败: {}</div>", e),
        ),
    }
}

async fn kelly_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state.config_path)?;
        let portfolio_state = storage::load_state(&state.state_path)?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date);

        let market_provider = crate::api::create_market_provider(&config.market, Some("yahoo"));
        let fx_provider = crate::api::create_fx_provider(&config.fx, None);

        let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
            &config.risk,
            &config.regime,
            market_provider.as_ref(),
            fx_provider.as_ref(),
        );

        let instruments =
            storage::instrument_store::load_instruments(&state_clone.instruments_path)
                .unwrap_or_default();
        let mut regimes = std::collections::HashMap::new();
        for asset in &config.assets {
            let symbol_opt = if let Some(rid) = &asset.reference_instrument_id {
                instruments
                    .iter()
                    .find(|i| i.instrument_id == *rid)
                    .map(|i| i.provider_symbol.clone())
            } else {
                asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone())
            };

            if let Some(s) = symbol_opt {
                if let Ok(candles) =
                    market_provider.fetch_daily_candles(&s, config.regime.default_lookback_days)
                {
                    let regime =
                        engine::regime::calculate_market_regime(&s, &candles, &config.regime);
                    regimes.insert(asset.asset_id.clone(), regime);
                }
            }
        }

        let preview =
            engine::kelly::calculate_kelly_preview(&config, &decision, &risk_overlay, &regimes);

        Ok::<models::KellyPortfolioPreview, anyhow::Error>(preview)
    })
    .await
    .unwrap();

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
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state.config_path)?;
        let portfolio_state = storage::load_state(&state.state_path)?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date);

        let market_provider = crate::api::create_market_provider(&config.market, Some("yahoo"));
        let fx_provider = crate::api::create_fx_provider(&config.fx, None);

        let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
            &config.risk,
            &config.regime,
            market_provider.as_ref(),
            fx_provider.as_ref(),
        );

        let instruments =
            storage::instrument_store::load_instruments(&state_clone.instruments_path)
                .unwrap_or_default();
        let mut regimes = std::collections::HashMap::new();
        for asset in &config.assets {
            let symbol_opt = if let Some(rid) = &asset.reference_instrument_id {
                instruments
                    .iter()
                    .find(|i| i.instrument_id == *rid)
                    .map(|i| i.provider_symbol.clone())
            } else {
                asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone())
            };

            if let Some(s) = symbol_opt {
                if let Ok(candles) =
                    market_provider.fetch_daily_candles(&s, config.regime.default_lookback_days)
                {
                    let regime =
                        engine::regime::calculate_market_regime(&s, &candles, &config.regime);
                    regimes.insert(asset.asset_id.clone(), regime);
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
    })
    .await
    .unwrap();

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
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state.config_path)?;
        let portfolio_state = storage::load_state(&state.state_path)?;
        let plans = storage::dca_store::load_dca_plans(&state.dca_plans_path)?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();

        let dca_preview = engine::dca::calculate_dca_preview(&config, &plans, &date);

        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date.clone());

        let market_provider = crate::api::create_market_provider(&config.market, Some("yahoo"));
        let fx_provider = crate::api::create_fx_provider(&config.fx, None);

        let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
            &config.risk,
            &config.regime,
            market_provider.as_ref(),
            fx_provider.as_ref(),
        );

        let instruments =
            storage::instrument_store::load_instruments(&state_clone.instruments_path)
                .unwrap_or_default();
        let mut regimes = std::collections::HashMap::new();
        for asset in &config.assets {
            let symbol_opt = if let Some(rid) = &asset.reference_instrument_id {
                instruments
                    .iter()
                    .find(|i| i.instrument_id == *rid)
                    .map(|i| i.provider_symbol.clone())
            } else {
                asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone())
            };

            if let Some(s) = symbol_opt {
                if let Ok(candles) =
                    market_provider.fetch_daily_candles(&s, config.regime.default_lookback_days)
                {
                    let regime =
                        engine::regime::calculate_market_regime(&s, &candles, &config.regime);
                    regimes.insert(asset.asset_id.clone(), regime);
                }
            }
        }

        let kelly_preview =
            engine::kelly::calculate_kelly_preview(&config, &decision, &risk_overlay, &regimes);

        let adjusted = engine::adjusted_decision::calculate_adjusted_decision(
            &config,
            &portfolio_state,
            &decision,
            &risk_overlay,
            &regimes,
        );

        Ok::<
            (
                models::DcaPreviewSummary,
                Vec<models::DcaPlan>,
                f64,
                f64,
                f64,
            ),
            anyhow::Error,
        >((
            dca_preview,
            plans,
            decision.suggested_total_buy,
            kelly_preview.preview_total_buy,
            adjusted.adjusted_total_buy,
        ))
    })
    .await
    .unwrap();

    match result {
        Ok((summary, all_plans, base_buy, _kelly_buy, adjusted_buy)) => {
            let mut plan_rows = String::new();
            for p in all_plans {
                let freq_str = match p.frequency {
                    models::DcaFrequency::Daily => "每日".to_string(),
                    models::DcaFrequency::Weekly => format!("每周(周{})", p.weekday.unwrap_or(1)),
                    models::DcaFrequency::Monthly => {
                        format!("每月({}日)", p.month_day.unwrap_or(1))
                    }
                };
                let status_badge = if p.enabled {
                    "<span class='badge badge-blue'>启用</span>"
                } else {
                    "<span class='badge badge-gray'>禁用</span>"
                };

                plan_rows.push_str(&format!(
                    "<tr>
                        <td><code>{}</code></td>
                        <td>{}</td>
                        <td>{:.2}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td><small>{}</small></td>
                    </tr>",
                    p.plan_id,
                    p.asset_id,
                    p.amount,
                    freq_str,
                    status_badge,
                    p.start_date,
                    p.note.unwrap_or_default()
                ));
            }

            let mut due_rows = String::new();
            for item in &summary.items {
                if item.status == "今日应投" {
                    due_rows.push_str(&format!(
                        "<tr>
                            <td>{}</td>
                            <td>{:.2}</td>
                            <td>{}</td>
                            <td>{}</td>
                        </tr>",
                        item.asset_id,
                        item.amount,
                        badge_status(&item.status),
                        item.warnings.join(", ")
                    ));
                }
            }

            if due_rows.is_empty() {
                due_rows =
                    "<tr><td colspan='4' style='text-align:center;'>今日无应投项目</td></tr>"
                        .to_string();
            }

            let content = format!(
                r#"
                <h1>定投计划管理</h1>

                <div class="dashboard-grid">
                    <div class="card">
                        <h3>今日应投总额</h3>
                        <div class="value">{:.2} CNY</div>
                        <div class="sub-value">日期: {}</div>
                    </div>
                    <div class="card">
                        <h3>基础建议买入</h3>
                        <div class="value">{:.2} CNY</div>
                        <div class="sub-value">基于目标缺口</div>
                    </div>
                    <div class="card">
                        <h3>风险调整建议</h3>
                        <div class="value">{:.2} CNY</div>
                        <div class="sub-value">综合多维因子</div>
                    </div>
                </div>

                <h2>今日待执行定投</h2>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>资产</th>
                                <th>金额</th>
                                <th>状态</th>
                                <th>说明</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <h2>所有定投计划</h2>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>计划ID</th>
                                <th>资产ID</th>
                                <th>金额</th>
                                <th>频率</th>
                                <th>状态</th>
                                <th>开始日期</th>
                                <th>备注</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="warning-box" style="background-color: #e8f8f5; border-left-color: #1abc9c; color: #16a085;">
                    <strong>定投对比说明:</strong><br>
                    1. <strong>Dca 定投计划</strong> 是您手动设定的固定频率买入计划。<br>
                    2. <strong>基础建议</strong> 是基于您的资产配置缺口自动计算的补仓建议。<br>
                    3. <strong>风险调整建议</strong> 是在基础建议之上，结合了市场冷热和全局风险的优化建议。<br>
                    <br>
                    通常情况下，若 <strong>风险调整建议</strong> 远低于 <strong>定投计划</strong>，说明当前市场处于高风险或过热状态，建议审慎执行定投。
                </div>
                "#,
                summary.total_due_amount, summary.date, base_buy, adjusted_buy, due_rows, plan_rows
            );

            layout("定投计划", content)
        }
        Err(e) => layout(
            "定投计划",
            format!("<div class='warning-box'>定投数据加载失败: {}</div>", e),
        ),
    }
}

async fn daily_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state_clone.config_path)?;
        let portfolio_state = storage::load_state(&state_clone.state_path)?;
        let date = Local::now().format("%Y-%m-%d").to_string();

        let fx_provider = api::create_fx_provider(&config.fx, None);
        let market_provider = api::create_market_provider(&config.market, None);

        let dca_plans = storage::dca_store::load_dca_plans(&state_clone.dca_plans_path)?;
        let dca_preview = engine::dca::calculate_dca_preview(&config, &dca_plans, &date);

        let decision =
            engine::decision::generate_buy_suggestions(&config, &portfolio_state, date.clone());
        let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
            &config.risk,
            &config.regime,
            market_provider.as_ref(),
            fx_provider.as_ref(),
        );

        let instruments =
            storage::instrument_store::load_instruments(&state_clone.instruments_path)
                .unwrap_or_default();
        let mut regimes = std::collections::HashMap::new();
        for asset in &config.assets {
            let symbol_opt = if let Some(rid) = &asset.reference_instrument_id {
                instruments
                    .iter()
                    .find(|i| i.instrument_id == *rid)
                    .map(|i| i.provider_symbol.clone())
            } else {
                asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone())
            };

            if let Some(s) = symbol_opt {
                if let Ok(candles) =
                    market_provider.fetch_daily_candles(&s, config.regime.default_lookback_days)
                {
                    let regime =
                        engine::regime::calculate_market_regime(&s, &candles, &config.regime);
                    regimes.insert(asset.asset_id.clone(), regime);
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

        let snapshots = storage::reconciliation_store::load_alipay_snapshots(
            &state_clone.alipay_snapshots_path,
        )?;
        let mut latest_snaps = std::collections::HashMap::new();
        for s in snapshots {
            let entry = latest_snaps.entry(s.asset_id.clone()).or_insert(s.clone());
            if s.snapshot_date >= entry.snapshot_date {
                *entry = s;
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
            date,
            &dca_preview,
            &adjusted,
            &kelly,
            &reconciliation_results,
        );

        Ok::<models::DailyExecutionPlan, anyhow::Error>(plan)
    })
    .await
    .unwrap();

    match result {
        Ok(plan) => {
            let mut rows = String::new();
            for item in plan.items {
                let badge_class = match item.status.as_str() {
                    "今日应执行" => "badge-green",
                    "暂停执行" | "等待对账" => "badge-red",
                    "建议观察" | "数据不足" => "badge-orange",
                    _ => "badge-gray",
                };

                rows.push_str(&format!(
                    r#"<tr>
                        <td>{}</td>
                        <td>{}</td>
                        <td><code>{}</code></td>
                        <td>{:.2}</td>
                        <td>{:.2}</td>
                        <td>{:.2}</td>
                        <td style='font-weight: bold;'>{:.2}</td>
                        <td>{}</td>
                        <td><span class='badge {}'>{}</span></td>
                        <td>{}</td>
                    </tr>"#,
                    item.sector,
                    item.fund_name,
                    item.fund_code,
                    item.dca_due_amount,
                    item.adjusted_decision_amount,
                    item.kelly_preview_amount,
                    item.recommended_amount,
                    badge_status(&item.reconciliation_status),
                    badge_class,
                    item.status,
                    item.explanation
                ));
                if !item.warnings.is_empty() {
                    rows.push_str(&format!(
                        "<tr><td colspan='10' style='font-size: 0.85em; color: #e74c3c; background-color: #fff5f5;'>&nbsp;&nbsp;⚠ {}</td></tr>",
                        item.warnings.join(" | ")
                    ));
                }
            }

            let mut global_warnings = String::new();
            if !plan.warnings.is_empty() {
                global_warnings = format!(
                    r#"<div class="warning-box" style="margin-top: 20px;">
                        <strong>全局警告:</strong><br>
                        {}
                    </div>"#,
                    plan.warnings.join("<br>")
                );
            }

            let content = format!(
                r#"
                <h1>今日执行计划预览: {}</h1>
                
                <div class="summary-grid">
                    <div class="summary-card">
                        <div class="label">定投应投总额</div>
                        <div class="value">{:.2} CNY</div>
                    </div>
                    <div class="summary-card">
                        <div class="label">风险调整总额</div>
                        <div class="value">{:.2} CNY</div>
                    </div>
                    <div class="summary-card highlighted">
                        <div class="label">最终建议买入</div>
                        <div class="value">{:.2} CNY</div>
                    </div>
                    <div class="summary-card">
                        <div class="label">可用现金</div>
                        <div class="value">{:.2} CNY</div>
                    </div>
                </div>

                <div class="summary-grid" style="margin-top: 20px;">
                    <div class="summary-card">
                        <div class="label">全局风险</div>
                        <div class="value">{}</div>
                    </div>
                    <div class="summary-card">
                        <div class="label">单日买入上限</div>
                        <div class="value">{:.2} CNY</div>
                    </div>
                </div>

                {}

                <div class="table-container" style="margin-top: 30px;">
                    <table>
                        <thead>
                            <tr>
                                <th>赛道</th>
                                <th>资产</th>
                                <th>代码</th>
                                <th>定投</th>
                                <th>风险调整</th>
                                <th>Kelly</th>
                                <th>最终建议</th>
                                <th>对账</th>
                                <th>状态</th>
                                <th>原因</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="warning-box" style="background-color: #f8f9fa; border-left-color: #6c757d; color: #495057; margin-top: 30px;">
                    <strong>预览说明:</strong><br>
                    1. 该页面综合了定投计划、风险调整决策模型和支付宝对账状态。<br>
                    2. <strong>最终建议</strong> 已经过单日买入上限和可用现金的自动对冲缩放。<br>
                    3. ⚠ 该页面仅为预览，不会自动执行买入，也不会写入交易记录。
                </div>
                "#,
                plan.date,
                plan.total_dca_due,
                plan.total_adjusted_decision,
                plan.total_recommended_amount,
                plan.available_cash,
                plan.global_risk_label,
                plan.max_daily_buy,
                global_warnings,
                rows
            );

            layout("今日执行", content)
        }
        Err(e) => layout(
            "今日执行",
            format!("<div class='warning-box'>执行计划加载失败: {}</div>", e),
        ),
    }
}

async fn dca_settlements_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        storage::dca_store::load_dca_settlements(
            &state_clone
                .dca_plans_path
                .replace("dca_plans.json", "dca_settlements.json"),
        )
    })
    .await
    .unwrap();

    match result {
        Ok::<Vec<models::DcaSettlement>, anyhow::Error>(settlements) => {
            let mut rows = String::new();
            for s in settlements {
                let status_badge = match s.status {
                    models::DcaSettlementStatus::Confirmed => badge_status("已确认"),
                    models::DcaSettlementStatus::Pending => badge_status("处理中"),
                    models::DcaSettlementStatus::Failed => badge_status("失败"),
                };

                let applied_badge = if s.applied {
                    "<span class='badge badge-green'>已入账</span>"
                } else {
                    "<span class='badge badge-gray'>未入账</span>"
                };

                rows.push_str(&format!(
                    r#"<tr>
                        <td><code>{}</code></td>
                        <td>{}</td>
                        <td>{:.2}</td>
                        <td>{:.4}</td>
                        <td>{:.4}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                    </tr>"#,
                    s.settlement_id,
                    s.asset_id,
                    s.amount,
                    s.confirmed_nav,
                    s.confirmed_units,
                    s.deduction_date,
                    s.confirmation_date,
                    status_badge,
                    applied_badge,
                    s.note.unwrap_or_default()
                ));
            }

            let content = format!(
                r#"
                <h1>定投确认管理 (DCA Settlements)</h1>
                
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>结算ID</th>
                                <th>资产ID</th>
                                <th>金额</th>
                                <th>确认净值</th>
                                <th>确认份额</th>
                                <th>扣款日期</th>
                                <th>确认日期</th>
                                <th>状态</th>
                                <th>入账状态</th>
                                <th>备注</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="warning-box" style="background-color: #f8f9fa; border-left-color: #6c757d; color: #495057; margin-top: 30px;">
                    <strong>定投确认说明:</strong><br>
                    1. <strong>定投确认</strong> 记录了由基金平台确认的真实成交数据（净值、份额）。<br>
                    2. 只有标记为 <strong>已入账</strong> 的记录才会被计入系统持仓。<br>
                    3. ⚠ Web 界面目前仅提供只读预览。请使用 CLI 命令 <code>dca settlement apply</code> 执行入账操作。
                </div>
                "#,
                rows
            );

            layout("定投确认", content)
        }
        Err(e) => layout(
            "定投确认",
            format!("<div class='warning-box'>定投确认数据加载失败: {}</div>", e),
        ),
    }
}

async fn reconcile_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state_clone.config_path)?;
        let portfolio_state = storage::load_state(&state_clone.state_path)?;
        let snapshots = storage::reconciliation_store::load_alipay_snapshots(
            &state_clone.alipay_snapshots_path,
        )?;

        let mut latest_snaps = std::collections::HashMap::new();
        for s in snapshots {
            let entry = latest_snaps.entry(s.asset_id.clone()).or_insert(s.clone());
            if s.snapshot_date >= entry.snapshot_date {
                *entry = s;
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
    })
    .await
    .unwrap();

    match result {
        Ok(results) => {
            let config = storage::load_config(&state.config_path).unwrap_or_default();
            let mut result_rows = String::new();
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

                    result_rows.push_str(&format!(
                        "<tr>
                            <td>{}</td>
                            <td><code>{}</code></td>
                            <td>{}</td>
                            <td>{:.2}</td>
                            <td>{:.2}</td>
                            <td class='{}'>{:.2} ({:.2}%)</td>
                            <td>{}</td>
                            <td>{}</td>
                        </tr>",
                        res.asset_id,
                        res.snapshot_date,
                        badge_status(&res.status),
                        res.system_market_value,
                        res.alipay_market_value,
                        diff_class,
                        res.market_value_diff,
                        res.market_value_diff_pct * 100.0,
                        res.suggested_action,
                        res.warnings.join("<br>")
                    ));
                } else {
                    result_rows.push_str(&format!(
                        "<tr>
                            <td>{}</td>
                            <td>-</td>
                            <td>{}</td>
                            <td>-</td>
                            <td>-</td>
                            <td>-</td>
                            <td>-</td>
                            <td>-</td>
                        </tr>",
                        asset.asset_id,
                        badge_status("缺少支付宝数据")
                    ));
                }
            }

            let content = format!(
                r#"
                <h1>支付宝对账概览</h1>
                
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>资产ID</th>
                                <th>快照日期</th>
                                <th>状态</th>
                                <th>系统市值</th>
                                <th>支付宝市值</th>
                                <th>差异</th>
                                <th>建议操作</th>
                                <th>警告</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="warning-box" style="background-color: #fdf2f2; border-left-color: #e74c3c; color: #9b1c1c;">
                    <strong>对账说明:</strong><br>
                    1. 该页面显示系统中记录的持仓与您手动录入的支付宝快照之间的对比。<br>
                    2. 若存在明显差异，请检查是否有遗漏的交易记录（申购中、赎回中）。<br>
                    3. 份额不一致通常意味着需要执行校准操作。<br>
                    <br>
                    <strong>注意:</strong> Web 界面仅供查看对比结果，校准操作请通过 CLI 命令执行。
                </div>
                "#,
                result_rows
            );

            layout("支付宝对账", content)
        }
        Err(e) => layout(
            "支付宝对账",
            format!("<div class='warning-box'>对账数据加载失败: {}</div>", e),
        ),
    }
}

async fn instruments_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state_clone.config_path)?;
        let instruments =
            storage::instrument_store::load_instruments(&state_clone.instruments_path)?;

        let mut snapshots = Vec::new();
        for i in &instruments {
            if !i.enabled {
                continue;
            }
            let provider = api::create_instrument_provider(&config.market, Some(&i.provider));
            snapshots.push(provider.latest(i));
        }

        Ok::<Vec<Result<models::InstrumentQuote, anyhow::Error>>, anyhow::Error>(snapshots)
    })
    .await
    .unwrap();

    match result {
        Ok(snapshots) => {
            let mut rows = String::new();
            for res in snapshots {
                match res {
                    Ok(q) => {
                        let price_class = if q.latest_price > 0.0 {
                            ""
                        } else {
                            "text-muted"
                        };
                        let status_class = match q.status.as_str() {
                            "正常" => "badge-blue",
                            "模拟" => "badge-gray",
                            _ => "badge-gray",
                        };

                        rows.push_str(&format!(
                            "<tr>
                                <td><code>{}</code></td>
                                <td>{}</td>
                                <td class='{}' style='font-family: monospace; font-weight: bold;'>{:.4}</td>
                                <td>{}</td>
                                <td>{}</td>
                                <td>{} ({})</td>
                                <td><span class='badge {}'>{}</span></td>
                            </tr>",
                            q.symbol,
                            q.name,
                            price_class,
                            q.latest_price,
                            q.currency,
                            q.quote_unit,
                            q.provider,
                            q.source,
                            status_class,
                            q.status
                        ));
                    }
                    Err(e) => {
                        rows.push_str(&format!(
                            "<tr>
                                <td colspan='7' class='text-down'>错误: {}</td>
                            </tr>",
                            e
                        ));
                    }
                }
            }

            let content = format!(
                r#"
                <h1>市场标的行情 (Watchlist)</h1>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>代码</th>
                                <th>名称</th>
                                <th>最新价格</th>
                                <th>币种</th>
                                <th>单位</th>
                                <th>提供商</th>
                                <th>状态</th>
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

            layout("市场标的", content)
        }
        Err(e) => layout(
            "市场标的",
            format!("<div class='warning-box'>标的数据加载失败: {}</div>", e),
        ),
    }
}

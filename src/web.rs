use crate::{engine, models, storage};
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
    dca_settlements_path: String,
    alipay_snapshots_path: String,
    instruments_path: String,
    cache_status_path: String,
    instrument_cache_path: String,
    risk_cache_path: String,
    proxy_cache_path: String,
    regime_cache_path: String,
}

pub async fn start_server(
    port: u16,
    config_path: String,
    state_path: String,
    transactions_path: String,
    dca_plans_path: String,
    dca_settlements_path: String,
    alipay_snapshots_path: String,
    instruments_path: String,
    cache_status_path: String,
    instrument_cache_path: String,
    risk_cache_path: String,
    proxy_cache_path: String,
    regime_cache_path: String,
) -> Result<()> {
    let app_state = Arc::new(AppState {
        config_path,
        state_path,
        transactions_path,
        dca_plans_path,
        dca_settlements_path,
        alipay_snapshots_path,
        instruments_path,
        cache_status_path,
        instrument_cache_path,
        risk_cache_path,
        proxy_cache_path,
        regime_cache_path,
    });

    let app = Router::new()
        .route("/", get(dashboard_handler))
        .route("/ops", get(ops_handler))
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
        <a href="/">组合概览</a>
        <a href="/ops">操作台</a>
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
        <a href="/dca/lifecycle">定投闭环</a>
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
        let config = storage::load_config(&state_clone.config_path)?;
        let portfolio_state = storage::load_state(&state_clone.state_path)?;
        let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date);

        // Load caches
        let cache_status =
            storage::cache_status_store::load_cache_status(&state_clone.cache_status_path)
                .unwrap_or_default();
        let risk_cache = storage::risk_cache_store::load_risk_cache(&state_clone.risk_cache_path)
            .unwrap_or(None);
        let instrument_cache = storage::instrument_cache_store::load_instrument_cache(
            &state_clone.instrument_cache_path,
        )
        .unwrap_or_default();

        let ret: Result<(
            models::ConfigRoot,
            engine::PortfolioSummary,
            engine::decision::DecisionResult,
            models::CacheStatusRegistry,
            Option<models::RiskCache>,
            models::InstrumentQuoteCache,
        )> = Ok((
            config,
            summary,
            decision,
            cache_status,
            risk_cache,
            instrument_cache,
        ));
        ret
    })
    .await
    .unwrap();

    match result {
        Ok((config, summary, decision, cache_status, risk_cache, instrument_cache)) => {
            let base_cur = &config.portfolio.base_currency;

            // 1. Risk & Info Cards (from cache)
            let mut info_cards = String::new();

            if let Some(rc) = risk_cache {
                info_cards.push_str(&format!(
                    r#"
                    <div class="card">
                        <h3>全局风险指数</h3>
                        <div class="value">{}</div>
                        <div class="sub-value">分数: {:.1} / 100</div>
                        <div class="sub-value" style="font-size: 0.75rem;">更新于: {}</div>
                    </div>
                    "#,
                    badge_risk(&rc.overlay.risk_label),
                    rc.overlay.risk_score,
                    rc.fetched_at
                ));
            } else {
                info_cards.push_str(
                    r#"
                    <div class="card">
                        <h3>全局风险指数</h3>
                        <div class="value" style="color: var(--text-muted);">暂无缓存</div>
                        <div class="sub-value">运行 <code>data refresh --risk</code></div>
                    </div>
                    "#,
                );
            }

            // Show top instruments from cache
            for item in instrument_cache.entries.iter().take(3) {
                info_cards.push_str(&format!(
                    r#"
                    <div class="card">
                        <h3>{}</h3>
                        <div class="value" style="font-size: 1.2rem;">{:.4} {}</div>
                        <div class="sub-value"><code>{}</code> · {}</div>
                    </div>
                    "#,
                    item.name_zh.as_deref().unwrap_or(&item.symbol),
                    item.price,
                    item.currency,
                    item.symbol,
                    item.status
                ));
            }

            // 3. Cache Status Table
            let mut cache_rows = String::new();
            let keys = vec![
                ("fund", "基金净值", "refresh --fund"),
                ("market", "市场行情", "refresh --market"),
                ("risk", "风险因子", "refresh --risk"),
                ("instrument", "市场标的", "refresh --instrument"),
                ("proxy", "估算净值", "refresh --proxy"),
            ];

            for (key, label, cmd) in keys {
                let status = cache_status.statuses.iter().find(|s| s.key == key);
                let (status_text, time_text) = match status {
                    Some(s) => (s.status.as_str(), s.last_updated_at.as_str()),
                    None => ("缺失", "-"),
                };

                let badge_class = match status_text {
                    "正常" => "badge-blue",
                    "缺失" => "badge-red",
                    _ => "badge-orange",
                };

                cache_rows.push_str(&format!(
                    r#"<tr>
                        <td>{}</td>
                        <td><span class="badge {}">{}</span></td>
                        <td>{}</td>
                        <td><code>{}</code></td>
                    </tr>"#,
                    label, badge_class, status_text, time_text, cmd
                ));
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
                        <div class="sub-value">可用: {:.2} {}</div>
                    </div>
                    <div class="card">
                        <h3>当前权益仓</h3>
                        <div class="value">{:.2} {}</div>
                        <div class="sub-value">占比: {}</div>
                    </div>
                    <div class="card">
                        <h3>目标权益仓</h3>
                        <div class="value">{:.2} {}</div>
                        <div class="sub-value">缺口: {:.2} {}</div>
                    </div>
                </div>

                <h2>风险与行情 (缓存)</h2>
                <div class="dashboard-grid">
                    {}
                </div>

                <div style="display: grid; grid-template-columns: 2fr 1fr; gap: 2rem; margin-top: 2rem;">
                    <div>
                        <h2>今日买入建议摘要</h2>
                        <div class="table-container">
                            <table>
                                <thead>
                                    <tr>
                                        <th>赛道</th>
                                        <th>资产</th>
                                        <th>建议买入</th>
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
                        <h2>数据刷新状态</h2>
                        <div class="table-container">
                            <table style="min-width: unset;">
                                <thead>
                                    <tr>
                                        <th>项目</th>
                                        <th>状态</th>
                                        <th>更新时间</th>
                                        <th>刷新命令</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {}
                                </tbody>
                            </table>
                        </div>
                        <p style="font-size: 0.8rem; color: var(--text-muted);">
                            提示: Web界面优先显示缓存数据。请定期在 CLI 运行 <code>cargo run -- data refresh --all</code>。
                        </p>
                    </div>
                </div>
                "#,
                summary.total_asset_value,
                base_cur,
                summary.cash,
                base_cur,
                summary.available_cash,
                base_cur,
                summary.equity_value,
                base_cur,
                fmt_pct(summary.equity_value / summary.total_asset_value),
                summary.target_equity_value,
                base_cur,
                summary.equity_gap,
                base_cur,
                info_cards,
                decision.sector_suggestions.iter().flat_map(|ss| {
                    ss.asset_suggestions.iter().map(move |s| {
                        format!(
                            "<tr><td>{}</td><td>{}</td><td class='text-up'>{:.2} {}</td><td>{}</td></tr>",
                            s.sector_name, s.fund_name, s.suggested_buy, base_cur, ss.sector_name // using ss.sector_name as a placeholder for status or similar
                        )
                    })
                }).collect::<Vec<_>>().join(""),
                cache_rows
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
        let cache = storage::proxy_cache_store::load_proxy_cache(&state_clone.proxy_cache_path)
            .unwrap_or(None);
        Ok::<Option<models::ProxyValuationCache>, anyhow::Error>(cache)
    })
    .await
    .unwrap();

    match result {
        Ok(Some(cache)) => {
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
        Ok(None) => layout(
            "估算净值",
            format!(
                "<div class='warning-box'>暂无估算净值缓存数据，请先在 CLI 运行 <code>cargo run -- data refresh --proxy</code></div>"
            ),
        ),
        Err(e) => layout(
            "估算净值",
            format!("<div class='warning-box'>加载估值数据失败: {}</div>", e),
        ),
    }
}

async fn regime_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let result = tokio::task::spawn_blocking(move || {
        let cache = storage::regime_cache_store::load_regime_cache(&state.regime_cache_path)
            .unwrap_or_default();
        Ok::<models::RegimeCache, anyhow::Error>(cache)
    })
    .await
    .unwrap();

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
    let result = tokio::task::spawn_blocking(move || {
        let cache =
            storage::risk_cache_store::load_risk_cache(&state.risk_cache_path).unwrap_or(None);
        Ok::<Option<models::RiskCache>, anyhow::Error>(cache)
    })
    .await
    .unwrap();

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
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state.config_path)?;
        let portfolio_state = storage::load_state(&state.state_path)?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date);

        // Load caches
        let risk_cache = storage::risk_cache_store::load_risk_cache(&state_clone.risk_cache_path)
            .unwrap_or(None);
        let regime_cache =
            storage::regime_cache_store::load_regime_cache(&state_clone.regime_cache_path)
                .unwrap_or_default();

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

        // Load caches
        let risk_cache = storage::risk_cache_store::load_risk_cache(&state_clone.risk_cache_path)
            .unwrap_or(None);
        let regime_cache =
            storage::regime_cache_store::load_regime_cache(&state_clone.regime_cache_path)
                .unwrap_or_default();

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

        let dca_plans = storage::dca_store::load_dca_plans(&state_clone.dca_plans_path)?;
        let dca_preview = engine::dca::calculate_dca_preview(&config, &dca_plans, &date);

        let decision =
            engine::decision::generate_buy_suggestions(&config, &portfolio_state, date.clone());

        // Load caches for risk and regime
        let risk_cache = storage::risk_cache_store::load_risk_cache(&state_clone.risk_cache_path)
            .unwrap_or(None);
        let regime_cache =
            storage::regime_cache_store::load_regime_cache(&state_clone.regime_cache_path)
                .unwrap_or_default();

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

        let snapshots = storage::reconciliation_store::load_alipay_snapshots(
            &state_clone.alipay_snapshots_path,
        )?;
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

        let settlements =
            storage::dca_store::load_dca_settlements(&state_clone.dca_settlements_path)?;
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
    })
    .await
    .unwrap();

    match result {
        Ok((plan, lifecycle)) => {
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
                    item.asset_id,
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

            // DCA Lifecycle Reminder
            let mut lifecycle_html = String::new();
            if lifecycle.count_waiting_confirmation > 0
                || lifecycle.count_unapplied > 0
                || lifecycle.count_attention_required > 0
            {
                lifecycle_html.push_str("<div class='card' style='border-left: 4px solid var(--up-color); margin-bottom: 2rem;'>");
                lifecycle_html.push_str("<h3>定投闭环提醒</h3>");
                lifecycle_html.push_str("<ul>");
                if lifecycle.count_waiting_confirmation > 0 {
                    lifecycle_html.push_str(&format!(
                        "<li>有 <strong>{}</strong> 笔定投扣款待录入确认单。</li>",
                        lifecycle.count_waiting_confirmation
                    ));
                }
                if lifecycle.count_unapplied > 0 {
                    lifecycle_html.push_str(&format!(
                        "<li>有 <strong>{}</strong> 笔确认单待入账。</li>",
                        lifecycle.count_unapplied
                    ));
                }
                if lifecycle.count_attention_required > 0 {
                    lifecycle_html.push_str(&format!(
                        "<li>有 <strong>{}</strong> 个资产对账不一致，需人工核对。</li>",
                        lifecycle.count_attention_required
                    ));
                }
                lifecycle_html.push_str("</ul>");
                lifecycle_html.push_str("<p style='font-size: 0.85rem; margin-top: 1rem;'><a href='/dca/lifecycle'>查看详细闭环状态 &rarr;</a></p>");
                lifecycle_html.push_str("</div>");
            }

            let content = format!(
                r#"
                <h1>今日执行计划预览: {}</h1>
                
                <div class="dashboard-grid">
                    <div class="card">
                        <h3>最终建议买入总额</h3>
                        <div class="value">{:.2} CNY</div>
                        <div class="sub-value">定投部分: {:.2}</div>
                    </div>
                    <div class="card">
                        <h3>风险调整总额</h3>
                        <div class="value">{:.2} CNY</div>
                        <div class="sub-value">相对于基础定投</div>
                    </div>
                    <div class="card">
                        <h3>可用现金 (余)</h3>
                        <div class="value">{:.2} CNY</div>
                        <div class="sub-value">单日买入上限: {:.2}</div>
                    </div>
                    <div class="card">
                        <h3>全局风险评分</h3>
                        <div class="value">{}</div>
                        <div class="sub-value">分数: {:.1}</div>
                    </div>
                </div>

                {}
                
                {}

                <h2>买入执行清单</h2>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>赛道</th>
                                <th>基金名称</th>
                                <th>资产ID</th>
                                <th>定投应投</th>
                                <th>风险调整</th>
                                <th>Kelly</th>
                                <th>最终建议</th>
                                <th>对账状态</th>
                                <th>状态</th>
                                <th>原因说明</th>
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
                plan.total_recommended_amount,
                plan.total_dca_due,
                plan.total_adjusted_decision,
                plan.available_cash,
                plan.max_daily_buy,
                badge_risk(&plan.global_risk_label),
                plan.global_risk_score,
                global_warnings,
                lifecycle_html,
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
    let result = tokio::task::spawn_blocking(move || {
        let cache =
            storage::instrument_cache_store::load_instrument_cache(&state.instrument_cache_path)
                .unwrap_or_default();
        Ok::<models::InstrumentQuoteCache, anyhow::Error>(cache)
    })
    .await
    .unwrap();

    match result {
        Ok(cache) => {
            let mut rows = String::new();
            if cache.entries.is_empty() {
                rows.push_str("<tr><td colspan='4' style='text-align: center; padding: 2rem;'>暂无缓存数据，请先在 CLI 运行 <code>cargo run -- data refresh --instrument</code></td></tr>");
            }

            for q in cache.entries {
                let display_name = q.name_zh.as_deref().unwrap_or(&q.symbol);
                let price_class = if q.price > 0.0 { "" } else { "text-muted" };
                let status_class = match q.status.as_str() {
                    "正常" => "badge-blue",
                    "模拟" => "badge-gray",
                    _ => "badge-gray",
                };

                rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-size: 1.1rem; font-weight: bold;'>{}</div>
                            <div style='font-size: 0.85rem; color: var(--text-muted); margin-top: 4px;'>
                                <code>{}</code> · {}
                            </div>
                        </td>
                        <td class='{}' style='font-family: monospace; font-weight: bold; font-size: 1.2rem; text-align: right;'>{:.4} {}</td>
                        <td style='text-align: center;'>{} ({})</td>
                        <td style='text-align: center;'><span class='badge {}'>{}</span></td>
                    </tr>",
                    display_name,
                    q.symbol,
                    q.quote_unit,
                    price_class,
                    q.price,
                    q.currency,
                    q.provider,
                    q.source,
                    status_class,
                    q.status
                ));
            }

            let content = format!(
                r#"
                <div style="display: flex; justify-content: space-between; align-items: baseline;">
                    <h1>市场标的行情 (Watchlist)</h1>
                    <div style="font-size: 0.85rem; color: var(--text-muted);">
                        缓存更新时间: {}
                    </div>
                </div>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>标的信息</th>
                                <th style='text-align: right;'>最新价格</th>
                                <th style='text-align: center;'>提供商</th>
                                <th style='text-align: center;'>状态</th>
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

            layout("市场标的", content)
        }
        Err(e) => layout(
            "市场标的",
            format!("<div class='warning-box'>标的数据加载失败: {}</div>", e),
        ),
    }
}

async fn dca_lifecycle_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state_clone.config_path)?;
        let portfolio_state = storage::load_state(&state_clone.state_path)?;
        let date = Local::now().format("%Y-%m-%d").to_string();

        let dca_plans = storage::dca_store::load_dca_plans(&state_clone.dca_plans_path)?;
        let settlements =
            storage::dca_store::load_dca_settlements(&state_clone.dca_settlements_path)?;
        let snapshots = storage::reconciliation_store::load_alipay_snapshots(
            &state_clone.alipay_snapshots_path,
        )?;

        let summary = engine::dca_lifecycle::calculate_dca_lifecycle(
            &config,
            &dca_plans,
            &settlements,
            &snapshots,
            &portfolio_state,
            &date,
        );

        Ok::<models::DcaLifecycleSummary, anyhow::Error>(summary)
    })
    .await
    .unwrap();

    match result {
        Ok(summary) => {
            let mut item_rows = String::new();
            for item in summary.items {
                let status_badge = badge_status(&item.lifecycle_status);
                let action_badge = format!(
                    "<span class='badge {}'>{}</span>",
                    if item.suggested_next_action == "无需处理" {
                        "badge-gray"
                    } else {
                        "badge-blue"
                    },
                    item.suggested_next_action
                );

                item_rows.push_str(&format!(
                    r#"<tr>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{:.2}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                    </tr>"#,
                    item.asset_id,
                    item.fund_name,
                    item.planned_amount,
                    item.settlement_amount
                        .map(|a| format!("{:.2}", a))
                        .unwrap_or_else(|| "-".to_string()),
                    status_badge,
                    item.reconciliation_status,
                    action_badge
                ));
            }

            let content = format!(
                r#"
                <h1>定投闭环生命周期 (DCA Lifecycle)</h1>
                <p>日期: {}</p>

                <div class="dashboard-grid">
                    <div class="card">
                        <h3>计划定投总额</h3>
                        <div class="value">{:.2} CNY</div>
                        <div class="sub-value">今日到期项: {}</div>
                    </div>
                    <div class="card">
                        <h3>已确认总额</h3>
                        <div class="value">{:.2} CNY</div>
                        <div class="sub-value">待入账项: {}</div>
                    </div>
                    <div class="card">
                        <h3>对账状态</h3>
                        <div class="value">{} 一致</div>
                        <div class="sub-value">待处理项: {}</div>
                    </div>
                </div>

                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>资产ID</th>
                                <th>名称</th>
                                <th>计划金额</th>
                                <th>确认金额</th>
                                <th>生命周期状态</th>
                                <th>对账状态</th>
                                <th>建议操作</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>
                "#,
                summary.date,
                summary.total_planned_amount,
                summary.count_due,
                summary.total_confirmed_amount,
                summary.count_unapplied,
                summary.count_reconciled,
                summary.count_attention_required,
                item_rows
            );

            layout("定投闭环", content)
        }
        Err(e) => layout(
            "定投闭环",
            format!("<div class='warning-box'>数据加载失败: {}</div>", e),
        ),
    }
}

async fn ops_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state_clone.config_path)?;
        let portfolio_state = storage::load_state(&state_clone.state_path)?;
        let date = Local::now().format("%Y-%m-%d").to_string();

        let dca_plans = storage::dca_store::load_dca_plans(&state_clone.dca_plans_path)?;
        let settlements =
            storage::dca_store::load_dca_settlements(&state_clone.dca_settlements_path)?;
        let snapshots = storage::reconciliation_store::load_alipay_snapshots(
            &state_clone.alipay_snapshots_path,
        )?;

        let lifecycle = engine::dca_lifecycle::calculate_dca_lifecycle(
            &config,
            &dca_plans,
            &settlements,
            &snapshots,
            &portfolio_state,
            &date,
        );

        let cache_status =
            storage::cache_status_store::load_cache_status(&state_clone.cache_status_path)
                .unwrap_or_default();
        let risk_cache = storage::risk_cache_store::load_risk_cache(&state_clone.risk_cache_path)
            .unwrap_or(None);

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
                models::DcaLifecycleSummary,
                models::CacheStatusRegistry,
                engine::decision::DecisionResult,
                models::GlobalRiskOverlay,
            ),
            anyhow::Error,
        >((lifecycle, cache_status, decision, risk_overlay))
    })
    .await
    .unwrap();

    match result {
        Ok((lifecycle, cache_status, decision, risk_overlay)) => {
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
                    "<span class='badge {}' style='margin-right: 0.5rem;'>{}: {}</span>",
                    color, key, status_text
                ));
            }

            let mut pending_html = String::new();
            let pending_items: Vec<_> = lifecycle
                .items
                .iter()
                .filter(|i| i.suggested_next_action != "无需处理" && i.lifecycle_status != "已暂停")
                .collect();

            if pending_items.is_empty() {
                pending_html
                    .push_str("<div class='text-down'>[✓] 暂无需要人工处理的定投事项。</div>");
            } else {
                pending_html.push_str("<ul>");
                for i in &pending_items {
                    pending_html.push_str(&format!(
                        "<li><strong>{}</strong>: {} ({})</li>",
                        i.asset_id, i.suggested_next_action, i.lifecycle_status
                    ));
                }
                pending_html.push_str("</ul>");
            }

            let content = format!(
                r#"
                <h1>每日操作台 (Ops Workspace)</h1>
                
                <div class="dashboard-grid">
                    <div class="card">
                        <h3>1. 数据准备</h3>
                        <div style="margin-bottom: 1rem;">{}</div>
                        <div class="sub-value">提示: 若数据过期，请在 CLI 运行 <code>ops refresh</code></div>
                    </div>
                    <div class="card">
                        <h3>2. 风险状态</h3>
                        <div class="value">{}</div>
                        <div class="sub-value">风险分数: {:.1}</div>
                    </div>
                    <div class="card">
                        <h3>3. 今日定投计划</h3>
                        <div class="value">{:.2} CNY</div>
                        <div class="sub-value">今日应投: {} 笔</div>
                    </div>
                </div>

                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 2rem; margin-top: 2rem;">
                    <div class="card">
                        <h3>4. 待处理事项 (定投闭环)</h3>
                        {}
                        <p style="margin-top: 1rem;"><a href="/dca/lifecycle">查看完整生命周期 &rarr;</a></p>
                    </div>
                    <div class="card">
                        <h3>5. 今日建议执行</h3>
                        <div class="value" style="font-size: 1.5rem;">建议买入总额: {:.2} CNY</div>
                        <div class="sub-value">基础定投: {:.2} CNY</div>
                        <p style="margin-top: 1rem;"><a href="/daily">查看今日执行计划 &rarr;</a></p>
                    </div>
                </div>

                <div class="card" style="margin-top: 2rem; background-color: #f8f9fa;">
                    <h3>下一步建议</h3>
                    <div id="next-steps">
                        加载中...
                    </div>
                </div>

                <script>
                    // Simple logic to generate next steps in JS for a bit of "alive" feel
                    const pendingCount = {};
                    const riskScore = {};
                    const staleData = {};
                    
                    let steps = [];
                    if (staleData) steps.push("运行 <code>cargo run -- ops refresh</code> 刷新行情。");
                    if (pendingCount > 0) steps.push("处理 <strong>" + pendingCount + "</strong> 项待办定投事项（录入确认单或支付宝快照）。");
                    steps.push("确认今日执行计划并执行手动买入。");
                    
                    document.getElementById('next-steps').innerHTML = "<ol>" + steps.map(s => "<li>" + s + "</li>").join("") + "</ol>";
                </script>
                "#,
                cache_html,
                badge_risk(&risk_overlay.risk_label),
                risk_overlay.risk_score,
                lifecycle.total_planned_amount,
                lifecycle.count_due,
                pending_html,
                decision.suggested_total_buy, // Placeholder, usually we'd want adjusted here if possible
                lifecycle.total_planned_amount,
                pending_items.len(),
                risk_overlay.risk_score,
                cache_status.statuses.iter().any(|s| s.status != "正常")
                    || cache_status.statuses.is_empty()
            );

            layout("操作台", content)
        }
        Err(e) => layout(
            "操作台",
            format!("<div class='warning-box'>加载操作台失败: {}</div>", e),
        ),
    }
}

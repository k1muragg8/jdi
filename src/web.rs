use crate::models::{FxRate, MarketPrice};
use crate::{engine, models, storage};
use anyhow::Result;
use axum::{Router, extract::State, response::Html, routing::get};
use std::net::SocketAddr;
use std::sync::Arc;

struct AppState {
    config_path: String,
    state_path: String,
    transactions_path: String,
}

pub async fn start_server(
    port: u16,
    config_path: String,
    state_path: String,
    transactions_path: String,
) -> Result<()> {
    let app_state = Arc::new(AppState {
        config_path,
        state_path,
        transactions_path,
    });

    let app = Router::new()
        .route("/", get(dashboard_handler))
        .route("/holdings", get(holdings_handler))
        .route("/sectors", get(sectors_handler))
        .route("/decisions", get(decisions_handler))
        .route("/decision", get(decisions_handler)) // Alias for stability
        .route("/transactions", get(transactions_handler))
        .route("/assets", get(assets_handler))
        .route("/valuation/proxy", get(proxy_valuation_handler))
        .route("/proxy", get(proxy_valuation_handler)) // Alias for stability
        .route("/regime", get(regime_handler))
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
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif; line-height: 1.6; color: #333; max-width: 1200px; margin: 0 auto; padding: 20px; background-color: #f4f7f6; }}
        nav {{ background-color: #2c3e50; padding: 10px; border-radius: 5px; margin-bottom: 20px; }}
        nav a {{ color: white; text-decoration: none; margin-right: 15px; font-weight: bold; }}
        nav a:hover {{ text-decoration: underline; }}
        h1, h2 {{ color: #2c3e50; }}
        table {{ width: 100%; border-collapse: collapse; margin-bottom: 20px; background-color: white; box-shadow: 0 2px 5px rgba(0,0,0,0.1); }}
        th, td {{ border: 1px solid #ddd; padding: 12px; text-align: left; }}
        th {{ background-color: #f8f9fa; color: #2c3e50; }}
        tr:nth-child(even) {{ background-color: #f2f2f2; }}
        .summary-card {{ background-color: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 5px rgba(0,0,0,0.1); margin-bottom: 20px; display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 20px; }}
        .summary-item {{ border-left: 4px solid #3498db; padding-left: 15px; }}
        .summary-item .label {{ font-size: 0.9em; color: #7f8c8d; }}
        .summary-item .value {{ font-size: 1.2em; font-weight: bold; color: #2c3e50; }}
        .summary-item .sub-value {{ font-size: 0.85em; color: #95a5a6; margin-top: 4px; }}
        .warning {{ background-color: #fff3cd; border: 1px solid #ffeeba; color: #856404; padding: 15px; border-radius: 5px; margin-bottom: 20px; }}
        .status-underweight {{ color: #e74c3c; font-weight: bold; }}
        .status-overweight {{ color: #27ae60; font-weight: bold; }}
        .status-neutral {{ color: #7f8c8d; }}
        .text-red {{ color: #e74c3c; }}
        .text-green {{ color: #27ae60; }}
    </style>
</head>
<body>
    <nav>
        <a href="/">首页</a>
        <a href="/holdings">当前持仓</a>
        <a href="/sectors">赛道概览</a>
        <a href="/decisions">今日买入建议</a>
        <a href="/regime">市场冷热</a>
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

fn fmt_pct(val: f64) -> String {
    format!("{:.2}%", val * 100.0)
}

fn safe_div(num: f64, den: f64) -> String {
    if den.abs() < 0.000001 {
        "N/A".to_string()
    } else {
        fmt_pct(num / den)
    }
}

async fn dashboard_handler(State(state): State<Arc<AppState>>) -> Html<String> {
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

        let ret: Result<(
            models::ConfigRoot,
            engine::PortfolioSummary,
            engine::decision::DecisionResult,
            Option<FxRate>,
            Option<MarketPrice>,
            Option<MarketPrice>,
            Option<MarketPrice>,
            Option<models::MarketRegimeResult>,
        )> = Ok((
            config, summary, decision, usd_cnh, btc, eth, sol, qqq_regime,
        ));
        ret
    })
    .await
    .unwrap();

    match result {
        Ok((config, summary, decision, usd_cnh, btc, eth, sol, qqq_regime)) => {
            let mut risk_cards = String::new();

            if let Some(fx) = usd_cnh {
                risk_cards.push_str(&format!(
                    r#"
                    <div class="summary-item">
                        <div class="label">USD/CNH</div>
                        <div class="value">{:.4}</div>
                        <div class="sub-value">{} | {}</div>
                    </div>
                    "#,
                    fx.rate, fx.source, fx.date
                ));
            } else {
                risk_cards.push_str(
                    r#"
                    <div class="summary-item">
                        <div class="label">USD/CNH</div>
                        <div class="value">查询失败</div>
                    </div>
                    "#,
                );
            }

            for crypto in vec![btc, eth, sol] {
                if let Some(c) = crypto {
                    risk_cards.push_str(&format!(
                        r#"
                        <div class="summary-item">
                            <div class="label">{}</div>
                            <div class="value">{:.2}</div>
                            <div class="sub-value">{} | {}</div>
                        </div>
                        "#,
                        c.symbol, c.price, c.source, c.date
                    ));
                }
            }

            if let Some(regime) = qqq_regime {
                risk_cards.push_str(&format!(
                    r#"
                    <div class="summary-item">
                        <div class="label">QQQ 市场状态</div>
                        <div class="value">{}</div>
                        <div class="sub-value">钟摆分数: {:.2}</div>
                    </div>
                    "#,
                    regime.regime_label, regime.pendulum_score
                ));
            }

            let content = format!(
                r#"
                <h1>组合概览</h1>
                <div class="summary-card">
                    <div class="summary-item">
                        <div class="label">当前现金</div>
                        <div class="value">{:.2} {}</div>
                    </div>
                    <div class="summary-item">
                        <div class="label">可用现金</div>
                        <div class="value">{:.2} {}</div>
                        <div class="sub-value">占总资产: {}</div>
                    </div>
                    <div class="summary-item">
                        <div class="label">目标权益仓</div>
                        <div class="value">{:.2} {}</div>
                    </div>
                    <div class="summary-item">
                        <div class="label">当前权益仓</div>
                        <div class="value">{:.2} {}</div>
                        <div class="sub-value">达成率: {} / 占总资产: {}</div>
                    </div>
                    <div class="summary-item">
                        <div class="label">权益缺口</div>
                        <div class="value">{:.2} {}</div>
                        <div class="sub-value">缺口率: {}</div>
                    </div>
                    <div class="summary-item">
                        <div class="label">今日建议总买入</div>
                        <div class="value">{:.2} {}</div>
                        <div class="sub-value">占单日上限: {}</div>
                    </div>
                    <div class="summary-item">
                        <div class="label">总资产</div>
                        <div class="value">{:.2} {}</div>
                    </div>
                </div>

                <h1>风险参考快照</h1>
                <div class="summary-card">
                    {}
                </div>
                "#,
                summary.cash,
                config.portfolio.base_currency,
                summary.available_cash,
                config.portfolio.base_currency,
                safe_div(summary.available_cash, summary.total_asset_value),
                summary.target_equity_value,
                config.portfolio.base_currency,
                summary.equity_value,
                config.portfolio.base_currency,
                safe_div(summary.equity_value, summary.target_equity_value),
                safe_div(summary.equity_value, summary.total_asset_value),
                summary.equity_gap,
                config.portfolio.base_currency,
                safe_div(summary.equity_gap, summary.target_equity_value),
                decision.suggested_total_buy,
                config.portfolio.base_currency,
                safe_div(decision.suggested_total_buy, decision.max_daily_buy_total),
                summary.total_asset_value,
                config.portfolio.base_currency,
                risk_cards
            );

            layout("首页", content)
        }
        Err(e) => layout(
            "首页",
            format!("<div class='warning'>数据加载失败: {}</div>", e),
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
                let pnl_pct = safe_div(pnl, cost);

                let pnl_class = if pnl >= 0.0 { "text-green" } else { "text-red" };

                rows.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td><td>{:.2}</td><td class='{}'>{} ({})</td><td>{}</td><td>{}</td></tr>",
                    holding.asset_id, holding.fund_code, fund_name, sector, holding.units, nav_str, nav_date, source, status, market_value, cost, pnl_class, pnl, pnl_pct, weight_total, weight_equity
                ));
            }

            let content = format!(
                r#"
                <h1>当前持仓</h1>
                <table>
                    <thead>
                        <tr>
                            <th>资产ID</th>
                            <th>基金代码</th>
                            <th>基金名称</th>
                            <th>赛道</th>
                            <th>持有份额</th>
                            <th>最新净值</th>
                            <th>净值日期</th>
                            <th>数据来源</th>
                            <th>数据状态</th>
                            <th>当前市值</th>
                            <th>持仓成本</th>
                            <th>浮动盈亏 (率)</th>
                            <th>占总资产</th>
                            <th>占权益仓</th>
                        </tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>
                "#,
                rows
            );

            layout("当前持仓", content)
        }
        Err(e) => layout(
            "当前持仓",
            format!(
                "<div class='warning'>行情数据获取失败，请稍后重试或运行 CLI 检查数据来源。<br>错误详情: {}</div>",
                e
            ),
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
                let target_pct = fmt_pct(s.target_weight);
                let current_pct = fmt_pct(s.current_weight);
                let gap_ratio = fmt_pct(s.gap_ratio);
                let (status_cn, status_class) = match s.status.as_str() {
                    "underweight" => ("低配", "status-underweight"),
                    "neutral" => ("均衡", "status-neutral"),
                    "overweight" => ("超配", "status-overweight"),
                    "disabled" => ("已禁用", "status-neutral"),
                    other => (other, "status-neutral"),
                };

                rows.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td><td>{:.2}</td><td>{:.2}</td><td class='{}'>{}</td></tr>",
                    s.sector_name, target_pct, current_pct, gap_ratio, s.target_value, s.current_value, s.gap_value, status_class, status_cn
                ));
            }

            let content = format!(
                r#"
                <h1>赛道概览</h1>
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
                "#,
                rows
            );

            layout("赛道概览", content)
        }
        Err(e) => layout(
            "赛道概览",
            format!(
                "<div class='warning'>行情数据获取失败，请稍后重试或运行 CLI 检查数据来源。<br>错误详情: {}</div>",
                e
            ),
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
                warnings.push_str(&format!("<div class='warning'>{}</div>", warning));
            }

            let mut rows = String::new();
            if result.suggested_total_buy > 0.0 {
                for sector in result.sector_suggestions {
                    let sector_pct = safe_div(sector.suggested_buy, result.suggested_total_buy);

                    for asset in sector.asset_suggestions {
                        let asset_pct = safe_div(asset.suggested_buy, result.suggested_total_buy);

                        rows.push_str(&format!(
                            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td><td>{:.2} ({})</td><td>{:.2} ({})</td><td>{}</td></tr>",
                            asset.sector_name, asset.fund_name, asset.fund_code, sector.gap_value, sector.suggested_buy, sector_pct, asset.suggested_buy, asset_pct, asset.reason
                        ));
                    }
                }
            }

            let content = format!(
                r#"
                <h1>今日买入建议</h1>
                {}
                <div class="summary-card">
                    <div class="summary-item">
                        <div class="label">可用现金</div>
                        <div class="value">{:.2} {}</div>
                    </div>
                    <div class="summary-item">
                        <div class="label">今日建议总买入</div>
                        <div class="value">{:.2} {}</div>
                        <div class="sub-value">占单日上限: {}</div>
                    </div>
                    <div class="summary-item">
                        <div class="label">单日买入上限</div>
                        <div class="value">{:.2} {}</div>
                    </div>
                </div>
                <table>
                    <thead>
                        <tr>
                            <th>赛道</th>
                            <th>资产</th>
                            <th>基金代码</th>
                            <th>缺口</th>
                            <th>赛道总买入 (占比)</th>
                            <th>资产建议买入 (占比)</th>
                            <th>原因</th>
                        </tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>
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
            format!(
                "<div class='warning'>行情数据获取失败，请稍后重试或运行 CLI 检查数据来源。<br>错误详情: {}</div>",
                e
            ),
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

                rows.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td><td>{}</td><td>{}</td><td>{:.2}</td><td>{}</td><td>{}</td></tr>",
                    tx.id, tx.date, type_cn, tx.asset_id.as_deref().unwrap_or("-"), tx.amount, 
                    tx.units.map(|u| format!("{:.2}", u)).unwrap_or_else(|| "-".to_string()),
                    tx.price.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "-".to_string()),
                    tx.fee, tx.currency, tx.note
                ));
            }

            let content = format!(
                r#"
                <h1>交易记录</h1>
                <table>
                    <thead>
                        <tr>
                            <th>ID</th>
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
                "#,
                rows
            );

            layout("交易记录", content)
        }
        Err(e) => layout(
            "交易记录",
            format!("<div class='warning'>数据加载失败: {}</div>", e),
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
                rows.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    asset.asset_id,
                    asset.fund_code,
                    asset.fund_name,
                    asset.sector,
                    asset.currency,
                    asset.valuation_method,
                    asset.enabled,
                    asset.reference_index_name.as_deref().unwrap_or("-"),
                    asset.reference_index_symbol.as_deref().unwrap_or("-"),
                    asset.market_data_provider.as_deref().unwrap_or("-"),
                ));
            }

            let content = format!(
                r#"
                <h1>资产列表</h1>
                <table>
                    <thead>
                        <tr>
                            <th>Asset ID</th>
                            <th>Fund Code</th>
                            <th>Fund Name</th>
                            <th>Sector</th>
                            <th>Currency</th>
                            <th>Val Method</th>
                            <th>Enabled</th>
                            <th>参考指数</th>
                            <th>指数代码</th>
                            <th>行情来源</th>
                        </tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>
                "#,
                rows
            );

            layout("资产列表", content)
        }
        Err(e) => layout(
            "资产列表",
            format!("<div class='warning'>数据加载失败: {}</div>", e),
        ),
    }
}

async fn proxy_valuation_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state.config_path)?;
        let portfolio_state = storage::load_state(&state.state_path)?;
        let market_provider = crate::api::create_market_provider(&config.market, None);
        let fx_provider = crate::api::create_fx_provider(&config.fx, None);
        let results = engine::calculate_proxy_valuations(
            &config,
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

                let combined_return_pct = fmt_pct(res.combined_proxy_return);
                let fx_adj_str = if res.use_fx_adjustment { "是" } else { "否" };

                let diff = res.estimated_market_value - res.official_market_value;
                let deviation_pct = safe_div(diff, res.official_market_value);

                let status_with_warning = if let Some(w) = &res.warning {
                    format!(
                        "{} <br><small style='color: #856404'>{}</small>",
                        res.status, w
                    )
                } else {
                    res.status.clone()
                };

                rows.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{:.4}</td><td>{}</td><td>{:.2}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.4}</td><td>{:.2}</td><td>{}</td><td>{}</td></tr>",
                    res.asset_id, res.fund_name, res.official_nav, res.official_nav_date, res.official_market_value,
                    res.reference_index_symbol, index_return_pct, fx_return_pct, combined_return_pct, fx_adj_str,
                    res.estimated_nav, res.estimated_market_value, deviation_pct, status_with_warning
                ));
            }

            let content = format!(
                r#"
                <h1>估值预览 (指数代理估算)</h1>
                <table>
                    <thead>
                        <tr>
                            <th>资产ID</th>
                            <th>基金名称</th>
                            <th>官方净值</th>
                            <th>净值日期</th>
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
                "#,
                rows
            );

            layout("估算净值", content)
        }
        Err(e) => layout(
            "估算净值",
            format!(
                "<div class='warning'>行情数据获取失败，请稍后重试或运行 CLI 检查数据来源。<br>错误详情: {}</div>",
                e
            ),
        ),
    }
}

async fn regime_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state.config_path)?;
        let market_provider = crate::api::create_market_provider(&config.market, None);

        let mut target_symbols = Vec::new();
        for asset in &config.assets {
            if let Some(s) = &asset.reference_index_symbol {
                if !target_symbols.contains(s) {
                    target_symbols.push(s.clone());
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
                    let z_str = res
                        .windows
                        .iter()
                        .find(|w| w.window_days == *w_days)
                        .and_then(|w| w.z_score)
                        .map(|z| format!("{:.2}", z))
                        .unwrap_or_else(|| "-".to_string());
                    window_cols.push_str(&format!("<td>{}</td>", z_str));
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

                rows.push_str(&format!(
                    "<tr><td>{}</td><td>{:.2}</td>{}<td>{}</td><td>{}</td><td>{:.2}</td><td>{}</td><td>{}</td></tr>",
                    res.symbol, res.latest_price, window_cols, drawdown_pct, vol_pct, res.pendulum_score, res.regime_label, res.warning.as_deref().unwrap_or("-")
                ));
            }

            let content = format!(
                r#"
                <h1>市场冷热分析</h1>
                <p>基于均值偏离 (Z-score) 和历史波动计算的钟摆分数。</p>
                <table>
                    <thead>
                        <tr>
                            <th>代码</th>
                            <th>最新价</th>
                            <th>20日 Z-score</th>
                            <th>60日 Z-score</th>
                            <th>120日 Z-score</th>
                            <th>250日 Z-score</th>
                            <th>最大回撤 (250日)</th>
                            <th>年化波动 (250日)</th>
                            <th>钟摆分数</th>
                            <th>市场状态</th>
                            <th>提示</th>
                        </tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>
                <p><small>风险提示: 金融市场收益并不严格服从正态分布，Z-score 仅用于衡量相对偏离程度，不应被理解为确定性预测。</small></p>
                "#,
                rows
            );

            layout("市场冷热", content)
        }
        Err(e) => layout(
            "市场冷热",
            format!("<div class='warning'>行情数据获取失败: {}</div>", e),
        ),
    }
}

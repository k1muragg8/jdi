use crate::{engine, storage};
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
        .route("/transactions", get(transactions_handler))
        .route("/assets", get(assets_handler))
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
        .summary-card {{ background-color: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 5px rgba(0,0,0,0.1); margin-bottom: 20px; display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; }}
        .summary-item {{ border-left: 4px solid #3498db; padding-left: 15px; }}
        .summary-item .label {{ font-size: 0.9em; color: #7f8c8d; }}
        .summary-item .value {{ font-size: 1.2em; font-weight: bold; color: #2c3e50; }}
        .warning {{ background-color: #fff3cd; border: 1px solid #ffeeba; color: #856404; padding: 15px; border-radius: 5px; margin-bottom: 20px; }}
        .status-underweight {{ color: #e74c3c; font-weight: bold; }}
        .status-overweight {{ color: #27ae60; font-weight: bold; }}
        .status-neutral {{ color: #7f8c8d; }}
    </style>
</head>
<body>
    <nav>
        <a href="/">组合概览</a>
        <a href="/holdings">当前持仓</a>
        <a href="/sectors">赛道概览</a>
        <a href="/decisions">今日买入建议</a>
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

async fn dashboard_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let config = storage::load_config(&state.config_path).unwrap();
    let portfolio_state = storage::load_state(&state.state_path).unwrap();
    let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);

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
            </div>
            <div class="summary-item">
                <div class="label">目标权益仓</div>
                <div class="value">{:.2} {}</div>
            </div>
            <div class="summary-item">
                <div class="label">当前权益仓</div>
                <div class="value">{:.2} {}</div>
            </div>
            <div class="summary-item">
                <div class="label">权益缺口</div>
                <div class="value">{:.2} {}</div>
            </div>
            <div class="summary-item">
                <div class="label">总资产</div>
                <div class="value">{:.2} {}</div>
            </div>
        </div>
        "#,
        summary.cash,
        config.portfolio.base_currency,
        summary.available_cash,
        config.portfolio.base_currency,
        summary.target_equity_value,
        config.portfolio.base_currency,
        summary.equity_value,
        config.portfolio.base_currency,
        summary.equity_gap,
        config.portfolio.base_currency,
        summary.total_asset_value,
        config.portfolio.base_currency
    );

    layout("组合概览", content)
}

async fn holdings_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let config = storage::load_config(&state.config_path).unwrap();
    let portfolio_state = storage::load_state(&state.state_path).unwrap();

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

        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td></tr>",
            holding.asset_id, holding.fund_code, fund_name, sector, holding.units, nav_str, nav_date, source, status, pnl
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
                    <th>浮动盈亏</th>
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

async fn sectors_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let config = storage::load_config(&state.config_path).unwrap();
    let portfolio_state = storage::load_state(&state.state_path).unwrap();
    let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);

    let mut rows = String::new();
    for s in summary.sector_summaries {
        let target_pct = format!("{:.2}%", s.target_weight * 100.0);
        let current_pct = format!("{:.2}%", s.current_weight * 100.0);
        let (status_cn, status_class) = match s.status.as_str() {
            "underweight" => ("低配", "status-underweight"),
            "neutral" => ("均衡", "status-neutral"),
            "overweight" => ("超配", "status-overweight"),
            "disabled" => ("已禁用", "status-neutral"),
            other => (other, "status-neutral"),
        };

        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td><td>{:.2}</td><td>{:.2}</td><td class='{}'>{}</td></tr>",
            s.sector_name, target_pct, current_pct, s.target_value, s.current_value, s.gap_value, status_class, status_cn
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
                    <th>目标市值</th>
                    <th>当前市值</th>
                    <th>缺口</th>
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

async fn decisions_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let config = storage::load_config(&state.config_path).unwrap();
    let portfolio_state = storage::load_state(&state.state_path).unwrap();
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let result = engine::generate_buy_suggestions(&config, &portfolio_state, date);

    let mut warnings = String::new();
    for warning in &result.warnings {
        warnings.push_str(&format!("<div class='warning'>{}</div>", warning));
    }

    let mut rows = String::new();
    if result.suggested_total_buy > 0.0 {
        for sector in result.sector_suggestions {
            for asset in sector.asset_suggestions {
                rows.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td><td>{:.2}</td><td>{}</td></tr>",
                    asset.sector_name, asset.fund_name, asset.fund_code, sector.gap_value, asset.suggested_buy, asset.reason
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
                    <th>建议买入</th>
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
        result.max_daily_buy_total,
        config.portfolio.base_currency,
        rows
    );

    layout("今日买入建议", content)
}

async fn transactions_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let transactions = storage::load_transactions(&state.transactions_path).unwrap();

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

async fn assets_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let config = storage::load_config(&state.config_path).unwrap();

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

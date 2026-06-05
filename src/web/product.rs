//! Personal portfolio UI helpers (概览 / 持仓 / 市场).

use crate::models::{ConfigRoot, PortfolioState, PortfolioSummary};
use crate::repository::RepositoryContext;
use crate::web::AppState;
use std::sync::Arc;

pub fn equity_region_bucket(sector: &str) -> &'static str {
    let s = sector;
    if s.contains("美国") || s.contains("美股") || s.contains("纳指") || s.contains("标普")
    {
        "美国"
    } else if s.contains("日本") {
        "日本"
    } else if s.contains("越南") {
        "越南"
    } else if s.contains("印度") {
        "印度"
    } else if s.contains("欧洲") || s.contains("欧股") {
        "欧洲"
    } else if s.contains("港股") || s.contains("中国") || s.contains("A股") || s.contains("沪深")
    {
        "中国/港股"
    } else {
        "其他"
    }
}

pub fn pct_str(part: f64, total: f64) -> String {
    if total.abs() < 1e-6 {
        "—".to_string()
    } else {
        format!("{:.1}%", part / total * 100.0)
    }
}

pub fn allocation_row(label: &str, amount: f64, total: f64) -> String {
    format!(
        r#"<div class="alloc-row"><span class="alloc-label">{}</span><span class="alloc-amt tabular">{:.2} CNY</span><span class="alloc-pct">占总资产 {}</span></div>"#,
        label,
        amount,
        pct_str(amount, total)
    )
}

pub fn allocation_row_equity_region(
    region: &str,
    amount: f64,
    total_assets: f64,
    equity_total: f64,
) -> String {
    format!(
        r#"<div class="alloc-row"><span class="alloc-label">{}权益</span><span class="alloc-amt tabular">{:.2} CNY</span><span class="alloc-pct">占总资产 {}，占权益仓 {}</span></div>"#,
        region,
        amount,
        pct_str(amount, total_assets),
        pct_str(amount, equity_total)
    )
}

pub fn product_extra_css() -> &'static str {
    r#"
        .overview-metrics { display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 12px; margin-bottom: 20px; }
        .overview-metrics .card { padding: 14px 16px; margin-bottom: 0; }
        .overview-metrics .card-value { font-size: 1.35rem; margin: 4px 0; }
        .overview-compact .card { padding: 16px; margin-bottom: 16px; }
        .alloc-row { display: flex; flex-wrap: wrap; gap: 8px 16px; padding: 8px 0; border-bottom: 1px solid var(--bg-color); font-size: 0.88rem; align-items: baseline; }
        .alloc-row:last-child { border-bottom: none; }
        .alloc-label { font-weight: 700; min-width: 100px; }
        .alloc-amt { font-weight: 800; }
        .alloc-pct { color: var(--text-muted); font-size: 0.8rem; }
        .warn-compact { padding: 10px 14px; margin-bottom: 12px; border-radius: 8px; font-size: 0.85rem; background: #FFF7E8; border: 1px solid #FFE4BA; color: #996000; display: flex; justify-content: space-between; align-items: center; gap: 12px; }
        .auto-task-bar { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; padding: 10px 14px; background: var(--bg-color); border-radius: 8px; margin-bottom: 16px; font-size: 0.85rem; }
        .market-compact th { padding: 6px 8px; font-size: 0.7rem; position: sticky; top: 0; background: var(--bg-color); z-index: 1; }
        .market-compact td { padding: 5px 8px; font-size: 0.8rem; line-height: 1.3; vertical-align: middle; }
        .market-compact .price-cell { font-weight: 700; }
        .market-summary-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; margin-bottom: 12px; }
        .market-summary-grid .card { padding: 10px 12px; margin: 0; }
        .market-summary-grid .card-value { font-size: 1.1rem; }
        .market-toolbar { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-bottom: 10px; }
        .modal-overlay { display: none; position: fixed; inset: 0; background: rgba(0,0,0,0.45); z-index: 2000; align-items: center; justify-content: center; padding: 16px; }
        .modal-overlay.open { display: flex; }
        .modal-panel { background: #fff; border-radius: 12px; padding: 20px; max-width: 520px; width: 100%; max-height: 90vh; overflow-y: auto; box-shadow: var(--shadow-md); }
        .holdings-compact th { padding: 8px 10px; font-size: 0.75rem; }
        .holdings-compact td { padding: 8px 10px; font-size: 0.85rem; }
    "#
}

pub async fn auto_task_status_html(state: &Arc<AppState>, ctx: &RepositoryContext) -> String {
    use crate::models::WebJobStatus;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let daily = state
        .repo
        .get_latest_job(ctx, "daily_pipeline")
        .await
        .ok()
        .flatten();
    let market = state
        .repo
        .get_latest_job(ctx, "market_refresh")
        .await
        .ok()
        .flatten();

    let daily_label = daily
        .as_ref()
        .map(|j| {
            if matches!(j.status, WebJobStatus::Running | WebJobStatus::Queued) {
                "今日自动任务：运行中"
            } else if j
                .finished_at
                .as_deref()
                .is_some_and(|f| f.starts_with(&today))
                && matches!(
                    j.status,
                    WebJobStatus::Success | WebJobStatus::PartialSuccess
                )
            {
                "今日自动任务：已更新"
            } else if matches!(j.status, WebJobStatus::Failed) {
                "今日自动任务：失败"
            } else {
                "今日自动任务：待更新"
            }
        })
        .unwrap_or_else(|| "今日自动任务：待更新");

    let mkt_label = market
        .as_ref()
        .map(|j| {
            if matches!(j.status, WebJobStatus::Running | WebJobStatus::Queued) {
                "行情：刷新中"
            } else if j
                .finished_at
                .as_deref()
                .is_some_and(|f| f.starts_with(&today))
            {
                "行情：已同步"
            } else {
                "行情：待同步"
            }
        })
        .unwrap_or_else(|| "行情：待同步");

    format!(
        r#"<div class="auto-task-bar">
            <span>自动任务</span>
            <span class="badge badge-outline">{}</span>
            <span class="badge badge-outline">{}</span>
            <button type="button" class="btn btn-sm btn-outline" onclick="runDailyUpdate(this)">立即更新数据</button>
        </div>
        <script>
        async function runDailyUpdate(btn) {{
            if (btn) {{ btn.disabled = true; btn.innerText = '更新中…'; }}
            try {{
                await fetch('/api/jobs/market/refresh', {{ method: 'POST' }});
                await fetch('/api/jobs/daily/run', {{ method: 'POST' }});
                setTimeout(() => location.reload(), 2000);
            }} catch(e) {{
                alert('更新失败: ' + e);
                if (btn) {{ btn.disabled = false; btn.innerText = '立即更新数据'; }}
            }}
        }}
        </script>"#,
        daily_label, mkt_label
    )
}

pub fn aggregate_equity_by_region(
    config: &ConfigRoot,
    state: &PortfolioState,
) -> Vec<(String, f64)> {
    let mut buckets: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for h in &state.asset_holdings {
        let ac = config.assets.iter().find(|a| a.asset_id == h.asset_id);
        let sector = ac.map(|a| a.sector.as_str()).unwrap_or("未分类");
        let ac_class = config
            .sectors
            .iter()
            .find(|s| s.name == sector)
            .map(|s| s.asset_class.as_str())
            .unwrap_or("equity");
        if ac_class != "equity" {
            continue;
        }
        if !ac.map(|a| a.enabled).unwrap_or(false) {
            continue;
        }
        let region = equity_region_bucket(sector).to_string();
        *buckets.entry(region).or_insert(0.0) += h.last_market_value;
    }
    let mut v: Vec<_> = buckets.into_iter().collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    v
}

pub fn commodity_gold_value(summary: &PortfolioSummary, _config: &ConfigRoot) -> f64 {
    summary
        .sector_summaries
        .iter()
        .filter(|s| {
            s.asset_class == "spot"
                || s.sector_name.contains("黄金")
                || s.sector_name.contains("商品")
                || s.sector_name.contains("抗通胀")
        })
        .map(|s| s.current_value)
        .sum()
}

pub fn render_alipay_bootstrap_card(alipay_total: f64, snap_date: Option<&str>) -> String {
    let date_hint = snap_date
        .map(|d| format!("（快照日期 {}）", d))
        .unwrap_or_default();
    format!(
        r#"<div class="card" style="border:2px dashed var(--primary-color);margin-bottom:16px;">
            <h3 style="margin:0 0 8px;font-size:1rem;">检测到支付宝持仓快照</h3>
            <p style="color:var(--text-muted);font-size:0.88rem;margin:0 0 12px;">外部合计 <strong class="tabular">{:.2} CNY</strong>{}。本地账本为空，可一键初始化持仓。</p>
            <form action="/api/holdings/bootstrap-alipay" method="POST" style="display:inline;">
                <button type="submit" class="btn btn-sm">用支付宝快照初始化持仓</button>
            </form>
        </div>"#,
        alipay_total, date_hint
    )
}

pub fn snapshots_to_candidates(
    snaps: &std::collections::HashMap<String, crate::models::AlipaySnapshot>,
) -> Vec<crate::models::AlipayHoldingCandidate> {
    snaps
        .values()
        .map(|s| crate::models::AlipayHoldingCandidate {
            fund_code: s.fund_code.clone(),
            fund_name: s.fund_name.clone(),
            units: s.units.unwrap_or(0.0),
            market_value: s.market_value,
            nav: s.nav,
            nav_date: s.nav_date.clone(),
            cost_basis: s.cost_basis,
            total_profit: s.total_pnl,
            profit_rate: None,
            source: Some(s.source.clone()),
        })
        .collect()
}

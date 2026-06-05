//! Overview (概览) page handlers.

use crate::models;
use crate::web::product::{
    aggregate_equity_by_region, allocation_row, allocation_row_equity_region,
    auto_task_status_html, commodity_gold_value, product_extra_css,
};
use crate::web::services::overview_service::fetch_dashboard_summary;
use crate::web::state::AppState;
use crate::web::views::layout;
use axum::extract::State;
use axum::response::{Html, Json};
use chrono::Local;
use std::sync::Arc;

pub async fn dashboard_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let ctx = &state.ctx;
    match fetch_dashboard_summary(&state, ctx).await {
        Ok(summary) => {
            let portfolio_state = state.repo.load_state(ctx).await.unwrap_or_default();
            let config = state.repo.load_config(ctx).await.unwrap_or_default();

            let total_suggested: f64 = summary
                .decision
                .asset_explanations
                .iter()
                .map(|a| a.final_suggested_buy)
                .sum();

            let target_equity_pct = summary.operation_status.policy.target_equity_weight * 100.0;
            let current_equity_pct = if summary.portfolio.total_asset_value > 0.0 {
                summary.portfolio.equity_value / summary.portfolio.total_asset_value * 100.0
            } else {
                0.0
            };

            let display_total = if summary.portfolio.total_asset_value > 0.01 {
                summary.portfolio.total_asset_value
            } else {
                summary.alipay_total_value.unwrap_or(0.0)
            };

            let cash_mm = summary.portfolio.cash;
            let gold_val = commodity_gold_value(&summary.portfolio, &config);
            let other_val = (display_total
                - summary.portfolio.equity_value
                - summary.portfolio.bond_value
                - cash_mm
                - gold_val)
                .max(0.0);

            let mut warnings = String::new();
            if summary.alipay_total_value.is_some() && summary.portfolio.total_asset_value < 1.0 {
                warnings.push_str(r#"<div class="warn-compact"><span>检测到支付宝持仓快照，本地账本为空。请前往<a href="/holdings">持仓</a>初始化。</span><a href="/holdings" class="btn btn-sm">去持仓</a></div>"#);
            } else if summary.alipay_total_value.is_none() && summary.portfolio.fund_value < 1.0 {
                warnings.push_str(r#"<div class="warn-compact"><span>请先在<a href="/holdings">持仓</a>页导入或录入支付宝持仓。</span></div>"#);
            }
            if summary.unclassified_asset_count > 0 {
                warnings.push_str(&format!(
                    r#"<div class="warn-compact"><span>{} 个资产未分类，仓位统计可能不准。</span><button type="button" class="btn btn-sm btn-outline" onclick="autoClassify(this)">自动分类</button></div>"#,
                    summary.unclassified_asset_count
                ));
            }
            if summary.cache_status.market_cache_size == 0 {
                warnings.push_str(r#"<div class="warn-compact"><span>行情未更新，今日建议可能不可用。</span><button type="button" class="btn btn-sm btn-outline" onclick="refreshMarket(this)">刷新行情</button></div>"#);
            }
            if summary.portfolio.available_cash < 0.0 {
                warnings.push_str(r#"<div class="warn-compact"><span>现金余额异常，请在持仓页检查。</span><a href="/holdings" class="btn btn-sm btn-outline">持仓</a></div>"#);
            }

            let auto_task = auto_task_status_html(&state, ctx).await;

            let mut asset_class_html = String::new();
            asset_class_html.push_str(&allocation_row(
                "权益",
                summary.portfolio.equity_value,
                display_total,
            ));
            asset_class_html.push_str(&allocation_row(
                "债券",
                summary.portfolio.bond_value,
                display_total,
            ));
            asset_class_html.push_str(&allocation_row("货币基金/现金", cash_mm, display_total));
            if gold_val > 0.01 {
                asset_class_html.push_str(&allocation_row("黄金/商品", gold_val, display_total));
            }
            if other_val > 0.01 {
                asset_class_html.push_str(&allocation_row("其他", other_val, display_total));
            }

            let equity_total = summary.portfolio.equity_value.max(1e-6);
            let mut region_html = String::new();
            for (region, amt) in aggregate_equity_by_region(&config, &portfolio_state) {
                region_html.push_str(&allocation_row_equity_region(
                    &region,
                    amt,
                    display_total,
                    equity_total,
                ));
            }
            if region_html.is_empty() && summary.portfolio.equity_value < 1.0 {
                region_html =
                    "<div class='alloc-pct' style='padding:8px 0;'>暂无权益持仓数据</div>"
                        .to_string();
            }

            let mut sector_rows = String::new();
            for s in &summary.portfolio.sector_summaries {
                if !s.enabled {
                    continue;
                }
                let cur = s.current_weight * 100.0;
                let tgt = s.target_weight * 100.0;
                sector_rows.push_str(&format!(
                    r#"<tr><td>{}</td><td class="tabular">{:.2}</td><td class="tabular">{:.1}%</td><td class="tabular">{:.1}%</td><td class="tabular {:+}">{:+.1}%</td></tr>"#,
                    s.sector_name,
                    s.current_value,
                    cur,
                    tgt,
                    if (s.current_weight - s.target_weight) > 0.001 {
                        "text-up"
                    } else if (s.current_weight - s.target_weight) < -0.001 {
                        "text-down"
                    } else {
                        ""
                    },
                    (s.current_weight - s.target_weight) * 100.0
                ));
            }

            let content = format!(
                r#"
                <style>{}</style>
                <div class="overview-compact">
                    <h1 style="margin-bottom:4px;">概览</h1>
                    <p style="color:var(--text-muted);font-size:0.9rem;margin:0 0 16px 0;">我的资产分布、仓位比例与今日建议</p>
                    {}
                    {}
                    <div class="overview-metrics">
                        <div class="card"><div class="card-header"><span class="card-title">总资产</span></div><div class="card-value tabular">{:.2}</div><div class="card-sub">CNY</div></div>
                        <div class="card"><div class="card-header"><span class="card-title">权益仓</span></div><div class="card-value tabular">{:.2}</div></div>
                        <div class="card"><div class="card-header"><span class="card-title">债券</span></div><div class="card-value tabular">{:.2}</div></div>
                        <div class="card"><div class="card-header"><span class="card-title">货币/现金</span></div><div class="card-value tabular">{:.2}</div></div>
                        <div class="card"><div class="card-header"><span class="card-title">今日建议买入</span></div><div class="card-value tabular text-up">{:.2}</div></div>
                        <div class="card"><div class="card-header"><span class="card-title">权益仓位</span></div><div class="card-value tabular" style="font-size:1.1rem;">{:.1}% / {:.1}%</div><div class="card-sub">当前 / 目标</div></div>
                    </div>
                    <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px;">
                        <div class="card"><div class="card-header"><span class="card-title">大类资产分布</span></div>{}</div>
                        <div class="card"><div class="card-header"><span class="card-title">权益国家/地区</span></div>{}</div>
                    </div>
                    <div class="card">
                        <div class="card-header"><span class="card-title">赛道分布（当前 vs 目标）</span></div>
                        <div class="table-container"><table class="holdings-compact"><thead><tr><th>赛道</th><th>市值</th><th>当前%</th><th>目标%</th><th>偏差</th></tr></thead><tbody>{}</tbody></table></div>
                    </div>
                </div>
                <script>
                async function autoClassify(el) {{
                    if (el) el.disabled = true;
                    try {{ await fetch('/api/jobs/assets/auto-classify', {{method:'POST'}}); location.reload(); }}
                    catch(e) {{ alert('失败:'+e); if(el) el.disabled=false; }}
                }}
                </script>
                "#,
                product_extra_css(),
                auto_task,
                warnings,
                display_total,
                summary.portfolio.equity_value,
                summary.portfolio.bond_value,
                cash_mm,
                total_suggested,
                current_equity_pct,
                target_equity_pct,
                asset_class_html,
                region_html,
                sector_rows
            );

            layout("概览", content)
        }
        Err(e) => layout(
            "概览",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
    }
}

pub async fn api_dashboard_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::DashboardSummary> {
    let ctx = &state.ctx;
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

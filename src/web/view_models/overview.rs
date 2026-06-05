//! Overview (概览) display model.

use crate::models::{ConfigRoot, DashboardSummary, PortfolioState};
use crate::web::product::{
    aggregate_equity_by_region, allocation_row, allocation_row_equity_region,
    auto_task_status_html, commodity_gold_value,
};
use crate::web::state::AppState;
use anyhow::Result;
use std::sync::Arc;

pub struct OverviewPageVm {
    pub auto_task_html: String,
    pub warnings_html: String,
    pub display_total: f64,
    pub equity_value: f64,
    pub bond_value: f64,
    pub cash_mm: f64,
    pub total_suggested: f64,
    pub current_equity_pct: f64,
    pub target_equity_pct: f64,
    pub target_equity_weight: f64,
    pub asset_class_html: String,
    pub region_html: String,
    pub sector_rows_html: String,
}

pub async fn build_overview_vm(
    state: &Arc<AppState>,
    summary: &DashboardSummary,
    portfolio_state: &PortfolioState,
    config: &ConfigRoot,
) -> Result<OverviewPageVm> {
    let ctx = &state.ctx;

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
    let gold_val = commodity_gold_value(&summary.portfolio, config);
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

    let auto_task = auto_task_status_html(state, ctx).await;

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
    for (region, amt) in aggregate_equity_by_region(config, portfolio_state) {
        region_html.push_str(&allocation_row_equity_region(
            &region,
            amt,
            display_total,
            equity_total,
        ));
    }
    if region_html.is_empty() && summary.portfolio.equity_value < 1.0 {
        region_html =
            "<div class='alloc-pct' style='padding:8px 0;'>暂无权益持仓数据</div>".to_string();
    }

    let mut sector_rows = String::new();
    for s in &summary.portfolio.sector_summaries {
        if !s.enabled {
            continue;
        }
        let cur = s.current_weight * 100.0;
        let tgt = s.target_weight * 100.0;
        sector_rows.push_str(&format!(
            r#"<tr>
                <td>{}</td>
                <td class="tabular">{:.2}<div class="source-hint">来自持仓</div></td>
                <td class="tabular">{:.1}%<div class="source-hint">计算</div></td>
                <td class="tabular">{:.1}%<div class="source-hint">配置目标</div></td>
                <td class="tabular {:+}">{:+.1}%</td>
                <td><button type="button" class="btn-ghost btn-sm" onclick="editSectorTarget('{}', {:.4})">编辑目标</button></td>
            </tr>"#,
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
            (s.current_weight - s.target_weight) * 100.0,
            s.sector_name.replace('\'', "\\'"),
            s.target_weight
        ));
    }

    Ok(OverviewPageVm {
        auto_task_html: auto_task,
        warnings_html: warnings,
        display_total,
        equity_value: summary.portfolio.equity_value,
        bond_value: summary.portfolio.bond_value,
        cash_mm,
        total_suggested,
        current_equity_pct,
        target_equity_pct,
        target_equity_weight: summary.operation_status.policy.target_equity_weight,
        asset_class_html,
        region_html,
        sector_rows_html: sector_rows,
    })
}

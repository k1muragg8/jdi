use crate::engine::portfolio_summary::calculate_portfolio_summary;
use crate::models::{
    ConfigRoot, DailyExecutionPlan, DcaLifecycleSummary, GlobalRiskOverlay, InvestmentReport,
    PortfolioSnapshot, PortfolioState, PortfolioSummary, ReconciliationResult, ReportPeriod,
    ReportSection,
};
use chrono::Local;

pub fn create_portfolio_snapshot(config: &ConfigRoot, state: &PortfolioState) -> PortfolioSnapshot {
    let summary = calculate_portfolio_summary(config, state);
    PortfolioSnapshot {
        snapshot_id: format!("snap_{}", Local::now().timestamp_millis()),
        date: Local::now().format("%Y-%m-%d").to_string(),
        total_assets: summary.total_asset_value,
        cash: summary.cash,
        equity_value: summary.equity_value,
        fund_value: summary.fund_value,
        bond_value: summary.bond_value,
        crypto_value: summary.crypto_value,
        source: "system".to_string(),
        created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    }
}

pub fn generate_investment_report(
    report_type: ReportPeriod,
    title: &str,
    start_date: &str,
    end_date: &str,
    portfolio_summary: Option<PortfolioSummary>,
    dca_summary: Option<DcaLifecycleSummary>,
    risk_summary: Option<GlobalRiskOverlay>,
    _daily_plan: Option<DailyExecutionPlan>,
    reconciliation_results: &[ReconciliationResult],
    extended_summary: Option<crate::models::ReportSummary>,
) -> InvestmentReport {
    let mut sections = Vec::new();
    let mut warnings = Vec::new();
    let mut pending_actions = Vec::new();

    // 1. Portfolio Section
    if let Some(ref summary) = portfolio_summary {
        let mut details = vec![
            format!("总资产: {:.2}", summary.total_asset_value),
            format!("当前现金: {:.2}", summary.cash),
            format!("当前权益仓: {:.2}", summary.equity_value),
            format!("权益缺口: {:.2}", summary.equity_gap),
        ];

        for ss in &summary.sector_summaries {
            details.push(format!(
                "- {}: {:.2} ({:.2}%)",
                ss.sector_name,
                ss.current_value,
                ss.current_weight * 100.0
            ));
        }

        sections.push(ReportSection {
            title: "组合概览".to_string(),
            status: "正常".to_string(),
            summary: format!("当前总资产为 {:.2}。", summary.total_asset_value),
            details,
            warnings: vec![],
            suggested_actions: vec![],
        });
    }

    // 2. DCA Section
    if let Some(ref dca) = dca_summary {
        let mut status = "正常".to_string();
        if dca.count_waiting_confirmation > 0 || dca.count_unapplied > 0 {
            status = "需要关注".to_string();
            pending_actions.push("处理未闭环的定投事项".to_string());
        }

        sections.push(ReportSection {
            title: "定投执行".to_string(),
            status,
            summary: format!(
                "今日计划定投 {} 笔，共 {:.2} CNY。",
                dca.count_due, dca.total_planned_amount
            ),
            details: vec![
                format!("今日应投: {} 笔", dca.count_due),
                format!("已确认: {:.2} CNY", dca.total_confirmed_amount),
                format!("待确认: {} 笔", dca.count_waiting_confirmation),
                format!("待入账: {} 笔", dca.count_unapplied),
            ],
            warnings: vec![],
            suggested_actions: vec![],
        });
    }

    // 3. Reconciliation Section
    if !reconciliation_results.is_empty() {
        let mismatch_count = reconciliation_results
            .iter()
            .filter(|r| r.status != "一致")
            .count();
        let mut status = "正常".to_string();
        if mismatch_count > 0 {
            status = "存在对账差异".to_string();
            warnings.push(format!("有 {} 个资产存在对账差异。", mismatch_count));
            pending_actions.push("核对并修正对账差异".to_string());
        }

        sections.push(ReportSection {
            title: "对账状态".to_string(),
            status,
            summary: format!(
                "共核对 {} 个资产，{} 个一致。",
                reconciliation_results.len(),
                reconciliation_results.len() - mismatch_count
            ),
            details: reconciliation_results
                .iter()
                .map(|r| format!("{}: {}", r.asset_id, r.status))
                .collect(),
            warnings: vec![],
            suggested_actions: vec![],
        });
    }

    // 4. Risk Section
    if let Some(ref risk) = risk_summary {
        sections.push(ReportSection {
            title: "风险评估".to_string(),
            status: risk.risk_label.clone(),
            summary: format!(
                "全局风险评分为 {:.1}，等级为 {}。",
                risk.risk_score, risk.risk_label
            ),
            details: vec![risk.explanation.clone()],
            warnings: risk.warnings.clone(),
            suggested_actions: vec![],
        });
    }

    // 5. Extended Summary Section
    if let Some(ref ext) = extended_summary {
        let mut ext_details = vec![
            format!("期间交易: {} 笔，总额 {:.2}", ext.tx_summary.count, ext.tx_summary.total_amount),
            format!("买入: {:.2}，卖出: {:.2}，分红: {:.2}，手续费: {:.2}", ext.tx_summary.buy_amount, ext.tx_summary.sell_amount, ext.tx_summary.dividend_amount, ext.tx_summary.fee_amount),
            format!("现金流入: {:.2}，流出: {:.2}，净流入: {:.2}", ext.cash_flow.cash_in, ext.cash_flow.cash_out, ext.cash_flow.net_flow),
        ];
        if !ext.holding_changes.is_empty() {
            ext_details.push("主要持仓变动:".to_string());
            for hc in ext.holding_changes.iter().take(5) {
                ext_details.push(format!("- {}: 变动份额 {:.4}，变动价值 {:.2}", hc.asset_id, hc.units_changed, hc.value_changed));
            }
        }
        
        sections.push(ReportSection {
            title: "期间交易与现金流".to_string(),
            status: "统计完成".to_string(),
            summary: format!("期间净现金流入: {:.2}", ext.cash_flow.net_flow),
            details: ext_details,
            warnings: vec![],
            suggested_actions: vec![],
        });
    }

    InvestmentReport {
        report_id: format!("rep_{}", Local::now().timestamp_millis()),
        report_type,
        start_date: start_date.to_string(),
        end_date: end_date.to_string(),
        generated_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        title: title.to_string(),
        portfolio_summary,
        dca_summary,
        reconciliation_summary: None, // Simplified
        risk_summary,
        extended_summary,
        sections,
        warnings,
        pending_actions,
    }
}

pub fn render_report_to_markdown(report: &InvestmentReport) -> String {
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", report.title));
    md.push_str(&format!(
        "* 周期: {} 至 {}\n",
        report.start_date, report.end_date
    ));
    md.push_str(&format!("* 生成时间: {}\n\n", report.generated_at));

    if !report.warnings.is_empty() {
        md.push_str("## ⚠️ 警告\n\n");
        for w in &report.warnings {
            md.push_str(&format!("* {}\n", w));
        }
        md.push('\n');
    }

    if !report.pending_actions.is_empty() {
        md.push_str("## 📋 待处理事项\n\n");
        for a in &report.pending_actions {
            md.push_str(&format!("* [ ] {}\n", a));
        }
        md.push('\n');
    }

    for section in &report.sections {
        md.push_str(&format!("## {}\n\n", section.title));
        md.push_str(&format!("状态: **{}**\n\n", section.status));
        md.push_str(&format!("{}\n\n", section.summary));

        if !section.details.is_empty() {
            md.push_str("### 详细信息\n\n");
            for d in &section.details {
                md.push_str(&format!("* {}\n", d));
            }
            md.push('\n');
        }
    }

    md.push_str("---\n*本报告由 JDI Portfolio 自动生成*\n");
    md
}

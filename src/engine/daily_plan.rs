use crate::models::{
    AdjustedDecisionPreview, ConfigRoot, DailyExecutionItem, DailyExecutionPlan, DcaPreviewSummary,
    KellyPortfolioPreview, PortfolioState, ReconciliationResult,
};
use std::collections::HashMap;

pub fn generate_daily_execution_plan(
    config: &ConfigRoot,
    state: &PortfolioState,
    date: String,
    dca_preview: &DcaPreviewSummary,
    adjusted_decision: &AdjustedDecisionPreview,
    kelly_preview: &KellyPortfolioPreview,
    reconciliation_results: &[ReconciliationResult],
) -> DailyExecutionPlan {
    let mut total_dca_due = 0.0;
    let mut total_adjusted_decision = 0.0;
    let mut warnings: Vec<String> = Vec::new();

    let mut asset_map: HashMap<String, DailyExecutionItem> = HashMap::new();

    // 1. Process DCA items
    for dca_item in &dca_preview.items {
        if dca_item.status == "今日应投" {
            let entry = asset_map
                .entry(dca_item.asset_id.clone())
                .or_insert_with(|| {
                    let asset_config = config
                        .assets
                        .iter()
                        .find(|a| a.asset_id == dca_item.asset_id);
                    DailyExecutionItem {
                        asset_id: dca_item.asset_id.clone(),
                        fund_code: dca_item.fund_code.clone(),
                        fund_name: dca_item.fund_name.clone(),
                        sector: asset_config.map(|a| a.sector.clone()).unwrap_or_default(),
                        dca_due_amount: 0.0,
                        adjusted_decision_amount: 0.0,
                        kelly_preview_amount: 0.0,
                        recommended_amount: 0.0,
                        source: "DCA".to_string(),
                        reconciliation_status: "未知".to_string(),
                        reconciliation_warning: None,
                        data_status: "正常".to_string(),
                        confidence: 1.0,
                        status: "今日应执行".to_string(),
                        warnings: Vec::new(),
                        explanation: String::new(),
                    }
                });
            entry.dca_due_amount += dca_item.amount;
            total_dca_due += dca_item.amount;
        }
    }

    // 2. Process Adjusted Decision items
    for adj_item in &adjusted_decision.items {
        if adj_item.capped_adjusted_buy > 0.0 {
            let entry = asset_map
                .entry(adj_item.asset_id.clone())
                .or_insert_with(|| DailyExecutionItem {
                    asset_id: adj_item.asset_id.clone(),
                    fund_code: adj_item.fund_code.clone(),
                    fund_name: adj_item.fund_name.clone(),
                    sector: adj_item.sector.clone(),
                    dca_due_amount: 0.0,
                    adjusted_decision_amount: 0.0,
                    kelly_preview_amount: 0.0,
                    recommended_amount: 0.0,
                    source: "风险调整".to_string(),
                    reconciliation_status: "未知".to_string(),
                    reconciliation_warning: None,
                    data_status: "正常".to_string(),
                    confidence: 1.0,
                    status: "今日应执行".to_string(),
                    warnings: Vec::new(),
                    explanation: String::new(),
                });
            entry.adjusted_decision_amount += adj_item.capped_adjusted_buy;
            total_adjusted_decision += adj_item.capped_adjusted_buy;
            if entry.source == "DCA" {
                entry.source = "DCA+风险调整".to_string();
            }
        }
    }

    // 3. Process Kelly items (optional info)
    for k_item in &kelly_preview.results {
        if let Some(entry) = asset_map.get_mut(&k_item.asset_id) {
            entry.kelly_preview_amount = k_item.capped_preview_buy_amount;
        }
    }

    // 4. Enrich with reconciliation results
    for recon in reconciliation_results {
        if let Some(entry) = asset_map.get_mut(&recon.asset_id) {
            entry.reconciliation_status = recon.status.clone();
            if !recon.warnings.is_empty() {
                entry.reconciliation_warning = Some(recon.warnings.join("; "));
            }
        }
    }

    // 5. Apply combination rules and safety gates
    let mut final_items: Vec<DailyExecutionItem> = asset_map.into_values().collect();
    final_items.sort_by(|a, b| a.asset_id.cmp(&b.asset_id));

    let mut current_total = 0.0;
    let max_daily_buy = config.portfolio.max_daily_buy_total;

    for item in &mut final_items {
        item.recommended_amount = item.dca_due_amount + item.adjusted_decision_amount;

        // Gate 1: Reconciliation Mismatch
        if config.daily_plan.include_reconciliation_gate {
            if item.reconciliation_status == "明显差异"
                || item.reconciliation_status == "份额不一致"
                || item.reconciliation_status == "需要校准"
            {
                if config.daily_plan.pause_on_reconciliation_mismatch {
                    item.recommended_amount = 0.0;
                    item.status = "等待对账".to_string();
                    item.warnings
                        .push("支付宝对账存在明显差异，请先核对持仓后再买入。".to_string());
                }
            } else if item.reconciliation_status == "未知" {
                if config.daily_plan.pause_on_missing_reconciliation {
                    item.recommended_amount = 0.0;
                    item.status = "等待对账".to_string();
                    item.warnings
                        .push("缺少支付宝快照，建议执行前核对。".to_string());
                } else {
                    item.warnings
                        .push("缺少支付宝快照，建议执行前核对。".to_string());
                }
            }
        }

        // Gate 2: Extreme Global Risk
        if adjusted_decision.global_risk_label == "极高风险" {
            item.recommended_amount = 0.0;
            item.status = "暂停执行".to_string();
            item.warnings
                .push("全局风险极高，已自动暂停所有执行。".to_string());
        }

        // Gate 3: Mock Data
        if config.daily_plan.pause_on_mock_data {
            // Check if data status is mock.
            // We can infer this from adjusted_decision item status
            let adj_item = adjusted_decision
                .items
                .iter()
                .find(|a| a.asset_id == item.asset_id);
            if let Some(ai) = adj_item {
                if ai.status.contains("模拟") || ai.status.contains("数据不足") {
                    item.recommended_amount = 0.0;
                    item.status = "数据不足".to_string();
                    item.warnings
                        .push("行情数据为模拟或不足，已暂停执行。".to_string());
                }
            }
        }

        // Explanation
        let mut reasons = Vec::new();
        if item.dca_due_amount > 0.0 {
            reasons.push(format!("定投应投 {:.2}", item.dca_due_amount));
        }
        if item.adjusted_decision_amount > 0.0 {
            reasons.push(format!("风险调整建议 {:.2}", item.adjusted_decision_amount));
        }
        item.explanation = reasons.join(" + ");

        current_total += item.recommended_amount;
    }

    // Gate 4: Total Cap (Max Daily Buy & Available Cash)
    // We need to redistribute or cap. For now, simple capping.
    if current_total > max_daily_buy {
        let factor = max_daily_buy / current_total;
        warnings.push(format!(
            "建议买入总额 {:.2} 超过单日上限 {:.2}，已按比例缩减。",
            current_total, max_daily_buy
        ));
        current_total = 0.0;
        for item in &mut final_items {
            item.recommended_amount *= factor;
            current_total += item.recommended_amount;
        }
    }

    // Requirements say "cap total amount by available cash".
    // I need the "real" available cash (total - reserve - upcoming).
    // Let's use the summary engine.
    let summary = crate::engine::calculate_portfolio_summary(config, state);
    let real_available_cash = summary.available_cash;

    if current_total > real_available_cash {
        let factor = if current_total > 0.0 {
            real_available_cash / current_total
        } else {
            1.0
        };
        warnings.push(format!(
            "建议买入总额 {:.2} 超过可用现金 {:.2}，已按比例缩减。",
            current_total, real_available_cash
        ));
        current_total = 0.0;
        for item in &mut final_items {
            item.recommended_amount *= factor;
            current_total += item.recommended_amount;
        }
    }

    DailyExecutionPlan {
        date,
        total_dca_due,
        total_adjusted_decision,
        total_recommended_amount: current_total,
        available_cash: real_available_cash,
        max_daily_buy,
        global_risk_label: adjusted_decision.global_risk_label.clone(),
        global_risk_score: adjusted_decision.global_risk_score,
        items: final_items,
        warnings,
    }
}

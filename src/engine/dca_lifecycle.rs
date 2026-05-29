use crate::engine::dca::calculate_dca_preview;
use crate::engine::reconciliation::reconcile_asset;
use crate::models::{
    AlipaySnapshot, ConfigRoot, DcaLifecycleItem, DcaLifecycleSummary, DcaPlan, DcaSettlement,
    PortfolioState,
};

pub fn calculate_dca_lifecycle(
    config: &ConfigRoot,
    plans: &[DcaPlan],
    settlements: &[DcaSettlement],
    snapshots: &[AlipaySnapshot],
    state: &PortfolioState,
    target_date: &str,
) -> DcaLifecycleSummary {
    let mut items = Vec::new();
    let mut total_planned_amount = 0.0;
    let mut total_confirmed_amount = 0.0;
    let mut total_unapplied_settlement_amount = 0.0;
    let mut total_reconciliation_diff = 0.0;
    let mut count_due = 0;
    let mut count_waiting_confirmation = 0;
    let mut count_unapplied = 0;
    let mut count_reconciled = 0;
    let mut count_attention_required = 0;

    let dca_preview = calculate_dca_preview(config, plans, target_date);

    // Group items by asset_id
    // We care about assets that have a plan today, OR have unapplied settlements, OR have reconciliation issues.

    let asset_ids: std::collections::HashSet<String> =
        config.assets.iter().map(|a| a.asset_id.clone()).collect();

    for asset_id in asset_ids {
        let asset_config = config
            .assets
            .iter()
            .find(|a| a.asset_id == asset_id)
            .unwrap();
        let plan_item = dca_preview.items.iter().find(|i| i.asset_id == asset_id);

        // Find settlements for this asset
        // Matching rules: prefer scheduled_date matching target_date, OR deduction_date matching target_date
        let matching_settlements: Vec<&DcaSettlement> = settlements
            .iter()
            .filter(|s| s.asset_id == asset_id)
            .filter(|s| {
                s.scheduled_date.as_deref() == Some(target_date) || s.deduction_date == target_date
            })
            .collect();

        // Get latest snapshot for this asset
        let latest_snapshot = snapshots
            .iter()
            .filter(|s| s.asset_id == asset_id)
            .max_by_key(|s| s.snapshot_date.clone());

        // Get holding
        let holding = state.asset_holdings.iter().find(|h| h.asset_id == asset_id);

        let planned_amount = plan_item.map(|i| i.amount).unwrap_or(0.0);
        if plan_item.map(|i| i.status.as_str()) == Some("今日应投") {
            total_planned_amount += planned_amount;
            count_due += 1;
        }

        let mut lifecycle_status = "无计划".to_string();
        let mut suggested_next_action = "无需处理".to_string();

        let settlement = matching_settlements.first(); // Simplify to first for now

        if let Some(s) = settlement {
            total_confirmed_amount += s.amount;
            if s.applied {
                lifecycle_status = "已入账".to_string();
            } else {
                lifecycle_status = "已确认未入账".to_string();
                suggested_next_action = "执行定投确认入账".to_string();
                total_unapplied_settlement_amount += s.amount;
                count_unapplied += 1;
            }
        } else if plan_item.map(|i| i.status.as_str()) == Some("今日应投") {
            lifecycle_status = "今日应定投".to_string();
            suggested_next_action = "录入定投确认".to_string();
            count_waiting_confirmation += 1;
        }

        let mut current_reconciliation_status = "未知".to_string();

        // Refine status with reconciliation if applied
        if lifecycle_status == "已入账"
            || (settlement.is_none() && plan_item.is_none() && holding.is_some())
        {
            if let Some(snap) = latest_snapshot {
                let recon = reconcile_asset(config, state, snap);
                current_reconciliation_status = recon.status.clone();

                if recon.status == "一致" {
                    lifecycle_status = "对账一致".to_string();
                    count_reconciled += 1;
                } else if recon.status == "小幅差异" {
                    lifecycle_status = "对账小幅差异".to_string();
                    suggested_next_action = "先处理对账差异".to_string();
                    total_reconciliation_diff += recon.market_value_diff.abs();
                } else {
                    lifecycle_status = "对账明显差异".to_string();
                    suggested_next_action = "需要人工处理".to_string();
                    count_attention_required += 1;
                    total_reconciliation_diff += recon.market_value_diff.abs();
                }
            } else if holding.is_some() {
                lifecycle_status = "等待支付宝快照".to_string();
                suggested_next_action = "录入支付宝快照".to_string();
            }
        }
        let reconciliation_status: String = current_reconciliation_status;

        if !asset_config.enabled {
            lifecycle_status = "已暂停".to_string();
            suggested_next_action = "无需处理".to_string();
        }

        items.push(DcaLifecycleItem {
            date: target_date.to_string(),
            asset_id: asset_id.clone(),
            fund_code: asset_config.fund_code.clone(),
            fund_name: asset_config.fund_name.clone(),
            plan_id: plan_item.map(|i| i.plan_id.clone()),
            planned_amount,
            settlement_id: settlement.map(|s| s.settlement_id.clone()),
            settlement_amount: settlement.map(|s| s.amount),
            confirmed_nav: settlement.map(|s| s.confirmed_nav),
            confirmed_units: settlement.map(|s| s.confirmed_units),
            settlement_applied: settlement.map(|s| s.applied).unwrap_or(false),
            latest_alipay_snapshot_id: latest_snapshot.map(|s| s.snapshot_id.clone()),
            alipay_market_value: latest_snapshot.map(|s| s.market_value),
            system_market_value: holding.map(|h| h.last_market_value),
            reconciliation_status,
            lifecycle_status,
            warnings: Vec::new(),
            suggested_next_action,
        });
    }

    items.sort_by(|a, b| a.asset_id.cmp(&b.asset_id));

    DcaLifecycleSummary {
        date: target_date.to_string(),
        total_planned_amount,
        total_confirmed_amount,
        total_unapplied_settlement_amount,
        total_reconciliation_diff,
        count_due,
        count_waiting_confirmation,
        count_unapplied,
        count_reconciled,
        count_attention_required,
        items,
        warnings: Vec::new(),
    }
}

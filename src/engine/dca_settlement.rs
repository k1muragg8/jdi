use crate::models::{ConfigRoot, DcaSettlement, DcaSettlementImpact, PortfolioState};

pub fn calculate_settlement_impact(
    _config: &ConfigRoot,
    state: &PortfolioState,
    settlement: &DcaSettlement,
) -> DcaSettlementImpact {
    let mut warnings = Vec::new();

    let holding = state
        .asset_holdings
        .iter()
        .find(|h| h.asset_id == settlement.asset_id);

    let old_units = holding.map(|h| h.units).unwrap_or(0.0);
    let old_cost_basis = holding.map(|h| h.cost_basis).unwrap_or(0.0);
    let old_market_value = holding.map(|h| h.last_market_value).unwrap_or(0.0);

    let new_units = old_units + settlement.confirmed_units;

    // cost basis calculation: (old_units * old_cost_basis + settlement_amount) / new_units
    // assuming amount is the total cost including fees.
    let new_cost_basis = if new_units > 0.0 {
        (old_units * old_cost_basis + settlement.amount) / new_units
    } else {
        0.0
    };

    let estimated_new_market_value = new_units * settlement.confirmed_nav;

    if holding.is_none() {
        warnings.push("系统中未找到该资产的持仓记录，将初始化新持仓。".to_string());
    }

    DcaSettlementImpact {
        settlement_id: settlement.settlement_id.clone(),
        asset_id: settlement.asset_id.clone(),
        fund_code: settlement.fund_code.clone(),
        fund_name: settlement.fund_name.clone(),
        amount: settlement.amount,
        confirmed_nav: settlement.confirmed_nav,
        confirmed_units: settlement.confirmed_units,
        old_units,
        new_units,
        old_cost_basis,
        new_cost_basis,
        old_market_value,
        estimated_new_market_value,
        would_modify_state: true,
        would_create_transaction: true,
        warnings,
    }
}

pub fn apply_settlement(
    state: &mut PortfolioState,
    settlement: &DcaSettlement,
    impact: &DcaSettlementImpact,
) -> crate::models::DcaSettlementAudit {
    let holding = state
        .asset_holdings
        .iter_mut()
        .find(|h| h.asset_id == settlement.asset_id);

    let audit = crate::models::DcaSettlementAudit {
        audit_id: format!("audit_dca_{}", chrono::Local::now().timestamp_millis()),
        timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        settlement_id: settlement.settlement_id.clone(),
        asset_id: settlement.asset_id.clone(),
        old_units: impact.old_units,
        new_units: impact.new_units,
        old_cost_basis: impact.old_cost_basis,
        new_cost_basis: impact.new_cost_basis,
        transaction_id: None,
        note: None,
    };

    if let Some(h) = holding {
        h.units = impact.new_units;
        h.cost_basis = impact.new_cost_basis;
        // Update market value too if we have it
        h.last_market_value = impact.estimated_new_market_value;
        h.latest_nav = Some(settlement.confirmed_nav);
        h.latest_nav_date = Some(settlement.confirmation_date.clone());
    } else {
        // Create new holding
        state.asset_holdings.push(crate::models::AssetHolding {
            asset_id: settlement.asset_id.clone(),
            fund_code: settlement.fund_code.clone(),
            units: impact.new_units,
            units_estimated: false,
            cost_basis: impact.new_cost_basis,
            last_market_value: impact.estimated_new_market_value,
            latest_nav: Some(settlement.confirmed_nav),
            latest_nav_date: Some(settlement.confirmation_date.clone()),
            latest_nav_source: Some("dca_settlement".to_string()),
            latest_nav_status: Some("正常".to_string()),
        });
    }

    audit
}

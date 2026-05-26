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

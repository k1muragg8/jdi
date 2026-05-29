use crate::models::{
    PortfolioState, ReportCashFlowSummary, ReportHoldingChange, ReportSummary,
    ReportTransactionSummary, Transaction,
};
use std::collections::HashMap;

pub fn generate_report_summary(
    portfolio_id: &str,
    backend: &str,
    start_date: &str,
    end_date: &str,
    transactions: &[Transaction],
    state: &PortfolioState,
) -> ReportSummary {
    let mut tx_summary = ReportTransactionSummary::default();
    let mut cash_flow = ReportCashFlowSummary::default();
    let mut holding_deltas: HashMap<String, f64> = HashMap::new();

    for tx in transactions {
        if tx.date.as_str() >= start_date && tx.date.as_str() <= end_date {
            tx_summary.count += 1;
            tx_summary.total_amount += tx.amount;
            tx_summary.fee_amount += tx.fee;

            match tx.transaction_type.as_str() {
                "buy" | "买入" => {
                    tx_summary.buy_amount += tx.amount;
                    if let Some(asset_id) = &tx.asset_id {
                        *holding_deltas.entry(asset_id.clone()).or_default() += tx.units.unwrap_or(0.0);
                    }
                }
                "sell" | "卖出" => {
                    tx_summary.sell_amount += tx.amount;
                    if let Some(asset_id) = &tx.asset_id {
                        *holding_deltas.entry(asset_id.clone()).or_default() -= tx.units.unwrap_or(0.0);
                    }
                }
                "dividend" | "分红" => {
                    tx_summary.dividend_amount += tx.amount;
                    cash_flow.cash_in += tx.amount;
                }
                "cash_in" | "现金转入" => {
                    cash_flow.cash_in += tx.amount;
                }
                "cash_out" | "现金转出" => {
                    cash_flow.cash_out += tx.amount;
                }
                _ => {}
            }
        }
    }

    cash_flow.net_flow = cash_flow.cash_in - cash_flow.cash_out;

    let mut holding_changes: Vec<ReportHoldingChange> = holding_deltas
        .into_iter()
        .filter(|(_, units)| *units != 0.0)
        .map(|(asset_id, units_changed)| {
            // estimate value changed (very roughly if we don't have historical prices)
            let current_price = state
                .asset_holdings
                .iter()
                .find(|a| a.asset_id == *asset_id)
                .and_then(|a| a.latest_nav)
                .unwrap_or(1.0);
            ReportHoldingChange {
                asset_id,
                units_changed,
                value_changed: units_changed * current_price,
            }
        })
        .collect();

    holding_changes.sort_by(|a, b| {
        b.value_changed
            .abs()
            .partial_cmp(&a.value_changed.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut top_holdings: Vec<_> = state.asset_holdings.iter().collect();
    top_holdings.sort_by(|a, b| {
        b.last_market_value
            .partial_cmp(&a.last_market_value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let top_holdings_ids = top_holdings
        .into_iter()
        .take(5)
        .map(|a| a.asset_id.clone())
        .collect();

    let final_value = state.cash + state.asset_holdings.iter().map(|a| a.last_market_value).sum::<f64>();
    // Approximate initial value = final_value - net_flow (ignoring market returns for simplicity if historical prices are missing)
    let initial_value = final_value - cash_flow.net_flow; 
    let estimated_return = final_value - initial_value - cash_flow.net_flow;

    ReportSummary {
        portfolio_id: portfolio_id.to_string(),
        backend: backend.to_string(),
        period_start: start_date.to_string(),
        period_end: end_date.to_string(),
        initial_value,
        final_value,
        estimated_return,
        tx_summary,
        cash_flow,
        holding_changes,
        top_holdings: top_holdings_ids,
    }
}

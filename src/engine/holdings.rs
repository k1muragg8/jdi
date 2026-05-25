use crate::models::{AssetHolding, PortfolioState, Transaction};
use anyhow::Result;

pub fn rebuild_holdings_from_transactions(transactions: &[Transaction]) -> Result<PortfolioState> {
    let mut state = PortfolioState {
        cash: 0.0,
        asset_holdings: Vec::new(),
    };

    for tx in transactions {
        match tx.transaction_type.as_str() {
            "buy" => {
                let asset_id = tx.asset_id.clone().unwrap_or_default();
                let units = tx.units.unwrap_or(0.0);

                let mut found = false;
                for holding in &mut state.asset_holdings {
                    if holding.asset_id == asset_id {
                        holding.units += units;
                        holding.cost_basis += tx.amount + tx.fee;
                        found = true;
                        break;
                    }
                }

                if !found {
                    state.asset_holdings.push(AssetHolding {
                        asset_id: asset_id.clone(),
                        fund_code: "".to_string(), // In a real app we'd map this, for now rely on config/sync
                        units,
                        units_estimated: false,
                        cost_basis: tx.amount + tx.fee,
                        latest_nav: None,
                        latest_nav_date: None,
                        last_market_value: tx.amount,
                    });
                }
            }
            "sell" => {
                let asset_id = tx.asset_id.clone().unwrap_or_default();
                let units = tx.units.unwrap_or(0.0);

                for holding in &mut state.asset_holdings {
                    if holding.asset_id == asset_id {
                        holding.units -= units;
                        // TODO: Implement complex cost basis reduction. For now, proportional or simple reduction
                        let fraction = if holding.units + units > 0.0 {
                            units / (holding.units + units)
                        } else {
                            0.0
                        };
                        holding.cost_basis -= holding.cost_basis * fraction;
                        break;
                    }
                }
            }
            "cash_in" => {
                state.cash += tx.amount;
            }
            "cash_out" | "expense" => {
                state.cash -= tx.amount;
            }
            _ => {
                // Ignore unknown transaction types
            }
        }
    }

    Ok(state)
}

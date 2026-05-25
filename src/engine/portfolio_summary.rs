use crate::models::{ConfigRoot, PortfolioState};

#[derive(Debug, Clone)]
pub struct SectorSummary {
    pub sector_id: String,
    pub sector_name: String,
    pub asset_class: String,
    pub target_weight: f64,
    pub target_value: f64,
    pub current_value: f64,
    pub current_weight: f64,
    pub gap_value: f64,
    pub gap_ratio: f64,
    pub priority: i32,
    pub enabled: bool,
    pub status: String, // "underweight", "neutral", "overweight", "disabled"
}

#[derive(Debug, Clone)]
pub struct PortfolioSummary {
    pub cash: f64,
    pub equity_value: f64,
    pub bond_value: f64,
    pub crypto_value: f64,
    pub fund_value: f64,
    pub total_asset_value: f64,
    pub target_equity_value: f64,
    pub equity_gap: f64,
    pub available_cash: f64,
    pub reserve_cash: f64,
    pub upcoming_expense: f64,
    pub sector_summaries: Vec<SectorSummary>,
}

pub fn calculate_portfolio_summary(
    config: &ConfigRoot,
    state: &PortfolioState,
) -> PortfolioSummary {
    let mut fund_value = 0.0;
    let mut equity_value = 0.0;
    let mut bond_value = 0.0;
    let mut crypto_value = 0.0;

    for holding in &state.asset_holdings {
        let asset_config = config
            .assets
            .iter()
            .find(|a| a.asset_id == holding.asset_id);
        if let Some(ac) = asset_config {
            if ac.enabled {
                fund_value += holding.last_market_value;

                let sector_class = config
                    .sectors
                    .iter()
                    .find(|s| s.name == ac.sector)
                    .map(|s| s.asset_class.as_str())
                    .unwrap_or("equity");

                match sector_class {
                    "equity" => equity_value += holding.last_market_value,
                    "bond" => bond_value += holding.last_market_value,
                    "crypto" => crypto_value += holding.last_market_value,
                    _ => {} // default grouping behavior for unknown classes can be customized
                }
            }
        }
    }

    let cash = state.cash;
    let total_asset_value = cash + fund_value;
    let target_equity_value = config.portfolio.target_equity_value;
    let equity_gap = target_equity_value - equity_value;
    let available_cash = cash - config.portfolio.reserve_cash - config.portfolio.upcoming_expense;

    let mut sector_summaries = Vec::new();
    for sector in &config.sectors {
        let mut current_value = 0.0;

        for holding in &state.asset_holdings {
            let asset_config = config
                .assets
                .iter()
                .find(|a| a.asset_id == holding.asset_id);
            if let Some(ac) = asset_config {
                // Spec allows matching by sector name for now
                if ac.enabled && ac.sector == sector.name {
                    current_value += holding.last_market_value;
                }
            }
        }

        let target_value = target_equity_value * sector.target_weight;
        let current_weight = if target_equity_value > 0.0 {
            current_value / target_equity_value
        } else {
            0.0
        };

        let gap_value = target_value - current_value;
        let gap_ratio = if target_value > 0.0 {
            gap_value / target_value
        } else {
            0.0
        };

        let status = if !sector.enabled {
            "disabled".to_string()
        } else if gap_value > 1.0 {
            "underweight".to_string()
        } else if gap_value < -1.0 {
            "overweight".to_string()
        } else {
            "neutral".to_string()
        };

        sector_summaries.push(SectorSummary {
            sector_id: sector.sector_id.clone(),
            sector_name: sector.name.clone(),
            asset_class: sector.asset_class.clone(),
            target_weight: sector.target_weight,
            target_value,
            current_value,
            current_weight,
            gap_value,
            gap_ratio,
            priority: sector.priority,
            enabled: sector.enabled,
            status,
        });
    }

    PortfolioSummary {
        cash,
        equity_value,
        bond_value,
        crypto_value,
        fund_value,
        total_asset_value,
        target_equity_value,
        equity_gap,
        available_cash,
        reserve_cash: config.portfolio.reserve_cash,
        upcoming_expense: config.portfolio.upcoming_expense,
        sector_summaries,
    }
}

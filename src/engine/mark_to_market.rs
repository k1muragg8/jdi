use crate::api::FundProvider;
use crate::models::{ConfigRoot, PortfolioState};
use anyhow::Result;

pub fn mark_to_market<P: FundProvider>(
    config: &ConfigRoot,
    state: &mut PortfolioState,
    fund_provider: &P,
) -> Result<()> {
    for holding in &mut state.asset_holdings {
        // Find corresponding asset config
        if let Some(asset_config) = config
            .assets
            .iter()
            .find(|a| a.asset_id == holding.asset_id && a.enabled)
        {
            if holding.fund_code.is_empty() {
                holding.fund_code = asset_config.fund_code.clone();
            }

            if let Ok(nav_data) = fund_provider.fetch_latest_nav(&asset_config.fund_code) {
                holding.latest_nav = Some(nav_data.nav);
                holding.latest_nav_date = Some(nav_data.nav_date.clone());

                if holding.units > 0.0 {
                    holding.last_market_value = holding.units * nav_data.nav;
                }
            }
        }
    }

    Ok(())
}

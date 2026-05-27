use super::holding::AssetHolding;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PortfolioState {
    pub cash: f64,
    #[serde(default)]
    pub asset_holdings: Vec<AssetHolding>,
}

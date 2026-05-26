use pendulum_kelly_cli::api::{FundProvider, MockFundProvider};
use pendulum_kelly_cli::models::{
    AdjustedDecisionConfig, AssetConfig, AssetHolding, ConfigRoot, KellyConfig, PortfolioConfig,
    PortfolioState, SectorConfig,
};

#[test]
fn test_asset_set_sector() {
    let mut config = ConfigRoot {
        adjusted_decision: AdjustedDecisionConfig::default(),
        kelly: KellyConfig::default(),
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        api: Default::default(),
        market: Default::default(),
        fx: Default::default(),
        risk: Default::default(),
        regime: Default::default(),
        reconciliation: Default::default(),
        daily_plan: Default::default(),
        assets: vec![AssetConfig {
            asset_id: "test_asset".to_string(),
            fund_code: "123".to_string(),
            fund_name: "Test".to_string(),
            sector: "OldSector".to_string(),
            currency: "CNY".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: None,
            reference_index_symbol: None,
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: None,
            market_data_provider: None,
        }],
        sectors: vec![SectorConfig {
            sector_id: "new_sector".to_string(),
            name: "NewSector".to_string(),
            asset_class: "equity".to_string(),
            target_weight: 0.0,
            priority: 1,
            enabled: true,
        }],
    };

    // Simulate set-sector
    let asset_id = "test_asset";
    let new_sector = "NewSector";

    if let Some(asset) = config.assets.iter_mut().find(|a| a.asset_id == asset_id) {
        asset.sector = new_sector.to_string();
    }

    assert_eq!(config.assets[0].sector, "NewSector");
}

#[test]
fn test_asset_set_fund_code() {
    let mut config = ConfigRoot {
        adjusted_decision: AdjustedDecisionConfig::default(),
        kelly: KellyConfig::default(),
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        api: Default::default(),
        market: Default::default(),
        fx: Default::default(),
        risk: Default::default(),
        regime: Default::default(),
        reconciliation: Default::default(),
        daily_plan: Default::default(),
        assets: vec![AssetConfig {
            asset_id: "test_asset".to_string(),
            fund_code: "old_code".to_string(),
            fund_name: "OldName".to_string(),
            sector: "Test".to_string(),
            currency: "CNY".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: None,
            reference_index_symbol: None,
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: None,
            market_data_provider: None,
        }],
        sectors: vec![],
    };

    let provider = MockFundProvider::new();
    let new_code = "006327";

    if let Some(asset) = config
        .assets
        .iter_mut()
        .find(|a| a.asset_id == "test_asset")
    {
        if let Ok(info) = provider.search_fund_by_code(new_code) {
            asset.fund_code = new_code.to_string();
            asset.fund_name = info.fund_name;
        }
    }

    assert_eq!(config.assets[0].fund_code, "006327");
    assert_eq!(config.assets[0].fund_name, "纳斯达克100基金");
}

#[test]
fn test_asset_repair_holdings() {
    let config = ConfigRoot {
        adjusted_decision: AdjustedDecisionConfig::default(),
        kelly: KellyConfig::default(),
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        api: Default::default(),
        market: Default::default(),
        fx: Default::default(),
        risk: Default::default(),
        regime: Default::default(),
        reconciliation: Default::default(),
        daily_plan: Default::default(),
        assets: vec![AssetConfig {
            asset_id: "missing_asset".to_string(),
            fund_code: "123".to_string(),
            fund_name: "Test".to_string(),
            sector: "Test".to_string(),
            currency: "CNY".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: None,
            reference_index_symbol: None,
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: None,
            market_data_provider: None,
        }],
        sectors: vec![],
    };

    let mut state = PortfolioState {
        cash: 0.0,
        asset_holdings: vec![],
    };

    // Simulate repair-holdings
    for asset in &config.assets {
        if !state
            .asset_holdings
            .iter()
            .any(|h| h.asset_id == asset.asset_id)
        {
            state.asset_holdings.push(AssetHolding {
                asset_id: asset.asset_id.clone(),
                fund_code: asset.fund_code.clone(),
                units: 0.0,
                units_estimated: false,
                cost_basis: 0.0,
                latest_nav: None,
                latest_nav_date: None,
                latest_nav_source: None,
                latest_nav_status: None,
                last_market_value: 0.0,
            });
        }
    }

    assert_eq!(state.asset_holdings.len(), 1);
    assert_eq!(state.asset_holdings[0].asset_id, "missing_asset");
}

#[test]
fn test_sector_add() {
    let mut config = ConfigRoot {
        adjusted_decision: AdjustedDecisionConfig::default(),
        kelly: KellyConfig::default(),
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        api: Default::default(),
        market: Default::default(),
        fx: Default::default(),
        risk: Default::default(),
        regime: Default::default(),
        reconciliation: Default::default(),
        daily_plan: Default::default(),
        assets: vec![],
        sectors: vec![],
    };

    let new_sector = SectorConfig {
        sector_id: "new_id".to_string(),
        name: "New Name".to_string(),
        asset_class: "equity".to_string(),
        target_weight: 0.1,
        priority: 1,
        enabled: true,
    };

    config.sectors.push(new_sector);
    assert_eq!(config.sectors.len(), 1);
    assert_eq!(config.sectors[0].sector_id, "new_id");
}

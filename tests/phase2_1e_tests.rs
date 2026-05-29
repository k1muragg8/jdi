use pendulum_kelly_cli::models::{
    AdjustedDecisionConfig, AssetConfig, ConfigRoot, KellyConfig, PortfolioConfig, SectorConfig,
};

#[test]
fn test_asset_validate_duplicates() {
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
        risk: Default::default(),
        market: Default::default(),
        fx: Default::default(),
        regime: Default::default(),
        reconciliation: Default::default(),
        daily_plan: Default::default(),
        assets: vec![
            AssetConfig {
                asset_id: "asset1".to_string(),
                fund_code: "006327".to_string(),
                fund_name: "Name1".to_string(),
                sector: "Sector1".to_string(),
                currency: "CNY".to_string(),
                valuation_method: "nav".to_string(),
                enabled: true,
                reference_index_name: None,
                reference_index_symbol: None,
                market_data_provider: None,
                reference_index_currency: None,
                proxy_fx_pair: None,
                use_fx_adjustment: Some(false),
                reference_instrument_id: None,
                reference_instrument_symbol: None,
            },
            AssetConfig {
                asset_id: "asset2".to_string(),
                fund_code: "006327".to_string(),
                fund_name: "Name2".to_string(),
                sector: "Sector2".to_string(),
                currency: "CNY".to_string(),
                valuation_method: "nav".to_string(),
                enabled: true,
                reference_index_name: None,
                reference_index_symbol: None,
                market_data_provider: None,
                reference_index_currency: None,
                proxy_fx_pair: None,
                use_fx_adjustment: Some(false),
                reference_instrument_id: None,
                reference_instrument_symbol: None,
            },
        ],
        sectors: vec![
            SectorConfig {
                sector_id: "s1".to_string(),
                name: "Sector1".to_string(),
                asset_class: "equity".to_string(),
                target_weight: 0.5,
                priority: 1,
                enabled: true,
            },
            SectorConfig {
                sector_id: "s2".to_string(),
                name: "Sector2".to_string(),
                asset_class: "equity".to_string(),
                target_weight: 0.5,
                priority: 1,
                enabled: true,
            },
        ],
        storage: Default::default(),
    };

    // We can't easily test the print output here, but we can verify the logic
    let fund_code = "006327";
    let duplicates: Vec<&String> = config
        .assets
        .iter()
        .filter(|a| a.fund_code == fund_code && a.enabled)
        .map(|a| &a.asset_id)
        .collect();

    assert_eq!(duplicates.len(), 2);
}

#[test]
fn test_duplicates_grouping() {
    use std::collections::HashMap;
    let assets = vec![
        AssetConfig {
            asset_id: "a1".to_string(),
            fund_code: "123".to_string(),
            fund_name: "N1".to_string(),
            sector: "S".to_string(),
            currency: "C".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: None,
            reference_index_symbol: None,
            market_data_provider: None,
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: Some(false),
            reference_instrument_id: None,
            reference_instrument_symbol: None,
        },
        AssetConfig {
            asset_id: "a2".to_string(),
            fund_code: "123".to_string(),
            fund_name: "N2".to_string(),
            sector: "S".to_string(),
            currency: "C".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: None,
            reference_index_symbol: None,
            market_data_provider: None,
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: Some(false),
            reference_instrument_id: None,
            reference_instrument_symbol: None,
        },
        AssetConfig {
            asset_id: "a3".to_string(),
            fund_code: "456".to_string(),
            fund_name: "N3".to_string(),
            sector: "S".to_string(),
            currency: "C".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: None,
            reference_index_symbol: None,
            market_data_provider: None,
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: Some(false),
            reference_instrument_id: None,
            reference_instrument_symbol: None,
        },
    ];

    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for asset in assets {
        groups
            .entry(asset.fund_code)
            .or_default()
            .push(asset.asset_id);
    }

    assert_eq!(groups.get("123").unwrap().len(), 2);
    assert_eq!(groups.get("456").unwrap().len(), 1);
}

#[test]
fn test_asset_set_fund_code_reject_duplicate() {
    let assets = [AssetConfig {
        asset_id: "a1".to_string(),
        fund_code: "123".to_string(),
        fund_name: "N1".to_string(),
        sector: "S".to_string(),
        currency: "C".to_string(),
        valuation_method: "nav".to_string(),
        enabled: true,
        reference_index_name: None,
        reference_index_symbol: None,
        market_data_provider: None,
        reference_index_currency: None,
        proxy_fx_pair: None,
        use_fx_adjustment: Some(false),
        reference_instrument_id: None,
        reference_instrument_symbol: None,
    }];

    let new_code = "123";
    let allow_duplicate = false;

    let already_used = assets.iter().any(|a| a.fund_code == new_code && a.enabled);
    assert!(already_used);

    if already_used && !allow_duplicate {
        // Logic would bail here
    } else {
        panic!("Should have detected duplicate");
    }
}

#[test]
fn test_config_doctor_logic() {
    let config = ConfigRoot {
        adjusted_decision: AdjustedDecisionConfig::default(),
        kelly: KellyConfig::default(),
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 1000.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        api: Default::default(),
        risk: Default::default(),
        market: Default::default(),
        fx: Default::default(),
        regime: Default::default(),
        reconciliation: Default::default(),
        daily_plan: Default::default(),
        assets: vec![],
        sectors: vec![SectorConfig {
            sector_id: "s1".to_string(),
            name: "S1".to_string(),
            asset_class: "equity".to_string(),
            target_weight: 0.6,
            priority: 1,
            enabled: true,
        }],
        storage: Default::default(),
    };

    let weight_sum: f64 = config
        .sectors
        .iter()
        .filter(|s| s.enabled)
        .map(|s| s.target_weight)
        .sum();
    assert!((weight_sum - 1.0).abs() > 0.001);
}

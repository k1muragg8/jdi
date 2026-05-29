use pendulum_kelly_cli::engine::dca::calculate_dca_preview;
use pendulum_kelly_cli::models::{ConfigRoot, DcaFrequency, DcaPlan};
use pendulum_kelly_cli::storage::dca_store::{load_dca_plans, save_dca_plans};
use std::fs;

#[test]
fn test_dca_plans_storage() {
    let path = "data/test_dca_plans.json";
    if std::path::Path::new(path).exists() {
        fs::remove_file(path).unwrap();
    }

    // Test missing file
    let plans = load_dca_plans(path).unwrap();
    assert!(plans.is_empty());

    // Test save and load
    let plan = DcaPlan {
        plan_id: "test_1".to_string(),
        asset_id: "nasdaq_100".to_string(),
        fund_code: "006327".to_string(),
        fund_name: "Nasdaq".to_string(),
        amount: 100.0,
        currency: "CNY".to_string(),
        frequency: DcaFrequency::Daily,
        weekday: None,
        month_day: None,
        start_date: "2023-01-01".to_string(),
        end_date: None,
        enabled: true,
        priority: 0,
        note: None,
    };

    save_dca_plans(path, std::slice::from_ref(&plan)).unwrap();
    let loaded = load_dca_plans(path).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].plan_id, "test_1");

    fs::remove_file(path).unwrap();
}

#[test]
fn test_dca_preview_logic() {
    let mut config = ConfigRoot::default();
    config.assets.push(pendulum_kelly_cli::models::AssetConfig {
        asset_id: "test_asset".to_string(),
        fund_code: "000001".to_string(),
        fund_name: "Test Fund".to_string(),
        sector: "Tech".to_string(),
        enabled: true,
        currency: "CNY".to_string(),
        valuation_method: "nav".to_string(),
        reference_index_name: None,
        reference_index_symbol: None,
        market_data_provider: None,
        reference_index_currency: None,
        proxy_fx_pair: None,
        use_fx_adjustment: Some(false),
        reference_instrument_id: None,
        reference_instrument_symbol: None,
    });

    let plans = vec![
        DcaPlan {
            plan_id: "daily".to_string(),
            asset_id: "test_asset".to_string(),
            fund_code: "000001".to_string(),
            fund_name: "Test Fund".to_string(),
            amount: 100.0,
            currency: "CNY".to_string(),
            frequency: DcaFrequency::Daily,
            weekday: None,
            month_day: None,
            start_date: "2023-01-01".to_string(),
            end_date: None,
            enabled: true,
            priority: 1,
            note: None,
        },
        DcaPlan {
            plan_id: "weekly_mon".to_string(),
            asset_id: "test_asset".to_string(),
            fund_code: "000001".to_string(),
            fund_name: "Test Fund".to_string(),
            amount: 200.0,
            currency: "CNY".to_string(),
            frequency: DcaFrequency::Weekly,
            weekday: Some(1), // Monday
            month_day: None,
            start_date: "2023-01-01".to_string(),
            end_date: None,
            enabled: true,
            priority: 0,
            note: None,
        },
    ];

    // Monday
    let preview = calculate_dca_preview(&config, &plans, "2026-05-25");
    assert_eq!(preview.total_due_amount, 300.0);
    assert_eq!(preview.items.len(), 2);

    // Tuesday
    let preview = calculate_dca_preview(&config, &plans, "2026-05-26");
    assert_eq!(preview.total_due_amount, 100.0);
}

#[test]
fn test_dca_preview_disabled() {
    let config = ConfigRoot::default();
    let plans = vec![DcaPlan {
        plan_id: "daily".to_string(),
        asset_id: "unknown".to_string(),
        fund_code: "000001".to_string(),
        fund_name: "Test Fund".to_string(),
        amount: 100.0,
        currency: "CNY".to_string(),
        frequency: DcaFrequency::Daily,
        weekday: None,
        month_day: None,
        start_date: "2023-01-01".to_string(),
        end_date: None,
        enabled: false,
        priority: 1,
        note: None,
    }];

    let preview = calculate_dca_preview(&config, &plans, "2026-05-25");
    assert_eq!(preview.total_due_amount, 0.0);
    assert_eq!(preview.items[0].status, "已禁用");
}

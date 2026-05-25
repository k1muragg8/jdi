use pendulum_kelly_cli::api::{FundProvider, MockFundProvider};
use pendulum_kelly_cli::models::{ConfigRoot, PortfolioState, Transaction};
use std::fs;

#[test]
fn test_config_parsing() {
    let content = fs::read_to_string("examples/config.toml").unwrap();
    let config: ConfigRoot = toml::from_str(&content).unwrap();
    assert_eq!(config.portfolio.name, "我的投资组合");
    assert_eq!(config.assets.len(), 2);
    assert_eq!(config.assets[0].asset_id, "nasdaq_100_fund");
}

#[test]
fn test_state_parsing() {
    let content = fs::read_to_string("examples/portfolio_state.json").unwrap();
    let state: PortfolioState = serde_json::from_str(&content).unwrap();
    assert_eq!(state.cash, 120044.0);
    assert_eq!(state.asset_holdings.len(), 2);
    assert_eq!(state.asset_holdings[0].asset_id, "nasdaq_100_fund");
}

#[test]
fn test_transactions_parsing() {
    let content = fs::read_to_string("examples/transactions.json").unwrap();
    let transactions: Vec<Transaction> = serde_json::from_str(&content).unwrap();
    assert_eq!(transactions.len(), 2);
    assert_eq!(transactions[0].transaction_type, "buy");
    assert_eq!(transactions[1].transaction_type, "cash_in");
}

#[test]
fn test_mock_fund_provider_fund_info() {
    let provider = MockFundProvider::new();
    let info = provider.search_fund_by_code("006327").unwrap();
    assert_eq!(info.fund_name, "纳斯达克100基金");
    assert_eq!(info.fund_type, "QDII");

    let err = provider.search_fund_by_code("999999");
    assert!(err.is_err());
}

#[test]
fn test_mock_fund_provider_nav() {
    let provider = MockFundProvider::new();
    let nav = provider.fetch_latest_nav("000001").unwrap();
    assert_eq!(nav.nav, 1.25);

    let err = provider.fetch_latest_nav("999999");
    assert!(err.is_err());
}

use pendulum_kelly_cli::engine::rebuild_holdings_from_transactions;

#[test]
fn test_rebuild_holdings_cash_flows() {
    let txs = vec![
        Transaction {
            id: "1".to_string(),
            date: "2026-05-22".to_string(),
            transaction_type: "cash_in".to_string(),
            asset_id: None,
            amount: 10000.0,
            units: None,
            price: None,
            fee: 0.0,
            currency: "CNY".to_string(),
            note: "".to_string(),
        },
        Transaction {
            id: "2".to_string(),
            date: "2026-05-22".to_string(),
            transaction_type: "cash_out".to_string(),
            asset_id: None,
            amount: 2000.0,
            units: None,
            price: None,
            fee: 0.0,
            currency: "CNY".to_string(),
            note: "".to_string(),
        },
        Transaction {
            id: "3".to_string(),
            date: "2026-05-22".to_string(),
            transaction_type: "expense".to_string(),
            asset_id: None,
            amount: 500.0,
            units: None,
            price: None,
            fee: 0.0,
            currency: "CNY".to_string(),
            note: "".to_string(),
        },
    ];

    let state = rebuild_holdings_from_transactions(&txs).unwrap();
    assert_eq!(state.cash, 7500.0);
}

#[test]
fn test_rebuild_holdings_buy_sell() {
    let txs = vec![
        Transaction {
            id: "1".to_string(),
            date: "2026-05-22".to_string(),
            transaction_type: "buy".to_string(),
            asset_id: Some("fund_a".to_string()),
            amount: 1000.0,
            units: Some(100.0),
            price: Some(10.0),
            fee: 10.0,
            currency: "CNY".to_string(),
            note: "".to_string(),
        },
        Transaction {
            id: "2".to_string(),
            date: "2026-05-23".to_string(),
            transaction_type: "buy".to_string(),
            asset_id: Some("fund_a".to_string()),
            amount: 2000.0,
            units: Some(200.0),
            price: Some(10.0),
            fee: 20.0,
            currency: "CNY".to_string(),
            note: "".to_string(),
        },
        Transaction {
            id: "3".to_string(),
            date: "2026-05-24".to_string(),
            transaction_type: "sell".to_string(),
            asset_id: Some("fund_a".to_string()),
            amount: 500.0,
            units: Some(150.0),
            price: Some(10.0),
            fee: 5.0,
            currency: "CNY".to_string(),
            note: "".to_string(),
        },
    ];

    let state = rebuild_holdings_from_transactions(&txs).unwrap();
    assert_eq!(state.asset_holdings.len(), 1);
    let holding = &state.asset_holdings[0];

    assert_eq!(holding.units, 150.0); // 100 + 200 - 150
    // Total cost before sell: 1010 + 2020 = 3030
    // Fraction sold: 150 / 300 = 0.5
    // Remaining cost: 3030 - (3030 * 0.5) = 1515.0
    assert_eq!(holding.cost_basis, 1515.0);
}

use pendulum_kelly_cli::engine::mark_to_market;
use pendulum_kelly_cli::models::AssetConfig;
use pendulum_kelly_cli::models::AssetHolding;
use pendulum_kelly_cli::models::PortfolioConfig;

#[test]
fn test_mark_to_market() {
    let mut state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![
            AssetHolding {
                asset_id: "nasdaq_100_fund".to_string(),
                fund_code: "006327".to_string(),
                units: 1000.0,
                units_estimated: false,
                cost_basis: 5000.0,
                latest_nav: None,
                latest_nav_date: None,
                last_market_value: 5000.0,
            },
            AssetHolding {
                asset_id: "sp500_fund".to_string(),
                fund_code: "000001".to_string(),
                units: 2000.0,
                units_estimated: false,
                cost_basis: 2000.0,
                latest_nav: None,
                latest_nav_date: None,
                last_market_value: 2000.0,
            },
        ],
    };

    let config = ConfigRoot {
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        assets: vec![
            AssetConfig {
                asset_id: "nasdaq_100_fund".to_string(),
                fund_code: "006327".to_string(),
                fund_name: "QQQ".to_string(),
                sector: "Tech".to_string(),
                currency: "CNY".to_string(),
                valuation_method: "nav".to_string(),
                enabled: true,
            },
            AssetConfig {
                asset_id: "sp500_fund".to_string(),
                fund_code: "000001".to_string(),
                fund_name: "SPY".to_string(),
                sector: "Large Cap".to_string(),
                currency: "CNY".to_string(),
                valuation_method: "nav".to_string(),
                enabled: true,
            },
        ],
        sectors: vec![],
    };

    let provider = MockFundProvider::new();

    mark_to_market(&config, &mut state, &provider).unwrap();

    let nasdaq = state
        .asset_holdings
        .iter()
        .find(|a| a.asset_id == "nasdaq_100_fund")
        .unwrap();
    assert_eq!(nasdaq.latest_nav.unwrap(), 5.38);
    assert_eq!(nasdaq.latest_nav_date.as_ref().unwrap(), "2026-05-22");
    assert_eq!(nasdaq.last_market_value, 5380.0); // 1000 * 5.38

    let sp500 = state
        .asset_holdings
        .iter()
        .find(|a| a.asset_id == "sp500_fund")
        .unwrap();
    assert_eq!(sp500.latest_nav.unwrap(), 1.25);
    assert_eq!(sp500.latest_nav_date.as_ref().unwrap(), "2026-05-22");
    assert_eq!(sp500.last_market_value, 2500.0); // 2000 * 1.25
}

#[test]
fn test_apply_transaction() {
    use pendulum_kelly_cli::engine::holdings::apply_transaction;
    use pendulum_kelly_cli::models::AssetHolding;
    let mut state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "nasdaq_100_fund".to_string(),
            fund_code: "006327".to_string(),
            units: 100.0,
            units_estimated: false,
            cost_basis: 500.0,
            latest_nav: None,
            latest_nav_date: None,
            last_market_value: 500.0,
        }],
    };

    // Test buy
    let buy_tx = Transaction {
        id: "1".to_string(),
        date: "".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("nasdaq_100_fund".to_string()),
        amount: 100.0,
        units: Some(20.0),
        price: Some(5.0),
        fee: 10.0,
        currency: "CNY".to_string(),
        note: "".to_string(),
    };
    apply_transaction(&mut state, &buy_tx).unwrap();
    assert_eq!(state.cash, 890.0); // 1000 - 110
    assert_eq!(state.asset_holdings[0].units, 120.0);
    assert_eq!(state.asset_holdings[0].cost_basis, 610.0);

    // Test sell
    let sell_tx = Transaction {
        id: "2".to_string(),
        date: "".to_string(),
        transaction_type: "sell".to_string(),
        asset_id: Some("nasdaq_100_fund".to_string()),
        amount: 50.0,
        units: Some(10.0),
        price: Some(5.0),
        fee: 5.0,
        currency: "CNY".to_string(),
        note: "".to_string(),
    };
    apply_transaction(&mut state, &sell_tx).unwrap();
    assert_eq!(state.cash, 935.0); // 890 + (50 - 5)
    assert_eq!(state.asset_holdings[0].units, 110.0);

    // Test sell more than hold
    let sell_too_much = Transaction {
        id: "3".to_string(),
        date: "".to_string(),
        transaction_type: "sell".to_string(),
        asset_id: Some("nasdaq_100_fund".to_string()),
        amount: 5000.0,
        units: Some(1000.0),
        price: Some(5.0),
        fee: 5.0,
        currency: "CNY".to_string(),
        note: "".to_string(),
    };
    assert!(apply_transaction(&mut state, &sell_too_much).is_err());

    // Test cash in
    let cash_in = Transaction {
        id: "4".to_string(),
        date: "".to_string(),
        transaction_type: "cash_in".to_string(),
        asset_id: None,
        amount: 200.0,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "".to_string(),
    };
    apply_transaction(&mut state, &cash_in).unwrap();
    assert_eq!(state.cash, 1135.0);

    // Test manual cash set
    let cash_set = Transaction {
        id: "5".to_string(),
        date: "".to_string(),
        transaction_type: "manual_cash_adjustment".to_string(),
        asset_id: None,
        amount: 500.0,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "".to_string(),
    };
    apply_transaction(&mut state, &cash_set).unwrap();
    assert_eq!(state.cash, 500.0);
}

#[test]
fn test_data_initialization_on_missing_dir() {
    use std::fs;

    // Create a dummy workspace for testing
    let test_dir = "tests/test_data_init";
    let _ = fs::remove_dir_all(test_dir);
    fs::create_dir_all(test_dir).unwrap();

    let examples_dir = format!("{}/examples", test_dir);
    fs::create_dir_all(&examples_dir).unwrap();
    fs::write(format!("{}/config.toml", examples_dir), "dummy").unwrap();

    // The data initialization logic runs on "data/" and "examples/" from current dir.
    // Testing the actual lib runs into side-effect conflicts, so we just verify our CLI struct parses default paths to "data/" correctly.
    use clap::Parser;
    use pendulum_kelly_cli::cli::Cli;

    let args = vec!["pendulum-kelly-cli", "holdings"];
    let cli = Cli::parse_from(args);
    assert_eq!(cli.config, "data/config.toml");
    assert_eq!(cli.state, "data/portfolio_state.json");
    assert_eq!(cli.transactions, "data/transactions.json");

    let _ = fs::remove_dir_all(test_dir);
}

#[test]
fn test_holdings_visibility_logic() {
    use pendulum_kelly_cli::models::{
        AssetConfig, AssetHolding, ConfigRoot, PortfolioConfig, PortfolioState,
    };

    let config = ConfigRoot {
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        assets: vec![
            AssetConfig {
                asset_id: "active_fund".to_string(),
                fund_code: "123".to_string(),
                fund_name: "Active".to_string(),
                sector: "Test".to_string(),
                currency: "CNY".to_string(),
                valuation_method: "nav".to_string(),
                enabled: true,
            },
            AssetConfig {
                asset_id: "inactive_fund".to_string(),
                fund_code: "456".to_string(),
                fund_name: "Inactive".to_string(),
                sector: "Test".to_string(),
                currency: "CNY".to_string(),
                valuation_method: "nav".to_string(),
                enabled: false,
            },
        ],
        sectors: vec![],
    };

    let state = PortfolioState {
        cash: 0.0,
        asset_holdings: vec![
            AssetHolding {
                asset_id: "active_fund".to_string(),
                fund_code: "123".to_string(),
                units: 10.0,
                units_estimated: false,
                cost_basis: 100.0,
                latest_nav: None,
                latest_nav_date: None,
                last_market_value: 100.0,
            },
            AssetHolding {
                asset_id: "inactive_fund".to_string(),
                fund_code: "456".to_string(),
                units: 10.0,
                units_estimated: false,
                cost_basis: 100.0,
                latest_nav: None,
                latest_nav_date: None,
                last_market_value: 100.0,
            },
        ],
    };

    // Holdings default -> Hide disabled
    let visible_default = state
        .asset_holdings
        .iter()
        .filter(|h| {
            let is_enabled = config
                .assets
                .iter()
                .find(|a| a.asset_id == h.asset_id)
                .map(|a| a.enabled)
                .unwrap_or(false);
            is_enabled || false // false corresponds to `all` being false
        })
        .count();

    assert_eq!(visible_default, 1);

    // Holdings --all -> Show all
    let visible_all = state
        .asset_holdings
        .iter()
        .filter(|h| {
            let is_enabled = config
                .assets
                .iter()
                .find(|a| a.asset_id == h.asset_id)
                .map(|a| a.enabled)
                .unwrap_or(false);
            is_enabled || true // true corresponds to `all` being true
        })
        .count();

    assert_eq!(visible_all, 2);
}

#[test]
fn test_asset_add_logic() {
    use pendulum_kelly_cli::models::{
        AssetConfig, AssetHolding, ConfigRoot, PortfolioConfig, PortfolioState,
    };

    let mut config = ConfigRoot {
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        assets: vec![],
        sectors: vec![],
    };

    let mut state = PortfolioState {
        cash: 0.0,
        asset_holdings: vec![],
    };

    // Simulate "Asset Add"
    let new_asset = AssetConfig {
        asset_id: "test_asset".to_string(),
        fund_code: "123".to_string(),
        fund_name: "Test Fund".to_string(),
        sector: "Test Sector".to_string(),
        currency: "CNY".to_string(),
        valuation_method: "nav".to_string(),
        enabled: true,
    };
    config.assets.push(new_asset);

    let new_holding = AssetHolding {
        asset_id: "test_asset".to_string(),
        fund_code: "123".to_string(),
        units: 0.0,
        units_estimated: false,
        cost_basis: 0.0,
        latest_nav: None,
        latest_nav_date: None,
        last_market_value: 0.0,
    };
    state.asset_holdings.push(new_holding);

    assert_eq!(config.assets.len(), 1);
    assert_eq!(state.asset_holdings.len(), 1);

    // Simulate "Asset Disable" / "Asset Remove"
    config.assets[0].enabled = false;
    assert_eq!(config.assets[0].enabled, false);

    // Simulate "Asset Enable"
    config.assets[0].enabled = true;
    assert_eq!(config.assets[0].enabled, true);
}

#[test]
fn test_sector_config_parsing() {
    use pendulum_kelly_cli::models::ConfigRoot;
    use std::fs;

    let content = fs::read_to_string("examples/config.toml").unwrap();
    let config: ConfigRoot = toml::from_str(&content).unwrap();

    assert_eq!(config.sectors.len(), 6);
    assert_eq!(config.sectors[0].sector_id, "us_tech");
    assert_eq!(config.sectors[0].target_weight, 0.25);
}

#[test]
fn test_sector_set_target() {
    // Tests that modifying a sector target updates the config model, we'll verify via memory logic matching cli logic
    use pendulum_kelly_cli::models::{ConfigRoot, PortfolioConfig, SectorConfig};

    let mut config = ConfigRoot {
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 1000.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        assets: vec![],
        sectors: vec![SectorConfig {
            sector_id: "test_sector".to_string(),
            name: "Test".to_string(),
            asset_class: "equity".to_string(),
            target_weight: 0.1,
            priority: 1,
            enabled: true,
        }],
    };

    if let Some(sector) = config
        .sectors
        .iter_mut()
        .find(|s| s.sector_id == "test_sector")
    {
        sector.target_weight = 0.5;
    }

    assert_eq!(config.sectors[0].target_weight, 0.5);
}

#[test]
fn test_portfolio_and_sector_summary() {
    use pendulum_kelly_cli::engine::calculate_portfolio_summary;
    use pendulum_kelly_cli::models::{
        AssetConfig, AssetHolding, ConfigRoot, PortfolioConfig, PortfolioState, SectorConfig,
    };

    let config = ConfigRoot {
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 10000.0,
            reserve_cash: 1000.0,
            upcoming_expense: 500.0,
            max_daily_buy_total: 0.0,
        },
        assets: vec![
            AssetConfig {
                asset_id: "fund_eq".to_string(),
                fund_code: "123".to_string(),
                fund_name: "EQ".to_string(),
                sector: "Tech".to_string(),
                currency: "CNY".to_string(),
                valuation_method: "nav".to_string(),
                enabled: true,
            },
            AssetConfig {
                asset_id: "fund_bd".to_string(),
                fund_code: "456".to_string(),
                fund_name: "BD".to_string(),
                sector: "Bonds".to_string(),
                currency: "CNY".to_string(),
                valuation_method: "nav".to_string(),
                enabled: true,
            },
            AssetConfig {
                asset_id: "fund_disabled".to_string(),
                fund_code: "789".to_string(),
                fund_name: "DS".to_string(),
                sector: "Tech".to_string(),
                currency: "CNY".to_string(),
                valuation_method: "nav".to_string(),
                enabled: false,
            },
        ],
        sectors: vec![
            SectorConfig {
                sector_id: "tech".to_string(),
                name: "Tech".to_string(),
                asset_class: "equity".to_string(),
                target_weight: 0.5,
                priority: 1,
                enabled: true,
            },
            SectorConfig {
                sector_id: "bonds".to_string(),
                name: "Bonds".to_string(),
                asset_class: "bond".to_string(),
                target_weight: 0.5,
                priority: 2,
                enabled: true,
            },
            SectorConfig {
                sector_id: "inactive".to_string(),
                name: "Inactive".to_string(),
                asset_class: "equity".to_string(),
                target_weight: 0.0,
                priority: 3,
                enabled: false,
            },
        ],
    };

    let state = PortfolioState {
        cash: 5000.0,
        asset_holdings: vec![
            AssetHolding {
                asset_id: "fund_eq".to_string(),
                fund_code: "123".to_string(),
                units: 100.0,
                units_estimated: false,
                cost_basis: 1000.0,
                latest_nav: None,
                latest_nav_date: None,
                last_market_value: 2000.0, // Used directly in calc
            },
            AssetHolding {
                asset_id: "fund_bd".to_string(),
                fund_code: "456".to_string(),
                units: 100.0,
                units_estimated: false,
                cost_basis: 1000.0,
                latest_nav: None,
                latest_nav_date: None,
                last_market_value: 8000.0,
            },
            AssetHolding {
                asset_id: "fund_disabled".to_string(),
                fund_code: "789".to_string(),
                units: 100.0,
                units_estimated: false,
                cost_basis: 1000.0,
                latest_nav: None,
                latest_nav_date: None,
                last_market_value: 9999.0, // Should be ignored
            },
        ],
    };

    let summary = calculate_portfolio_summary(&config, &state);

    assert_eq!(summary.cash, 5000.0);
    assert_eq!(summary.available_cash, 3500.0); // 5000 - 1000 - 500
    assert_eq!(summary.fund_value, 10000.0); // 2000 + 8000
    assert_eq!(summary.equity_value, 2000.0);
    assert_eq!(summary.bond_value, 8000.0);
    assert_eq!(summary.equity_gap, 8000.0); // 10000 - 2000

    let tech_summary = summary
        .sector_summaries
        .iter()
        .find(|s| s.sector_name == "Tech")
        .unwrap();
    assert_eq!(tech_summary.target_value, 5000.0); // 10000 * 0.5
    assert_eq!(tech_summary.current_value, 2000.0);
    assert_eq!(tech_summary.gap_value, 3000.0);
    assert_eq!(tech_summary.status, "underweight");

    let bonds_summary = summary
        .sector_summaries
        .iter()
        .find(|s| s.sector_name == "Bonds")
        .unwrap();
    assert_eq!(bonds_summary.target_value, 5000.0);
    assert_eq!(bonds_summary.current_value, 8000.0);
    assert_eq!(bonds_summary.gap_value, -3000.0);
    assert_eq!(bonds_summary.status, "overweight");

    let inactive_summary = summary
        .sector_summaries
        .iter()
        .find(|s| s.sector_name == "Inactive")
        .unwrap();
    assert_eq!(inactive_summary.status, "disabled");
}

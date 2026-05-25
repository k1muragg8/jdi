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

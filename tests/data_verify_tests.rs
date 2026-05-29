use pendulum_kelly_cli::engine::verification::{VerificationSeverity, verify_data};
use pendulum_kelly_cli::models::{AssetHolding, PortfolioState, Transaction};
use pendulum_kelly_cli::repository::{
    JsonRepository, RepositoryContext, traits::PortfolioRepository,
};

fn create_temp_json_repo(dir: &std::path::Path) -> JsonRepository {
    JsonRepository::new_with_defaults(dir.to_str().unwrap())
}

#[tokio::test]
async fn test_verify_clean_data() {
    let dir_path = "data/test_verify_clean";
    let _ = std::fs::create_dir_all(dir_path);
    let repo = create_temp_json_repo(std::path::Path::new(dir_path));
    let ctx = RepositoryContext::default_json();

    let tx = Transaction {
        id: "tx1".to_string(),
        date: "2026-05-20".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("asset1".to_string()),
        amount: 100.0,
        units: Some(1.0),
        price: Some(100.0),
        fee: 0.0,
        currency: "USD".to_string(),
        note: "test".to_string(),
        source: "manual".to_string(),
        raw_description: "".to_string(),
    };

    let tx2 = Transaction {
        id: "tx2".to_string(),
        date: "2026-05-21".to_string(),
        transaction_type: "cash_in".to_string(),
        asset_id: None,
        amount: 100.0,
        units: None,
        price: None,
        fee: 0.0,
        currency: "USD".to_string(),
        note: "cash".to_string(),
        source: "manual".to_string(),
        raw_description: "".to_string(),
    };

    repo.save_transactions(&ctx, &[tx.clone(), tx2.clone()])
        .await
        .unwrap();

    let mut state = PortfolioState {
        cash: 0.0,
        ..Default::default()
    };
    state.asset_holdings.push(AssetHolding {
        asset_id: "asset1".to_string(),
        fund_code: "".to_string(),
        units: 1.0,
        units_estimated: false,
        cost_basis: 0.0,
        latest_nav: Some(0.0),
        latest_nav_date: Some("".to_string()),
        latest_nav_source: None,
        latest_nav_status: None,
        last_market_value: 0.0,
    });
    repo.save_state(&ctx, &state).await.unwrap();

    let report = verify_data(&repo, &ctx, false).await.unwrap();
    assert_eq!(report.summary.errors, 0);
    assert_eq!(report.summary.warnings, 0);
}

#[tokio::test]
async fn test_verify_duplicate_tx() {
    let dir_path = "data/test_verify_dup";
    let _ = std::fs::create_dir_all(dir_path);
    let repo = create_temp_json_repo(std::path::Path::new(dir_path));
    let ctx = RepositoryContext::default_json();

    let tx = Transaction {
        id: "tx1".to_string(),
        date: "2026-05-20".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("asset1".to_string()),
        amount: 100.0,
        units: Some(1.0),
        price: Some(100.0),
        fee: 0.0,
        currency: "USD".to_string(),
        note: "test".to_string(),
        source: "manual".to_string(),
        raw_description: "".to_string(),
    };

    // Duplicate ID
    repo.save_transactions(&ctx, &[tx.clone(), tx.clone()])
        .await
        .unwrap();

    let report = verify_data(&repo, &ctx, false).await.unwrap();
    assert!(report.summary.errors > 0);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.message.contains("Duplicate transaction ID"))
    );
}

#[tokio::test]
async fn test_verify_mismatches() {
    let dir_path = "data/test_verify_mismatch";
    let _ = std::fs::create_dir_all(dir_path);
    let repo = create_temp_json_repo(std::path::Path::new(dir_path));
    let ctx = RepositoryContext::default_json();

    let tx = Transaction {
        id: "tx1".to_string(),
        date: "2026-05-20".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("asset1".to_string()),
        amount: -100.0, // negative amount error
        units: Some(1.0),
        price: Some(100.0),
        fee: 0.0,
        currency: "USD".to_string(),
        note: "test".to_string(),
        source: "manual".to_string(),
        raw_description: "".to_string(),
    };

    repo.save_transactions(&ctx, std::slice::from_ref(&tx))
        .await
        .unwrap();

    let mut state = PortfolioState {
        cash: 999.0,
        ..Default::default()
    }; // cash mismatch
    state.asset_holdings.push(AssetHolding {
        asset_id: "asset1".to_string(),
        fund_code: "".to_string(),
        units: 2.0, // holding mismatch (tx gives 1.0)
        units_estimated: false,
        cost_basis: 0.0,
        latest_nav: Some(0.0),
        latest_nav_date: Some("".to_string()),
        latest_nav_source: None,
        latest_nav_status: None,
        last_market_value: 0.0,
    });
    repo.save_state(&ctx, &state).await.unwrap();

    let report = verify_data(&repo, &ctx, false).await.unwrap();
    assert!(report.summary.errors > 0);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.message.contains("Negative amount"))
    );
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.message.contains("Cash balance mismatch"))
    );
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.message.contains("Holding units mismatch"))
    );
}

#[tokio::test]
async fn test_verify_strict_mode() {
    let dir_path = "data/test_verify_strict";
    let _ = std::fs::create_dir_all(dir_path);
    let repo = create_temp_json_repo(std::path::Path::new(dir_path));
    let ctx = RepositoryContext::default_json();

    let tx1 = Transaction {
        id: "tx1".to_string(),
        date: "2026-05-20".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("asset1".to_string()),
        amount: 100.0,
        units: Some(1.0),
        price: Some(100.0),
        fee: 0.0,
        currency: "USD".to_string(),
        note: "test".to_string(),
        source: "manual".to_string(),
        raw_description: "".to_string(),
    };

    let mut tx2 = tx1.clone();
    tx2.id = "tx2".to_string(); // Different ID, but same contents = fingerprint match

    repo.save_transactions(&ctx, &[tx1.clone(), tx2.clone()])
        .await
        .unwrap();

    let report_normal = verify_data(&repo, &ctx, false).await.unwrap();
    // In normal mode, fingerprint match is a warning
    assert!(report_normal.summary.warnings > 0);
    assert!(
        report_normal
            .issues
            .iter()
            .any(|i| i.severity == VerificationSeverity::Warning
                && i.message.contains("Possible duplicate"))
    );

    let report_strict = verify_data(&repo, &ctx, true).await.unwrap();
    // In strict mode, fingerprint match is an error
    assert!(
        report_strict
            .issues
            .iter()
            .any(|i| i.severity == VerificationSeverity::Error
                && i.message.contains("Possible duplicate"))
    );
}

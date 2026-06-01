use pendulum_kelly_cli::engine::import::{commit_import, preview_import};
use pendulum_kelly_cli::models::PortfolioState;
use pendulum_kelly_cli::models::Transaction;
use pendulum_kelly_cli::models::import::{
    ImportSummary, ImportedTransactionCandidate, TransactionImportPreview,
};

#[test]
fn test_import_preview_logic() {
    let candidates = vec![ImportedTransactionCandidate {
        date: "2026-06-01".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("a1".to_string()),
        asset_name: None,
        amount: 1000.0,
        units: Some(10.0),
        price: Some(100.0),
        fee: 1.0,
        currency: "CNY".to_string(),
        source: "test".to_string(),
        note: "".to_string(),
        external_id: None,
        raw_description: "".to_string(),
    }];

    let existing = vec![];
    let preview = preview_import(candidates, &existing);

    assert_eq!(preview.summary.total_rows, 1);
    assert_eq!(preview.summary.valid_rows, 1);
    assert_eq!(preview.summary.duplicate_rows, 0);
    assert!(!preview.duplicates[0]);
}

#[test]
fn test_import_preview_duplicate_detection() {
    let candidates = vec![ImportedTransactionCandidate {
        date: "2026-06-01".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("a1".to_string()),
        asset_name: None,
        amount: 1000.0,
        units: Some(10.0),
        price: Some(100.0),
        fee: 1.0,
        currency: "CNY".to_string(),
        source: "test".to_string(),
        note: "".to_string(),
        external_id: Some("tx1".to_string()),
        raw_description: "".to_string(),
    }];

    let existing = vec![Transaction {
        id: "tx1".to_string(),
        date: "2026-06-01".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("a1".to_string()),
        amount: 1000.0,
        units: Some(10.0),
        price: Some(100.0),
        fee: 1.0,
        currency: "CNY".to_string(),
        note: "".to_string(),
        source: "test".to_string(),
        raw_description: "".to_string(),
    }];

    let preview = preview_import(candidates, &existing);

    assert_eq!(preview.summary.duplicate_rows, 1);
    assert!(preview.duplicates[0]);
}

#[test]
fn test_import_commit_logic() {
    let candidates = vec![ImportedTransactionCandidate {
        date: "2026-06-01".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("a1".to_string()),
        asset_name: None,
        amount: 1000.0,
        units: Some(10.0),
        price: Some(100.0),
        fee: 1.0,
        currency: "CNY".to_string(),
        source: "test".to_string(),
        note: "".to_string(),
        external_id: None,
        raw_description: "".to_string(),
    }];

    let preview = TransactionImportPreview {
        candidates: candidates.clone(),
        duplicates: vec![false],
        warnings: vec![vec![]],
        errors: vec![vec![]],
        summary: ImportSummary::default(),
    };

    let mut state = PortfolioState {
        cash: 2000.0,
        asset_holdings: vec![],
    };
    let mut transactions = vec![];

    let result = commit_import(&preview, &mut state, &mut transactions, true);

    assert_eq!(result.inserted, 1);
    assert_eq!(transactions.len(), 1);
    assert_eq!(state.cash, 999.0); // 2000 - 1000 - 1 (fee)
}

#[test]
fn test_reconciliation_logic_simple() {
    use pendulum_kelly_cli::engine::reconcile_portfolio;
    let state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![],
    };
    let transactions = vec![Transaction {
        id: "tx1".to_string(),
        date: "2026-06-01".to_string(),
        transaction_type: "cash_in".to_string(),
        asset_id: None,
        amount: 1000.0,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "".to_string(),
        source: "test".to_string(),
        raw_description: "".to_string(),
    }];
    let report = reconcile_portfolio("default", &state, &transactions);

    assert_eq!(report.summary.total_issues, 0);
}

#[test]
fn test_reconciliation_logic_with_mismatch() {
    use pendulum_kelly_cli::engine::reconcile_portfolio;
    let state = PortfolioState {
        cash: 1000.0, // Stored
        asset_holdings: vec![],
    };
    // Transaction implies cash = 500
    let transactions = vec![Transaction {
        id: "tx1".to_string(),
        date: "2026-06-01".to_string(),
        transaction_type: "cash_in".to_string(),
        asset_id: None,
        amount: 500.0,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "".to_string(),
        source: "test".to_string(),
        raw_description: "".to_string(),
    }];
    let report = reconcile_portfolio("default", &state, &transactions);

    assert!(report.summary.total_issues > 0);
    assert_eq!(report.summary.critical_issues, 1); // Cash mismatch
}

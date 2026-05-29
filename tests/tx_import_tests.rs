use pendulum_kelly_cli::engine::import::{parse_transactions_from_csv, preview_import};
use pendulum_kelly_cli::models::Transaction;

#[test]
fn test_parse_transactions_from_csv() {
    let csv = "date,transaction_type,asset_id,asset_name,amount,units,price,fee,currency,source,note,external_id,raw_description
2026-01-01,buy,AAPL,Apple Inc.,1500.0,10.0,150.0,5.0,USD,manual,Initial Buy,ext1,Manual entry";

    let candidates = parse_transactions_from_csv(csv).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].asset_id, Some("AAPL".to_string()));
    assert_eq!(candidates[0].amount, 1500.0);
    assert_eq!(candidates[0].external_id, Some("ext1".to_string()));
}

#[test]
fn test_preview_import_duplicates() {
    let csv = "date,transaction_type,asset_id,asset_name,amount,units,price,fee,currency,source,note,external_id,raw_description
2026-01-01,buy,AAPL,Apple Inc.,1500.0,10.0,150.0,5.0,USD,manual,Initial Buy,ext1,Manual entry";
    let candidates = parse_transactions_from_csv(csv).unwrap();

    let existing = vec![Transaction {
        id: "ext1".to_string(),
        date: "2026-01-01".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("AAPL".to_string()),
        amount: 1500.0,
        units: Some(10.0),
        price: Some(150.0),
        fee: 5.0,
        currency: "USD".to_string(),
        note: "Initial Buy".to_string(),
        source: "manual".to_string(),
        raw_description: "Manual entry".to_string(),
    }];

    let preview = preview_import(candidates, &existing);
    assert!(preview.duplicates[0]);
    assert_eq!(preview.summary.duplicate_rows, 1);
    assert_eq!(preview.summary.new_rows, 0);
}

#[test]
fn test_preview_import_no_duplicates() {
    let csv = "date,transaction_type,asset_id,asset_name,amount,units,price,fee,currency,source,note,external_id,raw_description
2026-01-02,buy,AAPL,Apple Inc.,1500.0,10.0,150.0,5.0,USD,manual,Initial Buy,ext2,Manual entry";
    let candidates = parse_transactions_from_csv(csv).unwrap();

    let existing = vec![Transaction {
        id: "ext1".to_string(),
        date: "2026-01-01".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("AAPL".to_string()),
        amount: 1500.0,
        units: Some(10.0),
        price: Some(150.0),
        fee: 5.0,
        currency: "USD".to_string(),
        note: "Initial Buy".to_string(),
        source: "manual".to_string(),
        raw_description: "Manual entry".to_string(),
    }];

    let preview = preview_import(candidates, &existing);
    assert!(!preview.duplicates[0]);
    assert_eq!(preview.summary.duplicate_rows, 0);
    assert_eq!(preview.summary.new_rows, 1);
}

#[test]
fn test_preview_import_invalid_type() {
    let csv = "date,transaction_type,asset_id,asset_name,amount,units,price,fee,currency,source,note,external_id,raw_description
2026-01-01,magic,AAPL,Apple Inc.,1500.0,10.0,150.0,5.0,USD,manual,Initial Buy,ext1,Manual entry";
    let candidates = parse_transactions_from_csv(csv).unwrap();

    let preview = preview_import(candidates, &[]);
    assert!(!preview.errors[0].is_empty());
    assert!(preview.errors[0][0].contains("未知交易类型"));
    assert_eq!(preview.summary.error_rows, 1);
}

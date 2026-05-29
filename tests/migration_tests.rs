use pendulum_kelly_cli::models::Transaction;

#[test]
fn test_transaction_fingerprint_consistency() {
    let t1 = Transaction {
        id: "tx1".to_string(),
        date: "2026-05-22".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("asset1".to_string()),
        amount: 1000.0,
        units: Some(10.0),
        price: Some(100.0),
        fee: 5.0,
        currency: "CNY".to_string(),
        note: "Initial buy".to_string(),
        source: "manual".to_string(),
        raw_description: "".to_string(),
    };

    let t2 = t1.clone();

    assert_eq!(t1.fingerprint(), t2.fingerprint());

    let mut t3 = t1.clone();
    t3.id = "different_id".to_string();
    // Fingerprint should be the same even if ID differs
    assert_eq!(t1.fingerprint(), t3.fingerprint());

    let mut t4 = t1.clone();
    t4.amount = 1000.1;
    // Fingerprint should differ if any business field changes
    assert_ne!(t1.fingerprint(), t4.fingerprint());
}

#[test]
fn test_transaction_fingerprint_null_handling() {
    let t1 = Transaction {
        id: "tx1".to_string(),
        date: "2026-05-22".to_string(),
        transaction_type: "cash_in".to_string(),
        asset_id: None,
        amount: 5000.0,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "test".to_string(),
        source: "manual".to_string(),
        raw_description: "".to_string(),
    };

    let t2 = t1.clone();
    assert_eq!(t1.fingerprint(), t2.fingerprint());

    let mut t3 = t1.clone();
    t3.asset_id = Some("now_has_asset".to_string());
    assert_ne!(t1.fingerprint(), t3.fingerprint());
}

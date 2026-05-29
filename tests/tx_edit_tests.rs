use pendulum_kelly_cli::models::Transaction;
use pendulum_kelly_cli::repository::{
    JsonRepository, RepositoryContext, traits::PortfolioRepository,
};

fn create_temp_json_repo(dir: &std::path::Path) -> JsonRepository {
    JsonRepository::new_with_defaults(dir.to_str().unwrap())
}

#[tokio::test]
async fn test_tx_update_delete_json_backend() {
    let dir_path = "data/test_tx_edit";
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

    let tx2 = Transaction {
        id: "tx2".to_string(),
        date: "2026-05-21".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("asset2".to_string()),
        amount: 200.0,
        units: Some(2.0),
        price: Some(100.0),
        fee: 0.0,
        currency: "USD".to_string(),
        note: "test".to_string(),
        source: "manual".to_string(),
        raw_description: "".to_string(),
    };

    repo.save_transactions(&ctx, &[tx1.clone(), tx2.clone()])
        .await
        .unwrap();

    // Test show logic natively
    let loaded = repo.load_transactions(&ctx).await.unwrap();
    assert_eq!(loaded.len(), 2);

    // Test update
    let mut updated_tx = tx1.clone();
    updated_tx.amount = 150.0;
    updated_tx.note = "updated note".to_string();
    repo.update_transaction(&ctx, &updated_tx).await.unwrap();

    let loaded = repo.load_transactions(&ctx).await.unwrap();
    let fetched = loaded.iter().find(|t| t.id == "tx1").unwrap();
    assert_eq!(fetched.amount, 150.0);
    assert_eq!(fetched.note, "updated note");

    // Test delete
    repo.delete_transaction(&ctx, "tx2").await.unwrap();
    let loaded = repo.load_transactions(&ctx).await.unwrap();
    assert_eq!(loaded.len(), 1);
    assert!(!loaded.iter().any(|t| t.id == "tx2"));

    // Test unknown ID
    let err = repo.delete_transaction(&ctx, "unknown_id").await;
    assert!(err.is_err());
    let err = repo.update_transaction(&ctx, &tx2).await;
    assert!(err.is_err());
}

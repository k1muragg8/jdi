use clap::Parser;
use pendulum_kelly_cli::cli::{Cli, Commands, DataCommands};
use pendulum_kelly_cli::models::Transaction;
use pendulum_kelly_cli::repository::json::JsonRepository;
use pendulum_kelly_cli::repository::traits::*;
use pendulum_kelly_cli::repository::{MigrationReport, RepositoryContext, migrate_transactions};
use std::fs;

#[test]
fn test_cli_parsing_migrate_all() {
    let args = vec!["pendulum-kelly-cli", "data", "migrate", "--all"];
    let cli = Cli::parse_from(args);

    if let Commands::Data { command } = cli.command {
        if let DataCommands::Migrate { all, .. } = command {
            assert!(all);
        } else {
            panic!("Expected Migrate command");
        }
    } else {
        panic!("Expected Data command");
    }
}

#[test]
fn test_migration_report_aggregation_logic() {
    let mut reports = Vec::new();

    let mut r1 = MigrationReport::new("Instruments");
    r1.read = 10;
    r1.inserted = 10;
    reports.push(r1);

    let mut r2 = MigrationReport::new("Transactions");
    r2.read = 50;
    r2.inserted = 40;
    r2.skipped = 10;
    reports.push(r2);

    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].domain, "Instruments");
    assert_eq!(reports[1].domain, "Transactions");
    assert_eq!(reports[1].read, 50);
    assert_eq!(reports[1].skipped, 10);
}

#[tokio::test]
async fn test_migrate_transactions_idempotency_json() {
    let dir_source = "data/test_migrate_source";
    let dir_target = "data/test_migrate_target";
    let _ = fs::create_dir_all(dir_source);
    let _ = fs::create_dir_all(dir_target);

    let source_tx_path = format!("{}/transactions.json", dir_source);
    let target_tx_path = format!("{}/transactions.json", dir_target);
    let _ = fs::remove_file(&source_tx_path);
    let _ = fs::remove_file(&target_tx_path);

    let tx = Transaction {
        id: "migrate_test_1".to_string(),
        date: "2026-05-29".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("test_asset".to_string()),
        amount: 100.0,
        units: Some(1.0),
        price: Some(100.0),
        fee: 0.1,
        currency: "CNY".to_string(),
        note: "test".to_string(),
        source: "manual".to_string(),
        raw_description: "".to_string(),
    };

    let source_repo = JsonRepository::new(
        "".to_string(),
        "".to_string(),
        source_tx_path.clone(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
    );

    let target_repo = JsonRepository::new(
        "".to_string(),
        "".to_string(),
        target_tx_path.clone(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
    );

    let ctx = RepositoryContext::default();

    // 1. Initial save to source
    source_repo
        .save_transactions(&ctx, std::slice::from_ref(&tx))
        .await
        .unwrap();

    // Ensure target file exists but is empty
    target_repo.save_transactions(&ctx, &[]).await.unwrap();

    // 2. First migration
    let report1 = migrate_transactions(&source_repo, &target_repo, &ctx)
        .await
        .unwrap();
    assert_eq!(report1.read, 1);
    assert_eq!(report1.inserted, 1);
    assert_eq!(report1.skipped, 0);

    // 3. Second migration (idempotency check)
    let report2 = migrate_transactions(&source_repo, &target_repo, &ctx)
        .await
        .unwrap();
    assert_eq!(report2.read, 1);
    assert_eq!(report2.inserted, 0);
    assert_eq!(report2.skipped, 1);

    // Cleanup
    let _ = fs::remove_file(&source_tx_path);
    let _ = fs::remove_file(&target_tx_path);
}

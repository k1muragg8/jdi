use pendulum_kelly_cli::models::config::{ConfigRoot, StorageBackend};
use std::env;

#[test]
fn test_default_storage_config_is_json() {
    let config = ConfigRoot::default();
    assert_eq!(config.storage.backend, StorageBackend::Json);
    assert_eq!(config.storage.postgres.database_url_env, "DATABASE_URL");
    assert!(config.validate().is_ok());
}

#[test]
fn test_explicit_json_config() {
    let toml = r#"
    [portfolio]
    name = "Test"
    base_currency = "CNY"
    target_equity_value = 0.0
    reserve_cash = 0.0
    upcoming_expense = 0.0
    max_daily_buy_total = 0.0
    
    [storage]
    backend = "json"
    "#;

    let config: ConfigRoot = toml::from_str(toml).unwrap();
    assert_eq!(config.storage.backend, StorageBackend::Json);
    assert!(config.validate().is_ok());
}

#[test]
fn test_postgres_config_missing_env_var() {
    // Unset the env var just in case it exists in the test environment
    unsafe { env::remove_var("MISSING_DB_URL_VAR") };

    let toml = r#"
    [portfolio]
    name = "Test"
    base_currency = "CNY"
    target_equity_value = 0.0
    reserve_cash = 0.0
    upcoming_expense = 0.0
    max_daily_buy_total = 0.0

    [storage]
    backend = "postgres"

    [storage.postgres]
    database_url_env = "MISSING_DB_URL_VAR"
    "#;

    let config: ConfigRoot = toml::from_str(toml).unwrap();
    assert_eq!(config.storage.backend, StorageBackend::Postgres);
    assert_eq!(
        config.storage.postgres.database_url_env,
        "MISSING_DB_URL_VAR"
    );

    let validation_result = config.validate();
    assert!(validation_result.is_err());
    assert!(
        validation_result
            .unwrap_err()
            .to_string()
            .contains("MISSING_DB_URL_VAR")
    );
}

#[test]
fn test_postgres_config_with_env_var() {
    unsafe { env::set_var("PRESENT_DB_URL_VAR", "postgres://user:pass@localhost/db") };

    let toml = r#"
    [portfolio]
    name = "Test"
    base_currency = "CNY"
    target_equity_value = 0.0
    reserve_cash = 0.0
    upcoming_expense = 0.0
    max_daily_buy_total = 0.0

    [storage]
    backend = "postgres"

    [storage.postgres]
    database_url_env = "PRESENT_DB_URL_VAR"
    "#;

    let config: ConfigRoot = toml::from_str(toml).unwrap();
    assert_eq!(config.storage.backend, StorageBackend::Postgres);
    assert!(config.validate().is_ok());
}

#[test]
fn test_invalid_storage_backend() {
    let toml = r#"
    [portfolio]
    name = "Test"
    base_currency = "CNY"
    target_equity_value = 0.0
    reserve_cash = 0.0
    upcoming_expense = 0.0
    max_daily_buy_total = 0.0

    [storage]
    backend = "mysql"
    "#;

    let result: Result<ConfigRoot, _> = toml::from_str(toml);
    assert!(result.is_err());
}

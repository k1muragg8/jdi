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
    let err = validation_result.unwrap_err().to_string();
    assert!(err.contains("MISSING_DB_URL_VAR"));
    assert!(err.contains("Refusing to fallback to JSON"));
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

#[test]
fn test_resolve_data_dir_never_resolves_under_target_debug() {
    let d = pendulum_kelly_cli::resolve_data_dir();
    let s = d.to_string_lossy().to_lowercase();
    assert!(
        !s.contains("target/debug") && !s.contains("target\\debug"),
        "data_dir must NEVER resolve under target/debug (would cause 100GB bloat with runtime json/caches): got {}",
        d.display()
    );
    // When run from project via cargo, should end with /data or contain project data
    assert!(
        s.ends_with("/data") || s.contains("/data") || s.contains("\\data"),
        "expected project /data : {}",
        d.display()
    );
}

#[test]
fn test_jdi_data_dir_env_overrides() {
    unsafe { std::env::set_var("JDI_DATA_DIR", "/tmp/test_jdi_data_override") };
    let d = pendulum_kelly_cli::resolve_data_dir();
    assert_eq!(d, std::path::PathBuf::from("/tmp/test_jdi_data_override"));
    unsafe { std::env::remove_var("JDI_DATA_DIR") };
}

#[test]
fn test_postgres_backend_strict_no_silent_fallback_in_error() {
    // Even if env present, but we test the error path string contains refuse
    unsafe { std::env::remove_var("NONEXISTENT_FOR_TEST") };
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
    database_url_env = "NONEXISTENT_FOR_TEST"
    "#;
    let config: ConfigRoot = toml::from_str(toml).unwrap();
    let res = config.validate();
    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(err.contains("Refusing to fallback to JSON"));
}

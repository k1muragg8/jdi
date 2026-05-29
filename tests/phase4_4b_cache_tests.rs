use pendulum_kelly_cli::models::InstrumentQuoteCache;
use std::fs;

#[test]
fn test_cache_status_missing_does_not_panic() {
    let path = "data/test_missing_status.json";
    if std::path::Path::new(path).exists() {
        fs::remove_file(path).unwrap();
    }

    let registry =
        pendulum_kelly_cli::storage::cache_status_store::load_cache_status(path).unwrap();
    assert!(registry.statuses.is_empty());
}

#[test]
fn test_instrument_cache_io() {
    let path = "data/test_instrument_cache.json";
    let cache = InstrumentQuoteCache::default();
    pendulum_kelly_cli::storage::instrument_cache_store::save_instrument_cache(path, &cache)
        .unwrap();

    let loaded =
        pendulum_kelly_cli::storage::instrument_cache_store::load_instrument_cache(path).unwrap();
    assert!(loaded.entries.is_empty());

    fs::remove_file(path).unwrap();
}

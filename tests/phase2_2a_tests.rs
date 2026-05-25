use chrono::Local;
use pendulum_kelly_cli::api::{MarketDataProvider, MockMarketProvider, create_market_provider};
use pendulum_kelly_cli::models::{AssetConfig, MarketCache, MarketCacheEntry, MarketConfig};

#[test]
fn test_mock_market_provider_latest() {
    let provider = MockMarketProvider::new();
    let price = provider.fetch_latest_price("QQQ").unwrap();
    assert_eq!(price.symbol, "QQQ");
    assert_eq!(price.price, 450.50);
}

#[test]
fn test_mock_market_provider_history() {
    let provider = MockMarketProvider::new();
    let candles = provider.fetch_daily_candles("QQQ", 5).unwrap();
    assert_eq!(candles.len(), 5);
}

#[test]
fn test_market_provider_selection() {
    let mut config = MarketConfig::default();

    config.default_market_provider = "mock".to_string();
    let p1 = create_market_provider(&config);
    assert!(p1.fetch_latest_price("QQQ").is_ok());

    config.default_market_provider = "yahoo".to_string();
    let _p2 = create_market_provider(&config);
    // Don't assert on side effect that depends on network
}

#[test]
fn test_market_cache_logic() {
    let mut cache = MarketCache::default();
    let entry = MarketCacheEntry {
        symbol: "QQQ".to_string(),
        price: 440.0,
        date: "2024-05-22".to_string(),
        currency: "USD".to_string(),
        source: "yahoo".to_string(),
        fetched_at: Local::now().to_rfc3339(),
    };
    cache.entries.push(entry);

    let symbol = "QQQ";
    let found = cache.entries.iter().find(|e| e.symbol == symbol);
    assert!(found.is_some());
    assert_eq!(found.unwrap().price, 440.0);
}

#[test]
fn test_asset_reference_index_serialization() {
    let asset = AssetConfig {
        asset_id: "test".to_string(),
        fund_code: "123".to_string(),
        fund_name: "Test".to_string(),
        sector: "S".to_string(),
        currency: "CNY".to_string(),
        valuation_method: "nav".to_string(),
        enabled: true,
        reference_index_name: Some("Nasdaq".to_string()),
        reference_index_symbol: Some("QQQ".to_string()),
        market_data_provider: Some("yahoo".to_string()),
    };

    let toml = toml::to_string(&asset).unwrap();
    assert!(toml.contains("reference_index_name = \"Nasdaq\""));
    assert!(toml.contains("reference_index_symbol = \"QQQ\""));
    assert!(toml.contains("market_data_provider = \"yahoo\""));
}

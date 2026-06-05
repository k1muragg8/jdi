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
    let mut config = MarketConfig {
        default_market_provider: "mock".to_string(),
        ..MarketConfig::default()
    };
    let p1 = create_market_provider(&config, None);
    assert!(p1.fetch_latest_price("QQQ").is_ok());

    config.default_market_provider = "yahoo".to_string();
    let _p2 = create_market_provider(&config, None);
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
        previous_close: None,
        change: None,
        change_percent: None,
        status: Some("ok".to_string()),
        error_message: None,
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
        reference_index_currency: None,
        proxy_fx_pair: None,
        use_fx_adjustment: Some(false),
        reference_instrument_id: None,
        reference_instrument_symbol: None,
    };

    let toml = toml::to_string(&asset).unwrap();
    assert!(toml.contains("reference_index_name = \"Nasdaq\""));
    assert!(toml.contains("reference_index_symbol = \"QQQ\""));
    assert!(toml.contains("market_data_provider = \"yahoo\""));
}

#[test]
fn test_market_quote_cache_as_source_of_truth() {
    use pendulum_kelly_cli::models::{CacheStatusRegistry, MarketCache, MarketCacheEntry};
    let mut mc = MarketCache::default();
    mc.entries.push(MarketCacheEntry {
        symbol: "QQQ".to_string(),
        price: 740.61,
        date: "2026-06-05".to_string(),
        currency: "USD".to_string(),
        source: "yahoo".to_string(),
        fetched_at: "2026-06-05 12:00:00".to_string(),
        previous_close: Some(730.0),
        change: Some(10.61),
        change_percent: Some(1.45),
        status: Some("ok".to_string()),
        error_message: None,
    });
    mc.entries.push(MarketCacheEntry {
        symbol: "SPY".to_string(),
        price: 757.09,
        date: "2026-06-05".to_string(),
        currency: "USD".to_string(),
        source: "yahoo".to_string(),
        fetched_at: "2026-06-05 12:01:00".to_string(),
        previous_close: None,
        change: None,
        change_percent: None,
        status: Some("ok".to_string()),
        error_message: None,
    });

    // Simulate what /market now does for cards
    let depth = mc.entries.len();
    let last = mc
        .entries
        .iter()
        .map(|e| e.fetched_at.as_str())
        .max()
        .unwrap_or("从未刷新")
        .to_string();

    assert_eq!(depth, 2);
    assert!(last.contains("2026-06-05"));

    // cache status should be updatable from it
    let cs = CacheStatusRegistry {
        market_cache_size: mc.entries.len(),
        last_market_update: mc.entries.iter().map(|e| &e.fetched_at).max().cloned(),
        ..Default::default()
    };
    assert_eq!(cs.market_cache_size, 2);
    assert!(cs.last_market_update.is_some());

    // table would see 2 quotes too
    let map: std::collections::HashMap<_, _> =
        mc.entries.iter().map(|e| (e.symbol.clone(), e)).collect();
    assert!(map.contains_key("QQQ"));
    assert!(map.get("SPY").unwrap().price > 0.0);
}

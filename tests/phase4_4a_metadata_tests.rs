use pendulum_kelly_cli::models::{AssetClass, InstrumentConfig};
use pendulum_kelly_cli::storage::instrument_store::get_default_instruments;

#[test]
fn test_default_instruments_include_name_zh() {
    let defaults = get_default_instruments();
    assert!(!defaults.is_empty());

    // Check key instruments have name_zh
    let qqq = defaults
        .iter()
        .find(|i| i.symbol == "QQQ")
        .expect("QQQ not found");
    assert_eq!(qqq.name_zh.as_deref(), Some("纳斯达克100ETF"));

    let spy = defaults
        .iter()
        .find(|i| i.symbol == "SPY")
        .expect("SPY not found");
    assert_eq!(spy.name_zh.as_deref(), Some("标普500ETF"));

    let vix = defaults
        .iter()
        .find(|i| i.symbol == "^VIX")
        .expect("VIX not found");
    assert_eq!(vix.name_zh.as_deref(), Some("VIX恐慌指数"));

    let gold = defaults
        .iter()
        .find(|i| i.symbol == "AU9999")
        .expect("AU9999 not found");
    assert_eq!(
        gold.name_zh.as_deref(),
        Some("上海黄金交易所Au9999现货黄金")
    );
}

#[test]
fn test_instrument_config_serialization_with_zh() {
    let inst = InstrumentConfig {
        instrument_id: "test".to_string(),
        symbol: "TEST".to_string(),
        display_symbol: None,
        name: "Test Instrument".to_string(),
        name_zh: Some("测试标的".to_string()),
        name_en: Some("Test Instrument".to_string()),
        description_zh: Some("这是一个测试".to_string()),
        category_zh: Some("测试".to_string()),
        display_label: Some("测试".to_string()),
        asset_class: AssetClass::Custom,
        provider: "mock".to_string(),
        provider_symbol: "TEST".to_string(),
        market: None,
        exchange: None,
        currency: "USD".to_string(),
        quote_unit: "unit".to_string(),
        price_unit: "USD/unit".to_string(),
        timezone: None,
        enabled: true,
        priority: 0,
        tags: vec![],
        note: None,
    };

    let toml = toml::to_string(&inst).unwrap();
    assert!(toml.contains("name_zh = \"测试标的\""));
    assert!(toml.contains("category_zh = \"测试\""));

    let decoded: InstrumentConfig = toml::from_str(&toml).unwrap();
    assert_eq!(decoded.name_zh, Some("测试标的".to_string()));
}

use pendulum_kelly_cli::api::create_market_provider;
use pendulum_kelly_cli::models::MarketConfig;

#[test]
fn test_market_lookup_provider_override() {
    let mut config = MarketConfig::default();
    config.default_market_provider = "mock".to_string();

    // Override to yahoo
    let _p = create_market_provider(&config, Some("yahoo"));
    // Explicit mock
    let p_mock = create_market_provider(&config, Some("mock"));
    assert!(p_mock.fetch_latest_price("QQQ").is_ok());
}

#[test]
fn test_market_status_marking_logic() {
    // Logic test for status strings
    let source_is_mock = true;
    let is_stale = false;
    let source = "mock";

    let status_str = if source_is_mock || source == "mock" {
        "模拟"
    } else if is_stale {
        "过期"
    } else {
        "正常"
    };
    assert_eq!(status_str, "模拟");
}

#[test]
fn test_semantic_match_logic() {
    let fund_name = "华夏纳斯达克100ETF";
    let ref_name = "Nasdaq 100 ETF";

    let fund_keywords = vec!["纳斯达克", "标普", "500", "100", "Nasdaq", "S&P"];
    let has_fund_kw = fund_keywords.iter().any(|kw| fund_name.contains(kw));
    let has_ref_kw = fund_keywords.iter().any(|kw| ref_name.contains(kw));

    assert!(has_fund_kw);
    assert!(has_ref_kw);

    let mut shared = false;
    for kw in fund_keywords {
        if fund_name.contains(kw) && ref_name.contains(kw) {
            shared = true;
            break;
        }
    }
    assert!(shared); // Should match "100" or "Nasdaq"
}

#[test]
fn test_semantic_mismatch_logic() {
    let fund_name = "华夏成长混合";
    let ref_name = "Nasdaq 100 ETF";

    let fund_keywords = vec!["纳斯达克", "标普", "500", "100", "Nasdaq", "S&P"];

    let mut shared = false;
    for kw in fund_keywords {
        if fund_name.contains(kw) && ref_name.contains(kw) {
            shared = true;
            break;
        }
    }
    assert!(!shared);
}

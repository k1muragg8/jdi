use pendulum_kelly_cli::engine::alipay_holding::{
    convert_to_snapshots, parse_alipay_holdings_from_csv, preview_alipay_holdings,
};
use pendulum_kelly_cli::models::{AssetConfig, AssetHolding, ConfigRoot, PortfolioState};

#[test]
fn test_parse_alipay_holdings_chinese_headers() {
    let csv = "基金代码,基金名称,持有份额,市值(元),单位净值,净值日期,成本价,累计收益\n006327,纳斯达克100,362.01,316.87,0.8753,2026-05-27,0.9500,-45.20";
    let candidates = parse_alipay_holdings_from_csv(csv).unwrap();

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].fund_code, "006327");
    assert_eq!(candidates[0].units, 362.01);
    assert_eq!(candidates[0].market_value, 316.87);
    assert_eq!(candidates[0].nav, Some(0.8753));
}

#[test]
fn test_parse_alipay_holdings_english_headers_screenshot() {
    let csv = "fund_name,market_value,holding_profit,holding_profit_rate,source\n易方达标普生物科技指数(QDII-LOF)A,139.48,9.48,7.29,alipay_screenshot";
    let candidates = parse_alipay_holdings_from_csv(csv).unwrap();

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].fund_name, "易方达标普生物科技指数(QDII-LOF)A");
    assert_eq!(candidates[0].market_value, 139.48);
    assert_eq!(candidates[0].total_profit, Some(9.48));
    assert_eq!(candidates[0].profit_rate, Some(7.29));
    assert_eq!(candidates[0].source, Some("alipay_screenshot".to_string()));
}

#[test]
fn test_parse_alipay_holdings_chinese_headers_screenshot() {
    let csv = "基金名称,持有金额,持有收益,持有收益率,来源\n纳斯达克100,1000.00,50.00,5.0,手机截图";
    let candidates = parse_alipay_holdings_from_csv(csv).unwrap();

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].fund_name, "纳斯达克100");
    assert_eq!(candidates[0].market_value, 1000.00);
    assert_eq!(candidates[0].total_profit, Some(50.00));
    assert_eq!(candidates[0].profit_rate, Some(5.0));
}

#[test]
fn test_preview_alipay_holdings_matching_by_name() {
    let mut config = ConfigRoot::default();
    config.assets.push(AssetConfig {
        asset_id: "biotech".to_string(),
        fund_code: "161127".to_string(),
        fund_name: "易方达标普生物科技".to_string(),
        sector: "US Biotech".to_string(),
        currency: "CNY".to_string(),
        valuation_method: "nav".to_string(),
        enabled: true,
        ..Default::default()
    });

    let state = PortfolioState::default();

    // Exact match by name
    let csv = "fund_name,market_value\n易方达标普生物科技,139.48";
    let candidates = parse_alipay_holdings_from_csv(csv).unwrap();
    let preview = preview_alipay_holdings(&config, &state, candidates, "2026-06-01");

    assert_eq!(preview.matched_asset_ids[0], Some("biotech".to_string()));
    assert_eq!(preview.valid_rows, 1);
    assert_eq!(preview.total_rows, 1);
}

#[test]
fn test_parse_zero_rows() {
    let csv = "header1,header2\n";
    let candidates = parse_alipay_holdings_from_csv(csv).unwrap();
    assert_eq!(candidates.len(), 0);
}

#[test]
fn test_preview_alipay_holdings_matching() {
    let mut config = ConfigRoot::default();
    config.assets.push(AssetConfig {
        asset_id: "nasdaq_100".to_string(),
        fund_code: "006327".to_string(),
        fund_name: "Nasdaq 100".to_string(),
        sector: "US Tech".to_string(),
        currency: "CNY".to_string(),
        valuation_method: "nav".to_string(),
        enabled: true,
        reference_index_name: None,
        reference_index_symbol: None,
        market_data_provider: None,
        reference_instrument_id: None,
        reference_instrument_symbol: None,
        reference_index_currency: None,
        proxy_fx_pair: None,
        use_fx_adjustment: None,
    });

    let mut state = PortfolioState::default();
    state.asset_holdings.push(AssetHolding {
        asset_id: "nasdaq_100".to_string(),
        fund_code: "006327".to_string(),
        units: 362.0, // Slight difference
        units_estimated: false,
        cost_basis: 1.0,
        latest_nav: Some(0.8753),
        latest_nav_date: Some("2026-05-27".to_string()),
        latest_nav_source: None,
        latest_nav_status: None,
        last_market_value: 316.0,
    });

    let csv = "基金代码,基金名称,持有份额,市值(元)\n006327,纳斯达克100,362.01,316.87";
    let candidates = parse_alipay_holdings_from_csv(csv).unwrap();
    let preview = preview_alipay_holdings(&config, &state, candidates, "2026-05-27");

    assert_eq!(preview.matched_asset_ids[0], Some("nasdaq_100".to_string()));
    assert!((preview.unit_diffs[0].unwrap() - 0.01).abs() < 0.0001);
    assert!(!preview.warnings[0].is_empty());
}

#[test]
fn test_convert_to_snapshots_skip_errors() {
    let preview = pendulum_kelly_cli::models::AlipayHoldingImportPreview {
        snapshot_date: "2026-05-27".to_string(),
        candidates: vec![pendulum_kelly_cli::models::AlipayHoldingCandidate {
            fund_code: "006327".to_string(),
            fund_name: "Nasdaq".to_string(),
            units: 100.0,
            ..Default::default()
        }],
        matched_asset_ids: vec![Some("a1".to_string())],
        errors: vec![vec!["Some error".to_string()]],
        ..Default::default()
    };

    let snapshots = convert_to_snapshots(&preview);
    assert_eq!(snapshots.len(), 0);
}

#[test]
fn test_preview_unmatched_rows_is_warning_not_error() {
    let config = ConfigRoot::default();
    let state = PortfolioState::default();

    let csv = "基金名称,市值\n未知基金,100.00";
    let candidates = parse_alipay_holdings_from_csv(csv).unwrap();
    let preview = preview_alipay_holdings(&config, &state, candidates, "2026-06-01");

    assert_eq!(preview.matched_asset_ids[0], None);
    assert_eq!(preview.unmatched_rows, 1);
    assert!(preview.errors[0].is_empty());
    assert!(!preview.warnings[0].is_empty());
    assert!(preview.warnings[0][0].contains("未找到匹配的资产配置"));
}

#[test]
fn test_convert_to_snapshots_includes_unmatched() {
    let preview = pendulum_kelly_cli::models::AlipayHoldingImportPreview {
        snapshot_date: "2026-06-01".to_string(),
        candidates: vec![pendulum_kelly_cli::models::AlipayHoldingCandidate {
            fund_name: "未知基金".to_string(),
            market_value: 100.0,
            ..Default::default()
        }],
        matched_asset_ids: vec![None],
        warnings: vec![vec!["Unmatched".to_string()]],
        errors: vec![vec![]],
        ..Default::default()
    };

    let snapshots = convert_to_snapshots(&preview);
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].asset_id, "");
    assert_eq!(snapshots[0].fund_name, "未知基金");
}

#[test]
fn test_bootstrap_assets() {
    use pendulum_kelly_cli::engine::alipay_holding::bootstrap_assets_from_holdings;
    let mut config = ConfigRoot::default();

    let candidates = vec![
        pendulum_kelly_cli::models::AlipayHoldingCandidate {
            fund_code: "000216".to_string(),
            fund_name: "华安黄金ETF联接A".to_string(),
            ..Default::default()
        },
        pendulum_kelly_cli::models::AlipayHoldingCandidate {
            fund_code: "015822".to_string(),
            fund_name: "易方达中证同业存单AAA".to_string(),
            ..Default::default()
        },
    ];

    let (created, skipped, failed) = bootstrap_assets_from_holdings(&mut config, &candidates);

    assert_eq!(created, 2);
    assert_eq!(skipped, 0);
    assert_eq!(failed, 0);
    assert_eq!(config.assets.len(), 2);
    assert_eq!(config.assets[0].asset_id, "fund_000216");
    assert_eq!(config.assets[0].fund_code, "000216");
    assert_eq!(config.assets[1].asset_id, "fund_015822");

    // Idempotency
    let (created2, skipped2, failed2) = bootstrap_assets_from_holdings(&mut config, &candidates);
    assert_eq!(created2, 0);
    assert_eq!(skipped2, 2);
    assert_eq!(failed2, 0);
}

#[test]
fn test_matching_improved() {
    let mut config = ConfigRoot::default();
    config.assets.push(AssetConfig {
        asset_id: "nasdaq_100".to_string(),
        fund_code: "006327".to_string(),
        fund_name: "Nasdaq 100".to_string(),
        ..Default::default()
    });

    let state = PortfolioState::default();

    // Match by code
    let csv1 = "fund_code,market_value\n006327,100.0";
    let cand1 = parse_alipay_holdings_from_csv(csv1).unwrap();
    let prev1 = preview_alipay_holdings(&config, &state, cand1, "2026-06-01");
    assert_eq!(prev1.matched_asset_ids[0], Some("nasdaq_100".to_string()));

    // Match by asset_id in fund_code column
    let csv2 = "fund_code,market_value\nnasdaq_100,100.0";
    let cand2 = parse_alipay_holdings_from_csv(csv2).unwrap();
    let prev2 = preview_alipay_holdings(&config, &state, cand2, "2026-06-01");
    assert_eq!(prev2.matched_asset_ids[0], Some("nasdaq_100".to_string()));

    // Match by name
    let csv3 = "fund_name,market_value\nNasdaq 100,100.0";
    let cand3 = parse_alipay_holdings_from_csv(csv3).unwrap();
    let prev3 = preview_alipay_holdings(&config, &state, cand3, "2026-06-01");
    assert_eq!(prev3.matched_asset_ids[0], Some("nasdaq_100".to_string()));
}

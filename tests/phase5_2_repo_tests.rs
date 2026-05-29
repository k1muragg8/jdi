use pendulum_kelly_cli::models::config::*;
use pendulum_kelly_cli::models::*;
use pendulum_kelly_cli::repository::RepositoryContext;
use pendulum_kelly_cli::repository::json::JsonRepository;
use pendulum_kelly_cli::repository::traits::*;
use std::fs;

#[tokio::test]
async fn test_json_repository_audit() {
    let dir_path = "data/test_repo_audit";
    let _ = fs::create_dir_all(dir_path);
    let audit_path = format!("{}/web_audit.json", dir_path);
    let _ = fs::remove_file(&audit_path);

    // Create a JsonRepository shell (most paths can be dummy for this test)
    let repo = JsonRepository::new(
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
        audit_path.clone(),
        "".to_string(),
        "".to_string(),
    );

    let ctx = RepositoryContext::default();

    let record = WebAdminAudit {
        audit_id: "repo_test_1".to_string(),
        timestamp: "2026-05-28 12:00:00".to_string(),
        actor: "repo_test".to_string(),
        actor_user_id: Some("user1".to_string()),
        target_user_id: Some("user1".to_string()),
        portfolio_id: Some("p1".to_string()),
        role: Some("owner".to_string()),
        action: "test_repo".to_string(),
        target_file: "test.json".to_string(),
        target_id: Some("t1".to_string()),
        old_value_summary: "old".to_string(),
        new_value_summary: "new".to_string(),
        status: "success".to_string(),
        note: None,
    };

    repo.append_web_admin_audit(&ctx, record.clone())
        .await
        .unwrap();

    let log = repo.load_web_admin_audit(&ctx).await.unwrap();
    assert_eq!(log.records.len(), 1);
    assert_eq!(log.records[0].audit_id, "repo_test_1");

    // Cleanup
    let _ = fs::remove_file(&audit_path);
}

#[tokio::test]
async fn test_json_repository_dca() {
    let dir_path = "data/test_repo_dca";
    let _ = fs::create_dir_all(dir_path);
    let plans_path = format!("{}/dca_plans.json", dir_path);
    let settlements_path = format!("{}/dca_settlements.json", dir_path);
    let settlement_audit_path = format!("{}/dca_settlement_audit.json", dir_path);
    let _ = fs::remove_file(&plans_path);
    let _ = fs::remove_file(&settlements_path);
    let _ = fs::remove_file(&settlement_audit_path);

    let repo = JsonRepository::new(
        "".to_string(),
        "".to_string(),
        "".to_string(),
        plans_path.clone(),
        settlements_path.clone(),
        settlement_audit_path.clone(),
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

    // 1. Test Plans
    let plan = DcaPlan {
        plan_id: "test_plan".to_string(),
        asset_id: "asset1".to_string(),
        fund_code: "123".to_string(),
        fund_name: "test fund".to_string(),
        amount: 100.0,
        currency: "CNY".to_string(),
        frequency: DcaFrequency::Daily,
        weekday: None,
        month_day: None,
        start_date: "2026-05-28".to_string(),
        end_date: None,
        enabled: true,
        priority: 0,
        note: None,
    };
    repo.save_plans(&ctx, &[plan.clone()]).await.unwrap();
    let loaded_plans = repo.load_plans(&ctx).await.unwrap();
    assert_eq!(loaded_plans.len(), 1);
    assert_eq!(loaded_plans[0].plan_id, "test_plan");

    // 2. Test Settlements
    let settlement = DcaSettlement {
        settlement_id: "test_settle".to_string(),
        plan_id: Some("test_plan".to_string()),
        asset_id: "asset1".to_string(),
        fund_code: "123".to_string(),
        fund_name: "test fund".to_string(),
        scheduled_date: None,
        deduction_date: "2026-05-28".to_string(),
        confirmation_date: "2026-05-29".to_string(),
        amount: 100.0,
        confirmed_nav: 1.0,
        confirmed_units: 100.0,
        fee: Some(0.0),
        currency: "CNY".to_string(),
        source: "test".to_string(),
        status: DcaSettlementStatus::Confirmed,
        applied: false,
        note: None,
        created_at: "now".to_string(),
    };
    repo.save_settlements(&ctx, &[settlement.clone()])
        .await
        .unwrap();
    let loaded_settlements = repo.load_settlements(&ctx).await.unwrap();
    assert_eq!(loaded_settlements.len(), 1);
    assert_eq!(loaded_settlements[0].settlement_id, "test_settle");

    // 3. Test Settlement Audits
    let audit = DcaSettlementAudit {
        audit_id: "test_audit".to_string(),
        timestamp: "now".to_string(),
        settlement_id: "test_settle".to_string(),
        asset_id: "asset1".to_string(),
        old_units: 0.0,
        new_units: 100.0,
        old_cost_basis: 0.0,
        new_cost_basis: 1.0,
        transaction_id: None,
        note: None,
    };
    repo.save_settlement_audits(&ctx, &[audit.clone()])
        .await
        .unwrap();
    let loaded_audits = repo.load_settlement_audits(&ctx).await.unwrap();
    assert_eq!(loaded_audits.len(), 1);
    assert_eq!(loaded_audits[0].audit_id, "test_audit");

    // Cleanup
    let _ = fs::remove_file(&plans_path);
    let _ = fs::remove_file(&settlements_path);
    let _ = fs::remove_file(&settlement_audit_path);
}

#[tokio::test]
async fn test_json_repository_reconcile() {
    let dir_path = "data/test_repo_reconcile";
    let _ = fs::create_dir_all(dir_path);
    let snapshots_path = format!("{}/alipay_snapshots.json", dir_path);
    let audit_path = format!("{}/reconciliation_audit.json", dir_path);
    let _ = fs::remove_file(&snapshots_path);
    let _ = fs::remove_file(&audit_path);

    let repo = JsonRepository::new(
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        snapshots_path.clone(),
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
        audit_path.clone(),
        "".to_string(),
    );

    let ctx = RepositoryContext::default();

    // 1. Test Snapshots
    let snapshot = AlipaySnapshot {
        snapshot_id: "test_snap".to_string(),
        asset_id: "asset1".to_string(),
        fund_code: "123".to_string(),
        fund_name: "test fund".to_string(),
        snapshot_date: "2026-05-28".to_string(),
        market_value: 1000.0,
        units: Some(100.0),
        cost_basis: Some(900.0),
        nav: Some(1.0),
        nav_date: Some("2026-05-28".to_string()),
        daily_pnl: Some(10.0),
        total_pnl: Some(100.0),
        source: "alipay".to_string(),
        created_at: "now".to_string(),
        note: None,
    };
    repo.save_alipay_snapshots(&ctx, &[snapshot.clone()])
        .await
        .unwrap();
    let loaded_snaps = repo.load_alipay_snapshots(&ctx).await.unwrap();
    assert_eq!(loaded_snaps.len(), 1);
    assert_eq!(loaded_snaps[0].snapshot_id, "test_snap");

    // 2. Test Audits
    let audit = ReconciliationAudit {
        audit_id: "test_audit".to_string(),
        timestamp: "now".to_string(),
        snapshot_id: "test_snap".to_string(),
        asset_id: "asset1".to_string(),
        old_units: 0.0,
        new_units: 100.0,
        old_cost_basis: 0.0,
        new_cost_basis: 900.0,
        old_market_value: 0.0,
        new_market_value: 1000.0,
        reason: "initial".to_string(),
        note: None,
    };
    repo.save_reconciliation_audits(&ctx, &[audit.clone()])
        .await
        .unwrap();
    let loaded_audits = repo.load_reconciliation_audits(&ctx).await.unwrap();
    assert_eq!(loaded_audits.len(), 1);
    assert_eq!(loaded_audits[0].audit_id, "test_audit");

    // Cleanup
    let _ = fs::remove_file(&snapshots_path);
    let _ = fs::remove_file(&audit_path);
}

#[tokio::test]
async fn test_json_repository_instrument() {
    let dir_path = "data/test_repo_instrument";
    let _ = fs::create_dir_all(dir_path);
    let instruments_path = format!("{}/instruments.toml", dir_path);
    let cache_path = format!("{}/instrument_cache.json", dir_path);
    let _ = fs::remove_file(&instruments_path);
    let _ = fs::remove_file(&cache_path);

    let repo = JsonRepository::new(
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        instruments_path.clone(),
        "".to_string(),
        cache_path.clone(),
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

    // 1. Test Instruments
    let inst = InstrumentConfig {
        instrument_id: "test_inst".to_string(),
        symbol: "TEST".to_string(),
        display_symbol: Some("TEST".to_string()),
        name: "Test Instrument".to_string(),
        name_zh: Some("测试标的".to_string()),
        name_en: None,
        description_zh: None,
        category_zh: Some("指数".to_string()),
        display_label: Some("TEST".to_string()),
        asset_class: AssetClass::Index,
        provider: "mock".to_string(),
        provider_symbol: "TEST".to_string(),
        market: Some("US".to_string()),
        exchange: None,
        currency: "USD".to_string(),
        quote_unit: "1".to_string(),
        price_unit: "1".to_string(),
        timezone: None,
        enabled: true,
        priority: 0,
        tags: vec![],
        note: None,
    };
    repo.save_instruments(&ctx, &[inst.clone()]).await.unwrap();
    let loaded_insts = repo.load_instruments(&ctx).await.unwrap();
    assert_eq!(loaded_insts.len(), 1);
    assert_eq!(loaded_insts[0].instrument_id, "test_inst");
    assert_eq!(loaded_insts[0].name_zh, Some("测试标的".to_string()));

    // 2. Test Cache
    let cache = InstrumentQuoteCache {
        entries: vec![InstrumentQuoteCacheEntry {
            instrument_id: "test_inst".to_string(),
            symbol: "TEST".to_string(),
            name_zh: Some("测试标的".to_string()),
            price: 100.0,
            date: "2026-05-28".to_string(),
            currency: "USD".to_string(),
            quote_unit: "1".to_string(),
            provider: "mock".to_string(),
            source: "mock".to_string(),
            status: "正常".to_string(),
            fetched_at: "now".to_string(),
            warning: None,
        }],
        fetched_at: "now".to_string(),
    };
    repo.save_instrument_cache(&ctx, &cache).await.unwrap();
    let loaded_cache = repo.load_instrument_cache(&ctx).await.unwrap();
    assert_eq!(loaded_cache.entries.len(), 1);
    assert_eq!(loaded_cache.entries[0].instrument_id, "test_inst");

    // Cleanup
    let _ = fs::remove_file(&instruments_path);
    let _ = fs::remove_file(&cache_path);
}

#[tokio::test]
async fn test_json_repository_config() {
    let dir_path = "data/test_repo_config";
    let _ = fs::create_dir_all(dir_path);
    let config_path = format!("{}/config.toml", dir_path);
    let _ = fs::remove_file(&config_path);

    let repo = JsonRepository::new(
        config_path.clone(),
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

    // Use a minimal config for testing
    let config = ConfigRoot {
        portfolio: PortfolioConfig {
            name: "Test Portfolio".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 1000.0,
        },
        risk: RiskConfig {
            max_single_sector_daily_buy: 1500.0,
            max_single_asset_daily_buy: 1000.0,
            min_buy_amount: 10.0,
            allow_buy_overweight: false,
            vix_symbol: "^VIX".to_string(),
            us30y_symbol: "^TYX".to_string(),
            crypto_symbols: vec![],
            equity_symbols: vec![],
            lookback_days: 250,
            short_window_days: 20,
            medium_window_days: 60,
            high_vix_threshold: 25.0,
            extreme_vix_threshold: 35.0,
            us30y_fast_rise_bps_60d: 50.0,
            crypto_drawdown_warning: -0.20,
            risk_score_warning_threshold: 60.0,
            risk_score_extreme_threshold: 80.0,
        },
        regime: RegimeConfig {
            default_windows: vec![20, 60, 120, 250],
            default_lookback_days: 250,
            hot_z_threshold: 2.0,
            cold_z_threshold: -2.0,
            high_volatility_threshold: 0.35,
            deep_drawdown_threshold: -0.20,
        },
        api: ApiConfig {
            default_fund_provider: "eastmoney".to_string(),
            fund_provider_timeout_seconds: 10,
            fund_provider_retry_count: 2,
            fund_nav_stale_days: 3,
            allow_mock_fallback: true,
        },
        market: MarketConfig {
            default_market_provider: "mock".to_string(),
            allow_mock_market_fallback: true,
            market_provider_timeout_seconds: 10,
            market_provider_retry_count: 2,
            market_cache_stale_hours: 24,
        },
        fx: FxConfig {
            default_fx_provider: "mock".to_string(),
            usd_cnh_symbol: "USDCNH=X".to_string(),
            fx_cache_stale_hours: 24,
            allow_mock_fx_fallback: true,
        },
        reconciliation: ReconciliationConfig {
            market_value_tolerance_abs: 1.0,
            market_value_tolerance_pct: 0.001,
            units_tolerance_abs: 0.01,
            units_tolerance_pct: 0.0001,
            cost_basis_tolerance_abs: 1.0,
            cost_basis_tolerance_pct: 0.001,
            allow_calibration_apply: true,
        },
        kelly: KellyConfig::default(),
        adjusted_decision: AdjustedDecisionConfig::default(),
        daily_plan: DailyPlanConfig::default(),
        assets: vec![],
        sectors: vec![],
        storage: Default::default(),
    };

    repo.save_config(&ctx, &config).await.unwrap();
    let loaded_config = repo.load_config(&ctx).await.unwrap();
    assert_eq!(loaded_config.portfolio.name, "Test Portfolio");

    // Cleanup
    let _ = fs::remove_file(&config_path);
}

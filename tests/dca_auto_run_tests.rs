use pendulum_kelly_cli::engine::dca::{auto_execute_dca, calculate_dca_preview};
use pendulum_kelly_cli::models::{
    AssetConfig, ConfigRoot, DcaFrequency, DcaPlan, NavCache, NavCacheEntry, PortfolioState,
};
use pendulum_kelly_cli::repository::{Repository, RepositoryContext, json::JsonRepository};
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_dca_auto_execution_idempotency() {
    let tmp = tempdir().unwrap();
    let repo = JsonRepository::new_with_defaults(tmp.path().to_str().unwrap());
    let repo: Arc<dyn Repository> = Arc::new(repo);
    let ctx = RepositoryContext::default();

    let mut config = ConfigRoot::default();
    config.assets.push(AssetConfig {
        asset_id: "a1".to_string(),
        fund_code: "001".to_string(),
        fund_name: "Fund 1".to_string(),
        sector: "S1".to_string(),
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

    let plan = DcaPlan {
        plan_id: "p1".to_string(),
        asset_id: "a1".to_string(),
        fund_code: "001".to_string(),
        fund_name: "Fund 1".to_string(),
        amount: 100.0,
        currency: "CNY".to_string(),
        frequency: DcaFrequency::Daily,
        weekday: None,
        month_day: None,
        start_date: "2026-06-01".to_string(),
        end_date: None,
        enabled: true,
        priority: 0,
        note: None,
        created_at: "".to_string(),
        updated_at: "".to_string(),
    };

    repo.save_plans(&ctx, &[plan]).await.unwrap();

    let mut nav_cache = NavCache::default();
    nav_cache.entries.push(NavCacheEntry {
        fund_code: "001".to_string(),
        nav: 1.0,
        accumulated_nav: Some(1.0),
        nav_date: "2026-06-01".to_string(),
        currency: "CNY".to_string(),
        source: "test".to_string(),
        fetched_at: "".to_string(),
    });
    repo.save_nav_cache(&ctx, &nav_cache).await.unwrap();

    let state = PortfolioState {
        cash: 1000.0,
        ..PortfolioState::default()
    };
    repo.save_state(&ctx, &state).await.unwrap();

    let target_date = "2026-06-01";

    // First execution
    let res1 = auto_execute_dca(repo.as_ref(), &ctx, &config, target_date)
        .await
        .unwrap();
    assert_eq!(res1.executed_count, 1);
    assert_eq!(res1.skipped_count, 0);

    let updated_state = repo.load_state(&ctx).await.unwrap();
    assert_eq!(updated_state.cash, 900.0);
    assert_eq!(updated_state.asset_holdings.len(), 1);
    assert_eq!(updated_state.asset_holdings[0].units, 100.0);

    // Second execution (same day)
    let res2 = auto_execute_dca(repo.as_ref(), &ctx, &config, target_date)
        .await
        .unwrap();
    assert_eq!(res2.executed_count, 0);
    assert_eq!(res2.skipped_count, 1);

    let final_state = repo.load_state(&ctx).await.unwrap();
    assert_eq!(final_state.cash, 900.0); // Should not change
}

#[test]
fn test_dca_preview_weekly() {
    let mut config = ConfigRoot::default();
    config.assets.push(AssetConfig {
        asset_id: "a1".to_string(),
        fund_code: "001".to_string(),
        fund_name: "Fund 1".to_string(),
        sector: "S1".to_string(),
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

    let plan = DcaPlan {
        plan_id: "p1".to_string(),
        asset_id: "a1".to_string(),
        fund_code: "001".to_string(),
        fund_name: "Fund 1".to_string(),
        amount: 100.0,
        currency: "CNY".to_string(),
        frequency: DcaFrequency::Weekly,
        weekday: Some(1), // Monday
        month_day: None,
        start_date: "2026-06-01".to_string(),
        end_date: None,
        enabled: true,
        priority: 0,
        note: None,
        created_at: "".to_string(),
        updated_at: "".to_string(),
    };

    let monday = "2026-06-01"; // Monday
    let tuesday = "2026-06-02"; // Tuesday

    let nav_cache = NavCache::default();
    let preview_mon = calculate_dca_preview(&config, std::slice::from_ref(&plan), &nav_cache, monday);
    assert_eq!(preview_mon.items[0].status, "今日应投");

    let preview_tue = calculate_dca_preview(&config, std::slice::from_ref(&plan), &nav_cache, tuesday);
    assert_eq!(preview_tue.items[0].status, "未到日期");
}

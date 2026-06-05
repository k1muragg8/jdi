pub mod adjusted_decision;
pub mod alipay_holding;
pub mod asset_enrichment;
pub mod backtest;
pub mod daily_operation;
pub mod daily_plan;
pub mod dca;
pub mod dca_lifecycle;
pub mod dca_settlement;
pub mod decision;
pub mod explanation;
pub mod holdings;
pub mod import;
pub mod instrument;
pub mod instrument_watchlist;
pub mod kelly;
pub mod mark_to_market;
pub mod market_quote;
pub mod operation;
pub mod portfolio_reconciliation;
pub mod portfolio_summary;
pub mod reconciliation;
pub mod refresh;
pub mod regime;
pub mod report;
pub mod report_summary;
pub mod risk_overlay;
pub mod valuation;
pub mod verification;

pub use adjusted_decision::{calculate_adjusted_decision, calculate_single_adjusted_item};
pub use asset_enrichment::{
    FundLookupResult, apply_fund_info_to_asset, classify_unassigned_assets, infer_sector_from_text,
    is_asset_archived, lookup_fund,
};
pub use backtest::run_backtest;
pub use daily_plan::generate_daily_execution_plan;
pub use dca::calculate_dca_preview;
pub use dca_lifecycle::calculate_dca_lifecycle;
pub use dca_settlement::calculate_settlement_impact;
pub use decision::{
    AssetBuySuggestion, DecisionResult, SectorBuySuggestion, generate_buy_suggestions,
};
pub use explanation::explain_decision;
pub use holdings::rebuild_holdings_from_transactions;
pub use instrument::{get_instrument_history, lookup_instrument, validate_instruments};
pub use instrument_watchlist::{
    MarketListFilter, archive_instrument, cleanup_test_instruments, duplicate_instrument_ids,
    is_instrument_archived, is_test_instrument, matches_filter, migrate_au9999_provider,
    migrate_instrument_flags, restore_default_instruments, restore_instrument, upsert_instrument,
};
pub use kelly::{calculate_kelly_preview, calculate_single_asset_kelly};
pub use mark_to_market::mark_to_market;
pub use market_quote::{
    apply_price_to_cache_entry, new_cache_entry_from_price, normalize_market_price,
};
pub use operation::run_autonomous_operation;
pub use portfolio_reconciliation::reconcile_portfolio;
pub use portfolio_summary::calculate_portfolio_summary;
pub use reconciliation::{generate_calibration_suggestion, reconcile_asset};
pub use regime::calculate_market_regime;
pub use report::{
    create_portfolio_snapshot, generate_investment_report, render_report_to_markdown,
};
pub use risk_overlay::calculate_risk_overlay;
pub use valuation::calculate_proxy_valuations;

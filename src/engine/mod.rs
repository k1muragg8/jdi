pub mod adjusted_decision;
pub mod daily_plan;
pub mod dca;
pub mod dca_lifecycle;
pub mod dca_settlement;
pub mod decision;
pub mod explanation;
pub mod holdings;
pub mod import;
pub mod instrument;
pub mod kelly;
pub mod mark_to_market;
pub mod portfolio_reconciliation;
pub mod portfolio_summary;
pub mod reconciliation;
pub mod regime;
pub mod report;
pub mod report_summary;
pub mod risk_overlay;
pub mod valuation;
pub mod verification;

pub use adjusted_decision::{calculate_adjusted_decision, calculate_single_adjusted_item};
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
pub use kelly::{calculate_kelly_preview, calculate_single_asset_kelly};
pub use mark_to_market::mark_to_market;
pub use portfolio_reconciliation::reconcile_portfolio;
pub use portfolio_summary::calculate_portfolio_summary;
pub use reconciliation::{generate_calibration_suggestion, reconcile_asset};
pub use regime::calculate_market_regime;
pub use report::{
    create_portfolio_snapshot, generate_investment_report, render_report_to_markdown,
};
pub use risk_overlay::calculate_risk_overlay;
pub use valuation::calculate_proxy_valuations;

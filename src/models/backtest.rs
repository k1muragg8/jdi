use super::dca::DcaExecutionResult;
use super::operation::{OperationPolicy, OperationSuggestion};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestRequest {
    pub start_date: String,
    pub end_date: String,
    pub initial_cash: f64,
    pub portfolio_id: String,
    pub policy_override: Option<OperationPolicy>,
    pub include_baseline: bool, // Fixed DCA baseline
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestReport {
    pub request: BacktestRequest,
    pub timestamp: String,
    pub main_metrics: BacktestMetrics,
    pub baseline_metrics: Option<BacktestMetrics>,
    pub daily_results: Vec<BacktestDayResult>,
    pub warnings: Vec<BacktestWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BacktestMetrics {
    pub final_value: f64,
    pub total_invested: f64,
    pub cash_remaining: f64,
    pub total_buy_days: usize,
    pub total_skipped_days: usize,
    pub max_drawdown: f64,
    pub annualized_return: f64,
    pub volatility: f64,
    pub average_buy_amount: f64,
    pub largest_buy_amount: f64,
    pub kelly_cap_hit_count: usize,
    pub cash_reserve_block_count: usize,
    pub target_allocation_block_count: usize,
    pub high_volatility_reduction_count: usize,
    pub hot_market_reduction_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestDayResult {
    pub date: String,
    pub total_value: f64,
    pub cash: f64,
    pub equity_weight: f64,
    pub execution_result: DcaExecutionResult,
    pub suggestions: Vec<OperationSuggestion>,
    pub trades: Vec<BacktestTradeSimulation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestTradeSimulation {
    pub asset_id: String,
    pub fund_name: String,
    pub amount: f64,
    pub units: f64,
    pub price: f64,
    pub trade_type: String, // "buy"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestWarning {
    pub date: Option<String>,
    pub asset_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestComparison {
    pub strategy_name: String,
    pub metrics: BacktestMetrics,
}

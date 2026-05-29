use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReportTransactionSummary {
    pub count: usize,
    pub total_amount: f64,
    pub buy_amount: f64,
    pub sell_amount: f64,
    pub dividend_amount: f64,
    pub fee_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReportCashFlowSummary {
    pub cash_in: f64,
    pub cash_out: f64,
    pub net_flow: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportHoldingChange {
    pub asset_id: String,
    pub units_changed: f64,
    pub value_changed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub portfolio_id: String,
    pub backend: String,
    pub period_start: String,
    pub period_end: String,
    pub initial_value: f64,
    pub final_value: f64,
    pub estimated_return: f64,
    pub tx_summary: ReportTransactionSummary,
    pub cash_flow: ReportCashFlowSummary,
    pub holding_changes: Vec<ReportHoldingChange>,
    pub top_holdings: Vec<String>,
}

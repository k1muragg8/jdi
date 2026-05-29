use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ReconciliationIssue {
    HoldingMismatch {
        asset_id: String,
        expected: f64,
        actual: f64,
        difference: f64,
        severity: IssueSeverity,
    },
    CashMismatch {
        currency: String,
        expected: f64,
        actual: f64,
        difference: f64,
        severity: IssueSeverity,
    },
    DuplicateTransactionIssue {
        tx_id_1: String,
        tx_id_2: String,
        fingerprint: String,
        severity: IssueSeverity,
    },
    MissingTransactionIssue {
        asset_id: String,
        date: String,
        description: String,
        severity: IssueSeverity,
    },
    SuspiciousTransactionIssue {
        tx_id: String,
        amount: f64,
        reason: String,
        severity: IssueSeverity,
    },
    UnknownTransactionType {
        tx_id: String,
        tx_type: String,
        severity: IssueSeverity,
    },
    MissingPriceOrNav {
        asset_id: String,
        date: String,
        severity: IssueSeverity,
    },
    DateOutOfRange {
        tx_id: String,
        date: String,
        severity: IssueSeverity,
    },
    NegativeQuantity {
        tx_id: String,
        quantity: f64,
        severity: IssueSeverity,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReconciliationSummary {
    pub total_transactions_checked: usize,
    pub total_issues: usize,
    pub critical_issues: usize,
    pub warning_issues: usize,
    pub affected_assets: Vec<String>,
    pub affected_dates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationReport {
    pub portfolio_id: String,
    pub generated_at: String,
    pub summary: ReconciliationSummary,
    pub issues: Vec<ReconciliationIssue>,
}

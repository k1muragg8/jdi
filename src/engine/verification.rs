use crate::repository::{RepositoryContext, traits::PortfolioRepository};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct VerificationIssue {
    pub severity: VerificationSeverity,
    pub domain: String,
    pub message: String,
    pub affected_records: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct VerificationSummary {
    pub total_checks: usize,
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone)]
pub struct DataVerificationReport {
    pub summary: VerificationSummary,
    pub issues: Vec<VerificationIssue>,
    pub portfolio_id: String,
}

pub async fn verify_data(
    repo: &dyn PortfolioRepository,
    ctx: &RepositoryContext,
    strict: bool,
) -> anyhow::Result<DataVerificationReport> {
    let mut report = DataVerificationReport {
        summary: VerificationSummary::default(),
        issues: Vec::new(),
        portfolio_id: ctx.portfolio_id.clone(),
    };

    let txs = repo.load_transactions(ctx).await.unwrap_or_default();
    let state = repo.load_state(ctx).await.unwrap_or_default();

    // 1. Transaction checks
    let mut tx_ids = HashSet::new();
    let mut fingerprints = HashSet::new();
    let mut holding_units: HashMap<String, f64> = HashMap::new();
    let mut cash_flow = 0.0;

    for tx in &txs {
        report.summary.total_checks += 1;
        // Duplicate ID
        if !tx_ids.insert(tx.id.clone()) {
            report.issues.push(VerificationIssue {
                severity: VerificationSeverity::Error,
                domain: "Transaction".to_string(),
                message: format!("Duplicate transaction ID: {}", tx.id),
                affected_records: vec![tx.id.clone()],
            });
        }

        // Duplicate fingerprint
        let fp = format!(
            "{}_{}_{}_{}_{}",
            tx.date,
            tx.transaction_type,
            tx.asset_id.as_deref().unwrap_or(""),
            tx.amount,
            tx.units.unwrap_or(0.0)
        );
        if !fingerprints.insert(fp) {
            report.issues.push(VerificationIssue {
                severity: if strict {
                    VerificationSeverity::Error
                } else {
                    VerificationSeverity::Warning
                },
                domain: "Transaction".to_string(),
                message: format!("Possible duplicate transaction detected: {}", tx.id),
                affected_records: vec![tx.id.clone()],
            });
        }

        if tx.date.is_empty() || tx.transaction_type.is_empty() {
            report.issues.push(VerificationIssue {
                severity: VerificationSeverity::Error,
                domain: "Transaction".to_string(),
                message: format!("Missing required fields in tx: {}", tx.id),
                affected_records: vec![tx.id.clone()],
            });
        }

        if tx.amount < 0.0 {
            report.issues.push(VerificationIssue {
                severity: VerificationSeverity::Error,
                domain: "Transaction".to_string(),
                message: format!("Negative amount in tx: {}", tx.id),
                affected_records: vec![tx.id.clone()],
            });
        }
        if tx.units.unwrap_or(0.0) < 0.0 {
            report.issues.push(VerificationIssue {
                severity: VerificationSeverity::Error,
                domain: "Transaction".to_string(),
                message: format!("Negative units in tx: {}", tx.id),
                affected_records: vec![tx.id.clone()],
            });
        }
        if tx.price.unwrap_or(0.0) < 0.0 {
            report.issues.push(VerificationIssue {
                severity: VerificationSeverity::Error,
                domain: "Transaction".to_string(),
                message: format!("Negative price in tx: {}", tx.id),
                affected_records: vec![tx.id.clone()],
            });
        }

        // Relationship
        if let (Some(u), Some(p)) = (tx.units, tx.price) {
            if u > 0.0 && p > 0.0 && tx.amount > 0.0 {
                let diff = (u * p - tx.amount).abs();
                if diff > 1.0 {
                    report.issues.push(VerificationIssue {
                        severity: if strict {
                            VerificationSeverity::Error
                        } else {
                            VerificationSeverity::Warning
                        },
                        domain: "Transaction".to_string(),
                        message: format!("Amount != Units * Price in tx: {}", tx.id),
                        affected_records: vec![tx.id.clone()],
                    });
                }
            }
        }

        // Holdings and cash simulation
        if tx.transaction_type == "buy" || tx.transaction_type == "买入" {
            if let Some(asset) = &tx.asset_id {
                *holding_units.entry(asset.clone()).or_insert(0.0) += tx.units.unwrap_or(0.0);
            }
            cash_flow -= tx.amount + tx.fee;
        } else if tx.transaction_type == "sell" || tx.transaction_type == "卖出" {
            if let Some(asset) = &tx.asset_id {
                *holding_units.entry(asset.clone()).or_insert(0.0) -= tx.units.unwrap_or(0.0);
            }
            cash_flow += tx.amount - tx.fee;
        } else if tx.transaction_type == "dividend"
            || tx.transaction_type == "分红"
            || tx.transaction_type == "cash_in"
            || tx.transaction_type == "现金转入"
        {
            cash_flow += tx.amount;
        } else if tx.transaction_type == "cash_out" || tx.transaction_type == "现金转出" {
            cash_flow -= tx.amount;
        }
    }

    // 2. Holdings checks
    for holding in &state.asset_holdings {
        report.summary.total_checks += 1;
        let derived = holding_units.get(&holding.asset_id).copied().unwrap_or(0.0);
        let diff = (holding.units - derived).abs();
        if diff > 0.01 {
            report.issues.push(VerificationIssue {
                severity: VerificationSeverity::Error,
                domain: "Holdings".to_string(),
                message: format!(
                    "Holding units mismatch for {}: stored={}, derived={}",
                    holding.asset_id, holding.units, derived
                ),
                affected_records: vec![holding.asset_id.clone()],
            });
        }
        if holding.units < 0.0 {
            report.issues.push(VerificationIssue {
                severity: VerificationSeverity::Error,
                domain: "Holdings".to_string(),
                message: format!(
                    "Negative holding units for {}: {}",
                    holding.asset_id, holding.units
                ),
                affected_records: vec![holding.asset_id.clone()],
            });
        }
    }

    // 3. Cash checks
    report.summary.total_checks += 1;
    let cash_diff = (state.cash - cash_flow).abs();
    if cash_diff > 0.01 {
        report.issues.push(VerificationIssue {
            severity: VerificationSeverity::Error,
            domain: "Cash".to_string(),
            message: format!(
                "Cash balance mismatch: stored={}, derived={}",
                state.cash, cash_flow
            ),
            affected_records: vec!["cash".to_string()],
        });
    }

    // Compile summary
    for issue in &report.issues {
        match issue.severity {
            VerificationSeverity::Error => report.summary.errors += 1,
            VerificationSeverity::Warning => report.summary.warnings += 1,
            _ => {}
        }
    }

    Ok(report)
}

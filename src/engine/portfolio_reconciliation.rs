use crate::engine::holdings::rebuild_holdings_from_transactions;
use crate::models::{
    IssueSeverity, PortfolioState, ReconciliationIssue, ReconciliationReport,
    ReconciliationSummary, Transaction,
};
use std::collections::{HashMap, HashSet};

pub fn reconcile_portfolio(
    portfolio_id: &str,
    current_state: &PortfolioState,
    transactions: &[Transaction],
) -> ReconciliationReport {
    let mut issues = Vec::new();
    let mut affected_assets = HashSet::new();
    let mut affected_dates = HashSet::new();
    let mut critical_count = 0;
    let mut warning_count = 0;

    // 1. Check transactions for anomalies
    let mut fingerprint_map: HashMap<String, String> = HashMap::new();
    let expected_tx_types = vec![
        "buy",
        "sell",
        "cash_in",
        "cash_out",
        "expense",
        "cash_set",
        "manual_cash_adjustment",
        "dividend",
        "fee",
    ];

    for tx in transactions {
        // Fingerprint / Duplicate check
        let fp = tx.fingerprint();
        if let Some(existing_id) = fingerprint_map.get(&fp) {
            issues.push(ReconciliationIssue::DuplicateTransactionIssue {
                tx_id_1: existing_id.clone(),
                tx_id_2: tx.id.clone(),
                fingerprint: fp.clone(),
                severity: IssueSeverity::Warning,
            });
            warning_count += 1;
            affected_dates.insert(tx.date.clone());
            if let Some(asset) = &tx.asset_id {
                affected_assets.insert(asset.clone());
            }
        } else {
            fingerprint_map.insert(fp, tx.id.clone());
        }

        // Unknown type
        if !expected_tx_types.contains(&tx.transaction_type.as_str()) {
            issues.push(ReconciliationIssue::UnknownTransactionType {
                tx_id: tx.id.clone(),
                tx_type: tx.transaction_type.clone(),
                severity: IssueSeverity::Warning,
            });
            warning_count += 1;
        }

        // Negative/impossible quantities
        if tx.amount < 0.0 {
            issues.push(ReconciliationIssue::NegativeQuantity {
                tx_id: tx.id.clone(),
                quantity: tx.amount,
                severity: IssueSeverity::Critical,
            });
            critical_count += 1;
        }
        if let Some(units) = tx.units {
            if units < 0.0 {
                issues.push(ReconciliationIssue::NegativeQuantity {
                    tx_id: tx.id.clone(),
                    quantity: units,
                    severity: IssueSeverity::Critical,
                });
                critical_count += 1;
            }
        }
        if tx.fee < 0.0 {
            issues.push(ReconciliationIssue::NegativeQuantity {
                tx_id: tx.id.clone(),
                quantity: tx.fee,
                severity: IssueSeverity::Critical,
            });
            critical_count += 1;
        }

        // Suspicious large amount
        if tx.amount > 10_000_000.0 {
            issues.push(ReconciliationIssue::SuspiciousTransactionIssue {
                tx_id: tx.id.clone(),
                amount: tx.amount,
                reason: "Amount exceeds 10,000,000".to_string(),
                severity: IssueSeverity::Warning,
            });
            warning_count += 1;
        }

        // Date check - basic format check for YYYY-MM-DD
        if tx.date.len() != 10 || tx.date.chars().nth(4) != Some('-') {
            issues.push(ReconciliationIssue::DateOutOfRange {
                tx_id: tx.id.clone(),
                date: tx.date.clone(),
                severity: IssueSeverity::Warning,
            });
            warning_count += 1;
        }
    }

    // 2. Rebuild state and compare with current
    let computed_state = match rebuild_holdings_from_transactions(transactions) {
        Ok(state) => state,
        Err(e) => {
            // We failed to rebuild, this might be due to selling more than held, etc.
            // Still, we try our best or return a critical error
            issues.push(ReconciliationIssue::SuspiciousTransactionIssue {
                tx_id: "N/A".to_string(),
                amount: 0.0,
                reason: format!("Failed to rebuild holdings: {}", e),
                severity: IssueSeverity::Critical,
            });
            critical_count += 1;
            PortfolioState {
                cash: 0.0,
                asset_holdings: vec![],
            }
        }
    };

    // Cash mismatch
    let cash_diff = computed_state.cash - current_state.cash;
    if cash_diff.abs() > 0.01 {
        issues.push(ReconciliationIssue::CashMismatch {
            currency: "CNY".to_string(), // Defaulting, maybe should come from config
            expected: computed_state.cash,
            actual: current_state.cash,
            difference: cash_diff,
            severity: IssueSeverity::Critical,
        });
        critical_count += 1;
    }

    // Holding mismatch
    let mut computed_holdings: HashMap<String, f64> = HashMap::new();
    for h in &computed_state.asset_holdings {
        computed_holdings.insert(h.asset_id.clone(), h.units);
    }

    let mut current_holdings_map: HashMap<String, f64> = HashMap::new();
    for h in &current_state.asset_holdings {
        current_holdings_map.insert(h.asset_id.clone(), h.units);

        // Check for missing price/NAV
        if h.units > 0.0 && h.latest_nav.is_none() && h.last_market_value <= 0.0 {
            issues.push(ReconciliationIssue::MissingPriceOrNav {
                asset_id: h.asset_id.clone(),
                date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                severity: IssueSeverity::Warning,
            });
            warning_count += 1;
            affected_assets.insert(h.asset_id.clone());
        }
    }

    // Compare
    let mut all_assets = HashSet::new();
    for a in computed_holdings.keys() {
        all_assets.insert(a.clone());
    }
    for a in current_holdings_map.keys() {
        all_assets.insert(a.clone());
    }

    for asset in all_assets {
        let expected = computed_holdings.get(&asset).copied().unwrap_or(0.0);
        let actual = current_holdings_map.get(&asset).copied().unwrap_or(0.0);
        let diff = expected - actual;

        if diff.abs() > 0.0001 {
            issues.push(ReconciliationIssue::HoldingMismatch {
                asset_id: asset.clone(),
                expected,
                actual,
                difference: diff,
                severity: IssueSeverity::Critical,
            });
            critical_count += 1;
            affected_assets.insert(asset);
        }
    }

    let total_issues = issues.len();

    let mut affected_assets_vec: Vec<String> = affected_assets.into_iter().collect();
    affected_assets_vec.sort();

    let mut affected_dates_vec: Vec<String> = affected_dates.into_iter().collect();
    affected_dates_vec.sort();

    ReconciliationReport {
        portfolio_id: portfolio_id.to_string(),
        generated_at: chrono::Local::now().to_rfc3339(),
        summary: ReconciliationSummary {
            total_transactions_checked: transactions.len(),
            total_issues,
            critical_issues: critical_count,
            warning_issues: warning_count,
            affected_assets: affected_assets_vec,
            affected_dates: affected_dates_vec,
        },
        issues,
    }
}

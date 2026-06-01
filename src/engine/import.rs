use crate::models::PortfolioState;
use crate::models::Transaction;
use crate::models::import::{
    ImportResult, ImportSummary, ImportedTransactionCandidate, TransactionImportPreview,
};
use anyhow::Result;
use csv::ReaderBuilder;
use std::collections::HashSet;
use std::io::Cursor;

pub fn parse_transactions_from_csv(csv_content: &str) -> Result<Vec<ImportedTransactionCandidate>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(Cursor::new(csv_content));

    let mut candidates = Vec::new();

    for result in rdr.deserialize() {
        let candidate: ImportedTransactionCandidate = result?;
        candidates.push(candidate);
    }

    Ok(candidates)
}

pub fn preview_import(
    candidates: Vec<ImportedTransactionCandidate>,
    existing_transactions: &[Transaction],
) -> TransactionImportPreview {
    let mut duplicates = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut summary = ImportSummary::default();

    let existing_fingerprints: HashSet<String> = existing_transactions
        .iter()
        .map(|tx| tx.fingerprint())
        .collect();

    summary.total_rows = candidates.len();

    for candidate in &candidates {
        let mut row_warnings = Vec::new();
        let mut row_errors = Vec::new();

        // Basic validation
        if candidate.date.is_empty() {
            row_errors.push("日期不能为空 (Date cannot be empty)".to_string());
        }

        let valid_types = [
            "buy",
            "sell",
            "dividend",
            "cash_in",
            "cash_out",
            "fee",
            "manual_cash_adjustment",
            "cash_set",
            "expense",
        ];
        if !valid_types.contains(&candidate.transaction_type.as_str()) {
            row_errors.push(format!(
                "未知交易类型 (Unknown type): {}",
                candidate.transaction_type
            ));
        }

        if candidate.amount == 0.0
            && (candidate.transaction_type == "buy" || candidate.transaction_type == "sell")
        {
            row_warnings.push("交易金额为0 (Amount is 0 for buy/sell)".to_string());
        }

        if (candidate.transaction_type == "buy" || candidate.transaction_type == "sell")
            && candidate.asset_id.is_none()
        {
            row_errors.push("买卖交易缺少资产ID (Asset ID missing for buy/sell)".to_string());
        }

        let tx = candidate.to_transaction();
        let is_duplicate = existing_fingerprints.contains(&tx.fingerprint());

        if is_duplicate {
            summary.duplicate_rows += 1;
        } else {
            summary.new_rows += 1;
        }

        if !row_errors.is_empty() {
            summary.error_rows += 1;
        } else {
            summary.valid_rows += 1;
        }

        if !row_warnings.is_empty() {
            summary.warning_rows += 1;
        }

        duplicates.push(is_duplicate);
        warnings.push(row_warnings);
        errors.push(row_errors);
    }

    TransactionImportPreview {
        candidates,
        duplicates,
        warnings,
        errors,
        summary,
    }
}

pub fn commit_import(
    preview: &TransactionImportPreview,
    state: &mut PortfolioState,
    transactions: &mut Vec<Transaction>,
    skip_duplicates: bool,
) -> ImportResult {
    let mut inserted = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for i in 0..preview.candidates.len() {
        let candidate = &preview.candidates[i];
        let is_duplicate = preview.duplicates[i];
        let has_errors = !preview.errors[i].is_empty();

        if has_errors {
            failed += 1;
            continue;
        }

        if is_duplicate && skip_duplicates {
            skipped += 1;
            continue;
        }

        let tx = candidate.to_transaction();
        if let Err(_e) = crate::engine::holdings::apply_transaction(state, &tx) {
            failed += 1;
            continue;
        }
        transactions.push(tx);
        inserted += 1;
    }

    ImportResult {
        inserted,
        skipped,
        failed,
        success: failed == 0,
        message: format!(
            "导入完成: 成功 {}, 跳过 {}, 失败 {}",
            inserted, skipped, failed
        ),
    }
}

pub fn print_preview_summary(preview: &TransactionImportPreview) {
    println!("\n导入预览摘要 (Import Preview Summary):");
    println!("------------------------------------");
    println!("总行数 (Total Rows):      {}", preview.summary.total_rows);
    println!("有效行数 (Valid Rows):    {}", preview.summary.valid_rows);
    println!("错误行数 (Error Rows):    {}", preview.summary.error_rows);
    println!("警告行数 (Warning Rows):  {}", preview.summary.warning_rows);
    println!(
        "重复行数 (Duplicate Rows): {}",
        preview.summary.duplicate_rows
    );
    println!("新增行数 (New Rows):       {}", preview.summary.new_rows);
    println!("------------------------------------");

    if !preview.candidates.is_empty() {
        println!(
            "\n{:<12} | {:<10} | {:<15} | {:<10} | {:<10} | {:<10} | Status",
            "Date", "Type", "Asset ID", "Amount", "Units", "Price"
        );
        println!("{:-<90}", "");

        for i in 0..preview.candidates.len() {
            let candidate = &preview.candidates[i];
            let is_duplicate = preview.duplicates[i];
            let row_errors = &preview.errors[i];
            let row_warnings = &preview.warnings[i];

            let status = if !row_errors.is_empty() {
                "ERROR"
            } else if is_duplicate {
                "DUPLICATE"
            } else if !row_warnings.is_empty() {
                "WARNING"
            } else {
                "NEW"
            };

            println!(
                "{:<12} | {:<10} | {:<15} | {:<10.2} | {:<10} | {:<10} | {}",
                candidate.date,
                candidate.transaction_type,
                candidate.asset_id.as_deref().unwrap_or("-"),
                candidate.amount,
                candidate
                    .units
                    .map(|u| format!("{:.2}", u))
                    .unwrap_or_else(|| "-".to_string()),
                candidate
                    .price
                    .map(|p| format!("{:.2}", p))
                    .unwrap_or_else(|| "-".to_string()),
                status
            );

            for err in row_errors {
                println!("  [ERROR] {}", err);
            }
            for warn in row_warnings {
                println!("  [WARNING] {}", warn);
            }
        }
    }
}

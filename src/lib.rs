pub mod api;
pub mod cli;
pub mod engine;
pub mod error;
pub mod models;
pub mod storage;

use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use cli::{CashCommands, Cli, Commands, ExpenseCommands, TxAddCommands, TxCommands};
use models::Transaction;
use std::fs;
use std::path::Path;

fn ensure_data_dir() -> Result<()> {
    let data_dir = Path::new("data");
    if !data_dir.exists() {
        fs::create_dir_all(data_dir).context("Failed to create data/ directory")?;
    }

    let examples = vec![
        ("examples/config.toml", "data/config.toml"),
        ("examples/portfolio_state.json", "data/portfolio_state.json"),
        ("examples/transactions.json", "data/transactions.json"),
    ];

    for (src, dest) in examples {
        if !Path::new(dest).exists() && Path::new(src).exists() {
            fs::copy(src, dest).context(format!("Failed to copy {} to {}", src, dest))?;
        }
    }

    Ok(())
}

pub fn run() -> Result<()> {
    ensure_data_dir()?;
    let cli = Cli::parse();

    let config = storage::load_config(&cli.config)?;
    let mut state = storage::load_state(&cli.state)?;
    let mut transactions = storage::load_transactions(&cli.transactions)?;

    let fund_provider = api::MockFundProvider::new();

    let generate_tx_id = || format!("tx_{}", Local::now().format("%Y%m%d_%H%M%S"));

    match &cli.command {
        Commands::Holdings => {
            println!("Holdings:");
            println!(
                "{:<20} | {:<10} | {:<20} | {:<10} | {:<15} | {:<10} | {:<15} | {:<15} | {:<15}",
                "Asset ID",
                "Fund Code",
                "Fund Name",
                "Sector",
                "Units",
                "NAV",
                "Market Value",
                "Cost",
                "P&L"
            );
            println!("{:-<155}", "");

            for holding in &state.asset_holdings {
                let asset_config = config
                    .assets
                    .iter()
                    .find(|a| a.asset_id == holding.asset_id);
                let fund_name = asset_config
                    .map(|a| a.fund_name.as_str())
                    .unwrap_or("Unknown");
                let sector = asset_config.map(|a| a.sector.as_str()).unwrap_or("Unknown");

                let nav_str = holding
                    .latest_nav
                    .map(|n| format!("{:.4}", n))
                    .unwrap_or_else(|| "N/A".to_string());

                let market_value = holding.last_market_value;
                let cost = holding.cost_basis;
                let pnl = market_value - cost;

                println!(
                    "{:<20} | {:<10} | {:<20} | {:<10} | {:<15.2} | {:<10} | {:<15.2} | {:<15.2} | {:<15.2}",
                    holding.asset_id,
                    holding.fund_code,
                    fund_name,
                    sector,
                    holding.units,
                    nav_str,
                    market_value,
                    cost,
                    pnl
                );
            }
        }
        Commands::Mtm => {
            engine::mark_to_market(&config, &mut state, &fund_provider)?;
            storage::save_state(&cli.state, &state)?;
            println!("Mark-to-market completed successfully.");

            for holding in &state.asset_holdings {
                let nav_str = holding
                    .latest_nav
                    .map(|n| format!("{:.4}", n)) // Requirement: NAV with 4 decimals
                    .unwrap_or_else(|| "N/A".to_string());
                println!(
                    "Updated {} - NAV: {}, Market Value: {:.2}",
                    holding.asset_id, nav_str, holding.last_market_value
                );
            }
        }
        Commands::Tx { command } => match command {
            TxCommands::List => {
                println!(
                    "{:<20} | {:<12} | {:<10} | {:<15} | {:<10} | {:<10} | {:<10} | {:<5} | {:<10} | {}",
                    "ID",
                    "Date",
                    "Type",
                    "Asset ID",
                    "Amount",
                    "Units",
                    "Price",
                    "Fee",
                    "Currency",
                    "Note"
                );
                println!("{:-<135}", "");
                for tx in &transactions {
                    println!(
                        "{:<20} | {:<12} | {:<10} | {:<15} | {:<10.2} | {:<10} | {:<10} | {:<5.2} | {:<10} | {}",
                        tx.id,
                        tx.date,
                        tx.transaction_type,
                        tx.asset_id.as_deref().unwrap_or("-"),
                        tx.amount,
                        tx.units
                            .map(|u| format!("{:.2}", u))
                            .unwrap_or_else(|| "-".to_string()),
                        tx.price
                            .map(|p| format!("{:.2}", p))
                            .unwrap_or_else(|| "-".to_string()),
                        tx.fee,
                        tx.currency,
                        tx.note
                    );
                }
            }
            TxCommands::Add { command } => match command {
                TxAddCommands::Buy {
                    asset_id,
                    amount,
                    price,
                    date,
                    note,
                    units,
                    fee,
                    currency,
                } => {
                    let mut final_units = *amount / *price;
                    if let Some(u) = units {
                        if (*amount - (*price * u)).abs() > 0.01 {
                            println!(
                                "Warning: Provided amount and units*price are not roughly equal."
                            );
                        }
                        final_units = *u;
                    }

                    let tx = Transaction {
                        id: generate_tx_id(),
                        date: date.clone(),
                        transaction_type: "buy".to_string(),
                        asset_id: Some(asset_id.clone()),
                        amount: *amount,
                        units: Some(final_units),
                        price: Some(*price),
                        fee: *fee,
                        currency: currency.clone(),
                        note: note.clone(),
                    };

                    engine::holdings::apply_transaction(&mut state, &tx)?;
                    transactions.push(tx);

                    storage::save_state(&cli.state, &state)?;
                    storage::save_transactions(&cli.transactions, &transactions)?;

                    if let Some(holding) = state
                        .asset_holdings
                        .iter()
                        .find(|a| a.asset_id == *asset_id)
                    {
                        println!(
                            "Buy recorded. {} Units: {:.2}, Cost: {:.2}, Cash: {:.2}",
                            asset_id, holding.units, holding.cost_basis, state.cash
                        );
                    }
                }
                TxAddCommands::Sell {
                    asset_id,
                    units,
                    price,
                    date,
                    note,
                    amount,
                    fee,
                    currency,
                } => {
                    let mut final_units = 0.0;
                    let mut final_amount = 0.0;

                    if let Some(u) = units {
                        final_units = *u;
                        final_amount = final_units * *price;
                        if let Some(a) = amount {
                            if (*a - final_amount).abs() > 0.01 {
                                println!(
                                    "Warning: Provided amount and units*price are not roughly equal."
                                );
                            }
                            final_amount = *a;
                        }
                    } else if let Some(a) = amount {
                        final_amount = *a;
                        final_units = final_amount / *price;
                    }

                    let tx = Transaction {
                        id: generate_tx_id(),
                        date: date.clone(),
                        transaction_type: "sell".to_string(),
                        asset_id: Some(asset_id.clone()),
                        amount: final_amount,
                        units: Some(final_units),
                        price: Some(*price),
                        fee: *fee,
                        currency: currency.clone(),
                        note: note.clone(),
                    };

                    engine::holdings::apply_transaction(&mut state, &tx)?;
                    transactions.push(tx);

                    storage::save_state(&cli.state, &state)?;
                    storage::save_transactions(&cli.transactions, &transactions)?;

                    if let Some(holding) = state
                        .asset_holdings
                        .iter()
                        .find(|a| a.asset_id == *asset_id)
                    {
                        println!(
                            "Sell recorded. {} Units: {:.2}, Cash: {:.2}",
                            asset_id, holding.units, state.cash
                        );
                    }
                }
            },
        },
        Commands::Cash { command } => match command {
            CashCommands::Set { amount } => {
                let tx = Transaction {
                    id: generate_tx_id(),
                    date: Local::now().format("%Y-%m-%d").to_string(),
                    transaction_type: "manual_cash_adjustment".to_string(),
                    asset_id: None,
                    amount: *amount,
                    units: None,
                    price: None,
                    fee: 0.0,
                    currency: config.portfolio.base_currency.clone(),
                    note: "Manual cash set".to_string(),
                };
                engine::holdings::apply_transaction(&mut state, &tx)?;
                transactions.push(tx);
                storage::save_state(&cli.state, &state)?;
                storage::save_transactions(&cli.transactions, &transactions)?;
                println!("Cash set to {:.2}", state.cash);
            }
            CashCommands::In { amount, note } => {
                let tx = Transaction {
                    id: generate_tx_id(),
                    date: Local::now().format("%Y-%m-%d").to_string(),
                    transaction_type: "cash_in".to_string(),
                    asset_id: None,
                    amount: *amount,
                    units: None,
                    price: None,
                    fee: 0.0,
                    currency: config.portfolio.base_currency.clone(),
                    note: note.clone(),
                };
                engine::holdings::apply_transaction(&mut state, &tx)?;
                transactions.push(tx);
                storage::save_state(&cli.state, &state)?;
                storage::save_transactions(&cli.transactions, &transactions)?;
                println!("Cash in recorded. New balance: {:.2}", state.cash);
            }
            CashCommands::Out { amount, note } => {
                let tx = Transaction {
                    id: generate_tx_id(),
                    date: Local::now().format("%Y-%m-%d").to_string(),
                    transaction_type: "cash_out".to_string(),
                    asset_id: None,
                    amount: *amount,
                    units: None,
                    price: None,
                    fee: 0.0,
                    currency: config.portfolio.base_currency.clone(),
                    note: note.clone(),
                };
                engine::holdings::apply_transaction(&mut state, &tx)?;
                transactions.push(tx);
                storage::save_state(&cli.state, &state)?;
                storage::save_transactions(&cli.transactions, &transactions)?;
                println!("Cash out recorded. New balance: {:.2}", state.cash);
            }
        },
        Commands::Expense { command } => match command {
            ExpenseCommands::Add { amount, note } => {
                let tx = Transaction {
                    id: generate_tx_id(),
                    date: Local::now().format("%Y-%m-%d").to_string(),
                    transaction_type: "expense".to_string(),
                    asset_id: None,
                    amount: *amount,
                    units: None,
                    price: None,
                    fee: 0.0,
                    currency: config.portfolio.base_currency.clone(),
                    note: note.clone(),
                };
                engine::holdings::apply_transaction(&mut state, &tx)?;
                transactions.push(tx);
                storage::save_state(&cli.state, &state)?;
                storage::save_transactions(&cli.transactions, &transactions)?;
                println!("Expense recorded. New balance: {:.2}", state.cash);
            }
        },
    }

    Ok(())
}

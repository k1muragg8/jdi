pub mod api;
pub mod cli;
pub mod engine;
pub mod error;
pub mod models;
pub mod storage;

use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use cli::{
    CashCommands, Cli, Commands, ExpenseCommands, PortfolioCommands, SectorCommands, TxAddCommands,
    TxCommands,
};
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
        Commands::Holdings { all } => {
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

                let is_enabled = asset_config.map(|a| a.enabled).unwrap_or(false);
                if !is_enabled && !all {
                    continue;
                }

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
        Commands::Portfolio { command } => match command {
            PortfolioCommands::Summary => {
                let summary = engine::calculate_portfolio_summary(&config, &state);

                println!("Portfolio Summary\n");

                println!(
                    "Cash: {:.2} {}",
                    summary.cash, config.portfolio.base_currency
                );
                println!(
                    "Reserve Cash: {:.2} {}",
                    summary.reserve_cash, config.portfolio.base_currency
                );
                println!(
                    "Upcoming Expense: {:.2} {}",
                    summary.upcoming_expense, config.portfolio.base_currency
                );
                println!(
                    "Available Cash: {:.2} {}\n",
                    summary.available_cash, config.portfolio.base_currency
                );

                println!(
                    "Target Equity Value: {:.2} {}",
                    summary.target_equity_value, config.portfolio.base_currency
                );
                println!(
                    "Current Equity Value: {:.2} {}",
                    summary.equity_value, config.portfolio.base_currency
                );
                println!(
                    "Equity Gap: {:.2} {}\n",
                    summary.equity_gap, config.portfolio.base_currency
                );

                println!(
                    "Fund Value: {:.2} {}",
                    summary.fund_value, config.portfolio.base_currency
                );
                println!(
                    "Bond Value: {:.2} {}",
                    summary.bond_value, config.portfolio.base_currency
                );
                println!(
                    "Crypto Value: {:.2} {}",
                    summary.crypto_value, config.portfolio.base_currency
                );
                println!(
                    "Total Asset Value: {:.2} {}",
                    summary.total_asset_value, config.portfolio.base_currency
                );
            }
        },
        Commands::Sector { command } => match command {
            SectorCommands::List => {
                for sector in &config.sectors {
                    println!(
                        "Sector ID: {}, Name: {}, Asset Class: {}, Target Weight: {:.2}, Priority: {}, Enabled: {}",
                        sector.sector_id,
                        sector.name,
                        sector.asset_class,
                        sector.target_weight,
                        sector.priority,
                        sector.enabled
                    );
                }
            }
            SectorCommands::Summary => {
                let summary = engine::calculate_portfolio_summary(&config, &state);

                let enabled_weight_sum: f64 = config
                    .sectors
                    .iter()
                    .filter(|s| s.enabled)
                    .map(|s| s.target_weight)
                    .sum();
                if (enabled_weight_sum - 1.0).abs() > 0.001 {
                    eprintln!(
                        "Warning: enabled sector target weights sum to {:.2}, expected 1.00.",
                        enabled_weight_sum
                    );
                }

                println!(
                    "{:<20} | {:<10} | {:<10} | {:<15} | {:<15} | {:<15} | {}",
                    "Sector",
                    "Target %",
                    "Current %",
                    "Target Value",
                    "Current Value",
                    "Gap",
                    "Status"
                );
                println!("{:-<110}", "");
                for s in summary.sector_summaries {
                    println!(
                        "{:<20} | {:<10.2}% | {:<10.2}% | {:<15.2} | {:<15.2} | {:<15.2} | {}",
                        s.sector_name,
                        s.target_weight * 100.0,
                        s.current_weight * 100.0,
                        s.target_value,
                        s.current_value,
                        s.gap_value,
                        s.status
                    );
                }
            }
            SectorCommands::SetTarget {
                sector_id,
                target_weight,
            } => {
                if *target_weight < 0.0 || *target_weight > 1.0 {
                    anyhow::bail!("Target weight must be between 0.0 and 1.0");
                }
                let mut config_clone = config.clone();
                if let Some(sector) = config_clone
                    .sectors
                    .iter_mut()
                    .find(|s| s.sector_id == *sector_id)
                {
                    sector.target_weight = *target_weight;
                    storage::save_config(&cli.config, &config_clone)?;
                    println!(
                        "Set target weight for {} to {:.2}",
                        sector_id, target_weight
                    );

                    let enabled_weight_sum: f64 = config_clone
                        .sectors
                        .iter()
                        .filter(|s| s.enabled)
                        .map(|s| s.target_weight)
                        .sum();
                    println!(
                        "Current total enabled target weight: {:.2}",
                        enabled_weight_sum
                    );
                } else {
                    println!("Sector not found.");
                }
            }
        },
        Commands::Fund { command } => match command {
            cli::FundCommands::Lookup { fund_code } => {
                use api::FundProvider;
                match fund_provider.search_fund_by_code(fund_code) {
                    Ok(info) => {
                        println!("Fund Code: {}", info.fund_code);
                        println!("Fund Name: {}", info.fund_name);
                        println!("Fund Type: {}", info.fund_type);
                        println!("Currency: {}", info.currency);

                        if let Ok(nav) = fund_provider.fetch_latest_nav(fund_code) {
                            println!("Latest NAV: {:.4}", nav.nav);
                            println!("NAV Date: {}", nav.nav_date);
                        }
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }
        },
        Commands::Asset { command } => match command {
            cli::AssetCommands::List => {
                println!(
                    "{:<20} | {:<10} | {:<20} | {:<10} | {:<8} | {:<10} | {}",
                    "Asset ID",
                    "Fund Code",
                    "Fund Name",
                    "Sector",
                    "Currency",
                    "Val Method",
                    "Enabled"
                );
                println!("{:-<105}", "");
                for asset in &config.assets {
                    println!(
                        "{:<20} | {:<10} | {:<20} | {:<10} | {:<8} | {:<10} | {}",
                        asset.asset_id,
                        asset.fund_code,
                        asset.fund_name,
                        asset.sector,
                        asset.currency,
                        asset.valuation_method,
                        asset.enabled
                    );
                }
            }
            cli::AssetCommands::Add {
                asset_id,
                fund_code,
                fund_name,
                sector,
                currency,
                valuation_method,
                units,
                cost_basis,
            } => {
                let mut config_clone = config.clone();
                if config_clone.assets.iter().any(|a| a.asset_id == *asset_id) {
                    anyhow::bail!("Asset ID '{}' already exists", asset_id);
                }

                if config_clone
                    .assets
                    .iter()
                    .any(|a| a.fund_code == *fund_code)
                {
                    println!(
                        "Warning: Fund code '{}' is already associated with another asset.",
                        fund_code
                    );
                }

                let final_fund_name = match fund_name {
                    Some(name) => name.clone(),
                    None => {
                        use api::FundProvider;
                        match fund_provider.search_fund_by_code(fund_code) {
                            Ok(info) => info.fund_name,
                            Err(_) => "Unknown".to_string(),
                        }
                    }
                };

                let new_asset = models::AssetConfig {
                    asset_id: asset_id.clone(),
                    fund_code: fund_code.clone(),
                    fund_name: final_fund_name,
                    sector: sector.clone(),
                    currency: currency.clone(),
                    valuation_method: valuation_method.clone(),
                    enabled: true,
                };

                config_clone.assets.push(new_asset);

                let new_holding = models::AssetHolding {
                    asset_id: asset_id.clone(),
                    fund_code: fund_code.clone(),
                    units: *units,
                    units_estimated: false,
                    cost_basis: *cost_basis,
                    latest_nav: None,
                    latest_nav_date: None,
                    last_market_value: 0.0,
                };

                state.asset_holdings.push(new_holding);

                storage::save_config(&cli.config, &config_clone)?;
                storage::save_state(&cli.state, &state)?;
                println!("Asset {} added successfully.", asset_id);
            }
            cli::AssetCommands::Disable { asset_id } => {
                let mut config_clone = config.clone();
                if let Some(asset) = config_clone
                    .assets
                    .iter_mut()
                    .find(|a| a.asset_id == *asset_id)
                {
                    asset.enabled = false;
                    storage::save_config(&cli.config, &config_clone)?;
                    println!("Asset {} disabled.", asset_id);
                } else {
                    println!("Asset not found.");
                }
            }
            cli::AssetCommands::Enable { asset_id } => {
                let mut config_clone = config.clone();
                if let Some(asset) = config_clone
                    .assets
                    .iter_mut()
                    .find(|a| a.asset_id == *asset_id)
                {
                    asset.enabled = true;
                    storage::save_config(&cli.config, &config_clone)?;
                    println!("Asset {} enabled.", asset_id);
                } else {
                    println!("Asset not found.");
                }
            }
            cli::AssetCommands::Remove { asset_id } => {
                let mut config_clone = config.clone();
                if let Some(asset) = config_clone
                    .assets
                    .iter_mut()
                    .find(|a| a.asset_id == *asset_id)
                {
                    asset.enabled = false;
                    storage::save_config(&cli.config, &config_clone)?;
                    println!("Asset disabled, holding preserved.");
                } else {
                    println!("Asset not found.");
                }
            }
        },
    }

    Ok(())
}

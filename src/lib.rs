pub mod api;
pub mod cli;
pub mod engine;
pub mod error;
pub mod models;
pub mod storage;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    let config = storage::load_config(&cli.config)?;
    let mut state = storage::load_state(&cli.state)?;
    let _transactions = storage::load_transactions(&cli.transactions)?;

    let fund_provider = api::MockFundProvider::new();

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
                println!(
                    "Updated {} - NAV: {:?}, Market Value: {}",
                    holding.asset_id, holding.latest_nav, holding.last_market_value
                );
            }
        }
    }

    Ok(())
}

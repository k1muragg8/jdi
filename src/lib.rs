pub mod api;
pub mod cli;
pub mod engine;
pub mod error;
pub mod models;
pub mod storage;
pub mod web;

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
    let mut cache = storage::load_cache(&cli.cache)?;

    let fund_provider = api::create_fund_provider(&config.api);

    let generate_tx_id = || format!("tx_{}", Local::now().format("%Y%m%d_%H%M%S"));

    match &cli.command {
        Commands::Holdings { all } => {
            println!("当前持仓:");
            println!(
                "{:<20} | {:<10} | {:<20} | {:<10} | {:<15} | {:<10} | {:<12} | {:<10} | {:<10} | {:<10}",
                "资产ID",
                "基金代码",
                "基金名称",
                "赛道",
                "持有份额",
                "最新净值",
                "净值日期",
                "数据来源",
                "数据状态",
                "浮动盈亏"
            );
            println!("{:-<165}", "");

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

                let nav_date = holding.latest_nav_date.as_deref().unwrap_or("N/A");
                let source = holding.latest_nav_source.as_deref().unwrap_or("N/A");
                let status = holding.latest_nav_status.as_deref().unwrap_or("N/A");

                let market_value = holding.last_market_value;
                let cost = holding.cost_basis;
                let pnl = market_value - cost;

                println!(
                    "{:<20} | {:<10} | {:<20} | {:<10} | {:<15.2} | {:<10} | {:<12} | {:<10} | {:<10} | {:<10.2}",
                    holding.asset_id,
                    holding.fund_code,
                    fund_name,
                    sector,
                    holding.units,
                    nav_str,
                    nav_date,
                    source,
                    status,
                    pnl
                );
            }
        }
        Commands::Mtm => {
            engine::mark_to_market(&config, &mut state, fund_provider.as_ref(), &mut cache)?;
            storage::save_state(&cli.state, &state)?;
            storage::save_cache(&cli.cache, &cache)?;
            println!("估值更新完成。");

            for holding in &state.asset_holdings {
                if let Some(nav) = holding.latest_nav {
                    let nav_date = holding.latest_nav_date.as_deref().unwrap_or("N/A");
                    println!(
                        "已更新 {} - 净值: {:.4}, 净值日期: {}, 当前市值: {:.2}",
                        holding.asset_id, nav, nav_date, holding.last_market_value
                    );
                }
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
                    let type_cn = match tx.transaction_type.as_str() {
                        "buy" => "买入",
                        "sell" => "卖出",
                        "cash_in" => "现金转入",
                        "cash_out" => "现金转出",
                        "expense" => "支出",
                        "manual_cash_adjustment" | "cash_set" => "手动现金调整",
                        other => other,
                    };
                    println!(
                        "{:<20} | {:<12} | {:<10} | {:<15} | {:<10.2} | {:<10} | {:<10} | {:<5.2} | {:<10} | {}",
                        tx.id,
                        tx.date,
                        type_cn,
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
        Commands::Decision { command } => match command {
            cli::DecisionCommands::Preview => {
                let date = Local::now().format("%Y-%m-%d").to_string();
                let result = engine::generate_buy_suggestions(&config, &state, date);

                println!("今日买入建议\n");

                println!(
                    "可用现金: {:.2} {}",
                    result.available_cash, config.portfolio.base_currency
                );
                println!(
                    "目标权益仓: {:.2} {}",
                    result.target_equity_value, config.portfolio.base_currency
                );
                println!(
                    "当前权益仓: {:.2} {}",
                    result.current_equity_value, config.portfolio.base_currency
                );
                println!(
                    "权益缺口: {:.2} {}",
                    result.equity_gap, config.portfolio.base_currency
                );
                println!(
                    "单日买入上限: {:.2} {}\n",
                    result.max_daily_buy_total, config.portfolio.base_currency
                );

                println!(
                    "今日建议总买入: {:.2} {}\n",
                    result.suggested_total_buy, config.portfolio.base_currency
                );

                if !result.warnings.is_empty() {
                    for warning in &result.warnings {
                        println!("Warning: {}", warning);
                    }
                    println!();
                }

                if result.suggested_total_buy > 0.0 {
                    println!("建议:\n");
                    println!(
                        "{:<15} | {:<20} | {:<10} | {:<15} | {:<15} | {}",
                        "赛道", "资产", "基金代码", "缺口", "建议买入", "原因"
                    );
                    println!("{:-<105}", "");

                    for sector in result.sector_suggestions {
                        for asset in sector.asset_suggestions {
                            println!(
                                "{:<15} | {:<20} | {:<10} | {:<15.2} | {:<15.2} | {}",
                                asset.sector_name,
                                asset.fund_name,
                                asset.fund_code,
                                sector.gap_value,
                                asset.suggested_buy,
                                asset.reason
                            );
                        }
                    }
                }
            }
            cli::DecisionCommands::Explain => {
                let date = Local::now().format("%Y-%m-%d").to_string();
                let result = engine::generate_buy_suggestions(&config, &state, date);

                println!(
                    "今日建议买入：{:.2} {}。\n",
                    result.suggested_total_buy, config.portfolio.base_currency
                );
                println!("原因：\n");
                let mut step = 1;
                println!(
                    "{}. 扣除现金安全垫和近期支出后，可用现金为 {:.2} {}；",
                    step, result.available_cash, config.portfolio.base_currency
                );
                step += 1;

                if result.equity_gap > 0.0 {
                    println!("{}. 当前权益仓仍低于目标权益仓；", step);
                } else {
                    println!("{}. 当前权益仓已经达到或超过目标权益仓；", step);
                }
                step += 1;

                if result.suggested_total_buy > 0.0 {
                    println!("{}. 当前组合仍有多个权益赛道处于低配状态；", step);
                    step += 1;
                }

                if result.max_daily_buy_total > 0.0
                    && (result.max_daily_buy_total - result.suggested_total_buy).abs() < 0.01
                {
                    println!("{}. 今日买入金额受到单日买入上限限制；", step);
                    step += 1;
                }

                println!("{}. 所有建议均未超过单赛道和单资产风控上限。", step);
            }
        },
        Commands::Portfolio { command } => match command {
            PortfolioCommands::Summary => {
                let summary = engine::calculate_portfolio_summary(&config, &state);

                println!("组合概览\n");

                println!(
                    "当前现金: {:.2} {}",
                    summary.cash, config.portfolio.base_currency
                );
                println!(
                    "现金安全垫: {:.2} {}",
                    summary.reserve_cash, config.portfolio.base_currency
                );
                println!(
                    "近期支出: {:.2} {}",
                    summary.upcoming_expense, config.portfolio.base_currency
                );
                println!(
                    "可用现金: {:.2} {}\n",
                    summary.available_cash, config.portfolio.base_currency
                );

                println!(
                    "目标权益仓: {:.2} {}",
                    summary.target_equity_value, config.portfolio.base_currency
                );
                println!(
                    "当前权益仓: {:.2} {}",
                    summary.equity_value, config.portfolio.base_currency
                );
                println!(
                    "权益缺口: {:.2} {}\n",
                    summary.equity_gap, config.portfolio.base_currency
                );

                println!(
                    "基金市值: {:.2} {}",
                    summary.fund_value, config.portfolio.base_currency
                );
                println!(
                    "债券市值: {:.2} {}",
                    summary.bond_value, config.portfolio.base_currency
                );
                println!(
                    "加密资产市值: {:.2} {}",
                    summary.crypto_value, config.portfolio.base_currency
                );
                println!(
                    "总资产: {:.2} {}",
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
                    "赛道", "目标占比", "当前占比", "目标市值", "当前市值", "缺口", "状态"
                );
                println!("{:-<110}", "");
                for s in summary.sector_summaries {
                    let target_pct = format!("{:.2}%", s.target_weight * 100.0);
                    let current_pct = format!("{:.2}%", s.current_weight * 100.0);
                    let status_cn = match s.status.as_str() {
                        "underweight" => "低配",
                        "neutral" => "均衡",
                        "overweight" => "超配",
                        "disabled" => "已禁用",
                        other => other,
                    };
                    println!(
                        "{:<20} | {:<10} | {:<10} | {:<15.2} | {:<15.2} | {:<15.2} | {}",
                        s.sector_name,
                        target_pct,
                        current_pct,
                        s.target_value,
                        s.current_value,
                        s.gap_value,
                        status_cn
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
                match fund_provider.search_fund_by_code(fund_code) {
                    Ok(info) => {
                        println!("基金代码: {}", info.fund_code);
                        println!("基金名称: {}", info.fund_name);
                        println!("基金类型: {}", info.fund_type);
                        println!("币种: {}", info.currency);
                        println!("数据来源: {}", info.source);

                        if let Ok(nav) = fund_provider.fetch_latest_nav(fund_code) {
                            println!("最新净值: {:.4}", nav.nav);
                            println!("净值日期: {}", nav.nav_date);
                            if let Some(acc) = nav.accumulated_nav {
                                println!("累计净值: {:.4}", acc);
                            }
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
                    anyhow::bail!("资产ID已存在: {}", asset_id);
                }

                if config_clone
                    .assets
                    .iter()
                    .any(|a| a.fund_code == *fund_code)
                {
                    println!("警告: 基金代码 '{}' 已经被其他资产使用。", fund_code);
                }

                let final_fund_name = match fund_name {
                    Some(name) => name.clone(),
                    None => match fund_provider.search_fund_by_code(fund_code) {
                        Ok(info) => info.fund_name,
                        Err(_) => "未找到该基金代码".to_string(),
                    },
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
                    latest_nav_source: None,
                    latest_nav_status: None,
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
        Commands::Web { port } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                web::start_server(
                    *port,
                    cli.config.clone(),
                    cli.state.clone(),
                    cli.transactions.clone(),
                )
                .await
            })?;
        }
    }

    Ok(())
}

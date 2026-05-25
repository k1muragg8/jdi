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
                    let source = holding.latest_nav_source.as_deref().unwrap_or("N/A");
                    println!(
                        "已更新 {} - 净值: {:.4}, 净值日期: {}, 当前市值: {:.2}, 数据来源: {}",
                        holding.asset_id, nav, nav_date, holding.last_market_value, source
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
            SectorCommands::Add {
                sector_id,
                name,
                asset_class,
                target_weight,
                priority,
            } => {
                let mut config_clone = config.clone();
                if config_clone
                    .sectors
                    .iter()
                    .any(|s| s.sector_id == *sector_id)
                {
                    anyhow::bail!("赛道ID已存在: {}", sector_id);
                }

                if *target_weight < 0.0 || *target_weight > 1.0 {
                    anyhow::bail!("目标占比必须在 0 到 1 之间。");
                }

                let valid_classes = vec!["equity", "bond", "crypto", "cash", "other"];
                if !valid_classes.contains(&asset_class.as_str()) {
                    anyhow::bail!("无效的资产类别。可选值: {:?}", valid_classes);
                }

                let new_sector = models::SectorConfig {
                    sector_id: sector_id.clone(),
                    name: name.clone(),
                    asset_class: asset_class.clone(),
                    target_weight: *target_weight,
                    priority: *priority,
                    enabled: true,
                };

                config_clone.sectors.push(new_sector);
                storage::save_config(&cli.config, &config_clone)?;
                println!("已成功添加赛道: {}", name);
            }
            SectorCommands::Disable { sector_id } => {
                let mut config_clone = config.clone();
                if let Some(sector) = config_clone
                    .sectors
                    .iter_mut()
                    .find(|s| s.sector_id == *sector_id)
                {
                    sector.enabled = false;
                    storage::save_config(&cli.config, &config_clone)?;
                    println!("已禁用赛道: {}", sector_id);
                } else {
                    println!("Sector not found: {}", sector_id);
                }
            }
            SectorCommands::Enable { sector_id } => {
                let mut config_clone = config.clone();
                if let Some(sector) = config_clone
                    .sectors
                    .iter_mut()
                    .find(|s| s.sector_id == *sector_id)
                {
                    sector.enabled = true;
                    storage::save_config(&cli.config, &config_clone)?;
                    println!("已启用赛道: {}", sector_id);
                } else {
                    println!("Sector not found: {}", sector_id);
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

                        if let Ok(nav) = fund_provider.fetch_latest_nav(fund_code) {
                            println!("最新净值: {:.4}", nav.nav);
                            if let Some(acc) = nav.accumulated_nav {
                                println!("累计净值: {:.4}", acc);
                            }
                            println!("净值日期: {}", nav.nav_date);
                        }
                        println!("数据来源: {}", info.source);
                        println!("数据状态: 正常");
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }
            cli::FundCommands::Validate => {
                println!(
                    "{:<20} | {:<10} | {:<20} | {:<20} | {:<10}",
                    "资产ID", "基金代码", "本地基金名称", "真实基金名称", "状态"
                );
                println!("{:-<100}", "");

                for asset in &config.assets {
                    if asset.fund_code.is_empty() {
                        continue;
                    }

                    let mut real_name = "N/A".to_string();
                    let mut status = "正常";

                    // Check for duplicate fund_code
                    let duplicates: Vec<&String> = config
                        .assets
                        .iter()
                        .filter(|a| {
                            a.fund_code == asset.fund_code
                                && a.asset_id != asset.asset_id
                                && a.enabled
                        })
                        .map(|a| &a.asset_id)
                        .collect();

                    if !duplicates.is_empty() && asset.enabled {
                        status = "重复基金代码";
                    }

                    match fund_provider.search_fund_by_code(&asset.fund_code) {
                        Ok(info) => {
                            real_name = info.fund_name.clone();
                            if status == "正常" && asset.fund_name != info.fund_name {
                                status = "名称不一致";
                            }
                            if info.source == "mock" && status == "正常" {
                                status = "使用模拟数据";
                            }
                        }
                        Err(_) => {
                            // Try cache
                            if cache.entries.iter().any(|e| e.fund_code == asset.fund_code) {
                                real_name = "Unknown (From Cache)".to_string();
                                if status == "正常" {
                                    status = "使用缓存";
                                }
                            } else if status == "正常" {
                                status = "查询失败";
                            }
                        }
                    }

                    println!(
                        "{:<20} | {:<10} | {:<20} | {:<20} | {:<10}",
                        asset.asset_id, asset.fund_code, asset.fund_name, real_name, status
                    );
                }
            }

            cli::FundCommands::SyncName { asset_id } => {
                let mut config_clone = config.clone();
                let fund_code = config_clone
                    .assets
                    .iter()
                    .find(|a| a.asset_id == *asset_id)
                    .map(|a| a.fund_code.clone());

                if let Some(code) = fund_code {
                    match fund_provider.search_fund_by_code(&code) {
                        Ok(info) => {
                            let asset = config_clone
                                .assets
                                .iter_mut()
                                .find(|a| a.asset_id == *asset_id)
                                .unwrap();
                            let old_name = asset.fund_name.clone();
                            asset.fund_name = info.fund_name.clone();
                            storage::save_config(&cli.config, &config_clone)?;
                            println!(
                                "已更新 {} ({}): {} -> {}",
                                asset_id, code, old_name, info.fund_name
                            );
                        }
                        Err(e) => {
                            println!("警告：基金 {} 名称获取失败: {}", code, e);
                        }
                    }
                } else {
                    println!("Error: Asset not found: {}", asset_id);
                }
            }
            cli::FundCommands::SyncAllNames => {
                let mut config_clone = config.clone();
                let mut updated_count = 0;

                for asset in config_clone.assets.iter_mut() {
                    if asset.fund_code.is_empty() {
                        continue;
                    }

                    match fund_provider.search_fund_by_code(&asset.fund_code) {
                        Ok(info) => {
                            if asset.fund_name != info.fund_name {
                                let old_name = asset.fund_name.clone();
                                asset.fund_name = info.fund_name.clone();
                                println!(
                                    "已更新 {} ({}): {} -> {}",
                                    asset.asset_id, asset.fund_code, old_name, info.fund_name
                                );
                                updated_count += 1;
                            }
                        }
                        Err(e) => {
                            println!(
                                "警告：基金 {} 名称获取失败，已跳过。({})",
                                asset.fund_code, e
                            );
                        }
                    }
                }

                if updated_count > 0 {
                    storage::save_config(&cli.config, &config_clone)?;
                    println!("共更新了 {} 个资产名称。", updated_count);
                } else {
                    println!("所有资产名称均已是最新，无需更新。");
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
                allow_duplicate_fund_code,
            } => {
                let mut config_clone = config.clone();
                if config_clone.assets.iter().any(|a| a.asset_id == *asset_id) {
                    anyhow::bail!("资产ID已存在: {}", asset_id);
                }

                if let Some(other) = config_clone
                    .assets
                    .iter()
                    .find(|a| a.fund_code == *fund_code && a.enabled)
                {
                    if !allow_duplicate_fund_code {
                        anyhow::bail!(
                            "错误：基金代码 {} 已被资产 {} 使用。\n如果你确认要让多个资产指向同一只基金，请添加 --allow-duplicate-fund-code。",
                            fund_code,
                            other.asset_id
                        );
                    } else {
                        println!(
                            "警告：多个资产正在使用同一个基金代码，可能导致重复统计或配置误解。"
                        );
                    }
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
            cli::AssetCommands::SetSector { asset_id, sector } => {
                let mut config_clone = config.clone();
                let asset = config_clone
                    .assets
                    .iter_mut()
                    .find(|a| a.asset_id == *asset_id);

                if let Some(asset) = asset {
                    if sector.trim().is_empty() {
                        anyhow::bail!("赛道名称不能为空。");
                    }

                    if !config_clone.sectors.iter().any(|s| s.name == *sector) {
                        println!("警告：赛道 “{}” 尚未在 sectors 中配置。", sector);
                    }

                    asset.sector = sector.clone();
                    storage::save_config(&cli.config, &config_clone)?;
                    println!("已更新资产 {} 的赛道为: {}", asset_id, sector);
                } else {
                    println!("Error: Asset not found: {}", asset_id);
                }
            }
            cli::AssetCommands::SetFundCode {
                asset_id,
                fund_code,
                keep_name,
                allow_duplicate_fund_code,
            } => {
                let mut config_clone = config.clone();

                if let Some(other) = config_clone
                    .assets
                    .iter()
                    .find(|a| a.fund_code == *fund_code && a.enabled && a.asset_id != *asset_id)
                {
                    if !allow_duplicate_fund_code {
                        anyhow::bail!(
                            "错误：基金代码 {} 已被资产 {} 使用。\n如果你确认要让多个资产指向同一只基金，请添加 --allow-duplicate-fund-code。",
                            fund_code,
                            other.asset_id
                        );
                    } else {
                        println!(
                            "警告：多个资产正在使用同一个基金代码，可能导致重复统计或配置误解。"
                        );
                    }
                }

                let asset = config_clone
                    .assets
                    .iter_mut()
                    .find(|a| a.asset_id == *asset_id);

                if let Some(asset) = asset {
                    match fund_provider.search_fund_by_code(fund_code) {
                        Ok(info) => {
                            let old_code = asset.fund_code.clone();
                            let old_name = asset.fund_name.clone();

                            asset.fund_code = fund_code.clone();
                            if !keep_name {
                                asset.fund_name = info.fund_name.clone();
                            }
                            storage::save_config(&cli.config, &config_clone)?;

                            println!("已更新资产 {} 的基金代码：", asset_id);
                            println!("旧代码: {}", old_code);
                            println!("新代码: {}", fund_code);
                            println!("旧名称: {}", old_name);
                            if !keep_name {
                                println!("新名称: {}", info.fund_name);
                            }

                            // Reset holding values if they belong to this asset
                            let mut state_clone = state.clone();
                            if let Some(holding) = state_clone
                                .asset_holdings
                                .iter_mut()
                                .find(|h| h.asset_id == *asset_id)
                            {
                                holding.fund_code = fund_code.clone();
                                holding.latest_nav = None;
                                holding.latest_nav_date = None;
                                holding.latest_nav_source = None;
                                holding.latest_nav_status = None;
                                holding.last_market_value = 0.0;
                                storage::save_state(&cli.state, &state_clone)?;
                                println!("\n注意：持有份额、持仓成本、现金和交易记录未被修改。");
                                println!("已重置持仓估值数据，请运行 mtm 更新。");
                                println!("请确认这是否符合你的真实持仓。");
                            }
                        }
                        Err(e) => {
                            println!("Error: 无法获取基金代码 {} 的信息: {}", fund_code, e);
                        }
                    }
                } else {
                    println!("Error: Asset not found: {}", asset_id);
                }
            }

            cli::AssetCommands::Rename {
                asset_id,
                fund_name,
            } => {
                let mut config_clone = config.clone();
                let fund_code = config_clone
                    .assets
                    .iter()
                    .find(|a| a.asset_id == *asset_id)
                    .map(|a| a.fund_code.clone());

                if let Some(code) = fund_code {
                    let asset = config_clone
                        .assets
                        .iter_mut()
                        .find(|a| a.asset_id == *asset_id)
                        .unwrap();
                    asset.fund_name = fund_name.clone();
                    storage::save_config(&cli.config, &config_clone)?;
                    println!("已更新资产 {} 的本地名称为: {}", asset_id, fund_name);

                    if let Ok(info) = fund_provider.search_fund_by_code(&code) {
                        if info.fund_name != *fund_name {
                            println!(
                                "警告：本地名称可能与真实基金名称不一致。可运行 fund validate 检查。"
                            );
                        }
                    }
                } else {
                    println!("Error: Asset not found: {}", asset_id);
                }
            }

            cli::AssetCommands::Validate => {
                println!(
                    "{:<20} | {:<10} | {:<20} | {:<20} | {:<10} | {}",
                    "资产ID", "基金代码", "基金名称", "赛道", "状态", "说明"
                );
                println!("{:-<120}", "");

                for asset in &config.assets {
                    let mut status = "正常";
                    let mut note = String::new();

                    if asset.asset_id.is_empty() {
                        status = "错误";
                        note.push_str("资产ID为空 ");
                    }
                    if asset.fund_code.is_empty() {
                        status = "错误";
                        note.push_str("基金代码为空 ");
                    }
                    if asset.fund_name.is_empty() {
                        status = "错误";
                        note.push_str("基金名称为空 ");
                    }
                    if asset.sector.is_empty() {
                        status = "错误";
                        note.push_str("赛道为空 ");
                    }

                    // Check valuation method
                    if asset.valuation_method != "nav" && asset.valuation_method != "proxy_index" {
                        status = "错误";
                        note.push_str("未知估值方法 ");
                    }

                    // Check duplicate fund_code
                    let duplicates: Vec<&String> = config
                        .assets
                        .iter()
                        .filter(|a| {
                            a.fund_code == asset.fund_code
                                && a.asset_id != asset.asset_id
                                && a.enabled
                        })
                        .map(|a| &a.asset_id)
                        .collect();

                    if !duplicates.is_empty() && asset.enabled {
                        status = "重复基金代码";
                        let ids: Vec<String> = duplicates.iter().map(|s| s.to_string()).collect();
                        note.push_str(&format!("同代码资产: {} ", ids.join(", ")));
                    } else if !duplicates.is_empty() && !asset.enabled {
                        note.push_str("警告：此禁用资产的基金代码被其他启用资产重复 ");
                    }

                    // Check sector existence
                    if !config.sectors.iter().any(|s| s.name == asset.sector) {
                        if status == "正常" {
                            status = "赛道未配置";
                        }
                        note.push_str(&format!("赛道 “{}” 未定义 ", asset.sector));
                    }

                    // Check holding
                    let has_holding = state
                        .asset_holdings
                        .iter()
                        .any(|h| h.asset_id == asset.asset_id);
                    if !has_holding && asset.enabled {
                        status = "持仓缺失";
                        note.push_str("启用资产但在 state 中无对应持仓 ");
                    }

                    // Check fund provider consistency
                    if !asset.fund_code.is_empty() {
                        match fund_provider.search_fund_by_code(&asset.fund_code) {
                            Ok(info) => {
                                if asset.fund_name != info.fund_name {
                                    if status == "正常" {
                                        status = "名称不一致";
                                    }
                                    note.push_str(&format!("真实名称: {} ", info.fund_name));
                                }
                            }
                            Err(_) => {
                                if status == "正常" {
                                    status = "基金查询失败";
                                }
                                note.push_str("无法从 provider 获取基金信息 ");
                            }
                        }
                    }

                    if !asset.enabled && status == "正常" {
                        status = "已禁用";
                    }

                    println!(
                        "{:<20} | {:<10} | {:<20} | {:<20} | {:<10} | {}",
                        asset.asset_id,
                        asset.fund_code,
                        asset.fund_name,
                        asset.sector,
                        status,
                        note
                    );
                }

                // Check for orphan holdings
                for holding in &state.asset_holdings {
                    if !config.assets.iter().any(|a| a.asset_id == holding.asset_id) {
                        println!(
                            "{:<20} | {:<10} | {:<20} | {:<20} | {:<10} | {}",
                            holding.asset_id,
                            holding.fund_code,
                            "N/A",
                            "N/A",
                            "孤立持仓",
                            "config 中无此资产"
                        );
                    }
                }
            }
            cli::AssetCommands::RepairHoldings => {
                let mut state_clone = state.clone();
                let mut repaired_count = 0;

                for asset in &config.assets {
                    if !state_clone
                        .asset_holdings
                        .iter()
                        .any(|h| h.asset_id == asset.asset_id)
                    {
                        println!("正在为资产 {} 创建缺失的持仓记录...", asset.asset_id);
                        state_clone.asset_holdings.push(models::AssetHolding {
                            asset_id: asset.asset_id.clone(),
                            fund_code: asset.fund_code.clone(),
                            units: 0.0,
                            units_estimated: false,
                            cost_basis: 0.0,
                            latest_nav: None,
                            latest_nav_date: None,
                            latest_nav_source: None,
                            latest_nav_status: None,
                            last_market_value: 0.0,
                        });
                        repaired_count += 1;
                    }
                }

                if repaired_count > 0 {
                    storage::save_state(&cli.state, &state_clone)?;
                    println!("共修复了 {} 个缺失持仓。", repaired_count);
                } else {
                    println!("未发现缺失持仓，无需修复。");
                }
            }
            cli::AssetCommands::Duplicates => {
                use std::collections::HashMap;
                let mut groups: HashMap<String, Vec<String>> = HashMap::new();
                for asset in &config.assets {
                    if !asset.fund_code.is_empty() {
                        groups
                            .entry(asset.fund_code.clone())
                            .or_default()
                            .push(asset.asset_id.clone());
                    }
                }

                let mut found = false;
                println!("{:<10} | {}", "基金代码", "资产ID");
                println!("{:-<40}", "");
                for (code, ids) in groups {
                    if ids.len() > 1 {
                        println!("{:<10} | {}", code, ids.join(", "));
                        found = true;
                    }
                }

                if !found {
                    println!("未发现重复基金代码。");
                }
            }
        },
        Commands::Config { command } => match command {
            cli::ConfigCommands::Doctor => {
                println!("正在进行配置健康检查...\n");

                #[derive(Debug)]
                struct Finding {
                    level: &'static str,
                    category: &'static str,
                    object: String,
                    description: String,
                    suggestion: String,
                }

                let mut findings = Vec::new();

                // 1. Data files exist
                if !Path::new(&cli.config).exists() {
                    findings.push(Finding {
                        level: "error",
                        category: "文件",
                        object: cli.config.clone(),
                        description: "配置文件不存在".to_string(),
                        suggestion: "请确保 data/config.toml 存在".to_string(),
                    });
                }
                if cli.config.contains("examples/") {
                    findings.push(Finding {
                        level: "warning",
                        category: "环境",
                        object: cli.config.clone(),
                        description: "正在直接使用示例配置文件".to_string(),
                        suggestion: "建议将示例文件拷贝到 data/ 目录下使用".to_string(),
                    });
                }

                // 2. Duplicate fund_code
                use std::collections::HashMap;
                let mut fund_code_map: HashMap<String, Vec<String>> = HashMap::new();
                for asset in &config.assets {
                    if !asset.fund_code.is_empty() && asset.enabled {
                        fund_code_map
                            .entry(asset.fund_code.clone())
                            .or_default()
                            .push(asset.asset_id.clone());
                    }
                }
                for (code, ids) in fund_code_map {
                    if ids.len() > 1 {
                        findings.push(Finding {
                            level: "error",
                            category: "重复代码",
                            object: format!("基金代码 {}", code),
                            description: format!("被多个启用资产使用: {}", ids.join(", ")),
                            suggestion: "建议合并资产或为不同资产指定正确的基金代码".to_string(),
                        });
                    }
                }

                // 3. Sector target weight sum
                let enabled_weight_sum: f64 = config
                    .sectors
                    .iter()
                    .filter(|s| s.enabled)
                    .map(|s| s.target_weight)
                    .sum();
                if (enabled_weight_sum - 1.0).abs() > 0.001 {
                    findings.push(Finding {
                        level: "warning",
                        category: "权重",
                        object: "赛道配置".to_string(),
                        description: format!(
                            "启用赛道的目标权重总和为 {:.2}，不等于 1.0",
                            enabled_weight_sum
                        ),
                        suggestion: "请调整各赛道的 target_weight".to_string(),
                    });
                }

                // 4. Asset checks
                for asset in &config.assets {
                    // Unknown sector
                    if !config.sectors.iter().any(|s| s.name == asset.sector) {
                        findings.push(Finding {
                            level: "error",
                            category: "赛道",
                            object: asset.asset_id.clone(),
                            description: format!("使用了未定义的赛道 “{}”", asset.sector),
                            suggestion:
                                "使用 sector add 添加赛道或 asset set-sector 修改资产所属赛道"
                                    .to_string(),
                        });
                    }

                    // Disabled sector with active asset
                    if let Some(sector) = config.sectors.iter().find(|s| s.name == asset.sector) {
                        if !sector.enabled && asset.enabled {
                            findings.push(Finding {
                                level: "warning",
                                category: "赛道",
                                object: asset.asset_id.clone(),
                                description: format!(
                                    "所属赛道 “{}” 已禁用，但资产仍启用",
                                    sector.name
                                ),
                                suggestion: "启用赛道或禁用该资产".to_string(),
                            });
                        }

                        // Target weight = 0 but current value > 0
                        if sector.target_weight == 0.0 && asset.enabled {
                            let holding = state
                                .asset_holdings
                                .iter()
                                .find(|h| h.asset_id == asset.asset_id);
                            if let Some(h) = holding {
                                if h.last_market_value > 0.0 {
                                    findings.push(Finding {
                                        level: "info",
                                        category: "配置",
                                        object: asset.asset_id.clone(),
                                        description: format!("所属赛道权重为 0，但资产仍有市值"),
                                        suggestion: "确认是否需要调整赛道目标权重".to_string(),
                                    });
                                }
                            }
                        }
                    }

                    // Missing holding
                    if asset.enabled
                        && !state
                            .asset_holdings
                            .iter()
                            .any(|h| h.asset_id == asset.asset_id)
                    {
                        findings.push(Finding {
                            level: "error",
                            category: "持仓",
                            object: asset.asset_id.clone(),
                            description: "启用资产但在 state 文件中无持仓记录".to_string(),
                            suggestion: "运行 asset repair-holdings 修复".to_string(),
                        });
                    }

                    // Invalid valuation method
                    if asset.valuation_method != "nav" && asset.valuation_method != "proxy_index" {
                        findings.push(Finding {
                            level: "error",
                            category: "估值",
                            object: asset.asset_id.clone(),
                            description: format!("无效的估值方法: {}", asset.valuation_method),
                            suggestion: "修改为 'nav'".to_string(),
                        });
                    }
                }

                // 5. Orphan holdings
                for holding in &state.asset_holdings {
                    if !config.assets.iter().any(|a| a.asset_id == holding.asset_id) {
                        findings.push(Finding {
                            level: "warning",
                            category: "持仓",
                            object: holding.asset_id.clone(),
                            description: "在 state 中有持仓记录，但在 config 中未定义".to_string(),
                            suggestion: "建议在 config 中添加该资产或手动清理 state 文件"
                                .to_string(),
                        });
                    }
                }

                // Output report
                if findings.is_empty() {
                    println!("状态: 正常");
                    println!("未发现配置问题。");
                } else {
                    let errors = findings.iter().filter(|f| f.level == "error").count();
                    let warnings = findings.iter().filter(|f| f.level == "warning").count();

                    if errors > 0 {
                        println!("状态: 有错误");
                    } else if warnings > 0 {
                        println!("状态: 有警告");
                    } else {
                        println!("状态: 正常");
                    }

                    println!(
                        "{:<10} | {:<10} | {:<20} | {:<40} | {}",
                        "等级", "类型", "对象", "说明", "建议操作"
                    );
                    println!("{:-<120}", "");

                    for f in findings {
                        println!(
                            "{:<10} | {:<10} | {:<20} | {:<40} | {}",
                            f.level, f.category, f.object, f.description, f.suggestion
                        );
                    }
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

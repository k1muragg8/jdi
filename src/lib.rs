#![allow(clippy::collapsible_if)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::print_literal)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::let_and_return)]
#![allow(clippy::unnecessary_cast)]

pub mod api;
pub mod cli;
pub mod db;
pub mod engine;
pub mod error;
pub mod models;
pub mod repository;
pub mod storage;
pub mod web;

use anyhow::{Context, Result, anyhow};
use api::{FxProvider, MarketDataProvider};
use chrono::Local;
use clap::Parser;
use cli::{
    CashCommands, Cli, Commands, DecisionCommands, ExpenseCommands, PortfolioCommands,
    SectorCommands, TxAddCommands, TxCommands, TxImportCommands,
};
use models::Transaction;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Resolve the canonical data directory for runtime files (caches, DBs/JSONs, snapshots, reports, downloads etc).
/// Always uses project root/data (detected via CARGO_MANIFEST_DIR at build or by walking up from current_exe
/// to the nearest dir containing Cargo.toml + src/). This ensures data is NEVER written under target/debug,
/// target/release, or next to the executable, even if cwd is inside target/ or the binary is invoked via
/// full path from a different shell cwd.
/// Falls back to <cwd>/data only if no project root detectable (e.g. fully installed binary outside tree).
/// User can override specific paths via CLI --config etc (those are used as-is if not matching the default "data/..." string).
pub fn resolve_data_dir() -> PathBuf {
    // 1. CARGO_MANIFEST_DIR is set by cargo at compile time for cargo (run|test|check). Points to source root.
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let root = PathBuf::from(manifest_dir);
        if root.join("Cargo.toml").is_file() {
            let d = root.join("data");
            return if d.exists() {
                d.canonicalize().unwrap_or(d)
            } else {
                d
            };
        }
    }

    // 2. Walk up from executable's directory. If exe lives in .../target/debug/foo, parent walk finds project root.
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        for _ in 0..10 {
            if dir.join("Cargo.toml").is_file() && dir.join("src").is_dir() {
                let d = dir.join("data");
                return if d.exists() {
                    d.canonicalize().unwrap_or(d)
                } else {
                    d
                };
            }
            if let Some(p) = dir.parent() {
                dir = p.to_path_buf();
            } else {
                break;
            }
        }
    }

    // 3. Fallback to cwd/data, but try one more time to escape target/ if cwd is polluted.
    let mut cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cwd_str = cwd.to_string_lossy();
    if cwd_str.contains("target/") || cwd_str.contains("target\\") {
        for _ in 0..6 {
            if cwd.join("Cargo.toml").is_file() {
                return cwd.join("data");
            }
            if let Some(p) = cwd.parent() {
                cwd = p.to_path_buf();
            } else {
                break;
            }
        }
    }
    cwd.join("data")
}

fn ensure_data_dir() -> Result<PathBuf> {
    let data_dir = resolve_data_dir();
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).context("Failed to create data/ directory")?;
    }

    let project_root = data_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let examples = vec![
        (
            project_root.join("examples/config.toml"),
            data_dir.join("config.toml"),
        ),
        (
            project_root.join("examples/portfolio_state.json"),
            data_dir.join("portfolio_state.json"),
        ),
        (
            project_root.join("examples/transactions.json"),
            data_dir.join("transactions.json"),
        ),
    ];

    for (src, dest) in examples {
        if !dest.exists() && src.exists() {
            fs::copy(&src, &dest).context(format!(
                "Failed to copy {} to {}",
                src.display(),
                dest.display()
            ))?;
        }
    }

    Ok(data_dir)
}

fn apply_resolved_data_defaults(cli: &mut Cli) {
    let base = resolve_data_dir();
    fn remap(current: &str, rel: &str, base: &Path) -> String {
        if current == format!("data/{}", rel) {
            base.join(rel).to_string_lossy().to_string()
        } else {
            current.to_string()
        }
    }
    cli.config = remap(&cli.config, "config.toml", &base);
    cli.state = remap(&cli.state, "portfolio_state.json", &base);
    cli.transactions = remap(&cli.transactions, "transactions.json", &base);
    cli.cache = remap(&cli.cache, "fund_nav_cache.json", &base);
    cli.market_cache = remap(&cli.market_cache, "market_price_cache.json", &base);
    cli.fx_cache = remap(&cli.fx_cache, "fx_usd_cnh_cache.json", &base);
    cli.dca_plans = remap(&cli.dca_plans, "dca_plans.json", &base);
    cli.alipay_snapshots = remap(&cli.alipay_snapshots, "alipay_snapshots.json", &base);
    cli.instruments = remap(&cli.instruments, "instruments.toml", &base);
    cli.dca_settlements = remap(&cli.dca_settlements, "dca_settlements.json", &base);
    cli.reconciliation_audit = remap(
        &cli.reconciliation_audit,
        "reconciliation_audit.json",
        &base,
    );
    cli.dca_settlement_audit = remap(
        &cli.dca_settlement_audit,
        "dca_settlement_audit.json",
        &base,
    );
    cli.cache_status = remap(&cli.cache_status, "cache_status.json", &base);
    cli.instrument_cache = remap(&cli.instrument_cache, "instrument_quote_cache.json", &base);
    cli.risk_cache = remap(&cli.risk_cache, "risk_cache.json", &base);
    cli.proxy_cache = remap(&cli.proxy_cache, "proxy_valuation_cache.json", &base);
    cli.regime_cache = remap(&cli.regime_cache, "regime_cache.json", &base);
    cli.web_audit = remap(&cli.web_audit, "web_admin_audit.json", &base);
    cli.operation_policy = remap(&cli.operation_policy, "operation_policy.json", &base);
    cli.operation_status = remap(&cli.operation_status, "operation_status.json", &base);
    cli.daily_operation_report = remap(
        &cli.daily_operation_report,
        "daily_operation_report.json",
        &base,
    );
    if cli.web_jobs_dir == "data/jobs" {
        cli.web_jobs_dir = base.join("jobs").to_string_lossy().to_string();
    }
}

pub fn run() -> Result<()> {
    dotenvy::dotenv().ok();
    let _data_dir = ensure_data_dir()?;
    let mut cli = Cli::parse();
    apply_resolved_data_defaults(&mut cli);
    let mut ctx = repository::RepositoryContext::default();
    let rt = tokio::runtime::Runtime::new()?;

    // Load config manually to determine storage backend
    let config_path = cli.config.clone();
    let config: models::ConfigRoot = rt.block_on(async {
        tokio::task::spawn_blocking(move || storage::load_config(config_path)).await?
    })?;
    config
        .validate()
        .context("Configuration validation failed")?;

    ctx.storage_mode = match config.storage.backend {
        models::StorageBackend::Json => repository::StorageMode::Json,
        models::StorageBackend::Postgres => repository::StorageMode::Postgres,
    };

    // Create repository using factory
    let repo: Arc<dyn repository::Repository> =
        rt.block_on(repository::RepositoryFactory::from_config(&config, &cli))?;

    // Resolve portfolio if specified
    if let Some(p_id_or_name) = &cli.portfolio {
        let p = rt.block_on(repo.get_portfolio(&ctx, p_id_or_name))?;
        if let Some(portfolio) = p {
            ctx.portfolio_id = portfolio.id;
        } else {
            anyhow::bail!("Portfolio '{}' not found.", p_id_or_name);
        }
    }

    let mut state: models::PortfolioState = rt.block_on(repo.load_state(&ctx))?;
    let mut transactions: Vec<models::Transaction> = rt.block_on(repo.load_transactions(&ctx))?;

    // Use repo for caches consistently
    let mut cache = rt.block_on(repo.load_nav_cache(&ctx))?;
    let mut market_cache = rt.block_on(repo.load_market_cache(&ctx))?;
    let mut fx_cache = rt.block_on(repo.load_fx_cache(&ctx))?;

    let fund_provider = api::create_fund_provider(&config.api);
    let fx_provider = api::create_fx_provider(&config.fx, None);

    let (risk_overlay, regimes) = match &cli.command {
        Commands::Decision {
            command:
                DecisionCommands::AdjustedPreview
                | DecisionCommands::Compare
                | DecisionCommands::Explain { .. },
        } => {
            let instruments = rt.block_on(repo.load_instruments(&ctx)).unwrap_or_default();
            let config = config.clone();
            let _ctx = ctx.clone();
            rt.block_on(async move {
                tokio::task::spawn_blocking(move || {
                    let fx_provider = api::create_fx_provider(&config.fx, None);
                    let market_provider =
                        api::create_market_provider(&config.market, Some("yahoo"));
                    let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
                        &config.risk,
                        &config.regime,
                        market_provider.as_ref(),
                        fx_provider.as_ref(),
                    );

                    let mut regimes = std::collections::HashMap::new();
                    for asset in &config.assets {
                        let symbol_opt = if let Some(rid) = &asset.reference_instrument_id {
                            instruments
                                .iter()
                                .find(|i| i.instrument_id == *rid)
                                .map(|i| i.provider_symbol.clone())
                        } else {
                            asset
                                .reference_instrument_symbol
                                .clone()
                                .or_else(|| asset.reference_index_symbol.clone())
                        };

                        if let Some(s) = symbol_opt {
                            if let Ok(candles) = market_provider
                                .fetch_daily_candles(&s, config.regime.default_lookback_days)
                            {
                                let regime = engine::regime::calculate_market_regime(
                                    &s,
                                    &candles,
                                    &config.regime,
                                );
                                regimes.insert(asset.asset_id.clone(), regime);
                            }
                        }
                    }
                    (risk_overlay, regimes)
                })
                .await
            })?
        }
        _ => (
            models::GlobalRiskOverlay::default(),
            std::collections::HashMap::new(),
        ),
    };

    let generate_tx_id = || format!("tx_{}", Local::now().format("%Y-%m-%d_%H%M%S"));

    rt.block_on(async {
        match &cli.command {
            Commands::Backtest { command } => {
                let req = match command {
                    cli::BacktestCommands::Run {
                        start,
                        end,
                        initial_cash,
                        baseline,
                    } => models::BacktestRequest {
                        start_date: start.clone(),
                        end_date: end.clone(),
                        initial_cash: *initial_cash,
                        portfolio_id: ctx.portfolio_id.clone(),
                        policy_override: None,
                        include_baseline: *baseline,
                    },
                    cli::BacktestCommands::Compare { start, end } => models::BacktestRequest {
                        start_date: start.clone(),
                        end_date: end.clone(),
                        initial_cash: 100000.0,
                        portfolio_id: ctx.portfolio_id.clone(),
                        policy_override: None,
                        include_baseline: true,
                    },
                };

                println!(
                    "正在启动回测仿真 ({} 至 {})...",
                    req.start_date, req.end_date
                );
                let report = engine::run_backtest(repo.as_ref(), &ctx, &config, req).await?;

                println!("\n=== 回测汇总报告 ===");
                println!(
                    "周期: {} 至 {}",
                    report.request.start_date, report.request.end_date
                );
                println!("初始资金: {:.2}", report.request.initial_cash);
                println!("最终估值: {:.2}", report.main_metrics.final_value);
                println!("总投入:   {:.2}", report.main_metrics.total_invested);
                println!("现金余额: {:.2}", report.main_metrics.cash_remaining);
                println!("买入天数: {} 天", report.main_metrics.total_buy_days);
                println!(
                    "最大回撤: {:.2}%",
                    report.main_metrics.max_drawdown * 100.0
                );

                if let Some(baseline) = &report.baseline_metrics {
                    println!("\n=== 与固定定投 (Baseline) 对比 ===");
                    println!("策略最终价值: {:.2}", report.main_metrics.final_value);
                    println!("基准最终价值: {:.2}", baseline.final_value);
                    let diff = report.main_metrics.final_value - baseline.final_value;
                    let pct = (diff / baseline.final_value.max(1.0)) * 100.0;
                    println!("超额收益:     {:.2} ({:+.2}%)", diff, pct);
                }

                if !report.warnings.is_empty() {
                    println!("\n⚠️  回测警告 ({} 条):", report.warnings.len());
                    for w in report.warnings.iter().take(5) {
                        println!(" - {}", w.message);
                    }
                }
            }
            Commands::Portfolio { command } => {
                match command {
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
                        Ok::<(), anyhow::Error>(())
                    }
                    PortfolioCommands::List => {
                        let portfolios = repo.list_portfolios(&ctx).await?;
                        println!("Available Portfolios:\n");
                        println!(
                            "{:<12} | {:<25} | {:>15} | {:<20}",
                            "ID", "Name", "Cash Balance", "Created At"
                        );
                        println!("{}", "-".repeat(80));
                        for p in &portfolios {
                            let current = if p.id == ctx.portfolio_id { "*" } else { " " };
                            println!(
                                "{}{:<11} | {:<25} | {:>15.2} | {:<20}",
                                current, p.id, p.name, p.current_cash, p.created_at
                            );
                        }
                        println!("\nTotal: {} portfolio(s)", portfolios.len());
                        Ok::<(), anyhow::Error>(())
                    }
                    PortfolioCommands::Create { name } => {
                        let p = repo.create_portfolio(&ctx, name).await?;
                        println!("Successfully created portfolio:");
                        println!("  - ID:   {}", p.id);
                        println!("  - Name: {}", p.name);
                        Ok::<(), anyhow::Error>(())
                    }
                    PortfolioCommands::Show { id_or_name } => {
                        let target = id_or_name.as_deref().unwrap_or(&ctx.portfolio_id);
                        let p = repo.get_portfolio(&ctx, target).await?;
                        if let Some(portfolio) = p {
                            println!("Portfolio Details:\n");
                            println!("  - ID:            {}", portfolio.id);
                            println!("  - Name:          {}", portfolio.name);
                            println!("  - Owner:         {}", portfolio.owner_user_id);
                            println!("  - Cash Balance:  {:.2}", portfolio.current_cash);
                            println!("  - Created At:    {}", portfolio.created_at);
                            println!("  - Updated At:    {}", portfolio.updated_at);
                            if let Some(desc) = portfolio.description {
                                println!("  - Description:   {}", desc);
                            }
                            Ok::<(), anyhow::Error>(())
                        } else {
                            anyhow::bail!("Portfolio '{}' not found.", target);
                        }
                    }
                }?;
            }
            Commands::Holdings { all, proxy } => {

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

            let proxy_results = if *proxy {
                let config = config.clone();
                let state = state.clone();
                Some(tokio::task::spawn_blocking(move || {
                    let market_provider = api::create_market_provider(&config.market, None);
                    let fx_provider = api::create_fx_provider(&config.fx, None);
                    engine::calculate_proxy_valuations(
                        &config,
                        &state,
                        market_provider.as_ref(),
                        fx_provider.as_ref(),
                    )
                })
                .await?)
            } else {
                None
            };

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

                if let Some(results) = &proxy_results {
                    if let Some(res) = results.iter().find(|r| r.asset_id == holding.asset_id) {
                        if res.status == "正常" {
                            let proxy_pnl = res.estimated_market_value - cost;
                            println!(
                                "{:<20} | {:<10} | {:<20} | {:<10} | {:<15} | {:<10.4} | {:<12} | {:<10} | {:<10} | {:<10.2} (估算)",
                                "",
                                "",
                                "",
                                "",
                                "",
                                res.estimated_nav,
                                res.reference_latest_date,
                                res.data_source,
                                "估算",
                                proxy_pnl
                            );
                        }
                    }
                }
            }
        }
        Commands::Mtm => {
            let config_clone = config.clone();
            let mut state_clone = state.clone();
            let mut cache_clone = cache.clone();

            (state_clone, cache_clone) = tokio::task::spawn_blocking(move || {
                let fund_provider = api::create_fund_provider(&config_clone.api);
                engine::mark_to_market(
                    &config_clone,
                    &mut state_clone,
                    fund_provider.as_ref(),
                    &mut cache_clone,
                )?;
                Ok::<(models::PortfolioState, models::NavCache), anyhow::Error>((
                    state_clone,
                    cache_clone,
                ))
            })
            .await??;

            state = state_clone;
            cache = cache_clone;

            repo.save_state(&ctx, &state).await?;
            repo.save_nav_cache(&ctx, &cache).await?;
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
                    "{:<20} | {:<12} | {:<10} | {:<15} | {:<10} | {:<10} | {:<10} | {:<5} | {:<10} | Note",
                    "ID",
                    "Date",
                    "Type",
                    "Asset ID",
                    "Amount",
                    "Units",
                    "Price",
                    "Fee",
                    "Currency"
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
            TxCommands::Show { id } => {
                let transactions = repo.load_transactions(&ctx).await?;
                if let Some(tx) = transactions.iter().find(|t| t.id == *id) {
                    println!("{}", serde_json::to_string_pretty(tx)?);
                } else {
                    anyhow::bail!("Transaction {} not found", id);
                }
            }
            TxCommands::Delete { id } => {
                repo.delete_transaction(&ctx, id).await?;
                println!("Transaction {} deleted successfully.", id);

                // Rebuild state
                let transactions = repo.load_transactions(&ctx).await?;
                let old_state = repo.load_state(&ctx).await.unwrap_or_default();
                let mut new_state = engine::holdings::rebuild_holdings_from_transactions(&transactions)?;

                // Preserve valuations
                for new_holding in &mut new_state.asset_holdings {
                    if let Some(old_holding) = old_state.asset_holdings.iter().find(|h| h.asset_id == new_holding.asset_id) {
                        new_holding.latest_nav = old_holding.latest_nav;
                        new_holding.latest_nav_date = old_holding.latest_nav_date.clone();
                        new_holding.latest_nav_source = old_holding.latest_nav_source.clone();
                        new_holding.latest_nav_status = old_holding.latest_nav_status.clone();
                        if let Some(nav) = new_holding.latest_nav {
                            new_holding.last_market_value = new_holding.units * nav;
                        }
                    }
                }
                repo.save_state(&ctx, &new_state).await?;
                println!("Holdings state successfully rebuilt from transactions.");
            }
            TxCommands::Update {
                id,
                date,
                transaction_type,
                asset_id,
                amount,
                price,
                units,
                fee,
                currency,
                note,
                source,
                raw_description,
            } => {
                let transactions = repo.load_transactions(&ctx).await?;
                let mut target_tx = None;
                for t in &transactions {
                    if t.id == *id {
                        target_tx = Some(t.clone());
                        break;
                    }
                }

                if let Some(mut tx) = target_tx {
                    if let Some(v) = date { tx.date = v.clone(); }
                    if let Some(v) = transaction_type { tx.transaction_type = v.clone(); }
                    if let Some(v) = asset_id { tx.asset_id = Some(v.clone()); }
                    if let Some(v) = amount { tx.amount = *v; }
                    if let Some(v) = price { tx.price = Some(*v); }
                    if let Some(v) = units { tx.units = Some(*v); }
                    if let Some(v) = fee { tx.fee = *v; }
                    if let Some(v) = currency { tx.currency = v.clone(); }
                    if let Some(v) = note { tx.note = v.clone(); }
                    if let Some(v) = source { tx.source = v.clone(); }
                    if let Some(v) = raw_description { tx.raw_description = v.clone(); }

                    repo.update_transaction(&ctx, &tx).await?;
                    println!("Transaction {} updated successfully.", id);

                    // Rebuild state
                    let updated_transactions = repo.load_transactions(&ctx).await?;
                    let old_state = repo.load_state(&ctx).await.unwrap_or_default();
                    let mut new_state = engine::holdings::rebuild_holdings_from_transactions(&updated_transactions)?;

                    // Preserve valuations
                    for new_holding in &mut new_state.asset_holdings {
                        if let Some(old_holding) = old_state.asset_holdings.iter().find(|h| h.asset_id == new_holding.asset_id) {
                            new_holding.latest_nav = old_holding.latest_nav;
                            new_holding.latest_nav_date = old_holding.latest_nav_date.clone();
                            new_holding.latest_nav_source = old_holding.latest_nav_source.clone();
                            new_holding.latest_nav_status = old_holding.latest_nav_status.clone();
                            if let Some(nav) = new_holding.latest_nav {
                                new_holding.last_market_value = new_holding.units * nav;
                            }
                        }
                    }
                    repo.save_state(&ctx, &new_state).await?;
                    println!("Holdings state successfully rebuilt from transactions.");
                } else {
                    anyhow::bail!("Transaction {} not found", id);
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
                        source: "manual".to_string(),
                        raw_description: "".to_string(),
                    };

                    engine::holdings::apply_transaction(&mut state, &tx)?;
                    transactions.push(tx);

                    repo.save_state(&ctx, &state).await?;
                    repo.save_transactions(&ctx, &transactions).await?;

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
                        source: "manual".to_string(),
                        raw_description: "".to_string(),
                    };

                    engine::holdings::apply_transaction(&mut state, &tx)?;
                    transactions.push(tx);

                    repo.save_state(&ctx, &state).await?;
                    repo.save_transactions(&ctx, &transactions).await?;

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
            TxCommands::Import { command } => match command {
                TxImportCommands::Preview { file, format } => {
                    if format != "csv" {
                        anyhow::bail!("Only 'csv' format is supported currently");
                    }
                    let content = fs::read_to_string(file)?;
                    let candidates = engine::import::parse_transactions_from_csv(&content)?;
                    let preview = engine::import::preview_import(candidates, &transactions);
                    engine::import::print_preview_summary(&preview);
                }
                TxImportCommands::Commit {
                    file,
                    format,
                    skip_duplicates,
                } => {
                    if format != "csv" {
                        anyhow::bail!("Only 'csv' format is supported currently");
                    }
                    let content = fs::read_to_string(file)?;
                    let candidates = engine::import::parse_transactions_from_csv(&content)?;
                    let preview = engine::import::preview_import(candidates, &transactions);

                    if preview.summary.error_rows > 0 {
                        engine::import::print_preview_summary(&preview);
                        anyhow::bail!("Import rejected: file contains errors. Please fix them before committing.");
                    }

                    let result = engine::import::commit_import(
                        &preview,
                        &mut state,
                        &mut transactions,
                        *skip_duplicates,
                    );

                    repo.save_state(&ctx, &state).await?;
                    repo.save_transactions(&ctx, &transactions).await?;

                    println!("\n{}", result.message);
                }
            },
        },
        Commands::Report { command } => {
            run_report_command(&*repo, &ctx, &cli, command).await?;
        }
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
                    source: "manual".to_string(),
                    raw_description: "".to_string(),
                };
                engine::holdings::apply_transaction(&mut state, &tx)?;
                transactions.push(tx);
                repo.save_state(&ctx, &state).await?;
                repo.save_transactions(&ctx, &transactions).await?;
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
                    source: "manual".to_string(),
                    raw_description: "".to_string(),
                };
                engine::holdings::apply_transaction(&mut state, &tx)?;
                transactions.push(tx);
                repo.save_state(&ctx, &state).await?;
                repo.save_transactions(&ctx, &transactions).await?;
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
                    source: "manual".to_string(),
                    raw_description: "".to_string(),
                };
                engine::holdings::apply_transaction(&mut state, &tx)?;
                transactions.push(tx);
                repo.save_state(&ctx, &state).await?;
                repo.save_transactions(&ctx, &transactions).await?;
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
                    source: "manual".to_string(),
                    raw_description: "".to_string(),
                };
                engine::holdings::apply_transaction(&mut state, &tx)?;
                transactions.push(tx);
                repo.save_state(&ctx, &state).await?;
                repo.save_transactions(&ctx, &transactions).await?;
                println!("Expense recorded. New balance: {:.2}", state.cash);
            }
        },
        Commands::Valuation { command } => match command {
            cli::ValuationCommands::ProxyPreview => {
                let results = {
                    let config = config.clone();
                    let state = state.clone();
                    tokio::task::spawn_blocking(move || {
                        let market_provider = api::create_market_provider(&config.market, None);
                        let fx_provider = api::create_fx_provider(&config.fx, None);
                        engine::calculate_proxy_valuations(
                            &config,
                            &state,
                            market_provider.as_ref(),
                            fx_provider.as_ref(),
                        )
                    })
                    .await?
                };

                println!("估算净值预览\n");
                println!(
                    "{:<15} | {:<20} | {:<8} | {:<12} | {:<8} | {:<8} | {:<8} | {:<6} | {:<8} | {:<12} | 状态",
                    "资产ID",
                    "基金名称",
                    "官方净值",
                    "净值日期",
                    "指数涨跌",
                    "汇率涨跌",
                    "综合涨跌",
                    "汇率调",
                    "估算净值",
                    "估算市值"
                );
                println!("{:-<150}", "");

                for res in results {
                    let index_return_pct = format!("{:.2}%", res.index_return * 100.0);

                    let fx_return_pct = if res.use_fx_adjustment
                        && (res.status.contains("汇率")
                            || res.warning.as_ref().is_some_and(|w| w.contains("汇率")))
                        {
                        if res.fx_return.abs() < 0.000001 {
                            "N/A".to_string()
                        } else {
                            format!("{:.2}%", res.fx_return * 100.0)
                        }
                    } else {
                        format!("{:.2}%", res.fx_return * 100.0)
                    };

                    let combined_return_pct = format!("{:.2}%", res.combined_proxy_return * 100.0);
                    let fx_adj_str = if res.use_fx_adjustment { "是" } else { "否" };

                    println!(
                        "{:<15} | {:<20} | {:<8.4} | {:<12} | {:<8} | {:<8} | {:<8} | {:<6} | {:<8.4} | {:<12.2} | {}",
                        res.asset_id,
                        res.fund_name,
                        res.official_nav,
                        res.official_nav_date,
                        index_return_pct,
                        fx_return_pct,
                        combined_return_pct,
                        fx_adj_str,
                        res.estimated_nav,
                        res.estimated_market_value,
                        res.status
                    );
                    if let Some(w) = &res.warning {
                        println!("  └─ 警告: {}", w);
                    }
                }
            }
            cli::ValuationCommands::ProxyExplain { asset_id } => {
                let results = {
                    let config = config.clone();
                    let state = state.clone();
                    tokio::task::spawn_blocking(move || {
                        let market_provider = api::create_market_provider(&config.market, None);
                        let fx_provider = api::create_fx_provider(&config.fx, None);
                        engine::calculate_proxy_valuations(
                            &config,
                            &state,
                            market_provider.as_ref(),
                            fx_provider.as_ref(),
                        )
                    })
                    .await?
                };

                if let Some(res) = results.iter().find(|r| r.asset_id == *asset_id) {
                    if res.status != "正常" {
                        println!(
                            "无法为资产 {} 提供详细说明，状态为: {}",
                            asset_id, res.status
                        );
                        if let Some(w) = &res.warning {
                            println!("说明: {}", w);
                        }
                    } else {
                        println!("资产 {} 的估算逻辑：\n", asset_id);
                        println!(
                            "1. 官方基金净值日期为 {}，净值为 {:.4}；\n",
                            res.official_nav_date, res.official_nav
                        );
                        println!(
                            "2. 参考指数 {} 在该日期附近的收盘价为 {:.2}；\n",
                            res.reference_index_symbol, res.reference_price_on_nav_date
                        );
                        println!(
                            "3. 参考指数最新价格为 {:.2}，涨跌为 {:.2}%；\n",
                            res.reference_latest_price,
                            res.index_return * 100.0
                        );

                        if res.use_fx_adjustment {
                            if res.fx_return.abs() > 0.000001
                                || !res.warning.as_ref().is_some_and(|w| w.contains("汇率"))
                            {
                                println!(
                                    "4. 汇率调整已启用。USD/CNH 汇率涨跌为 {:.2}%；\n",
                                    res.fx_return * 100.0
                                );
                                println!(
                                    "5. 综合估算涨跌 = (1 + 指数涨跌) * (1 + 汇率涨跌) - 1 = {:.2}%；\n",
                                    res.combined_proxy_return * 100.0
                                );
                            } else {
                                println!("4. 汇率调整虽已启用，但因缺少汇率历史数据而未能应用。\n");
                                println!(
                                    "5. 估算涨跌已退回指数涨跌：{:.2}%；\n",
                                    res.proxy_return * 100.0
                                );
                            }
                        } else {
                            println!("4. 汇率调整未启用或不适用。\n");
                            println!(
                                "5. 估算涨跌即指数涨跌：{:.2}%；\n",
                                res.proxy_return * 100.0
                            );
                        }

                        println!("6. 因此估算基金净值为 {:.4}；\n", res.estimated_nav);
                        println!("7. 估算市值为 {:.2}；\n", res.estimated_market_value);
                        println!("8. 该结果仅用于当日估算，不会覆盖官方净值。");

                        if let Some(w) = &res.warning {
                            println!("\n警告: {}", w);
                        }
                    }
                } else {
                    println!("Error: Asset not found: {}", asset_id);
                }
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
                        "{:<15} | {:<20} | {:<10} | {:<15} | {:<15} | 原因",
                        "赛道", "资产", "基金代码", "缺口", "建议买入"
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
            cli::DecisionCommands::AdjustedPreview => {
                let instruments = repo.load_instruments(&ctx).await.unwrap_or_default();
                let (risk_overlay, regimes) = {
                    let config = config.clone();
                    let instruments = instruments.clone();
                    tokio::task::spawn_blocking(move || {
                        let fx_provider = api::create_fx_provider(&config.fx, None);
                        let market_provider =
                            api::create_market_provider(&config.market, Some("yahoo"));
                        let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
                            &config.risk,
                            &config.regime,
                            market_provider.as_ref(),
                            fx_provider.as_ref(),
                        );

                        let mut regimes = std::collections::HashMap::new();
                        for asset in &config.assets {
                            let symbol_opt = if let Some(rid) = &asset.reference_instrument_id {
                                instruments
                                    .iter()
                                    .find(|i| i.instrument_id == *rid)
                                    .map(|i| i.provider_symbol.clone())
                            } else {
                                asset
                                    .reference_instrument_symbol
                                    .clone()
                                    .or(asset.reference_index_symbol.clone())
                            };

                            if let Some(s) = symbol_opt {
                                if let Ok(candles) = market_provider.fetch_daily_candles(
                                    &s,
                                    config.regime.default_lookback_days,
                                ) {
                                    let regime = engine::regime::calculate_market_regime(
                                        &s,
                                        &candles,
                                        &config.regime,
                                    );
                                    regimes.insert(asset.asset_id.clone(), regime);
                                }
                            }
                        }
                        (risk_overlay, regimes)
                    })
                    .await?
                };

                let date = Local::now().format("%Y-%m-%d").to_string();
                let decision = engine::generate_buy_suggestions(&config, &state, date);

                let adjusted = engine::adjusted_decision::calculate_adjusted_decision(
                    &config,
                    &state,
                    &decision,
                    &risk_overlay,
                    &regimes,
                );

                println!("风险调整买入建议预览\n");
                println!(
                    "可用现金: {:.2} {}",
                    adjusted.available_cash, config.portfolio.base_currency
                );
                println!(
                    "基础建议总买入: {:.2} {}",
                    adjusted.base_total_buy, config.portfolio.base_currency
                );
                println!(
                    "调整后总建议: {:.2} {}",
                    adjusted.adjusted_total_buy, config.portfolio.base_currency
                );
                println!("综合总倍率: {:.2}x\n", adjusted.total_multiplier);

                println!(
                    "{:<10} | {:<20} | {:<10} | {:>10} | {:<6} | {:>8} | {:<8} | {:>8} | {:>8} | {:>12} | {:<10}",
                    "赛道",
                    "资产",
                    "基金代码",
                    "基础建议",
                    "市场状态",
                    "钟摆分数",
                    "全局风险",
                    "Kelly倍率",
                    "综合倍率",
                    "调整后建议",
                    "状态"
                );
                println!("{:-<145}", "");

                for item in &adjusted.items {
                    println!(
                        "{:<10} | {:<20} | {:<10} | {:>10.2} | {:<6} | {:>8.1} | {:<8} | {:>8.2}x | {:>8.2}x | {:>12.2} | {:<10}",
                        item.sector,
                        item.asset_id,
                        item.fund_code,
                        item.base_suggested_buy,
                        item.regime_label,
                        item.pendulum_score,
                        item.global_risk_label,
                        item.kelly_multiplier,
                        item.combined_multiplier,
                        item.capped_adjusted_buy,
                        item.status
                    );
                }

                if !adjusted.warnings.is_empty() {
                    println!("\n警告:");
                    for w in &adjusted.warnings {
                        println!("- {}", w);
                    }
                }

                println!("\n该结果仅为预览，不会自动执行买入，也不会修改组合状态。");
            }
            cli::DecisionCommands::AdjustedExplain { asset_id } => {
                let asset_config = config.assets.iter().find(|a| a.asset_id == *asset_id);
                if let Some(a) = asset_config {
                    let (risk_overlay, regime) = {
                        let config = config.clone();
                        let a = a.clone();
                        tokio::task::spawn_blocking(move || {
                            let fx_provider = api::create_fx_provider(&config.fx, None);
                            let market_provider =
                                api::create_market_provider(&config.market, Some("yahoo"));
                            let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
                                &config.risk,
                                &config.regime,
                                market_provider.as_ref(),
                                fx_provider.as_ref(),
                            );

                            let mut regime = None;
                            if let Some(s) = &a.reference_index_symbol {
                                if let Ok(candles) = market_provider
                                    .fetch_daily_candles(s, config.regime.default_lookback_days)
                                {
                                    regime = Some(engine::regime::calculate_market_regime(
                                        s,
                                        &candles,
                                        &config.regime,
                                    ));
                                }
                            }
                            (risk_overlay, regime)
                        })
                        .await?
                    };

                    let date = Local::now().format("%Y-%m-%d").to_string();
                    let decision = engine::generate_buy_suggestions(&config, &state, date);

                    let mut base_buy = 0.0;
                    for s in &decision.sector_suggestions {
                        if let Some(ad) = s
                            .asset_suggestions
                            .iter()
                            .find(|ad| ad.asset_id == *asset_id)
                        {
                            base_buy = ad.suggested_buy;
                        }
                    }

                    let item = engine::adjusted_decision::calculate_single_adjusted_item(
                        engine::adjusted_decision::AdjustedDecisionContext {
                            config: &config,
                            state: &state,
                            asset_id: a.asset_id.clone(),
                            fund_code: a.fund_code.clone(),
                            fund_name: a.fund_name.clone(),
                            sector: a.sector.clone(),
                            base_suggested_buy: base_buy,
                            risk_overlay: &risk_overlay,
                            regime: regime.as_ref(),
                        },
                    );

                    println!("风险调整建议详情: {}\n", asset_id);
                    println!("1. 基础建议买入额: {:.2}", item.base_suggested_buy);
                    println!(
                        "2. 市场周期倍率: {} (分数 {:.1}, 倍率 {:.2}x)",
                        item.regime_label, item.pendulum_score, item.regime_multiplier
                    );
                    println!(
                        "3. 全局风险倍率: {} (分数 {:.1}, 倍率 {:.2}x)",
                        item.global_risk_label, item.global_risk_score, item.risk_multiplier
                    );
                    println!("4. Kelly 倍率: {:.2}x", item.kelly_multiplier);
                    println!("5. 数据质量倍率: {:.2}x", item.data_quality_multiplier);
                    println!("6. 综合倍率: {:.2}x", item.combined_multiplier);
                    println!(
                        "7. 最终建议买入: {:.2} (状态: {})",
                        item.capped_adjusted_buy, item.status
                    );
                    println!("\n计算路径: {}", item.explanation);

                    if !item.warnings.is_empty() {
                        println!("\n警告:");
                        for w in &item.warnings {
                            println!("- {}", w);
                        }
                    }

                    println!("\n该结果仅为预览，不会自动执行买入。");
                } else {
                    println!("Error: 未找到资产 {}", asset_id);
                }
            }
            cli::DecisionCommands::Compare => {
                let instruments = repo.load_instruments(&ctx).await.unwrap_or_default();
                let (risk_overlay, regimes) = {
                    let config = config.clone();
                    let instruments = instruments.clone();
                    tokio::task::spawn_blocking(move || {
                        let fx_provider = api::create_fx_provider(&config.fx, None);
                        let market_provider =
                            api::create_market_provider(&config.market, Some("yahoo"));
                        let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
                            &config.risk,
                            &config.regime,
                            market_provider.as_ref(),
                            fx_provider.as_ref(),
                        );

                        let mut regimes = std::collections::HashMap::new();
                        for asset in &config.assets {
                            let symbol_opt = if let Some(rid) = &asset.reference_instrument_id {
                                instruments
                                    .iter()
                                    .find(|i| i.instrument_id == *rid)
                                    .map(|i| i.provider_symbol.clone())
                            } else {
                                asset
                                    .reference_instrument_symbol
                                    .clone()
                                    .or(asset.reference_index_symbol.clone())
                            };

                            if let Some(s) = symbol_opt {
                                if let Ok(candles) = market_provider.fetch_daily_candles(
                                    &s,
                                    config.regime.default_lookback_days,
                                ) {
                                    let regime = engine::regime::calculate_market_regime(
                                        &s,
                                        &candles,
                                        &config.regime,
                                    );
                                    regimes.insert(asset.asset_id.clone(), regime);
                                }
                            }
                        }
                        (risk_overlay, regimes)
                    })
                    .await?
                };

                let date = Local::now().format("%Y-%m-%d").to_string();
                let decision = engine::generate_buy_suggestions(&config, &state, date);

                let kelly_preview = engine::kelly::calculate_kelly_preview(
                    &config,
                    &decision,
                    &risk_overlay,
                    &regimes,
                );
                let adjusted = engine::adjusted_decision::calculate_adjusted_decision(
                    &config,
                    &state,
                    &decision,
                    &risk_overlay,
                    &regimes,
                );

                println!("决策建议版本对比\n");
                println!(
                    "{:<20} | {:>15} | {:>15} | {:>15}",
                    "项目", "基础建议", "Kelly 预览", "风险调整建议"
                );
                println!("{:-<75}", "");
                println!(
                    "{:<20} | {:>15.2} | {:>15.2} | {:>15.2}",
                    "总买入额",
                    decision.suggested_total_buy,
                    kelly_preview.preview_total_buy,
                    adjusted.adjusted_total_buy
                );

                println!("\n资产明细:");
                println!(
                    "{:<20} | {:>12} | {:>12} | {:>12} | {:<10}",
                    "资产ID", "基础建议", "Kelly", "风险调整", "差异原因"
                );
                println!("{:-<80}", "");

                for item in &adjusted.items {
                    let kelly_val = kelly_preview
                        .results
                        .iter()
                        .find(|r| r.asset_id == item.asset_id)
                        .map(|r| r.capped_preview_buy_amount)
                        .unwrap_or(0.0);

                    let mut reasons = Vec::new();
                    if item.regime_multiplier != 1.0 {
                        reasons.push(format!("周期{}", item.regime_label));
                    }
                    if item.risk_multiplier != 1.0 {
                        reasons.push(format!("风险{}", item.global_risk_label));
                    }
                    if item.data_quality_multiplier != 1.0 {
                        reasons.push("数据质量".to_string());
                    }

                    println!(
                        "{:<20} | {:>12.2} | {:>12.2} | {:>12.2} | {}",
                        item.asset_id,
                        item.base_suggested_buy,
                        kelly_val,
                        item.capped_adjusted_buy,
                        reasons.join(",")
                    );
                }
            }
            cli::DecisionCommands::Explain { json } => {
                let date = Local::now().format("%Y-%m-%d").to_string();
                let explanation = engine::explanation::explain_decision(
                    &config,
                    &state,
                    ctx.portfolio_id.clone(),
                    date.clone(),
                    &risk_overlay,
                    &regimes,
                );

                if *json {
                    println!("{}", serde_json::to_string_pretty(&explanation)?);
                } else {
                    println!("投资决策深度解析 - {}\n", explanation.date);

                    println!("1. 组合整体状态:");
                    println!("   - 组合ID:     {}", explanation.portfolio_id);
                    println!(
                        "   - 可用现金:   {:.2} {}",
                        explanation.available_cash, explanation.base_currency
                    );
                    println!(
                        "   - 单日预算:   {:.2} {}",
                        explanation.daily_budget, explanation.base_currency
                    );
                    println!(
                        "   - 权益仓位:   {:.2} / {:.2} (缺口: {:.2})",
                        explanation.current_equity_value,
                        explanation.target_equity_value,
                        explanation.equity_gap
                    );

                    println!("\n2. 全局风险评估:");
                    println!("   - 风险等级:   {}", explanation.risk_summary.label);
                    println!("   - 风险分数:   {:.1}", explanation.risk_summary.score);
                    if !explanation.risk_summary.factors.is_empty() {
                        println!(
                            "   - 关键因子:   {}",
                            explanation.risk_summary.factors.join(", ")
                        );
                    }

                    println!("\n3. 赛道分配详情:");
                    println!(
                        "   {:<15} | {:>10} | {:>10} | {:>12} | {:>12}",
                        "赛道", "当前占比", "目标占比", "分配金额", "状态"
                    );
                    println!("   {}", "-".repeat(65));
                    for s in &explanation.sector_explanations {
                        let status = if s.gap_value > 0.0 { "低配" } else { "均衡" };
                        println!(
                            "   {:<15} | {:>10.1}% | {:>10.1}% | {:>12.2} | {}",
                            s.sector_name,
                            s.current_weight * 100.0,
                            s.target_weight * 100.0,
                            s.allocated_amount,
                            status
                        );
                    }

                    println!("\n4. 资产决策详情:");
                    for a in &explanation.asset_explanations {
                        if a.base_suggested_buy > 0.0 || a.final_suggested_buy > 0.0 {
                            println!("\n   > {} ({}):", a.fund_name, a.asset_id);
                            println!("     - 状态:       {}", a.status);
                            println!("     - 基础建议:   {:.2}", a.base_suggested_buy);

                            let mut mults = Vec::new();
                            if a.regime_adjustment.multiplier != 1.0 {
                                mults.push(format!(
                                    "周期({}): {:.2}x",
                                    a.regime_adjustment.label, a.regime_adjustment.multiplier
                                ));
                            }
                            if a.risk_adjustment.multiplier != 1.0 {
                                mults.push(format!(
                                    "风险({}): {:.2}x",
                                    a.risk_adjustment.label, a.risk_adjustment.multiplier
                                ));
                            }
                            if a.kelly_adjustment.multiplier != 1.0 {
                                mults.push(format!("Kelly: {:.2}x", a.kelly_adjustment.multiplier));
                            }
                            if a.data_quality_multiplier != 1.0 {
                                mults.push(format!("数据质量: {:.2}x", a.data_quality_multiplier));
                            }

                            if !mults.is_empty() {
                                println!("     - 调整因子:   {}", mults.join(" * "));
                            }

                            println!("     - 最终建议:   {:.2}", a.final_suggested_buy);
                            println!("     - 解析:       {}", a.summary);

                            for cap in &a.caps {
                                if cap.applied {
                                    println!(
                                        "     - [限制]:     {} (上限 {:.2})",
                                        cap.name, cap.limit_value
                                    );
                                }
                            }
                        }
                    }

                    if !explanation.warnings.is_empty() {
                        println!("\n⚠️ 注意事项:");
                        for w in &explanation.warnings {
                            println!("   - {}", w);
                        }
                    }
                }
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
                    "{:<20} | {:<10} | {:<10} | {:<15} | {:<15} | {:<15} | 状态",
                    "赛道", "目标占比", "当前占比", "目标市值", "当前市值", "缺口"
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
                    repo.save_config(&ctx, &config_clone).await?;
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
                repo.save_config(&ctx, &config_clone).await?;
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
                    repo.save_config(&ctx, &config_clone).await?;
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
                    repo.save_config(&ctx, &config_clone).await?;
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
                            repo.save_config(&ctx, &config_clone).await?;
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
                    repo.save_config(&ctx, &config_clone).await?;
                    println!("共更新了 {} 个资产名称。", updated_count);
                } else {
                    println!("所有资产名称均已是最新，无需更新。");
                }
            }
        },
        Commands::Market { command } => match command {
            cli::MarketCommands::Lookup { symbol, provider } => {
                let market_data_res = {
                    let config = config.clone();
                    let provider_name = provider.clone();
                    let symbol = symbol.clone();
                    tokio::task::spawn_blocking(move || {
                        let effective_provider =
                            api::create_market_provider(&config.market, provider_name.as_deref());
                        effective_provider.fetch_latest_price(&symbol)
                    })
                    .await?
                };

                let mut market_data = None;
                let mut source_is_mock = false;

                // 1. Try primary provider
                match market_data_res {
                    Ok(data) => {
                        market_data = Some(data);
                    }
                    Err(e) => {
                        if provider.is_some() {
                            println!(
                                "Error: 显式指定的数据源 {} 获取失败: {}",
                                provider.as_ref().unwrap(),
                                e
                            );
                        } else {
                            println!("警告：获取代码 {} 的市场价格失败: {}", symbol, e);
                        }
                    }
                }

                // 2. Try cache if provider failed and no explicit provider was requested
                if market_data.is_none() && provider.is_none() {
                    if let Some(entry) = market_cache.entries.iter().find(|e| e.symbol == *symbol) {
                        let is_stale = if let Ok(fetched_at) =
                            chrono::DateTime::parse_from_rfc3339(&entry.fetched_at)
                        {
                            let hours = Local::now().signed_duration_since(fetched_at).num_hours();
                            hours >= config.market.market_cache_stale_hours
                        } else {
                            true
                        };

                        market_data = Some(models::MarketPrice {
                            symbol: entry.symbol.clone(),
                            price: entry.price,
                            date: entry.date.clone(),
                            currency: entry.currency.clone(),
                            source: entry.source.clone(),
                            is_stale,
                            previous_close: entry.previous_close,
                            change: entry.change,
                            change_percent: entry.change_percent,
                        });
                        println!("使用缓存数据。");
                    }
                }

                // 3. Fallback to mock if allowed and no data yet
                if market_data.is_none()
                    && provider.is_none()
                    && config.market.allow_mock_market_fallback
                {
                    let mock = api::MockMarketProvider::new();
                    if let Ok(data) = mock.fetch_latest_price(symbol) {
                        market_data = Some(data);
                        source_is_mock = true;
                    }
                }

                if let Some(data) = market_data {
                    println!("指数代码: {}", data.symbol);
                    println!("最新价格: {:.2}", data.price);
                    println!("日期: {}", data.date);
                    println!("货币: {}", data.currency);
                    println!("数据来源: {}", data.source);

                    let status_str = if source_is_mock || data.source == "mock" {
                        "模拟"
                    } else if data.is_stale {
                        "过期"
                    } else {
                        "正常"
                    };
                    println!("数据状态: {}", status_str);

                    // Update cache on success from real provider
                    if !data.is_stale && data.source != "cache" && data.source != "mock" {
                        let entry = models::MarketCacheEntry {
                            symbol: data.symbol.clone(),
                            price: data.price,
                            date: data.date.clone(),
                            currency: data.currency.clone(),
                            source: data.source.clone(),
                            fetched_at: Local::now().to_rfc3339(),
                            previous_close: data.previous_close,
                            change: data.change,
                            change_percent: data.change_percent,
                            status: Some("ok".to_string()),
                            error_message: None,
                        };
                        if let Some(existing) = market_cache
                            .entries
                            .iter_mut()
                            .find(|e| e.symbol == data.symbol)
                        {
                            *existing = entry;
                        } else {
                            market_cache.entries.push(entry);
                        }
                        repo.save_market_cache(&ctx, &market_cache).await?;
                    }
                } else {
                    println!("Error: 无法获取代码 {} 的价格且无可用备份。", symbol);
                }
            }
            cli::MarketCommands::History {
                symbol,
                days,
                provider,
            } => {
                let effective_provider =
                    api::create_market_provider(&config.market, provider.as_deref());
                match effective_provider.fetch_daily_candles(symbol, *days) {
                    Ok(candles) => {
                        println!(
                            "代码 {} 的历史数据 (最近 {} 天, 来源: {}):",
                            symbol,
                            days,
                            candles
                                .first()
                                .map(|c| c.source.as_str())
                                .unwrap_or("unknown")
                        );
                        println!(
                            "{:<12} | {:<10} | {:<10} | {:<10} | {:<10} | {:<12}",
                            "日期", "开盘", "最高", "最低", "收盘", "成交量"
                        );
                        println!("{:-<75}", "");
                        for c in candles {
                            println!(
                                "{:<12} | {:<10.2} | {:<10.2} | {:<10.2} | {:<10.2} | {:<12}",
                                c.date, c.open, c.high, c.low, c.close, c.volume
                            );
                        }
                    }
                    Err(e) => {
                        println!("Error: 获取历史数据失败: {}", e);
                    }
                }
            }
            cli::MarketCommands::Regime {
                symbol,
                asset_id,
                days,
                provider,
            } => {
                let instruments = repo.load_instruments(&ctx).await.unwrap_or_default();
                let target_symbol = if let Some(s) = symbol {
                    Some(s.clone())
                } else if let Some(aid) = asset_id {
                    config
                        .assets
                        .iter()
                        .find(|a| a.asset_id == *aid)
                        .and_then(|a| {
                            if let Some(rid) = &a.reference_instrument_id {
                                instruments
                                    .iter()
                                    .find(|i| i.instrument_id == *rid)
                                    .map(|i| i.provider_symbol.clone())
                            } else {
                                a.reference_instrument_symbol
                                    .clone()
                                    .or(a.reference_index_symbol.clone())
                            }
                        })
                } else {
                    None
                };

                if let Some(s) = target_symbol {
                    let market_provider =
                        api::create_market_provider(&config.market, provider.as_deref());
                    match market_provider.fetch_daily_candles(&s, *days) {
                        Ok(candles) => {
                            let regime = engine::regime::calculate_market_regime(
                                &s,
                                &candles,
                                &config.regime,
                            );
                            display_regime_result(&regime);
                        }
                        Err(e) => println!("Error: {}", e),
                    }
                } else {
                    println!("错误: 请提供 symbol 或 asset-id (且该资产已配置参考指数或标的ID)。");
                }
            }
            cli::MarketCommands::RegimeAll => {
                let instruments = repo.load_instruments(&ctx).await.unwrap_or_default();
                let market_provider = api::create_market_provider(&config.market, None);
                let mut symbols = Vec::new();
                for asset in &config.assets {
                    let s_opt = if let Some(rid) = &asset.reference_instrument_id {
                        instruments
                            .iter()
                            .find(|i| i.instrument_id == *rid)
                            .map(|i| i.provider_symbol.clone())
                    } else {
                        asset
                            .reference_instrument_symbol
                            .clone()
                            .or(asset.reference_index_symbol.clone())
                    };

                    if let Some(s) = s_opt {
                        if !symbols.contains(&s) {
                            symbols.push(s);
                        }
                    }
                }

                println!("全市场冷热分析:\n");
                println!(
                    "{:<10} | {:<10} | {:>8} | 状态",
                    "代码", "价格", "钟摆分数"
                );
                println!("{:-<50}", "");
                for s in symbols {
                    match market_provider
                        .fetch_daily_candles(&s, config.regime.default_lookback_days)
                    {
                        Ok(candles) => {
                            let regime = engine::regime::calculate_market_regime(
                                &s,
                                &candles,
                                &config.regime,
                            );
                            println!(
                                "{:<10} | {:<10.2} | {:>8.2} | {}",
                                regime.symbol,
                                regime.latest_price,
                                regime.pendulum_score,
                                regime.regime_label
                            );
                        }
                        Err(e) => println!("{:<10} | 查询失败: {}", s, e),
                    }
                }
            }
            cli::MarketCommands::RegimeExplain { symbol, provider } => {
                let instruments = repo.load_instruments(&ctx).await.unwrap_or_default();
                let target_symbol =
                    if let Some(i) = instruments.iter().find(|i| i.instrument_id == *symbol) {
                        i.provider_symbol.clone()
                    } else {
                        symbol.clone()
                    };

                let market_provider =
                    api::create_market_provider(&config.market, provider.as_deref());
                match market_provider
                    .fetch_daily_candles(&target_symbol, config.regime.default_lookback_days)
                {
                    Ok(candles) => {
                        let regime = engine::regime::calculate_market_regime(
                            &target_symbol,
                            &candles,
                            &config.regime,
                        );
                        explain_regime_result(&regime, &config.regime);
                    }
                    Err(e) => println!("Error: {}", e),
                }
            }
            cli::MarketCommands::Provider { command } => match command {
                Some(cli::MarketProviderCommands::Set { provider }) => {
                    let mut config_clone = config.clone();
                    if provider != "yahoo" && provider != "mock" {
                        anyhow::bail!("不支持的行情来源: {}", provider);
                    }
                    config_clone.market.default_market_provider = provider.clone();
                    repo.save_config(&ctx, &config_clone).await?;
                    println!("已将默认行情来源设置为: {}", provider);
                    if provider == "yahoo" {
                        println!("警告：实时行情取决于网络连通性。");
                    }
                }
                None => {
                    println!("默认行情来源: {}", config.market.default_market_provider);
                    println!(
                        "允许模拟数据回退: {}",
                        config.market.allow_mock_market_fallback
                    );
                    println!("行情缓存路径: {}", cli.market_cache);
                    println!(
                        "缓存过期时间: {} 小时",
                        config.market.market_cache_stale_hours
                    );
                }
            },
        },
        Commands::Asset { command } => match command {
            cli::AssetCommands::List => {
                println!(
                    "{:<20} | {:<10} | {:<20} | {:<10} | {:<8} | {:<10} | Enabled",
                    "Asset ID",
                    "Fund Code",
                    "Fund Name",
                    "Sector",
                    "Currency",
                    "Val Method"
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
                    reference_index_name: None,
                    reference_index_symbol: None,
                    market_data_provider: None,
                    reference_index_currency: None,
                    proxy_fx_pair: None,
                    use_fx_adjustment: Some(false),
                    reference_instrument_id: None,
                    reference_instrument_symbol: None,
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

                repo.save_config(&ctx, &config_clone).await?;
                repo.save_state(&ctx, &state).await?;
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
                    repo.save_config(&ctx, &config_clone).await?;
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
                    repo.save_config(&ctx, &config_clone).await?;
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
                    repo.save_config(&ctx, &config_clone).await?;
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
                    repo.save_config(&ctx, &config_clone).await?;
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
                            repo.save_config(&ctx, &config_clone).await?;

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
                                repo.save_state(&ctx, &state_clone).await?;
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
                    repo.save_config(&ctx, &config_clone).await?;
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
                    "{:<20} | {:<10} | {:<20} | {:<20} | {:<10} | 说明",
                    "资产ID", "基金代码", "基金名称", "赛道", "状态"
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
                            "{:<20} | {:<10} | {:<20} | {:<20} | {:<10} | config 中无此资产",
                            holding.asset_id,
                            holding.fund_code,
                            "N/A",
                            "N/A",
                            "孤立持仓"
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
                    repo.save_state(&ctx, &state_clone).await?;
                    println!("共修复了 {} 个缺失持仓。", repaired_count);
                } else {
                    println!("未发现缺失持仓，无需修复。");
                }
            }
            cli::AssetCommands::ReferenceValidate => {
                println!(
                    "{:<20} | {:<20} | {:<10} | {:<15} | {:<10} | {:<10} | {:<10} | 说明",
                    "资产ID",
                    "基金名称",
                    "赛道",
                    "参考指数",
                    "指数代码",
                    "行情来源",
                    "查询状态"
                );
                println!("{:-<130}", "");

                for asset in &config.assets {
                    if let Some(symbol) = &asset.reference_index_symbol {
                        let mut status = "正常";
                        let mut note = String::new();
                        let provider_name = asset.market_data_provider.as_deref();
                        let effective_provider =
                            api::create_market_provider(&config.market, provider_name);

                        match effective_provider.fetch_latest_price(symbol) {
                            Ok(_) => {}
                            Err(_) => {
                                // Try cache
                                if market_cache.entries.iter().any(|e| e.symbol == *symbol) {
                                    status = "使用缓存";
                                } else if config.market.allow_mock_market_fallback {
                                    status = "模拟";
                                } else {
                                    status = "查询失败";
                                }
                            }
                        }

                        // Semantic check (very basic)
                        if let Some(ref_name) = &asset.reference_index_name {
                            let fund_keywords =
                                vec!["纳斯达克", "标普", "500", "100", "Nasdaq", "S&P"];
                            let has_ref_kw = fund_keywords.iter().any(|kw| ref_name.contains(kw));

                            if has_ref_kw {
                                // Check if they share any keyword
                                let mut shared = false;
                                for kw in fund_keywords {
                                    if asset.fund_name.contains(kw) && ref_name.contains(kw) {
                                        shared = true;
                                        break;
                                    }
                                }
                                if !shared {
                                    note.push_str("请人工确认该基金与参考指数是否匹配。");
                                }
                            }
                        }

                        println!(
                            "{:<20} | {:<20} | {:<10} | {:<15} | {:<10} | {:<10} | {:<10} | {}",
                            asset.asset_id,
                            asset.fund_name,
                            asset.sector,
                            asset.reference_index_name.as_deref().unwrap_or("-"),
                            symbol,
                            asset.market_data_provider.as_deref().unwrap_or("-"),
                            status,
                            note
                        );
                    }
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
                println!("{:<10} | 资产ID", "基金代码");
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
            cli::AssetCommands::SetReference {
                asset_id,
                reference_index_name,
                reference_index_symbol,
                market_data_provider,
                reference_index_currency,
                proxy_fx_pair,
                use_fx_adjustment,
                reference_instrument_id,
                reference_instrument_symbol,
            } => {
                let mut config_clone = config.clone();
                let asset = config_clone
                    .assets
                    .iter_mut()
                    .find(|a| a.asset_id == *asset_id);

                if let Some(asset) = asset {
                    asset.reference_index_name = Some(reference_index_name.clone());
                    asset.reference_index_symbol = Some(reference_index_symbol.clone());
                    asset.market_data_provider = Some(market_data_provider.clone());
                    asset.reference_index_currency = reference_index_currency.clone();
                    asset.proxy_fx_pair = proxy_fx_pair.clone();
                    asset.use_fx_adjustment = *use_fx_adjustment;
                    asset.reference_instrument_id = reference_instrument_id.clone();
                    asset.reference_instrument_symbol = reference_instrument_symbol.clone();

                    repo.save_config(&ctx, &config_clone).await?;
                    println!(
                        "已为资产 {} 设置参考指数: {} ({})",
                        asset_id, reference_index_name, reference_index_symbol
                    );
                    if let Some(cur) = reference_index_currency {
                        println!("参考指数货币: {}", cur);
                    }
                    if let Some(pair) = proxy_fx_pair {
                        println!("估算汇率对: {}", pair);
                    }
                    if let Some(adj) = use_fx_adjustment {
                        println!("启用汇率调整: {}", if *adj { "是" } else { "否" });
                    }
                    if let Some(rid) = reference_instrument_id {
                        println!("关联标的ID: {}", rid);
                    }
                    if let Some(rsym) = reference_instrument_symbol {
                        println!("关联标代码: {}", rsym);
                    }
                } else {
                    println!("Error: Asset not found: {}", asset_id);
                }
            }
            cli::AssetCommands::ReferenceList => {
                println!(
                    "{:<15} | {:<10} | {:<15} | {:<10} | {:<15} | {:<10} | {:<10}",
                    "资产ID",
                    "基金代码",
                    "参考指数",
                    "指数代码",
                    "关联标的ID",
                    "标的代码",
                    "行情来源"
                );
                println!("{:-<100}", "");
                for asset in &config.assets {
                    println!(
                        "{:<15} | {:<10} | {:<15} | {:<10} | {:<15} | {:<10} | {:<10}",
                        asset.asset_id,
                        asset.fund_code,
                        asset.reference_index_name.as_deref().unwrap_or("-"),
                        asset.reference_index_symbol.as_deref().unwrap_or("-"),
                        asset.reference_instrument_id.as_deref().unwrap_or("-"),
                        asset.reference_instrument_symbol.as_deref().unwrap_or("-"),
                        asset.market_data_provider.as_deref().unwrap_or("-"),
                    );
                }
            }
        },
        Commands::Fx { command } => match command {
            cli::FxCommands::UsdCnh => {
                let symbol = &config.fx.usd_cnh_symbol;
                let mut fx_data = None;
                let mut source_is_mock = false;

                // 1. Try provider
                match fx_provider.fetch_latest_rate(symbol) {
                    Ok(data) => {
                        fx_data = Some(data);
                    }
                    Err(e) => {
                        println!("警告：获取 USD/CNH 汇率失败: {}", e);
                    }
                }

                // 2. Try cache
                if fx_data.is_none() {
                    if let Some(entry) = fx_cache.entries.iter().find(|e| e.pair == *symbol) {
                        let is_stale = if let Ok(fetched_at) =
                            chrono::DateTime::parse_from_rfc3339(&entry.fetched_at)
                        {
                            let hours = Local::now().signed_duration_since(fetched_at).num_hours();
                            hours >= config.fx.fx_cache_stale_hours
                        } else {
                            true
                        };

                        fx_data = Some(models::FxRate {
                            pair: entry.pair.clone(),
                            base_currency: "USD".to_string(),
                            quote_currency: "CNH".to_string(),
                            rate: entry.rate,
                            date: entry.date.clone(),
                            source: entry.source.clone(),
                            is_stale,
                            is_estimated: false,
                        });
                        println!("使用缓存数据。");
                    }
                }

                // 3. Fallback to mock
                if fx_data.is_none() && config.fx.allow_mock_fx_fallback {
                    let mock = api::MockFxProvider;
                    if let Ok(data) = mock.fetch_latest_rate(symbol) {
                        fx_data = Some(data);
                        source_is_mock = true;
                    }
                }

                if let Some(data) = fx_data {
                    println!("汇率: {}", data.pair);
                    println!("汇率值: {:.4}", data.rate);
                    println!("日期: {}", data.date);
                    println!("数据来源: {}", data.source);

                    let status_str = if source_is_mock || data.source == "mock" {
                        "模拟"
                    } else if data.is_stale {
                        "过期"
                    } else {
                        "正常"
                    };
                    println!("数据状态: {}", status_str);

                    // Update cache
                    if !data.is_stale && data.source != "cache" && data.source != "mock" {
                        let entry = models::FxCacheEntry {
                            pair: data.pair.clone(),
                            rate: data.rate,
                            date: data.date.clone(),
                            source: data.source.clone(),
                            fetched_at: Local::now().to_rfc3339(),
                        };
                        if let Some(existing) =
                            fx_cache.entries.iter_mut().find(|e| e.pair == data.pair)
                        {
                            *existing = entry;
                        } else {
                            fx_cache.entries.push(entry);
                        }
                        repo.save_fx_cache(&ctx, &fx_cache).await?;
                    }
                } else {
                    println!("错误：无法获取 USD/CNH 汇率且无可用备份。");
                }
            }
            cli::FxCommands::UsdCnhHistory { days } => {
                let symbol = &config.fx.usd_cnh_symbol;
                match fx_provider.fetch_daily_rates(symbol, *days) {
                    Ok(candles) => {
                        println!(
                            "USD/CNH 历史汇率 (最近 {} 天, 来源: {}):",
                            days,
                            candles
                                .first()
                                .map(|c| c.source.as_str())
                                .unwrap_or("unknown")
                        );
                        println!("{:<12} | {:<10}", "日期", "收盘价");
                        println!("{:-<25}", "");
                        for c in candles {
                            println!("{:<12} | {:<10.4}", c.date, c.close);
                        }
                    }
                    Err(e) => {
                        println!("Error: 获取历史汇率失败: {}", e);
                    }
                }
            }
        },
        Commands::Risk { command } => match command {
            cli::RiskCommands::Crypto { symbol } => {
                let symbols = if let Some(s) = symbol {
                    vec![s.clone()]
                } else {
                    vec![
                        "BTC-USD".to_string(),
                        "ETH-USD".to_string(),
                        "SOL-USD".to_string(),
                    ]
                };

                let results = {
                    let config = config.clone();
                    let symbols = symbols.clone();
                    tokio::task::spawn_blocking(move || {
                        let market_provider =
                            api::create_market_provider(&config.market, Some("yahoo"));
                        let mut res = Vec::new();
                        for sym in &symbols {
                            let price_res = market_provider.fetch_latest_price(sym);
                            res.push((sym.clone(), price_res));
                        }
                        res
                    })
                    .await?
                };

                println!(
                    "{:<12} | {:<12} | {:<12} | {:<10} | {:<10} | 数据状态",
                    "资产", "最新价格", "日期", "货币", "数据来源"
                );
                println!("{:-<80}", "");

                for (sym, res) in results {
                    match res {
                        Ok(data) => {
                            println!(
                                "{:<12} | {:<12.2} | {:<12} | {:<10} | {:<10} | 正常",
                                data.symbol,
                                data.price,
                                data.date,
                                data.currency,
                                data.source
                            );
                        }
                        Err(_) => {
                            println!(
                                "{:<12} | {:<12} | {:<12} | {:<10} | {:<10} | 查询失败",
                                sym, "-", "-", "-", "yahoo"
                            );
                        }
                    }
                }
            }
            cli::RiskCommands::Snapshot => {
                println!("风险参考快照\n");
                println!(
                    "{:<12} | {:<12} | {:<12} | {:<10} | 数据状态",
                    "项目", "最新值", "日期", "数据来源"
                );
                println!("{:-<65}", "");

                let instruments = repo.load_instruments(&ctx).await.unwrap_or_default();

                let results = {
                    let config = config.clone();
                    let instruments = instruments.clone();
                    tokio::task::spawn_blocking(move || {
                        let market_provider =
                            api::create_market_provider(&config.market, Some("yahoo"));
                        let fx_provider = api::create_fx_provider(&config.fx, None);
                        let mut res = Vec::new();

                        // 1. USD/CNH
                        let usd_cnh_symbol = &config.fx.usd_cnh_symbol;
                        let mut usd_cnh_res = None;
                        if let Some(i) = instruments
                            .iter()
                            .find(|i| i.instrument_id == "usd_cnh" && i.enabled)
                        {
                            if let Ok(q) = engine::instrument::lookup_instrument(
                                &config.market,
                                &instruments,
                                &i.instrument_id,
                            ) {
                                usd_cnh_res = Some(("USD/CNH".to_string(), format!("{:.4}", q.latest_price), q.latest_date, q.source, q.status));
                            }
                        }

                        if usd_cnh_res.is_none() {
                            match fx_provider.fetch_latest_rate(usd_cnh_symbol) {
                                Ok(data) => {
                                    usd_cnh_res = Some(("USD/CNH".to_string(), format!("{:.4}", data.rate), data.date, data.source, "正常".to_string()));
                                }
                                Err(_) => {
                                    usd_cnh_res = Some(("USD/CNH".to_string(), "-".to_string(), "-".to_string(), "yahoo".to_string(), "查询失败".to_string()));
                                }
                            }
                        }
                        res.push(usd_cnh_res.unwrap());

                        // 2. Cryptos
                        let cryptos = vec![
                            ("btc_usd", "BTC-USD"),
                            ("eth_usd", "ETH-USD"),
                            ("sol_usd", "SOL-USD"),
                        ];
                        for (id, sym) in cryptos {
                            let mut crypto_res = None;
                            if let Some(i) = instruments
                                .iter()
                                .find(|i| i.instrument_id == id && i.enabled)
                            {
                                if let Ok(q) = engine::instrument::lookup_instrument(
                                    &config.market,
                                    &instruments,
                                    &i.instrument_id,
                                ) {
                                    crypto_res = Some((q.symbol, format!("{:.2}", q.latest_price), q.latest_date, q.source, q.status));
                                }
                            }

                            if crypto_res.is_none() {
                                match market_provider.fetch_latest_price(sym) {
                                    Ok(data) => {
                                        crypto_res = Some((data.symbol, format!("{:.2}", data.price), data.date, data.source, "正常".to_string()));
                                    }
                                    Err(_) => {
                                        crypto_res = Some((sym.to_string(), "-".to_string(), "-".to_string(), "yahoo".to_string(), "查询失败".to_string()));
                                    }
                                }
                            }
                            res.push(crypto_res.unwrap());
                        }

                        // 3. Indices
                        let indices = vec![("nasdaq_qqq", "QQQ"), ("sp500_spy", "SPY")];
                        for (id, sym) in indices {
                            let mut index_res = None;
                            if let Some(i) = instruments
                                .iter()
                                .find(|i| i.instrument_id == id && i.enabled)
                            {
                                if let Ok(q) = engine::instrument::lookup_instrument(
                                    &config.market,
                                    &instruments,
                                    &i.instrument_id,
                                ) {
                                    index_res = Some((q.symbol, format!("{:.2}", q.latest_price), q.latest_date, q.source, q.status));
                                }
                            }

                            if index_res.is_none() {
                                match market_provider.fetch_latest_price(sym) {
                                    Ok(data) => {
                                        index_res = Some((data.symbol, format!("{:.2}", data.price), data.date, data.source, "正常".to_string()));
                                    }
                                    Err(_) => {
                                        index_res = Some((sym.to_string(), "-".to_string(), "-".to_string(), "yahoo".to_string(), "查询失败".to_string()));
                                    }
                                }
                            }
                            res.push(index_res.unwrap());
                        }
                        res
                    })
                    .await?
                };

                for (name, price_str, date, source, status) in results {
                    println!(
                        "{:<12} | {:<12} | {:<12} | {:<10} | {}",
                        name, price_str, date, source, status
                    );
                }
            }
            cli::RiskCommands::Factors => {
                let overlay = {
                    let config = config.clone();
                    tokio::task::spawn_blocking(move || {
                        let market_provider =
                            api::create_market_provider(&config.market, Some("yahoo"));
                        let fx_provider = api::create_fx_provider(&config.fx, None);
                        engine::risk_overlay::calculate_risk_overlay(
                            &config.risk,
                            &config.regime,
                            market_provider.as_ref(),
                            fx_provider.as_ref(),
                        )
                    })
                    .await?
                };

                println!("全局风险因子明细\n");
                println!(
                    "{:<10} | {:<12} | {:<10} | {:<10} | {:<10} | {:<10} | {:<8} | {:<10} | 状态",
                    "风险因子",
                    "代码",
                    "最新值",
                    "日期",
                    "20日变化",
                    "60日变化",
                    "Z-score",
                    "回撤"
                );
                println!("{:-<120}", "");

                for f in overlay.factor_results {
                    let short_change = format!("{:.2}%", f.short_return * 100.0);
                    let medium_change = format!("{:.2}%", f.medium_return * 100.0);
                    let z_str = f
                        .z_score
                        .map(|z| format!("{:.2}", z))
                        .unwrap_or_else(|| "N/A".to_string());
                    let drawdown = format!("{:.2}%", f.drawdown * 100.0);

                    println!(
                        "{:<10} | {:<12} | {:<10.2} | {:<10} | {:>10} | {:>10} | {:>8} | {:>10} | {}",
                        f.name,
                        f.symbol,
                        f.latest_value,
                        f.latest_date,
                        short_change,
                        medium_change,
                        z_str,
                        drawdown,
                        f.status
                    );
                }
            }
            cli::RiskCommands::Overlay => {
                let market_provider = api::create_market_provider(&config.market, Some("yahoo"));
                let overlay = engine::risk_overlay::calculate_risk_overlay(
                    &config.risk,
                    &config.regime,
                    market_provider.as_ref(),
                    fx_provider.as_ref(),
                );

                println!("全局风险覆盖分析\n");
                println!("风险分数: {:.2} / 100", overlay.risk_score);
                println!("风险等级: {}", overlay.risk_label);
                println!("\n主要风险来源:");
                if overlay.explanation.is_empty() {
                    println!("- 各项指标正常");
                } else {
                    for line in overlay.explanation.split('；') {
                        println!("- {}", line.trim_end_matches('。'));
                    }
                }

                if !overlay.warnings.is_empty() {
                    println!("\n警告:");
                    for w in overlay.warnings {
                        println!("! {}", w);
                    }
                }
            }
            cli::RiskCommands::Explain => {
                let market_provider = api::create_market_provider(&config.market, Some("yahoo"));
                let overlay = engine::risk_overlay::calculate_risk_overlay(
                    &config.risk,
                    &config.regime,
                    market_provider.as_ref(),
                    fx_provider.as_ref(),
                );

                println!("风险评分逻辑说明 (Phase 3.2)\n");
                println!(
                    "1. VIX 恐慌指数 (权重: 0-30): 高 VIX (> 25) 或快速上升会显著增加风险评分；"
                );
                println!(
                    "2. 美债收益率 (权重: 0-20): 60日内收益率快速上升 (> 50bps) 会增加风险评分；"
                );
                println!(
                    "3. 加密货币篮子 (权重: 0-20): BTC/ETH/SOL 的深度回撤 (> 20%) 是市场风险厌恶的信号；"
                );
                println!(
                    "4. 权益市场偏离 (权重: 0-20): QQQ/SPY 处于极度过热状态 (Z-score > 2) 增加调整风险；"
                );
                println!(
                    "5. 汇率波动 (权重: 0-10): 离岸人民币快速贬值可能影响跨境资产估值，增加波动风险。"
                );
                println!("\n当前分析结论: {}", overlay.explanation);
                println!("\n风险提示: 该评分目前仅用于分析，尚未接入买入建议引擎。");
            }
            cli::RiskCommands::History {
                symbol,
                symbol_opt,
                days,
                provider,
            } => {
                let target_symbol = match (symbol, symbol_opt) {
                    (Some(s1), Some(s2)) => {
                        if s1 == s2 {
                            Some(s1.clone())
                        } else {
                            println!("错误：同时提供了两个不同的风险因子代码，请只保留一个。");
                            None
                        }
                    }
                    (Some(s), None) => Some(s.clone()),
                    (None, Some(s)) => Some(s.clone()),
                    (None, None) => {
                        println!("错误：请提供风险因子代码 (positional 或 --symbol)。");
                        None
                    }
                };

                if let Some(s) = target_symbol {
                    let market_provider =
                        api::create_market_provider(&config.market, provider.as_deref());
                    match market_provider.fetch_daily_candles(&s, *days) {
                        Ok(candles) => {
                            println!("代码 {} 的历史行情 (最近 {} 天):", s, days);
                            println!("{:<12} | {:<10}", "日期", "收盘价");
                            println!("{:-<25}", "");
                            for c in candles {
                                println!("{:<12} | {:<10.2}", c.date, c.close);
                            }
                        }
                        Err(e) => println!("Error: {}", e),
                    }
                }
            }
        },

        Commands::Config { command } => match command {
            cli::ConfigCommands::DbStatus => {
                let status = repo.get_db_status(&ctx).await?;
                println!("Backend: {}", status.backend);
                println!("DATABASE_URL source: {}", status.database_url_source);
                if let Some(db_name) = &status.database_name {
                    println!("Database: {}", db_name);
                }
                if let Some(schema) = &status.schema {
                    println!("Schema: {}", schema);
                }
                if let Some(user) = &status.user {
                    println!("User: {}", user);
                }
                if let Some(host) = &status.host {
                    println!("Host: {}", host);
                }
                if let Some(port) = &status.port {
                    println!("Port: {}", port);
                }
                println!("Fallback: {}", status.fallback);
                println!("Active Portfolio ID: {}", status.active_portfolio_id);
                if let Some(data_dir) = &status.data_dir {
                    println!("Data directory: {}", data_dir);
                }
                println!("Migrations active: {}", status.migrations_active);
                println!("\nKey table counts:");
                for t in &status.tables {
                    println!("  - {}: {}", t.name, t.count);
                }
                if !status.portfolio_records.is_empty() {
                    println!("\nRecord counts for portfolio '{}':", status.active_portfolio_id);
                    for t in &status.portfolio_records {
                        println!("  - {}: {}", t.name, t.count);
                    }
                }
            }
            cli::ConfigCommands::LocateData => {
                println!("正在定位实际数据源...\n");
                let status = repo.get_db_status(&ctx).await?;
                println!("活跃后端: {}", status.backend);

                if status.backend == "PostgreSQL" {
                    println!("活跃数据库: {}", status.database_name.as_deref().unwrap_or("未知"));
                } else {
                    println!("活跃 JSON 目录: {}", status.data_dir.as_deref().unwrap_or("data"));
                }

                println!("\n检测到记录数:");
                let mut total_records = 0;
                for t in &status.tables {
                    println!("  - {}: {}", t.name, t.count);
                    total_records += t.count;
                }

                if total_records == 0 {
                    println!("\n警告: 未检测到任何业务数据。这可能是因为连接到了空数据库或 JSON 文件为空。");
                }

                // Check for suspicious data sources
                if status.backend == "PostgreSQL" {
                    if let Some(db_name) = &status.database_name {
                        if db_name == "postgres" {
                            println!("\n警告: 正在使用 PostgreSQL 默认数据库 'postgres'。通常建议使用专用数据库如 'jdi'。");
                        }
                    }
                }

                if status.fallback {
                    println!("\n警告: 检测到降级行为。虽然配置为 PostgreSQL，但实际可能正在使用 JSON。");
                }
            }
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

                // 2. Market provider checks
                if config.market.default_market_provider == "mock" {
                    findings.push(Finding {
                        level: "info",
                        category: "行情",
                        object: "默认行情源".to_string(),
                        description: "当前市场行情使用 mock 数据，仅适合测试，不适合真实决策。"
                            .to_string(),
                        suggestion: "运行 market provider set yahoo 切换到真实行情".to_string(),
                    });
                }

                // 3. Duplicate fund_code
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
                                        description: "所属赛道权重为 0，但资产仍有市值".to_string(),
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

                    // Reference index checks
                    if let Some(symbol) = &asset.reference_index_symbol {
                        let provider_name = asset.market_data_provider.as_deref();
                        let effective_provider =
                            api::create_market_provider(&config.market, provider_name);

                        match effective_provider.fetch_latest_price(symbol) {
                            Ok(data) => {
                                if data.source == "mock" && provider_name == Some("yahoo") {
                                    findings.push(Finding {
                                        level: "warning",
                                        category: "行情",
                                        object: asset.asset_id.clone(),
                                        description: format!(
                                            "行情来源为 yahoo 但回退到了 mock (代码: {})",
                                            symbol
                                        ),
                                        suggestion: "检查网络连通性或 API 状态".to_string(),
                                    });
                                }
                            }
                            Err(_) => {
                                if !market_cache.entries.iter().any(|e| e.symbol == *symbol) {
                                    findings.push(Finding {
                                        level: "error",
                                        category: "行情",
                                        object: asset.asset_id.clone(),
                                        description: format!("无法获取参考指数 {} 的行情", symbol),
                                        suggestion: "检查指数代码是否正确或更换行情来源"
                                            .to_string(),
                                    });
                                }
                            }
                        }

                        // Semantic mismatch warning
                        if let Some(ref_name) = &asset.reference_index_name {
                            let fund_keywords =
                                vec!["纳斯达克", "标普", "500", "100", "Nasdaq", "S&P"];
                            let has_ref_kw = fund_keywords.iter().any(|kw| ref_name.contains(kw));

                            if has_ref_kw {
                                let mut shared = false;
                                for kw in fund_keywords {
                                    if asset.fund_name.contains(kw) && ref_name.contains(kw) {
                                        shared = true;
                                        break;
                                    }
                                }
                                if !shared {
                                    findings.push(Finding {
                                        level: "warning",
                                        category: "配置",
                                        object: asset.asset_id.clone(),
                                        description: format!(
                                            "该基金与参考指数 ({}) 可能不匹配",
                                            ref_name
                                        ),
                                        suggestion: "请人工确认基金与指数的相关性".to_string(),
                                    });
                                }
                            }
                        }
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
                        "{:<10} | {:<10} | {:<20} | {:<40} | 建议操作",
                        "等级", "类型", "对象", "说明"
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
        Commands::Kelly { command } => match command {
            cli::KellyCommands::Preview => {
                let market_provider = api::create_market_provider(&config.market, Some("yahoo"));
                let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
                    &config.risk,
                    &config.regime,
                    market_provider.as_ref(),
                    fx_provider.as_ref(),
                );
                let date = Local::now().format("%Y-%m-%d").to_string();
                let decision = engine::generate_buy_suggestions(&config, &state, date);

                let instruments = repo.load_instruments(&ctx).await.unwrap_or_default();
                let mut regimes = std::collections::HashMap::new();
                for asset in &config.assets {
                    let symbol_opt = if let Some(rid) = &asset.reference_instrument_id {
                        instruments
                            .iter()
                            .find(|i| i.instrument_id == *rid)
                            .map(|i| i.provider_symbol.clone())
                    } else {
                        asset
                            .reference_instrument_symbol
                            .clone()
                            .or(asset.reference_index_symbol.clone())
                    };

                    if let Some(s) = symbol_opt {
                        if let Ok(candles) = market_provider
                            .fetch_daily_candles(&s, config.regime.default_lookback_days)
                        {
                            let regime = engine::regime::calculate_market_regime(
                                &s,
                                &candles,
                                &config.regime,
                            );
                            regimes.insert(asset.asset_id.clone(), regime);
                        }
                    }
                }

                let kelly_preview = engine::kelly::calculate_kelly_preview(
                    &config,
                    &decision,
                    &risk_overlay,
                    &regimes,
                );

                println!(
                    "Kelly 仓位预览 ( fractional Kelly = {:.2} )\n",
                    config.kelly.fractional_kelly
                );
                println!(
                    "{:<10} | {:<20} | {:<10} | {:>12} | {:>8} | {:<8} | {:<10} | {:>8} | {:>12} | 状态",
                    "赛道",
                    "资产",
                    "基金代码",
                    "基础建议",
                    "钟摆分数",
                    "市场状态",
                    "全局风险",
                    "Kelly倍率",
                    "Kelly预览"
                );
                println!("{:-<145}", "");

                for res in &kelly_preview.results {
                    println!(
                        "{:<10} | {:<20} | {:<10} | {:>12.2} | {:>8.1} | {:<8} | {:<10} | {:>8.2}x | {:>12.2} | {}",
                        res.sector,
                        res.asset_id,
                        res.fund_code,
                        res.base_suggested_buy,
                        res.pendulum_score,
                        res.market_regime_label,
                        res.global_risk_label,
                        res.kelly_multiplier,
                        res.capped_preview_buy_amount,
                        res.status
                    );
                }

                println!(
                    "\n警告: Kelly 参数基于模型估计，并非真实胜率。该结果仅用于仓位参考，不应被视为确定性预测。"
                );
                for w in &kelly_preview.warnings {
                    println!("Warning: {}", w);
                }
            }
            cli::KellyCommands::Portfolio => {
                let market_provider = api::create_market_provider(&config.market, Some("yahoo"));
                let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
                    &config.risk,
                    &config.regime,
                    market_provider.as_ref(),
                    fx_provider.as_ref(),
                );
                let date = Local::now().format("%Y-%m-%d").to_string();
                let decision = engine::generate_buy_suggestions(&config, &state, date);

                let instruments = repo.load_instruments(&ctx).await.unwrap_or_default();
                let mut regimes = std::collections::HashMap::new();
                for asset in &config.assets {
                    let symbol_opt = if let Some(rid) = &asset.reference_instrument_id {
                        instruments
                            .iter()
                            .find(|i| i.instrument_id == *rid)
                            .map(|i| i.provider_symbol.clone())
                    } else {
                        asset
                            .reference_instrument_symbol
                            .clone()
                            .or(asset.reference_index_symbol.clone())
                    };

                    if let Some(s) = symbol_opt {
                        if let Ok(candles) = market_provider
                            .fetch_daily_candles(&s, config.regime.default_lookback_days)
                        {
                            let regime = engine::regime::calculate_market_regime(
                                &s,
                                &candles,
                                &config.regime,
                            );
                            regimes.insert(asset.asset_id.clone(), regime);
                        }
                    }
                }

                let kelly_preview = engine::kelly::calculate_kelly_preview(
                    &config,
                    &decision,
                    &risk_overlay,
                    &regimes,
                );

                println!("Kelly 组合预览\n");
                println!(
                    "基础建议总买入: {:.2} {}",
                    kelly_preview.base_total_buy, config.portfolio.base_currency
                );
                println!(
                    "Kelly 预览总买入: {:.2} {}",
                    kelly_preview.preview_total_buy, config.portfolio.base_currency
                );
                println!("总倍率: {:.2}x", kelly_preview.total_multiplier);
                println!(
                    "全局风险: {} ({:.1})",
                    kelly_preview.global_risk_label, kelly_preview.global_risk_score
                );

                println!("\n资产调整详情:");
                println!(
                    "{:<20} | {:>12} | {:>12} | {:>8} | 状态",
                    "资产ID", "基础金额", "Kelly预览", "倍率"
                );
                println!("{:-<80}", "");
                for res in &kelly_preview.results {
                    println!(
                        "{:<20} | {:>12.2} | {:>12.2} | {:>8.2}x | {}",
                        res.asset_id,
                        res.base_suggested_buy,
                        res.capped_preview_buy_amount,
                        res.kelly_multiplier,
                        res.status
                    );
                }

                if !kelly_preview.warnings.is_empty() {
                    println!("\n警告:");
                    for w in &kelly_preview.warnings {
                        println!("- {}", w);
                    }
                }
            }
            cli::KellyCommands::Explain { asset_id } => {
                let asset = config.assets.iter().find(|a| a.asset_id == *asset_id);
                if let Some(a) = asset {
                    let market_provider =
                        api::create_market_provider(&config.market, Some("yahoo"));
                    let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
                        &config.risk,
                        &config.regime,
                        market_provider.as_ref(),
                        fx_provider.as_ref(),
                    );
                    let date = Local::now().format("%Y-%m-%d").to_string();
                    let decision = engine::generate_buy_suggestions(&config, &state, date);

                    // Find base buy for this asset
                    let mut base_buy = 0.0;
                    for s in &decision.sector_suggestions {
                        if let Some(ad) = s
                            .asset_suggestions
                            .iter()
                            .find(|ad| ad.asset_id == *asset_id)
                        {
                            base_buy = ad.suggested_buy;
                        }
                    }

                    let mut regime = None;
                    if let Some(s) = &a.reference_index_symbol {
                        if let Ok(candles) = market_provider
                            .fetch_daily_candles(s, config.regime.default_lookback_days)
                        {
                            regime = Some(engine::regime::calculate_market_regime(
                                s,
                                &candles,
                                &config.regime,
                            ));
                        }
                    }

                    let res = engine::kelly::calculate_single_asset_kelly(engine::kelly::KellyContext {
                        config: &config,
                        asset_id: a.asset_id.clone(),
                        fund_code: a.fund_code.clone(),
                        fund_name: a.fund_name.clone(),
                        sector: a.sector.clone(),
                        base_suggested_buy: base_buy,
                        risk_overlay: &risk_overlay,
                        regime: regime.as_ref(),
                    });

                    println!("Kelly 计算详情: {}\n", asset_id);
                    println!("1. 基础建议买入额: {:.2}", res.base_suggested_buy);
                    println!(
                        "2. 市场周期状态: {} (钟摆分数 {:.1})",
                        res.market_regime_label, res.pendulum_score
                    );
                    println!(
                        "3. 全局风险评分: {} ({:.1})",
                        res.global_risk_label, res.global_risk_score
                    );
                    println!("4. 估算胜率 p: {:.2}", res.estimated_win_probability);
                    println!("5. 估算赔率 b: {:.2}", res.payoff_ratio);
                    println!("6. 原始 Kelly 分数 f*: {:.4}", res.raw_kelly_fraction);
                    println!(
                        "7. 分段 Kelly 分数 ({:.2}x): {:.4}",
                        config.kelly.fractional_kelly, res.fractional_kelly_fraction
                    );
                    println!("8. 最终倍率: {:.2}x", res.kelly_multiplier);
                    println!(
                        "9. 预览买入额: {:.2} (上限倍率 {:.2}x)",
                        res.capped_preview_buy_amount, config.kelly.max_single_asset_buy_multiplier
                    );
                    println!("\n计算路径: {}", res.explanation);
                    println!(
                        "\n警告: Kelly 参数基于模型估计，并非真实胜率。该结果仅用于仓位参考，不应被视为确定性预测。"
                    );
                } else {
                    println!("错误: 未找到资产 {}", asset_id);
                }
            }
        },
        Commands::Dca { command } => match command {
            cli::DcaCommands::Add {
                asset_id,
                amount,
                frequency,
                start_date,
                end_date,
                weekday,
                month_day,
                note,
                priority,
            } => {
                let asset = config.assets.iter().find(|a| a.asset_id == *asset_id);
                if let Some(a) = asset {
                    let mut plans = repo.load_plans(&ctx).await?;
                    let plan_id = format!("dca_{}", Local::now().timestamp_millis());
                    let freq = match frequency.to_lowercase().as_str() {
                        "daily" => models::DcaFrequency::Daily,
                        "weekly" => models::DcaFrequency::Weekly,
                        "monthly" => models::DcaFrequency::Monthly,
                        _ => {
                            println!(
                                "错误: 无效的频率 {}。可选: daily, weekly, monthly",
                                frequency
                            );
                            return Ok::<(), anyhow::Error>(());
                        }
                    };

                    if *amount <= 0.0 {
                        println!("错误: 定投金额必须大于 0");
                        return Ok::<(), anyhow::Error>(());
                    }

                    let start_dt = start_date
                        .clone()
                        .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());

                    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                    let plan = models::DcaPlan {
                        plan_id: plan_id.clone(),
                        asset_id: asset_id.clone(),
                        fund_code: a.fund_code.clone(),
                        fund_name: a.fund_name.clone(),
                        amount: *amount,
                        currency: "CNY".to_string(),
                        frequency: freq,
                        weekday: *weekday,
                        month_day: *month_day,
                        start_date: start_dt,
                        end_date: end_date.clone(),
                        enabled: true,
                        priority: *priority,
                        note: note.clone(),
                        created_at: now_str.clone(),
                        updated_at: now_str,
                    };

                    plans.push(plan);
                    repo.save_plans(&ctx, &plans).await?;
                    println!("成功添加定投计划: {}", plan_id);
                } else {
                    println!("错误: 未找到资产 {}", asset_id);
                }
            }
            cli::DcaCommands::List => {
                let plans = repo.load_plans(&ctx).await?;
                println!("定投计划列表\n");
                println!(
                    "{:<20} | {:<20} | {:<10} | {:>10} | {:<10} | {:<8} | {:<12} | 备注",
                    "计划ID", "资产ID", "基金代码", "金额", "频率", "状态", "开始日期"
                );
                println!("{:-<120}", "");
                for p in plans {
                    let freq_str = match p.frequency {
                        models::DcaFrequency::Daily => "每日".to_string(),
                        models::DcaFrequency::Weekly => {
                            format!("每周(周{})", p.weekday.unwrap_or(1))
                        }
                        models::DcaFrequency::Monthly => {
                            format!("每月({}日)", p.month_day.unwrap_or(1))
                        }
                    };
                    println!(
                        "{:<20} | {:<20} | {:<10} | {:>10.2} | {:<10} | {:<8} | {:<12} | {}",
                        p.plan_id,
                        p.asset_id,
                        p.fund_code,
                        p.amount,
                        freq_str,
                        if p.enabled { "启用" } else { "禁用" },
                        p.start_date,
                        p.note.unwrap_or_default()
                    );
                }
            }
            cli::DcaCommands::Preview { date } => {
                let plans = repo.load_plans(&ctx).await?;
                let nav_cache = repo.load_nav_cache(&ctx).await?;
                let target_date = date
                    .clone()
                    .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
                let preview = engine::dca::calculate_dca_preview(&config, &plans, &nav_cache, &target_date);

                println!("定投预览: {}\n", target_date);
                println!(
                    "{:<20} | {:<10} | {:>10} | {:<10} | {:<10} | 警告",
                    "资产ID", "基金代码", "金额", "频率", "状态"
                );
                println!("{:-<100}", "");
                for item in preview.items {
                    let freq_str = match item.frequency {
                        models::DcaFrequency::Daily => "每日",
                        models::DcaFrequency::Weekly => "每周",
                        models::DcaFrequency::Monthly => "每月",
                    };
                    println!(
                        "{:<20} | {:<10} | {:>10.2} | {:<10} | {:<10} | {}",
                        item.asset_id,
                        item.fund_code,
                        item.amount,
                        freq_str,
                        item.status,
                        item.warnings.join(", ")
                    );
                }
                println!("\n今日应投总额: {:.2} CNY", preview.total_due_amount);
                if !preview.warnings.is_empty() {
                    println!("\n警告:");
                    for w in preview.warnings {
                        println!("- {}", w);
                    }
                }
            }
            cli::DcaCommands::Disable { plan_id } => {
                let mut plans = repo.load_plans(&ctx).await?;
                if let Some(p) = plans.iter_mut().find(|p| p.plan_id == *plan_id) {
                    p.enabled = false;
                    repo.save_plans(&ctx, &plans).await?;
                    println!("已禁用定投计划: {}", plan_id);
                } else {
                    println!("错误: 未找到计划 {}", plan_id);
                }
            }
            cli::DcaCommands::Enable { plan_id } => {
                let mut plans = repo.load_plans(&ctx).await?;
                if let Some(p) = plans.iter_mut().find(|p| p.plan_id == *plan_id) {
                    p.enabled = true;
                    repo.save_plans(&ctx, &plans).await?;
                    println!("已启用定投计划: {}", plan_id);
                } else {
                    println!("错误: 未找到计划 {}", plan_id);
                }
            }
            cli::DcaCommands::Remove { plan_id } => {
                let mut plans = repo.load_plans(&ctx).await?;
                let len_before = plans.len();
                plans.retain(|p| p.plan_id != *plan_id);
                if plans.len() < len_before {
                    repo.save_plans(&ctx, &plans).await?;
                    println!("已删除定投计划: {}", plan_id);
                } else {
                    println!("错误: 未找到计划 {}", plan_id);
                }
            }
            cli::DcaCommands::CompareDecision => {
                let plans = repo.load_plans(&ctx).await?;
                let nav_cache = repo.load_nav_cache(&ctx).await?;
                let date = Local::now().format("%Y-%m-%d").to_string();
                let dca_preview = engine::dca::calculate_dca_preview(&config, &plans, &nav_cache, &date);

                let decision = engine::generate_buy_suggestions(&config, &state, date.clone());

                let market_provider = api::create_market_provider(&config.market, Some("yahoo"));
                let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
                    &config.risk,
                    &config.regime,
                    market_provider.as_ref(),
                    fx_provider.as_ref(),
                );

                let instruments = repo.load_instruments(&ctx).await.unwrap_or_default();
                let mut regimes = std::collections::HashMap::new();
                for asset in &config.assets {
                    let symbol_opt = if let Some(rid) = &asset.reference_instrument_id {
                        instruments
                            .iter()
                            .find(|i| i.instrument_id == *rid)
                            .map(|i| i.provider_symbol.clone())
                    } else {
                        asset
                            .reference_instrument_symbol
                            .clone()
                            .or(asset.reference_index_symbol.clone())
                    };

                    if let Some(s) = symbol_opt {
                        if let Ok(candles) = market_provider
                            .fetch_daily_candles(&s, config.regime.default_lookback_days)
                        {
                            let regime = engine::regime::calculate_market_regime(
                                &s,
                                &candles,
                                &config.regime,
                            );
                            regimes.insert(asset.asset_id.clone(), regime);
                        }
                    }
                }

                let kelly_preview = engine::kelly::calculate_kelly_preview(
                    &config,
                    &decision,
                    &risk_overlay,
                    &regimes,
                );

                let adjusted = engine::adjusted_decision::calculate_adjusted_decision(
                    &config,
                    &state,
                    &decision,
                    &risk_overlay,
                    &regimes,
                );

                println!("定投与决策建议对比 ({})\n", date);
                println!("{:<25} | {:>15}", "方案", "建议买入总额");
                println!("{:-<45}", "");
                println!(
                    "{:<25} | {:>15.2}",
                    "DCA 定投计划", dca_preview.total_due_amount
                );
                println!(
                    "{:<25} | {:>15.2}",
                    "基础决策建议", decision.suggested_total_buy
                );
                println!(
                    "{:<25} | {:>15.2}",
                    "Kelly 预览", kelly_preview.preview_total_buy
                );
                println!(
                    "{:<25} | {:>15.2}",
                    "风险调整建议", adjusted.adjusted_total_buy
                );

                if !dca_preview.warnings.is_empty() {
                    println!("\nDCA 警告:");
                    for w in dca_preview.warnings {
                        println!("- {}", w);
                    }
                }
            }
            cli::DcaCommands::Lifecycle { date, asset_id } => {
                let target_date = date
                    .clone()
                    .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
                let plans = repo.load_plans(&ctx).await?;
                let settlements = repo.load_settlements(&ctx).await?;
                let snapshots = repo.load_alipay_snapshots(&ctx).await?;
                let nav_cache = repo.load_nav_cache(&ctx).await?;

                let summary = engine::calculate_dca_lifecycle(
                    &config,
                    &plans,
                    &settlements,
                    &snapshots,
                    &state,
                    &nav_cache,
                    &target_date,
                );

                display_dca_lifecycle_summary(&summary, asset_id.as_deref());
            }
            cli::DcaCommands::Pending => {
                let target_date = Local::now().format("%Y-%m-%d").to_string();
                let plans = repo.load_plans(&ctx).await?;
                let settlements = repo.load_settlements(&ctx).await?;
                let snapshots = repo.load_alipay_snapshots(&ctx).await?;
                let nav_cache = repo.load_nav_cache(&ctx).await?;

                let summary = engine::calculate_dca_lifecycle(
                    &config,
                    &plans,
                    &settlements,
                    &snapshots,
                    &state,
                    &nav_cache,
                    &target_date,
                );

                println!("待处理定投事项 (今日: {})\n", target_date);
                let pending_items: Vec<_> = summary
                    .items
                    .iter()
                    .filter(|i| {
                        i.suggested_next_action != "无需处理" && i.lifecycle_status != "已暂停"
                    })
                    .collect();

                if pending_items.is_empty() {
                    println!("所有定投项目均已闭环，暂无待处理事项。");
                } else {
                    println!(
                        "{:<20} | {:<15} | {:<10} | 建议操作",
                        "资产ID", "生命周期状态", "定投金额"
                    );
                    println!("{:-<80}", "");
                    for i in pending_items {
                        println!(
                            "{:<20} | {:<15} | {:>10.2} | {}",
                            i.asset_id,
                            i.lifecycle_status,
                            i.planned_amount,
                            i.suggested_next_action
                        );
                    }
                }
            }
            cli::DcaCommands::LifecycleExplain { asset_id, date } => {
                let target_date = date
                    .clone()
                    .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
                let plans = repo.load_plans(&ctx).await?;
                let settlements = repo.load_settlements(&ctx).await?;
                let snapshots = repo.load_alipay_snapshots(&ctx).await?;
                let nav_cache = repo.load_nav_cache(&ctx).await?;

                let summary = engine::calculate_dca_lifecycle(
                    &config,
                    &plans,
                    &settlements,
                    &snapshots,
                    &state,
                    &nav_cache,
                    &target_date,
                );

                if let Some(i) = summary.items.iter().find(|i| i.asset_id == *asset_id) {
                    println!("定投生命周期详解: {} ({})\n", i.fund_name, i.asset_id);
                    println!(
                        "1. 定投计划: {}",
                        if i.plan_id.is_some() {
                            format!("活跃 ({:.2} CNY)", i.planned_amount)
                        } else {
                            "无".to_string()
                        }
                    );
                    println!(
                        "2. 今日是否应投: {}",
                        if i.lifecycle_status == "今日应定投" || i.settlement_id.is_some() {
                            "是"
                        } else {
                            "否"
                        }
                    );
                    println!(
                        "3. 确认结算单: {}",
                        i.settlement_id.as_deref().unwrap_or("未找到")
                    );
                    println!(
                        "4. 是否已入账: {}",
                        if i.settlement_applied { "是" } else { "否" }
                    );
                    println!(
                        "5. 支付宝快照: {}",
                        i.latest_alipay_snapshot_id.as_deref().unwrap_or("未找到")
                    );
                    println!("6. 对账状态: {}", i.reconciliation_status);
                    println!("7. 建议操作: {}", i.suggested_next_action);
                    println!("\n当前生命周期阶段: {}", i.lifecycle_status);
                    println!("\n(提示: 该命令为只读查询，不会修改任何数据)");
                } else {
                    println!("错误: 未找到资产 {}", asset_id);
                }
            }
            cli::DcaCommands::Settlement { command } => match command {
                cli::DcaSettlementCommands::Add {
                    asset_id,
                    amount,
                    confirmed_nav,
                    confirmed_units,
                    deduction_date,
                    confirmation_date,
                    plan_id,
                    fee,
                    note,
                } => {
                    let asset = config.assets.iter().find(|a| a.asset_id == *asset_id);
                    if let Some(a) = asset {
                        let mut settlements = repo.load_settlements(&ctx).await?;

                        if *amount <= 0.0 || *confirmed_nav <= 0.0 || *confirmed_units <= 0.0 {
                            println!("错误: 金额、净值和份额必须大于 0");
                            return Ok::<(), anyhow::Error>(());
                        }

                        let settlement_id = format!("settle_{}", Local::now().timestamp_millis());
                        let settlement = models::DcaSettlement {
                            settlement_id: settlement_id.clone(),
                            plan_id: plan_id.clone(),
                            asset_id: asset_id.clone(),
                            fund_code: a.fund_code.clone(),
                            fund_name: a.fund_name.clone(),
                            scheduled_date: None,
                            deduction_date: deduction_date.clone(),
                            confirmation_date: confirmation_date.clone(),
                            amount: *amount,
                            confirmed_nav: *confirmed_nav,
                            confirmed_units: *confirmed_units,
                            fee: *fee,
                            currency: "CNY".to_string(),
                            source: "alipay".to_string(),
                            status: models::DcaSettlementStatus::Confirmed,
                            applied: false,
                            note: note.clone(),
                            created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                        };

                        // Optional plan validation
                        if let Some(pid) = plan_id {
                            let plans = repo.load_plans(&ctx).await?;
                            if let Some(p) = plans.iter().find(|p| p.plan_id == *pid) {
                                if (p.amount - amount).abs() > 0.01 {
                                    println!(
                                        "警告: 实际扣款金额 ({:.2}) 与定投计划金额 ({:.2}) 不一致。",
                                        amount, p.amount
                                    );
                                }
                            } else {
                                println!("警告: 未找到关联的定投计划 {}。", pid);
                            }
                        } else {
                            println!("注意: 未关联定投计划。");
                        }

                        settlements.push(settlement);
                        repo.save_settlements(&ctx, &settlements).await?;
                        println!("成功添加定投确认记录: {}", settlement_id);
                    } else {
                        println!("错误: 未找到资产 {}", asset_id);
                    }
                }
                cli::DcaSettlementCommands::List => {
                    let settlements = repo.load_settlements(&ctx).await?;
                    println!("定投确认记录列表\n");
                    println!(
                        "{:<20} | {:<20} | {:>10} | {:>10} | {:>10} | {:<12} | {:<12} | {:<6} | 备注",
                        "结算ID",
                        "资产ID",
                        "金额",
                        "净值",
                        "份额",
                        "扣款日期",
                        "确认日期",
                        "已应用"
                    );
                    println!("{:-<150}", "");
                    for s in settlements {
                        println!(
                            "{:<20} | {:<20} | {:>10.2} | {:>10.4} | {:>10.4} | {:<12} | {:<12} | {:<6} | {}",
                            s.settlement_id,
                            s.asset_id,
                            s.amount,
                            s.confirmed_nav,
                            s.confirmed_units,
                            s.deduction_date,
                            s.confirmation_date,
                            if s.applied { "是" } else { "否" },
                            s.note.unwrap_or_default()
                        );
                    }
                }
                cli::DcaSettlementCommands::Preview { settlement_id } => {
                    let settlements = repo.load_settlements(&ctx).await?;
                    if let Some(s) = settlements
                        .iter()
                        .find(|s| s.settlement_id == *settlement_id)
                    {
                        let impact =
                            engine::dca_settlement::calculate_settlement_impact(&config, &state, s);
                        println!("定投确认影响预览: {}\n", settlement_id);
                        println!("资产: {} ({})", impact.fund_name, impact.asset_id);
                        println!("金额: {:.2} CNY", impact.amount);
                        println!("确认净值: {:.4}", impact.confirmed_nav);
                        println!("确认份额: {:.4}", impact.confirmed_units);
                        println!(
                            "\n维度                   |              当前 |             入账后 |              变化"
                        );
                        println!("{:-<80}", "");
                        println!(
                            "{:<20} | {:>15.4} | {:>15.4} | {:>15.4}",
                            "份额", impact.old_units, impact.new_units, impact.confirmed_units
                        );
                        println!(
                            "{:<20} | {:>15.4} | {:>15.4} | {:>15.4}",
                            "成本价格",
                            impact.old_cost_basis,
                            impact.new_cost_basis,
                            impact.new_cost_basis - impact.old_cost_basis
                        );
                        println!(
                            "{:<20} | {:>15.2} | {:>15.2} | {:>15.2}",
                            "估算市值",
                            impact.old_market_value,
                            impact.estimated_new_market_value,
                            impact.estimated_new_market_value - impact.old_market_value
                        );

                        if !impact.warnings.is_empty() {
                            println!("\n警告:");
                            for w in impact.warnings {
                                println!("- {}", w);
                            }
                        }
                        println!("\n提示: 该结果仅为预览。使用 apply --confirm 执行更新。");
                    } else {
                        println!("错误: 未找到记录 {}", settlement_id);
                    }
                }
                cli::DcaSettlementCommands::Apply {
                    settlement_id,
                    confirm,
                } => {
                    let mut settlements = repo.load_settlements(&ctx).await?;
                    let index = settlements
                        .iter()
                        .position(|s| s.settlement_id == *settlement_id);

                    if let Some(idx) = index {
                        if settlements[idx].applied {
                            println!("提示: 该定投记录已在之前应用，无需重复操作。");
                            return Ok::<(), anyhow::Error>(());
                        }

                        let impact = engine::dca_settlement::calculate_settlement_impact(
                            &config,
                            &state,
                            &settlements[idx],
                        );

                        if !confirm {
                            println!("待应用定投确认: {}\n", settlement_id);
                            println!("份额: {:.4} -> {:.4}", impact.old_units, impact.new_units);
                            println!(
                                "成本: {:.4} -> {:.4}",
                                impact.old_cost_basis, impact.new_cost_basis
                            );
                            println!("\n请添加 --confirm 参数执行应用。");
                            return Ok::<(), anyhow::Error>(());
                        }

                        // Apply to state
                        let mut new_state = state.clone();
                        let holding = new_state
                            .asset_holdings
                            .iter_mut()
                            .find(|h| h.asset_id == impact.asset_id);

                        let s = &settlements[idx];
                        let mut audit = models::DcaSettlementAudit {
                            audit_id: format!("audit_dca_{}", Local::now().timestamp_millis()),
                            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                            settlement_id: settlement_id.clone(),
                            asset_id: impact.asset_id.clone(),
                            old_units: impact.old_units,
                            new_units: impact.new_units,
                            old_cost_basis: impact.old_cost_basis,
                            new_cost_basis: impact.new_cost_basis,
                            transaction_id: None,
                            note: s.note.clone(),
                        };

                        if let Some(h) = holding {
                            h.units = impact.new_units;
                            h.cost_basis = impact.new_cost_basis;
                            h.last_market_value = impact.estimated_new_market_value;
                        } else {
                            // Create new holding
                            new_state.asset_holdings.push(models::AssetHolding {
                                asset_id: impact.asset_id.clone(),
                                fund_code: impact.fund_code.clone(),
                                units: impact.new_units,
                                units_estimated: false,
                                cost_basis: impact.new_cost_basis,
                                last_market_value: impact.estimated_new_market_value,
                                latest_nav: Some(impact.confirmed_nav),
                                latest_nav_date: Some(s.confirmation_date.clone()),
                                latest_nav_source: None,
                                latest_nav_status: None,
                            });
                        }

                        // Create transaction
                        let mut transactions = repo.load_transactions(&ctx).await?;
                        let tx_id = format!("tx_dca_{}", Local::now().timestamp_millis());
                        transactions.push(models::Transaction {
                            id: tx_id.clone(),
                            date: s.deduction_date.clone(),
                            asset_id: Some(impact.asset_id.clone()),
                            transaction_type: "buy".to_string(),
                            amount: impact.amount,
                            units: Some(impact.confirmed_units),
                            price: Some(impact.confirmed_nav),
                            fee: s.fee.unwrap_or(0.0),
                            currency: "CNY".to_string(),
                            note: s.note.clone().unwrap_or_default(),
                            source: "dca_settlement".to_string(),
                            raw_description: "".to_string(),
                        });
                        audit.transaction_id = Some(tx_id);

                        // Save all
                        repo.save_state(&ctx, &new_state).await?;
                        repo.save_transactions(&ctx, &transactions).await?;

                        let mut audits = repo.load_settlement_audits(&ctx).await?;
                        audits.push(audit);
                        repo.save_settlement_audits(&ctx, &audits).await?;

                        settlements[idx].applied = true;
                        repo.save_settlements(&ctx, &settlements).await?;

                        println!(
                            "成功应用定投确认并更新持仓。审计记录 ID: {}",
                            audits.last().unwrap().audit_id
                        );
                    } else {
                        println!("错误: 未找到定投记录 {}", settlement_id);
                    }
                }
                cli::DcaSettlementCommands::CompareAlipay { settlement_id } => {
                    let settlements = repo.load_settlements(&ctx).await?;
                    if let Some(s) = settlements
                        .iter()
                        .find(|s| s.settlement_id == *settlement_id)
                    {
                        let impact =
                            engine::dca_settlement::calculate_settlement_impact(&config, &state, s);

                        let snapshots = repo.load_alipay_snapshots(&ctx).await?;
                        let latest_snap = snapshots
                            .iter()
                            .filter(|sn| sn.asset_id == s.asset_id)
                            .max_by_key(|sn| sn.snapshot_date.clone());

                        println!("定投确认与支付宝对账对比: {}\n", settlement_id);
                        if let Some(snap) = latest_snap {
                            println!("最新支付宝快照日期: {}", snap.snapshot_date);
                            println!(
                                "\n维度                   |          入账后(预估) |             支付宝 |              差异"
                            );
                            println!("{:-<80}", "");

                            let units_diff = impact.new_units - snap.units.unwrap_or(0.0);
                            println!(
                                "{:<20} | {:>15.4} | {:>15.4} | {:>15.4}",
                                "份额",
                                impact.new_units,
                                snap.units.unwrap_or(0.0),
                                units_diff
                            );

                            let mv_diff = impact.estimated_new_market_value - snap.market_value;
                            println!(
                                "{:<20} | {:>15.2} | {:>15.2} | {:>15.2}",
                                "市值",
                                impact.estimated_new_market_value,
                                snap.market_value,
                                mv_diff
                            );

                            if units_diff.abs() > 0.01 {
                                println!(
                                    "\n警告: 入账后的份额与支付宝快照不符，请检查是否存在其他未记录的交易。"
                                );
                            } else {
                                println!("\n结果: 份额与支付宝快照一致。");
                            }
                        } else {
                            println!("警告: 缺少支付宝快照，无法确认入账后是否与支付宝一致。");
                        }
                    } else {
                        println!("错误: 未找到记录 {}", settlement_id);
                    }
                }
                cli::DcaSettlementCommands::Remove { settlement_id } => {
                    let mut settlements = repo.load_settlements(&ctx).await?;
                    let index = settlements
                        .iter()
                        .position(|s| s.settlement_id == *settlement_id);

                    if let Some(idx) = index {
                        if settlements[idx].applied {
                            println!("错误: 已应用的记录无法删除。");
                        } else {
                            settlements.remove(idx);
                            repo.save_settlements(&ctx, &settlements).await?;
                            println!("已删除定投记录: {}", settlement_id);
                        }
                    } else {
                        println!("错误: 未找到记录 {}", settlement_id);
                    }
                }
            },
        },
        Commands::Reconcile { command } => match command {
            cli::ReconcileCommands::Alipay { command } => match command {
                cli::AlipayReconcileCommands::Add {
                    asset_id,
                    date,
                    market_value,
                    units,
                    cost_basis,
                    nav,
                    nav_date,
                    daily_pnl,
                    total_pnl,
                    note,
                } => {
                    let asset = config.assets.iter().find(|a| a.asset_id == *asset_id);
                    if let Some(a) = asset {
                        let mut snapshots = repo.load_alipay_snapshots(&ctx).await?;
                        let snapshot_id = format!("snap_{}", Local::now().timestamp_millis());

                        if *market_value < 0.0 {
                            println!("错误: 市值不能为负数");
                            return Ok::<(), anyhow::Error>(());
                        }

                        let snapshot = models::AlipaySnapshot {
                            snapshot_id: snapshot_id.clone(),
                            asset_id: asset_id.clone(),
                            fund_code: a.fund_code.clone(),
                            fund_name: a.fund_name.clone(),
                            snapshot_date: date.clone(),
                            market_value: *market_value,
                            units: *units,
                            cost_basis: *cost_basis,
                            nav: *nav,
                            nav_date: nav_date.clone(),
                            daily_pnl: *daily_pnl,
                            total_pnl: *total_pnl,
                            source: "alipay".to_string(),
                            created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                            note: note.clone(),
                        };

                        snapshots.push(snapshot);
                        repo.save_alipay_snapshots(&ctx, &snapshots).await?;
                        println!("成功添加支付宝对账快照: {}", snapshot_id);
                    } else {
                        println!("错误: 未找到资产 {}", asset_id);
                    }
                }
                cli::AlipayReconcileCommands::List => {
                    let snapshots = repo.load_alipay_snapshots(&ctx).await?;
                    println!("支付宝对账快照列表\n");
                    println!(
                        "{:<20} | {:<20} | {:>12} | {:<12} | {:>10} | {:>10} | 备注",
                        "快照ID", "资产ID", "市值", "日期", "份额", "成本"
                    );
                    println!("{:-<120}", "");
                    for s in snapshots {
                        println!(
                            "{:<20} | {:<20} | {:>12.2} | {:<12} | {:>10.2} | {:>10.2} | {}",
                            s.snapshot_id,
                            s.asset_id,
                            s.market_value,
                            s.snapshot_date,
                            s.units.unwrap_or(0.0),
                            s.cost_basis.unwrap_or(0.0),
                            s.note.unwrap_or_default()
                        );
                    }
                }
                cli::AlipayReconcileCommands::Compare { asset_id, date } => {
                    let snapshots = repo.load_alipay_snapshots(&ctx).await?;
                    let snapshot = if let Some(d) = date {
                        snapshots
                            .iter()
                            .find(|s| s.asset_id == *asset_id && s.snapshot_date == *d)
                    } else {
                        snapshots
                            .iter()
                            .filter(|s| s.asset_id == *asset_id)
                            .max_by_key(|s| s.snapshot_date.clone())
                    };

                    if let Some(s) = snapshot {
                        let result = engine::reconciliation::reconcile_asset(&config, &state, s);
                        println!("支付宝对账结果: {} ({})\n", asset_id, s.snapshot_date);
                        println!("状态: {}", result.status);
                        println!("建议操作: {}\n", result.suggested_action);

                        println!(
                            "{:<20} | {:>15} | {:>15} | {:>15}",
                            "维度", "系统", "支付宝", "差异"
                        );
                        println!("{:-<70}", "");
                        println!(
                            "{:<20} | {:>15.2} | {:>15.2} | {:>15.2} ({:.2}%)",
                            "市值",
                            result.system_market_value,
                            result.alipay_market_value,
                            result.market_value_diff,
                            result.market_value_diff_pct * 100.0
                        );
                        if let (Some(su), Some(au), Some(ud)) =
                            (result.system_units, result.alipay_units, result.units_diff)
                        {
                            println!(
                                "{:<20} | {:>15.4} | {:>15.4} | {:>15.4}",
                                "份额", su, au, ud
                            );
                        }
                        if let (Some(sc), Some(ac), Some(cd)) = (
                            result.system_cost_basis,
                            result.alipay_cost_basis,
                            result.cost_basis_diff,
                        ) {
                            println!(
                                "{:<20} | {:>15.2} | {:>15.2} | {:>15.2}",
                                "成本", sc, ac, cd
                            );
                        }

                        if !result.warnings.is_empty() {
                            println!("\n警告:");
                            for w in result.warnings {
                                println!("- {}", w);
                            }
                        }

                        // DCA check
                        let dca_plans = repo.load_plans(&ctx).await?;
                        if dca_plans
                            .iter()
                            .any(|p| p.asset_id == *asset_id && p.enabled)
                        {
                            println!(
                                "\n注意: 该资产有活跃的定投计划，差异可能由定投确认延迟引起。"
                            );
                        }
                    } else {
                        println!("错误: 未找到资产 {} 的快照", asset_id);
                    }
                }
                cli::AlipayReconcileCommands::CompareAll => {
                    let snapshots = repo.load_alipay_snapshots(&ctx).await?;
                    let mut latest_snaps = std::collections::HashMap::new();
                    for s in snapshots {
                        let entry = latest_snaps.entry(s.asset_id.clone()).or_insert(s.clone());
                        if s.snapshot_date >= entry.snapshot_date {
                            *entry = s;
                        }
                    }

                    println!("全资产支付宝对账概览\n");
                    println!(
                        "{:<20} | {:<12} | {:>12} | {:>12} | {:>12} | 状态",
                        "资产ID", "日期", "系统市值", "支付宝市值", "差异"
                    );
                    println!("{:-<100}", "");

                    for asset in &config.assets {
                        if let Some(s) = latest_snaps.get(&asset.asset_id) {
                            let res = engine::reconciliation::reconcile_asset(&config, &state, s);
                            println!(
                                "{:<20} | {:<12} | {:>12.2} | {:>12.2} | {:>12.2} | {}",
                                asset.asset_id,
                                s.snapshot_date,
                                res.system_market_value,
                                res.alipay_market_value,
                                res.market_value_diff,
                                res.status
                            );
                        } else {
                            println!(
                                "{:<20} | {:<12} | {:>12} | {:>12} | {:>12} | 缺少支付宝数据",
                                asset.asset_id, "-", "-", "-", "-"
                            );
                        }
                    }
                }

                cli::AlipayReconcileCommands::Suggest { asset_id } => {
                    let snapshots = repo.load_alipay_snapshots(&ctx).await?;
                    let snapshot = snapshots
                        .iter()
                        .filter(|s| s.asset_id == *asset_id)
                        .max_by_key(|s| s.snapshot_date.clone());

                    if let Some(s) = snapshot {
                        let res = engine::reconciliation::reconcile_asset(&config, &state, s);
                        if let Some(suggest) =
                            engine::reconciliation::generate_calibration_suggestion(&res)
                        {
                            println!("校准建议: {}\n", asset_id);
                            println!("原因: {}", suggest.reason);
                            println!("风险等级: {}", suggest.risk_level);
                            if let Some(u) = suggest.suggested_units {
                                println!(
                                    "建议份额: {:.4} (当前: {:.4})",
                                    u,
                                    res.system_units.unwrap_or(0.0)
                                );
                            }
                            if let Some(c) = suggest.suggested_cost_basis {
                                println!(
                                    "建议成本: {:.2} (当前: {:.2})",
                                    c,
                                    res.system_cost_basis.unwrap_or(0.0)
                                );
                            }
                            println!("\n提示: 使用 apply 命令执行校准。");
                        } else {
                            println!("资产 {} 已对齐，无需校准。", asset_id);
                        }
                    } else {
                        println!("错误: 未找到快照");
                    }
                }
                cli::AlipayReconcileCommands::Apply {
                    snapshot_id,
                    confirm,
                    allow_calibration_apply,
                } => {
                    let snapshots = repo.load_alipay_snapshots(&ctx).await?;
                    let snapshot = snapshots.iter().find(|s| s.snapshot_id == *snapshot_id);

                    if let Some(s) = snapshot {
                        let res = engine::reconciliation::reconcile_asset(&config, &state, s);
                        if let Some(suggest) =
                            engine::reconciliation::generate_calibration_suggestion(&res)
                        {
                            if !confirm {
                                println!("待执行校准: {}\n", s.asset_id);
                                if let Some(u) = suggest.suggested_units {
                                    println!(
                                        "份额: {:.4} -> {:.4}",
                                        res.system_units.unwrap_or(0.0),
                                        u
                                    );
                                }
                                println!("\n请添加 --confirm 参数执行校准。");
                                return Ok::<(), anyhow::Error>(());
                            }

                            if !config.reconciliation.allow_calibration_apply
                                && !allow_calibration_apply
                            {
                                println!(
                                    "错误: 配置中禁止自动执行校准。请在 config.toml 中设置 allow_calibration_apply = true 或使用 --allow-calibration-apply 参数。"
                                );
                                return Ok::<(), anyhow::Error>(());
                            }

                            // Perform Apply
                            let mut new_state = state.clone();
                            let holding = new_state
                                .asset_holdings
                                .iter_mut()
                                .find(|h| h.asset_id == s.asset_id);

                            let mut audit = models::ReconciliationAudit {
                                audit_id: format!("audit_{}", Local::now().timestamp_millis()),
                                timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                                snapshot_id: snapshot_id.clone(),
                                asset_id: s.asset_id.clone(),
                                old_units: res.system_units.unwrap_or(0.0),
                                new_units: res.system_units.unwrap_or(0.0),
                                old_cost_basis: res.system_cost_basis.unwrap_or(0.0),
                                new_cost_basis: res.system_cost_basis.unwrap_or(0.0),
                                old_market_value: res.system_market_value,
                                new_market_value: s.market_value,
                                reason: suggest.reason.clone(),
                                note: None,
                            };

                            if let Some(h) = holding {
                                if let Some(u) = suggest.suggested_units {
                                    h.units = u;
                                    audit.new_units = u;
                                }
                                if let Some(c) = suggest.suggested_cost_basis {
                                    h.cost_basis = c;
                                    audit.new_cost_basis = c;
                                }
                                h.last_market_value = s.market_value;
                            } else {
                                // Create new holding
                                let new_h = models::AssetHolding {
                                    asset_id: s.asset_id.clone(),
                                    fund_code: s.fund_code.clone(),
                                    units: suggest.suggested_units.unwrap_or(0.0),
                                    units_estimated: false,
                                    cost_basis: suggest.suggested_cost_basis.unwrap_or(0.0),
                                    last_market_value: s.market_value,
                                    latest_nav: s.nav,
                                    latest_nav_date: s.nav_date.clone(),
                                    latest_nav_source: Some("alipay_reconcile".to_string()),
                                    latest_nav_status: Some("校准".to_string()),
                                };
                                new_state.asset_holdings.push(new_h);
                                audit.new_units = suggest.suggested_units.unwrap_or(0.0);
                                audit.new_cost_basis = suggest.suggested_cost_basis.unwrap_or(0.0);
                            }

                            repo.save_state(&ctx, &new_state).await?;

                            let mut audits = repo.load_reconciliation_audits(&ctx).await?;
                            audits.push(audit.clone());
                            repo.save_reconciliation_audits(&ctx, &audits).await?;

                            println!("成功执行校准!");
                            println!("审计ID: {}", audit.audit_id);
                        } else {
                            println!("无需校准。");
                        }
                    } else {
                        println!("错误: 未找到快照 {}", snapshot_id);
                    }
                }
                cli::AlipayReconcileCommands::Remove { snapshot_id } => {
                    let mut snapshots = repo.load_alipay_snapshots(&ctx).await?;
                    let len_before = snapshots.len();
                    snapshots.retain(|s| s.snapshot_id != *snapshot_id);
                    if snapshots.len() < len_before {
                        repo.save_alipay_snapshots(&ctx, &snapshots).await?;
                        println!("已删除对账快照: {}", snapshot_id);
                    } else {
                        println!("错误: 未找到快照 {}", snapshot_id);
                    }
                }
            },
            cli::ReconcileCommands::Preview => {
                let state = repo.load_state(&ctx).await?;
                let txs = repo.load_transactions(&ctx).await?;
                let report = crate::engine::reconcile_portfolio(&ctx.portfolio_id, &state, &txs);
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            cli::ReconcileCommands::Run => {
                let state = repo.load_state(&ctx).await?;
                let txs = repo.load_transactions(&ctx).await?;
                let report = crate::engine::reconcile_portfolio(&ctx.portfolio_id, &state, &txs);
                println!("Reconciliation run completed.");
                println!("Total issues found: {}", report.summary.total_issues);
                if report.summary.total_issues > 0 {
                    println!("Run `jdi reconcile report` or `jdi reconcile preview` for details.");
                }
            }
            cli::ReconcileCommands::Report => {
                let state = repo.load_state(&ctx).await?;
                let txs = repo.load_transactions(&ctx).await?;
                let report = crate::engine::reconcile_portfolio(&ctx.portfolio_id, &state, &txs);
                println!("=== Reconciliation Report ===");
                println!(
                    "Total Checked: {}",
                    report.summary.total_transactions_checked
                );
                println!("Total Issues: {}", report.summary.total_issues);
                println!("Critical Issues: {}", report.summary.critical_issues);
                println!("Warning Issues: {}", report.summary.warning_issues);
                for issue in report.issues {
                    println!("- {:?}", issue);
                }
            }
        },
        Commands::Alipay { command } => match command {
            cli::AlipayCommands::Holdings { command } => match command {
                cli::AlipayHoldingsCommands::Preview { file, date }
                | cli::AlipayHoldingsCommands::Align {
                    file,
                    date,
                    dry_run: true,
                    ..
                } => {
                    let content = std::fs::read_to_string(file)?;
                    let target_date = date
                        .clone()
                        .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
                    let config = repo.load_config(&ctx).await?;
                    let state = repo.load_state(&ctx).await?;

                    let candidates =
                        engine::alipay_holding::parse_alipay_holdings_from_csv(&content)?;
                    let preview = engine::alipay_holding::preview_alipay_holdings(
                        &config,
                        &state,
                        candidates,
                        &target_date,
                    );

                    println!("支付宝持仓导入预览 ({})\n", target_date);

                    if preview.total_rows == 0 {
                        println!("⚠️  未解析到任何持仓数据行。请检查 CSV 表头或文件编码。");
                        println!("支持的表头: fund_name, market_value, holding_profit, holding_profit_rate, source, 基金名称, 市值, 持有收益 等。");
                        return Ok::<(), anyhow::Error>(());
                    }

                    println!(
                        "{:<20} | {:<15} | {:>12} | {:>12} | 状态",
                        "基金名称", "匹配资产ID", "Alipay市值", "本地份额"
                    );
                    println!("{:-<90}", "");

                    for i in 0..preview.candidates.len() {
                        let c = &preview.candidates[i];
                        let matched = preview.matched_asset_ids[i].as_deref().unwrap_or("-");
                        let local = preview.system_units[i]
                            .map(|u| format!("{:.4}", u))
                            .unwrap_or_else(|| "-".to_string());

                        let status = if !preview.errors[i].is_empty() {
                            "ERROR"
                        } else if !preview.warnings[i].is_empty() {
                            "DIFF"
                        } else {
                            "OK"
                        };

                        println!(
                            "{:<20} | {:<15} | {:>12.2} | {:>12} | {}",
                            if c.fund_name.chars().count() > 10 {
                                format!("{}...", c.fund_name.chars().take(7).collect::<String>())
                            } else {
                                c.fund_name.clone()
                            },
                            matched,
                            c.market_value,
                            local,
                            status
                        );
                        for e in &preview.errors[i] {
                            println!("  [错误] {}", e);
                        }
                        for w in &preview.warnings[i] {
                            println!("  [警告] {}", w);
                        }
                    }

                    println!("\n数据统计:");
                    println!("- 总读取行数: {}", preview.total_rows);
                    println!("- 成功匹配行: {}", preview.valid_rows);
                    println!("- 未匹配行数: {}", preview.unmatched_rows);
                    if preview.invalid_rows > preview.unmatched_rows {
                        println!("- 异常行数:   {}", preview.invalid_rows);
                    }

                    println!("\n这是预览模式。使用 --apply 参数正式保存快照。");
                }
                cli::AlipayHoldingsCommands::Align {
                    file,
                    date,
                    apply,
                    dry_run: false,
                } => {
                    let content = std::fs::read_to_string(file)?;
                    let target_date = date
                        .clone()
                        .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
                    let config = repo.load_config(&ctx).await?;
                    let state = repo.load_state(&ctx).await?;

                    let candidates =
                        engine::alipay_holding::parse_alipay_holdings_from_csv(&content)?;
                    let preview = engine::alipay_holding::preview_alipay_holdings(
                        &config,
                        &state,
                        candidates,
                        &target_date,
                    );

                    if *apply {
                        let snapshots = engine::alipay_holding::convert_to_snapshots(&preview);
                        if snapshots.is_empty() {
                            println!("\n没有有效的快照可导入。");
                        } else {
                            let mut existing = repo.load_alipay_snapshots(&ctx).await?;
                            existing.extend(snapshots.clone());
                            repo.save_alipay_snapshots(&ctx, &existing).await?;
                            println!("\n成功导入 {} 笔快照。", snapshots.len());
                        }
                    } else {
                        println!("\n请指定 --apply 参数以执行导入，或使用 --dry-run 进行预览。");
                    }
                }
                cli::AlipayHoldingsCommands::BootstrapAssets { file } => {
                    let content = std::fs::read_to_string(file)?;
                    let mut config = repo.load_config(&ctx).await?;
                    let candidates =
                        engine::alipay_holding::parse_alipay_holdings_from_csv(&content)?;

                    let (created, skipped, failed) =
                        engine::alipay_holding::bootstrap_assets_from_holdings(
                            &mut config,
                            &candidates,
                        );

                    if created > 0 {
                        repo.save_config(&ctx, &config).await?;
                    }

                    println!("支付宝资产配置初始化完成:");
                    println!("- 新增资产配置: {}", created);
                    println!("- 跳过(已存在): {}", skipped);
                    if failed > 0 {
                        println!("- 失败行数:     {}", failed);
                    }
                    if created > 0 {
                        println!("\n已更新本地资产配置。您现在可以运行 preview 或 align 命令。");
                    }
                }
                cli::AlipayHoldingsCommands::BootstrapLocal {
                    file,
                    dry_run,
                    apply,
                    replace_existing,
                } => {
                    let content = std::fs::read_to_string(file)?;
                    let mut config = repo.load_config(&ctx).await?;
                    let state = repo.load_state(&ctx).await?;
                    let mut nav_cache = repo.load_nav_cache(&ctx).await?;

                    let candidates =
                        engine::alipay_holding::parse_alipay_holdings_from_csv(&content)?;

                    // Ensure assets exist first
                    let (created, _, _) = engine::alipay_holding::bootstrap_assets_from_holdings(
                        &mut config,
                        &candidates,
                    );
                    if created > 0 {
                        repo.save_config(&ctx, &config).await?;
                    }

                    let mut nav_map = std::collections::HashMap::new();
                    for entry in &nav_cache.entries {
                        nav_map.insert(entry.fund_code.clone(), crate::models::fund::FundNav {
                            fund_code: entry.fund_code.clone(),
                            nav: entry.nav,
                            accumulated_nav: entry.accumulated_nav,
                            nav_date: entry.nav_date.clone(),
                            currency: entry.currency.clone(),
                            source: entry.source.clone(),
                            is_stale: false,
                            is_estimated: false,
                        });
                    }

                    // Initial preview to find missing NAVs
                    let preview = engine::alipay_holding::preview_bootstrap_local(
                        &config,
                        &state,
                        &candidates,
                        &nav_map,
                        *replace_existing,
                    );

                    let mut missing_nav_codes = vec![];
                    for row in &preview.rows {
                        if row.latest_nav.is_none() && !row.fund_code.is_empty() {
                            missing_nav_codes.push(row.fund_code.clone());
                        }
                    }

                    if !missing_nav_codes.is_empty() {
                        let provider = crate::api::create_fund_provider(&config.api);
                        for code in missing_nav_codes {
                            if let Ok(nav) = provider.fetch_latest_nav(&code) {
                                if let Some(entry) = nav_cache.entries.iter_mut().find(|e| e.fund_code == code) {
                                    entry.nav = nav.nav;
                                    entry.nav_date = nav.nav_date.clone();
                                    entry.fetched_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                                } else {
                                    nav_cache.entries.push(crate::models::NavCacheEntry {
                                        fund_code: code.clone(),
                                        nav: nav.nav,
                                        nav_date: nav.nav_date.clone(),
                                        accumulated_nav: None,
                                        currency: "CNY".to_string(),
                                        source: nav.source.clone(),
                                        fetched_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                                    });
                                }
                            }
                        }
                        repo.save_nav_cache(&ctx, &nav_cache).await?;
                    }

                    // Final preview
                    let mut nav_map = std::collections::HashMap::new();
                    for entry in &nav_cache.entries {
                        nav_map.insert(entry.fund_code.clone(), crate::models::fund::FundNav {
                            fund_code: entry.fund_code.clone(),
                            nav: entry.nav,
                            accumulated_nav: entry.accumulated_nav,
                            nav_date: entry.nav_date.clone(),
                            currency: entry.currency.clone(),
                            source: entry.source.clone(),
                            is_stale: false,
                            is_estimated: false,
                        });
                    }
                    let preview = engine::alipay_holding::preview_bootstrap_local(
                        &config,
                        &state,
                        &candidates,
                        &nav_map,
                        *replace_existing,
                    );

                    println!("支付宝持仓本地初始化预览\n");
                    println!(
                        "{:<15} | {:<20} | {:>12} | {:>12} | {:>12} | 动作",
                        "基金代码", "基金名称", "市值", "单位净值", "预估份额"
                    );
                    println!("{:-<100}", "");

                    for row in &preview.rows {
                        let nav_str = row
                            .latest_nav
                            .map(|n| format!("{:.4}", n))
                            .unwrap_or_else(|| "-".to_string());
                        let shares_str = row
                            .estimated_shares
                            .map(|s| format!("{:.2}", s))
                            .unwrap_or_else(|| "-".to_string());

                        let fund_name_short = if row.fund_name.chars().count() > 10 {
                            format!("{}...", row.fund_name.chars().take(7).collect::<String>())
                        } else {
                            row.fund_name.clone()
                        };

                        println!(
                            "{:<15} | {:<20} | {:>12.2} | {:>12} | {:>12} | {}",
                            row.fund_code,
                            fund_name_short,
                            row.market_value,
                            nav_str,
                            shares_str,
                            row.action.to_uppercase()
                        );
                        if let Some(w) = &row.warning {
                            println!("  [警告] {}", w);
                        }
                    }
                    println!("\n总预估市值: {:.2}", preview.total_bootstrapped_value);

                    if *apply && !*dry_run {
                        let (new_state, updated_count) =
                            engine::alipay_holding::apply_bootstrap_local(state, &preview);
                        repo.save_state(&ctx, &new_state).await?;
                        println!("\n成功初始化 {} 个本地持仓。", updated_count);
                    } else if *dry_run {
                        println!("\nDry run completed. No data was written.");
                    } else {
                        println!("\n请指定 --apply 参数以执行导入，或使用 --dry-run 进行预览。");
                    }
                }
            },
        },
        Commands::Instrument { command } => {
            let config = repo.load_config(&ctx).await?;
            let mut instruments = repo.load_instruments(&ctx).await?;

            match command {
                cli::InstrumentCommands::List => {
                    println!("市场标的注册表\n");
                    println!(
                        "{:<20} | {:<10} | {:<25} | {:<15} | {:<10} | {:<10} | {:<6}",
                        "标的ID", "代码", "中文名称", "类型", "提供商", "币种", "启用"
                    );
                    println!("{:-<115}", "");
                    for i in &instruments {
                        let name_zh = i.name_zh.as_deref().unwrap_or(&i.symbol);
                        let category_zh = i.category_zh.as_deref().unwrap_or("-");
                        println!(
                            "{:<20} | {:<10} | {:<25} | {:<15} | {:<10} | {:<10} | {:<6}",
                            i.instrument_id,
                            i.symbol,
                            name_zh,
                            category_zh,
                            i.provider,
                            i.currency,
                            if i.enabled { "是" } else { "否" }
                        );
                    }
                }
                cli::InstrumentCommands::Lookup {
                    symbol,
                    instrument_id,
                } => {
                    let search = symbol
                        .as_ref()
                        .or(instrument_id.as_ref())
                        .ok_or_else(|| anyhow!("请提供 symbol 或 --instrument-id"))?
                        .clone();
                    let quote = {
                        let config = config.clone();
                        let instruments = instruments.clone();
                        tokio::task::spawn_blocking(move || {
                            engine::instrument::lookup_instrument(
                                &config.market,
                                &instruments,
                                &search,
                            )
                        })
                        .await?
                    }?;
                    let display_name = quote.name_zh.as_deref().unwrap_or(&quote.name);
                    println!("标的行情: {} ({})\n", display_name, quote.symbol);
                    if let Some(cat) = quote.category_zh {
                        println!("标的类型: {}", cat);
                    }
                    println!("最新价格: {:.4} {}", quote.latest_price, quote.currency);
                    println!("报价单位: {}", quote.quote_unit);
                    println!("行情日期: {}", quote.latest_date);
                    println!("提供商: {} ({})", quote.provider, quote.source);
                    println!("状态: {}", quote.status);
                    if let Some(w) = quote.warning {
                        println!("警告: {}", w);
                    }
                }
                cli::InstrumentCommands::History {
                    symbol,
                    instrument_id,
                    days,
                } => {
                    let search = symbol
                        .as_ref()
                        .or(instrument_id.as_ref())
                        .ok_or_else(|| anyhow!("请提供 symbol 或 --instrument-id"))?
                        .clone();
                    let days = *days;
                    let history = {
                        let config = config.clone();
                        let instruments = instruments.clone();
                        let search_inner = search.clone();
                        tokio::task::spawn_blocking(move || {
                            engine::instrument::get_instrument_history(
                                &config.market,
                                &instruments,
                                &search_inner,
                                days,
                            )
                        })
                        .await?
                    }?;
                    let inst_opt = instruments
                        .iter()
                        .find(|i| i.instrument_id == search || i.symbol == search);
                    let display_name = inst_opt
                        .and_then(|i| i.name_zh.as_deref())
                        .unwrap_or(&search);

                    println!(
                        "标的历史行情: {} ({})\n",
                        display_name,
                        history.first().map(|c| c.symbol.as_str()).unwrap_or("")
                    );
                    println!(
                        "{:<12} | {:>10} | {:>10} | {:>10} | {:>10}",
                        "日期", "开盘", "最高", "最低", "收盘"
                    );
                    println!("{:-<60}", "");
                    for c in history.iter().rev().take(20) {
                        // Show last 20 entries
                        println!(
                            "{:<12} | {:>10.4} | {:>10.4} | {:>10.4} | {:>10.4}",
                            c.date,
                            c.open.unwrap_or(0.0),
                            c.high.unwrap_or(0.0),
                            c.low.unwrap_or(0.0),
                            c.close
                        );
                    }
                    if history.len() > 20 {
                        println!("... (省略 {} 条数据)", history.len() - 20);
                    }
                }
                cli::InstrumentCommands::Add(args) => {
                    let cli::AddInstrumentArgs {
                        instrument_id,
                        symbol,
                        name,
                        name_zh,
                        name_en,
                        description_zh,
                        category_zh,
                        display_label,
                        asset_class,
                        provider,
                        provider_symbol,
                        currency,
                        quote_unit,
                        price_unit,
                        market,
                        note,
                    } = &**args;
                    if instruments.iter().any(|i| i.instrument_id == *instrument_id) {
                        println!("错误: 标的ID {} 已存在", instrument_id);
                        return Ok::<(), anyhow::Error>(());
                    }

                    let ac = match asset_class.as_str() {
                        "spot_commodity" => models::AssetClass::SpotCommodity,
                        "futures" => models::AssetClass::Futures,
                        "index" => models::AssetClass::Index,
                        "etf" => models::AssetClass::Etf,
                        "fx" => models::AssetClass::Fx,
                        "crypto" => models::AssetClass::Crypto,
                        "rate" => models::AssetClass::Rate,
                        "fund" => models::AssetClass::Fund,
                        _ => models::AssetClass::Custom,
                    };

                    let new_i = models::InstrumentConfig {
                        instrument_id: instrument_id.clone(),
                        symbol: symbol.clone(),
                        display_symbol: Some(symbol.clone()),
                        name: name.clone(),
                        name_zh: name_zh.clone(),
                        name_en: name_en.clone(),
                        description_zh: description_zh.clone(),
                        category_zh: category_zh.clone(),
                        display_label: display_label.clone(),
                        asset_class: ac,
                        provider: provider.clone(),
                        provider_symbol: provider_symbol.clone(),
                        market: market.clone(),
                        exchange: None,
                        currency: currency.clone(),
                        quote_unit: quote_unit.clone(),
                        price_unit: price_unit.clone(),
                        timezone: None,
                        enabled: true,
                        archived: false,
                        priority: 0,
                        tags: vec![],
                        note: note.clone(),
                    };

                    instruments.push(new_i);
                    repo.save_instruments(&ctx, &instruments).await?;
                    println!("成功添加标的: {}", instrument_id);
                }
                cli::InstrumentCommands::Disable { instrument_id } => {
                    if let Some(i) = instruments
                        .iter_mut()
                        .find(|i| i.instrument_id == *instrument_id)
                    {
                        i.enabled = false;
                        repo.save_instruments(&ctx, &instruments).await?;
                        println!("已禁用标: {}", instrument_id);
                    } else {
                        println!("错误: 未找到标的 {}", instrument_id);
                    }
                }
                cli::InstrumentCommands::Enable { instrument_id } => {
                    if let Some(i) = instruments
                        .iter_mut()
                        .find(|i| i.instrument_id == *instrument_id)
                    {
                        i.enabled = true;
                        repo.save_instruments(&ctx, &instruments).await?;
                        println!("已启用标的: {}", instrument_id);
                    } else {
                        println!("错误: 未找到标的 {}", instrument_id);
                    }
                }
                cli::InstrumentCommands::Validate => {
                    println!("验证所有启用的标的...\n");
                    let results = {
                        let config = config.clone();
                        let instruments = instruments.clone();
                        tokio::task::spawn_blocking(move || {
                            engine::instrument::validate_instruments(&config.market, &instruments)
                        })
                        .await?
                    };
                    for (id, res) in results {
                        let i = instruments.iter().find(|i| i.instrument_id == id);
                        let warning = if let Some(inst) = i {
                            if inst.name_zh.is_none() {
                                " [缺少中文名称]"
                            } else {
                                ""
                            }
                        } else {
                            ""
                        };

                        match res {
                            Ok(quote) => println!(
                                "✓ {:<20} | {:<10} | {:>10.4} {}{}",
                                id, quote.symbol, quote.latest_price, quote.currency, warning
                            ),
                            Err(e) => println!("✗ {:<20} | 错误: {}{}", id, e, warning),
                        }
                    }
                }
                cli::InstrumentCommands::Snapshot => {
                    println!("市场标的快照 (Watchlist)\n");
                    println!(
                        "{:<10} | {:<25} | {:<15} | {:>12} | {:<8} | {:<10} | {:<10}",
                        "代码", "中文名称", "类型", "最新价格", "币种", "单位", "提供商"
                    );
                    println!("{:-<105}", "");
                    let mut cache = models::InstrumentQuoteCache::default();

                    let results = {
                        let config = config.clone();
                        let instruments = instruments.clone();
                        tokio::task::spawn_blocking(move || {
                            let mut res = Vec::new();
                            for i in &instruments {
                                if !i.enabled {
                                    continue;
                                }
                                let quote_res = engine::instrument::lookup_instrument(
                                    &config.market,
                                    &instruments,
                                    &i.instrument_id,
                                );
                                res.push((i.clone(), quote_res));
                            }
                            res
                        })
                        .await?
                    };

                    for (i, quote_res) in results {
                        let name_zh = i.name_zh.as_deref().unwrap_or(&i.symbol);
                        let category_zh = i.category_zh.as_deref().unwrap_or("-");

                        match quote_res {
                            Ok(q) => {
                                println!(
                                    "{:<10} | {:<25} | {:<15} | {:>12.4} | {:<8} | {:<10} | {:<10}",
                                    q.symbol,
                                    name_zh,
                                    category_zh,
                                    q.latest_price,
                                    q.currency,
                                    q.quote_unit,
                                    q.provider
                                );
                                cache.entries.push(models::InstrumentQuoteCacheEntry {
                                    instrument_id: q.instrument_id,
                                    symbol: q.symbol,
                                    name_zh: q.name_zh,
                                    price: q.latest_price,
                                    date: q.latest_date,
                                    currency: q.currency,
                                    quote_unit: q.quote_unit,
                                    provider: q.provider,
                                    source: q.source,
                                    status: q.status,
                                    fetched_at: Local::now()
                                        .format("%Y-%m-%d %H:%M:%S")
                                        .to_string(),
                                    warning: q.warning,
                                });
                            }
                            Err(_) => {
                                println!(
                                    "{:<10} | {:<25} | {:<15} | {:>12} | {:<8} | {:<10} | {:<10}",
                                    i.symbol,
                                    name_zh,
                                    category_zh,
                                    "-",
                                    i.currency,
                                    i.quote_unit,
                                    i.provider
                                );
                            }
                        }
                    }
                    cache.fetched_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                    let _ = repo.save_instrument_cache(&ctx, &cache).await;
                }
            }
        }
        Commands::Daily { command } => {
            let config = repo.load_config(&ctx).await?;
            let state = repo.load_state(&ctx).await?;
            let nav_cache = repo.load_nav_cache(&ctx).await?;

            let date = match command {
                cli::DailyCommands::Plan { date } => date
                    .clone()
                    .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string()),
                _ => Local::now().format("%Y-%m-%d").to_string(),
            };

            // 1. DCA
            let dca_plans = repo.load_plans(&ctx).await?;
            let dca_preview = engine::dca::calculate_dca_preview(&config, &dca_plans, &nav_cache, &date);

            // 2. Decision components
            let (risk_overlay, regimes) = {
                let config = config.clone();
                tokio::task::spawn_blocking(move || {
                    let fx_provider = api::create_fx_provider(&config.fx, None);
                    let market_provider = api::create_market_provider(&config.market, None);

                    let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
                        &config.risk,
                        &config.regime,
                        market_provider.as_ref(),
                        fx_provider.as_ref(),
                    );

                    let mut regimes = std::collections::HashMap::new();
                    for asset in &config.assets {
                        let symbol_opt = asset
                            .reference_instrument_symbol
                            .clone()
                            .or(asset.reference_index_symbol.clone());
                        if let Some(symbol) = symbol_opt.filter(|s| !regimes.contains_key(s)) {
                            if let Ok(candles) = market_provider
                                .fetch_daily_candles(&symbol, config.regime.default_lookback_days)
                            {
                                let regime = engine::regime::calculate_market_regime(
                                    &symbol,
                                    &candles,
                                    &config.regime,
                                );
                                regimes.insert(asset.asset_id.clone(), regime);
                            }
                        }
                    }
                    (risk_overlay, regimes)
                })
                .await?
            };

            let decision =
                engine::decision::generate_buy_suggestions(&config, &state, date.clone());

            let adjusted = engine::adjusted_decision::calculate_adjusted_decision(
                &config,
                &state,
                &decision,
                &risk_overlay,
                &regimes,
            );
            let kelly =
                engine::kelly::calculate_kelly_preview(&config, &decision, &risk_overlay, &regimes);

            // 3. Reconciliation
            let snapshots = repo.load_alipay_snapshots(&ctx).await?;
            let mut latest_snaps = std::collections::HashMap::new();
            for s in snapshots {
                let entry = latest_snaps.entry(s.asset_id.clone()).or_insert(s.clone());
                if s.snapshot_date >= entry.snapshot_date {
                    *entry = s;
                }
            }
            let mut reconciliation_results = Vec::new();
            for asset in &config.assets {
                if let Some(s) = latest_snaps.get(&asset.asset_id) {
                    reconciliation_results
                        .push(engine::reconciliation::reconcile_asset(&config, &state, s));
                }
            }

            let plan = engine::daily_plan::generate_daily_execution_plan(
                &config,
                &state,
                date.clone(),
                &dca_preview,
                &adjusted,
                &kelly,
                &reconciliation_results,
            );

            match command {
                cli::DailyCommands::Plan { .. } => {
                    println!("今日执行计划预览: {}\n", plan.date);
                    println!("可用现金: {:.2} CNY", plan.available_cash);
                    println!("单日买入上限: {:.2} CNY", plan.max_daily_buy);
                    println!("今日定投应投: {:.2} CNY", plan.total_dca_due);
                    println!("风险调整建议: {:.2} CNY", plan.total_adjusted_decision);
                    println!("最终预览建议: {:.2} CNY", plan.total_recommended_amount);
                    println!("全局风险: {}", plan.global_risk_label);
                    println!(
                        "警告数量: {}",
                        plan.warnings.len()
                            + plan.items.iter().map(|i| i.warnings.len()).sum::<usize>()
                    );
                    println!();

                    println!(
                        "{:<15} | {:<20} | {:<10} | {:>10} | {:>10} | {:>10} | {:>10} | {:<10} | {:<10} | 原因",
                        "赛道",
                        "资产",
                        "基金代码",
                        "定投",
                        "风险调整",
                        "Kelly",
                        "最终建议",
                        "对账",
                        "状态"
                    );
                    println!("{:-<150}", "");

                    for item in &plan.items {
                        println!(
                            "{:<15} | {:<20} | {:<10} | {:>10.2} | {:>10.2} | {:>10.2} | {:>10.2} | {:<10} | {:<10} | {}",
                            item.sector,
                            item.fund_name,
                            item.fund_code,
                            item.dca_due_amount,
                            item.adjusted_decision_amount,
                            item.kelly_preview_amount,
                            item.recommended_amount,
                            item.reconciliation_status,
                            item.status,
                            item.explanation
                        );
                        for w in &item.warnings {
                            println!("  [警告] {}", w);
                        }
                    }

                    if !plan.warnings.is_empty() {
                        println!("\n全局警告:");
                        for w in &plan.warnings {
                            println!("- {}", w);
                        }
                    }

                    println!(
                        "\n提示: 该计划仅为预览，不会自动执行买入，也不会修改持仓或交易记录。"
                    );
                }
                cli::DailyCommands::Summary => {
                    println!("今日执行计划摘要 ({})", plan.date);
                    println!("----------------------------------");
                    println!("定投应投总额: {:.2} CNY", plan.total_dca_due);
                    println!("风险调整总额: {:.2} CNY", plan.total_adjusted_decision);
                    println!("最终建议总额: {:.2} CNY", plan.total_recommended_amount);
                    let execute_count = plan
                        .items
                        .iter()
                        .filter(|i| i.recommended_amount > 0.0)
                        .count();
                    let pause_count = plan
                        .items
                        .iter()
                        .filter(|i| i.status == "暂停执行" || i.status == "等待对账")
                        .count();
                    println!("待执行项: {}", execute_count);
                    println!("已暂停项: {}", pause_count);

                    let settlements = repo.load_settlements(&ctx).await?;
                    let snapshots = repo.load_alipay_snapshots(&ctx).await?;
                    let lifecycle = engine::calculate_dca_lifecycle(
                        &config,
                        &dca_plans,
                        &settlements,
                        &snapshots,
                        &state,
                        &nav_cache,
                        &plan.date,
                    );
                    println!(
                        "定投闭环事项: {} 个待确认, {} 个待入账",
                        lifecycle.count_waiting_confirmation, lifecycle.count_unapplied
                    );
                }
                cli::DailyCommands::Explain { asset_id } => {
                    if let Some(item) = plan.items.iter().find(|i| i.asset_id == *asset_id) {
                        println!("每日计划详细说明: {} ({})", item.fund_name, plan.date);
                        println!("----------------------------------");
                        println!(
                            "1. 定投是否到期: {}",
                            if item.dca_due_amount > 0.0 {
                                "是"
                            } else {
                                "否"
                            }
                        );
                        println!("2. 定投应投金额: {:.2} CNY", item.dca_due_amount);
                        println!("3. 风险调整建议: {:.2} CNY", item.adjusted_decision_amount);
                        println!("4. Kelly 预览金额: {:.2} CNY", item.kelly_preview_amount);
                        println!("5. 支付宝对账状态: {}", item.reconciliation_status);
                        if let Some(w) = &item.reconciliation_warning {
                            println!("   对账警告: {}", w);
                        }
                        println!("6. 数据质量状态: {}", item.data_status);
                        println!("7. 最终建议金额: {:.2} CNY", item.recommended_amount);
                        println!("8. 执行状态: {}", item.status);
                        println!("9. 原因说明: {}", item.explanation);

                        if !item.warnings.is_empty() {
                            println!("\n警告信息:");
                            for w in &item.warnings {
                                println!("- {}", w);
                            }
                        }
                        println!(
                            "\n提示: 本建议结合了定投计划、风险调整模型和对账安全门，仅供参考。"
                        );
                    } else {
                        println!("错误: 在今日计划中未找到资产 {}", asset_id);
                    }
                }
                cli::DailyCommands::Checklist => {
                    println!("每日操作清单 (Checklist - {})\n", plan.date);

                    println!("1. 数据准备 [Data]");
                    println!("   [ ] 刷新行情与净值: cargo run -- data refresh --all");
                    println!("   [ ] 检查缓存状态:   cargo run -- data cache-status");

                    println!("\n2. 定投管理 [DCA Lifecycle]");
                    let settlements = repo.load_settlements(&ctx).await?;
                    let snapshots = repo.load_alipay_snapshots(&ctx).await?;
                    let lifecycle = engine::calculate_dca_lifecycle(
                        &config,
                        &dca_plans,
                        &settlements,
                        &snapshots,
                        &state,
                        &nav_cache,
                        &plan.date,
                    );

                    if lifecycle.count_waiting_confirmation > 0 {
                        println!(
                            "   [ ] 录入定投确认单 ({} 个待办): cargo run -- dca settlement add ...",
                            lifecycle.count_waiting_confirmation
                        );
                    } else {
                        println!("   [x] 定投确认单已全部录入");
                    }

                    if lifecycle.count_unapplied > 0 {
                        println!(
                            "   [ ] 定投单入账 ({} 个待办): cargo run -- dca settlement apply ...",
                            lifecycle.count_unapplied
                        );
                    } else {
                        println!("   [x] 定投单已全部入账");
                    }

                    println!("\n3. 持仓核对 [Reconciliation]");
                    let mismatch_count = lifecycle.count_attention_required;
                    if mismatch_count > 0 {
                        println!(
                            "   [ ] 处理对账差异 ({} 个资产): cargo run -- reconcile alipay compare-all",
                            mismatch_count
                        );
                    } else {
                        println!("   [x] 支付宝对账已通过");
                    }

                    println!("\n4. 今日执行 [Execution]");
                    let execute_count = plan
                        .items
                        .iter()
                        .filter(|i| i.recommended_amount > 0.0)
                        .count();
                    if execute_count > 0 {
                        println!(
                            "   [ ] 按照计划执行买入 ({} 笔): cargo run -- daily plan",
                            execute_count
                        );
                        println!("   [ ] 手动录入交易记录: cargo run -- tx add ...");
                    } else {
                        println!("   [x] 今日无需额外买入");
                    }

                    println!("\n5. 事项摘要:");
                    println!("   - 计划定投总额: {:.2} CNY", plan.total_dca_due);
                    println!(
                        "   - 最终建议买入: {:.2} CNY",
                        plan.total_recommended_amount
                    );
                    if mismatch_count > 0 {
                        println!(
                            "   - ⚠️ 注意: 有 {} 个资产对账不一致，计划已自动暂停。",
                            mismatch_count
                        );
                    }

                    println!("\n(提示: 该命令为只读查询)");
                }
            }
        }
        Commands::Ops { command } => {
            run_ops_command(&*repo, &ctx, &cli, command).await?;
        }
        Commands::Data { command } => {
            run_data_command(&*repo, &ctx, &cli, command).await?;
        }
        Commands::Web { port, command } => {
            if let Some(cli::WebCommands::Doctor) = command {
                println!("Web UI 诊断 (Doctor)\n");
                println!("路由列表:");
                println!("  /                     - 首页 (Cache-first)");
                println!("  /instruments          - 市场标的 (Cache-first)");
                println!("  /risk                 - 全局风险 (Cache-first)");
                println!("  /regime               - 市场冷热 (Cache-first)");
                println!("  /valuation/proxy      - 估算净值 (Cache-first)");
                println!("  /daily                - 今日执行 (Cache-first)");

                let registry = repo.load_cache_status(&ctx).await.unwrap_or_default();
                println!("\n缓存状态:");
                let keys = vec!["fund", "market", "risk", "instrument", "proxy", "daily"];
                for key in keys {
                    let status = registry.statuses.iter().find(|s| s.key == key);
                    match status {
                        Some(s) => {
                            println!("  {:<12}: {} (更新于 {})", key, s.status, s.last_updated_at)
                        }
                        None => println!(
                            "  {:<12}: 缺失 (请运行 cargo run -- data refresh --{})",
                            key, key
                        ),
                    }
                }
                return Ok::<(), anyhow::Error>(());
            }

            web::start_server(*port, repo, ctx.clone()).await?;
        }
    }
    Ok::<(), anyhow::Error>(())
    })?;

    Ok::<(), anyhow::Error>(())
}

fn display_regime_result(regime: &models::MarketRegimeResult) {
    println!("市场冷热分析: {}\n", regime.symbol);
    println!("最新价格: {:.2}", regime.latest_price);
    println!("日期: {}", regime.latest_date);
    println!("数据来源: {}", regime.source);
    println!();

    println!(
        "{:<6} | {:<10} | {:<10} | {:<8} | {:<10} | {:<10} | {:<8}",
        "周期", "均值", "标准差", "Z-score", "回撤", "年化波动", "区间涨跌"
    );
    println!("{:-<85}", "");

    for w in &regime.windows {
        let label = match w.window_days {
            20 => "20日",
            60 => "60日",
            120 => "120日",
            250 => "250日",
            _ => "其他",
        };

        let z_str = w
            .z_score
            .map(|z| format!("{:.2}", z))
            .unwrap_or_else(|| "N/A".to_string());

        println!(
            "{:<6} | {:<10.2} | {:<10.2} | {:>8} | {:>10.2}% | {:>10.2}% | {:>8.2}%",
            label,
            w.moving_average,
            w.price_stddev,
            z_str,
            w.drawdown * 100.0,
            w.annualized_volatility * 100.0,
            w.cumulative_return * 100.0
        );
    }

    println!();
    println!("钟摆分数: {:.2}", regime.pendulum_score);
    println!("市场状态: {}", regime.regime_label);

    if let Some(w) = &regime.warning {
        println!("\n警告: {}", w);
    }
}

fn explain_regime_result(regime: &models::MarketRegimeResult, config: &models::RegimeConfig) {
    display_regime_result(regime);
    println!("\n详细说明:");
    println!(
        "1. 该分析使用过去 {} 天的历史数据；",
        config.default_lookback_days
    );
    println!("2. 均值偏离 (Z-score) = (当前价 - 均值) / 标准差；");
    println!(
        "3. Z-score > {:.1} 通常代表市场处于偏热区间，< {:.1} 代表市场处于偏冷区间；",
        config.hot_z_threshold, config.cold_z_threshold
    );
    println!("4. 回撤衡量当前价格相对于该周期内最高点的下跌幅度；");
    println!("5. 钟摆分数 (-100 到 +100) 是综合多个周期的 Z-score 计算得出的；");
    println!("   - [-100, -60]: 极冷");
    println!("   - [-60, -20]: 偏冷");
    println!("   - [-20, +20]: 中性");
    println!("   - [+20, +60]: 偏热");
    println!("   - [+60, +100]: 过热");
    println!(
        "\n风险提示: 金融市场收益并不严格服从正态分布，Z-score 仅用于衡量相对偏离程度，不应被理解为确定性预测。"
    );
}

fn display_dca_lifecycle_summary(
    summary: &models::DcaLifecycleSummary,
    filter_asset_id: Option<&str>,
) {
    println!("定投生命周期汇总 ({})\n", summary.date);
    println!("计划定投总额: {:.2} CNY", summary.total_planned_amount);
    println!("已确认总额:   {:.2} CNY", summary.total_confirmed_amount);
    println!(
        "未入账总额:   {:.2} CNY",
        summary.total_unapplied_settlement_amount
    );
    println!("对账总差异:   {:.2} CNY", summary.total_reconciliation_diff);
    println!();

    println!("项目统计:");
    println!("- 今日到期计划: {}", summary.count_due);
    println!("- 等待录入确认: {}", summary.count_waiting_confirmation);
    println!("- 等待执行入账: {}", summary.count_unapplied);
    println!("- 对账一致项目: {}", summary.count_reconciled);
    println!("- 需要人工处理: {}", summary.count_attention_required);
    println!();

    println!(
        "{:<20} | {:<15} | {:>10} | {:>10} | {:<15} | 建议操作",
        "资产ID", "生命周期状态", "计划金额", "确认金额", "对账状态"
    );
    println!("{:-<120}", "");

    for i in &summary.items {
        if let Some(filter) = filter_asset_id {
            if i.asset_id != filter {
                continue;
            }
        }

        println!(
            "{:<20} | {:<15} | {:>10.2} | {:>10.2} | {:<15} | {}",
            i.asset_id,
            i.lifecycle_status,
            i.planned_amount,
            i.settlement_amount.unwrap_or(0.0),
            i.reconciliation_status,
            i.suggested_next_action
        );
    }
}

#[allow(clippy::needless_borrow)]
async fn run_data_command(
    repo: &dyn repository::Repository,
    ctx: &repository::RepositoryContext,
    cli: &cli::Cli,
    command: &cli::DataCommands,
) -> Result<()> {
    let config = repo.load_config(&ctx).await?;
    let mut registry = repo.load_cache_status(&ctx).await.unwrap_or_default();

    match command {
        cli::DataCommands::CacheStatus => {
            println!("数据缓存状态快照\n");
            println!(
                "{:<12} | {:<10} | {:<20} | {:<12} | 备注",
                "项目", "状态", "更新时间", "数据日期"
            );
            println!("{:-<85}", "");

            let keys = vec!["fund", "market", "risk", "instrument", "proxy", "daily"];
            for key in keys {
                let status = registry.statuses.iter().find(|s| s.key == key);
                match status {
                    Some(s) => {
                        println!(
                            "{:<12} | {:<10} | {:<20} | {:<12} | {}",
                            s.key,
                            s.status,
                            s.last_updated_at,
                            s.data_date.as_deref().unwrap_or("-"),
                            s.warning.as_deref().unwrap_or("-")
                        );
                    }
                    None => {
                        println!(
                            "{:<12} | {:<10} | {:<20} | {:<12} | 尚未刷新",
                            key, "缺失", "-", "-"
                        );
                    }
                }
            }
            println!("\n提示: 运行 cargo run -- data refresh --all 刷新所有数据。");
        }
        cli::DataCommands::Migrate {
            all,
            tx,
            portfolio_state,
            dca,
            reconciliation,
            migrate_instruments,
        } => {
            if *all || *tx || *portfolio_state || *dca || *reconciliation || *migrate_instruments {
                // Ensure target is PostgreSQL
                if config.storage.backend != models::StorageBackend::Postgres {
                    anyhow::bail!(
                        "Migration is only supported when target backend is set to 'postgres' in config."
                    );
                }

                println!("正在从 JSON 迁移数据到 PostgreSQL...");

                // Run SQL migrations first to ensure schema and default user/portfolio
                let env_var = &config.storage.postgres.database_url_env;
                let database_url = std::env::var(env_var).map_err(|_| {
                    anyhow::anyhow!(
                        "Environment variable '{}' for PostgreSQL not found",
                        env_var
                    )
                })?;
                let db = db::postgres::PostgresDb::connect(&database_url).await?;
                db.run_migrations().await?;

                // Create source repo (always JSON for this command)
                let source_repo = repository::RepositoryFactory::json_default();
                let mut reports = Vec::new();

                // Order: Instruments -> State -> Transactions -> DCA -> Reconciliation

                if *all || *migrate_instruments {
                    let report =
                        repository::migrate_instruments(source_repo.as_ref(), repo, ctx).await?;
                    reports.push(report);
                }

                if *all || *portfolio_state {
                    let report = repository::migrate_state(source_repo.as_ref(), repo, ctx).await?;
                    reports.push(report);
                }

                if *all || *tx {
                    let report =
                        repository::migrate_transactions(source_repo.as_ref(), repo, ctx).await?;
                    reports.push(report);
                }

                if *all || *dca {
                    let report = repository::migrate_dca(source_repo.as_ref(), repo, ctx).await?;
                    reports.push(report);
                }

                if *all || *reconciliation {
                    let report =
                        repository::migrate_reconciliation(source_repo.as_ref(), repo, ctx).await?;
                    reports.push(report);
                }

                // Output Summary
                println!("\n数据迁移完成报告:");
                println!(
                    "{:<20} {:>10} {:>10} {:>10} {:>10}",
                    "领域 (Domain)", "读取", "插入", "跳过", "失败"
                );
                println!("{}", "-".repeat(65));
                for r in reports {
                    println!(
                        "{:<20} {:>10} {:>10} {:>10} {:>10}",
                        r.domain, r.read, r.inserted, r.skipped, r.failed
                    );
                }
            } else {
                println!(
                    "请指定要迁移的数据类型, 例如: --all, --tx, --portfolio-state, --dca, --reconciliation 或 --market-instruments"
                );
            }
        }
        cli::DataCommands::Export { json, dir, force } => {
            if *json {
                let export_dir = if let Some(d) = dir {
                    d.clone()
                } else {
                    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
                    resolve_data_dir()
                        .join("export-json")
                        .join(timestamp)
                        .to_string_lossy()
                        .to_string()
                };

                let path = Path::new(&export_dir);
                if path.exists() && !*force {
                    anyhow::bail!(
                        "Export directory {:?} already exists. Use --force to overwrite.",
                        path
                    );
                }

                println!("正在导出数据到 JSON: {}...", export_dir);
                fs::create_dir_all(path)?;

                let target_repo = repository::RepositoryFactory::json_from_dir(&export_dir);
                let mut export_reports = Vec::new();

                // Order matches migration
                export_reports
                    .push(repository::migrate_instruments(repo, target_repo.as_ref(), ctx).await?);
                export_reports
                    .push(repository::migrate_state(repo, target_repo.as_ref(), ctx).await?);
                export_reports
                    .push(repository::migrate_transactions(repo, target_repo.as_ref(), ctx).await?);
                export_reports
                    .push(repository::migrate_dca(repo, target_repo.as_ref(), ctx).await?);
                export_reports.push(
                    repository::migrate_reconciliation(repo, target_repo.as_ref(), ctx).await?,
                );

                println!("\n数据导出完成报告:");
                println!(
                    "{:<20} {:>10} {:>10} {:<30}",
                    "领域 (Domain)", "记录数", "状态", "输出路径"
                );
                println!("{}", "-".repeat(75));
                for r in export_reports {
                    let status = if r.failed > 0 { "失败" } else { "成功" };
                    println!(
                        "{:<20} {:>10} {:>10} {}",
                        r.domain, r.read, status, export_dir
                    );
                }
            } else {
                println!("请指定导出格式, 例如: --json");
            }
        }
        cli::DataCommands::Verify {
            strict,
            json,
            portfolio,
        } => {
            let mut verify_ctx = ctx.clone();
            if let Some(p) = portfolio {
                verify_ctx.portfolio_id = p.clone();
            }

            let json_repo;
            let target_repo: &dyn repository::traits::PortfolioRepository = if *json {
                json_repo = repository::JsonRepository::new_with_defaults(
                    &crate::resolve_data_dir().to_string_lossy(),
                );
                &json_repo
            } else {
                repo
            };

            let report =
                engine::verification::verify_data(target_repo, &verify_ctx, *strict).await?;

            println!("=== 数据完整性验证报告 ===");
            println!("组合: {}", report.portfolio_id);
            println!("总检查项: {}", report.summary.total_checks);
            println!("错误 (Errors): {}", report.summary.errors);
            println!("警告 (Warnings): {}", report.summary.warnings);

            if !report.issues.is_empty() {
                println!("\n发现的问题:");
                for issue in &report.issues {
                    let sev = match issue.severity {
                        engine::verification::VerificationSeverity::Error => "[ERROR]",
                        engine::verification::VerificationSeverity::Warning => "[WARN ]",
                        engine::verification::VerificationSeverity::Info => "[INFO ]",
                    };
                    println!(
                        "{} [{}] {} (涉及: {:?})",
                        sev, issue.domain, issue.message, issue.affected_records
                    );
                }
            }

            if report.summary.errors > 0 {
                anyhow::bail!(
                    "Data verification failed with {} errors.",
                    report.summary.errors
                );
            } else {
                println!("\n数据验证通过.");
            }
        }
        cli::DataCommands::Repair { preview: _ } => {
            println!("Data repair preview is not yet implemented.");
        }
        cli::DataCommands::Refresh {
            all,
            fund,
            market,
            risk,
            instrument,
            proxy,
            daily,
        } => {
            println!("开始刷新数据提供商缓存...\n");

            if *all || *fund {
                refresh_fund_data(repo, ctx, cli, &config, &mut registry).await?;
            }
            if *all || *market {
                refresh_market_data(repo, ctx, cli, &config, &mut registry).await?;
            }
            if *all || *risk {
                refresh_risk_data(repo, ctx, cli, &config, &mut registry).await?;
            }
            if *all || *instrument {
                refresh_instrument_data(repo, ctx, cli, &config, &mut registry).await?;
            }
            if *all || *proxy {
                refresh_proxy_data(repo, ctx, cli, &config, &mut registry).await?;
            }
            if *all || *daily {
                refresh_daily_data(repo, ctx, cli, &config, &mut registry).await?;
            }

            repo.save_cache_status(&ctx, &registry).await?;
            println!("\n数据刷新完成。");
        }
    }

    Ok::<(), anyhow::Error>(())
}

fn update_cache_status(
    registry: &mut models::CacheStatusRegistry,
    key: &str,
    source: &str,
    status: &str,
    data_date: Option<String>,
    warning: Option<String>,
) {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if let Some(s) = registry.statuses.iter_mut().find(|s| s.key == key) {
        s.source = source.to_string();
        s.status = status.to_string();
        s.last_updated_at = now;
        s.data_date = data_date;
        s.warning = warning;
    } else {
        registry.statuses.push(models::CacheStatus {
            key: key.to_string(),
            source: source.to_string(),
            last_updated_at: now,
            data_date,
            status: status.to_string(),
            warning,
        });
    }
}

#[allow(clippy::needless_borrow)]
async fn refresh_fund_data(
    repo: &dyn repository::Repository,
    ctx: &repository::RepositoryContext,
    _cli: &cli::Cli,
    config: &models::ConfigRoot,
    registry: &mut models::CacheStatusRegistry,
) -> Result<()> {
    print!("- 正在刷新基金净值 ({} 个资产)... ", config.assets.len());

    let success_count = engine::refresh::refresh_fund_navs(repo, ctx, config).await?;

    let status = if success_count == config.assets.len() {
        "正常"
    } else {
        "部分失败"
    };
    update_cache_status(
        registry,
        "fund",
        "eastmoney",
        status,
        None,
        Some(format!("成功: {}/{}", success_count, config.assets.len())),
    );
    println!("完成。");
    Ok::<(), anyhow::Error>(())
}

#[allow(clippy::needless_borrow)]
async fn refresh_market_data(
    repo: &dyn repository::Repository,
    ctx: &repository::RepositoryContext,
    _cli: &cli::Cli,
    config: &models::ConfigRoot,
    registry: &mut models::CacheStatusRegistry,
) -> Result<()> {
    print!("- 正在刷新市场行情... ");

    let (success_count, _skipped, failed_count) =
        engine::refresh::refresh_market_data(repo, ctx, config).await?;

    let status = if success_count > 0 {
        "正常"
    } else {
        "失败"
    };
    update_cache_status(
        registry,
        "market",
        "yahoo",
        status,
        None,
        Some(format!("成功: {}, 失败: {}", success_count, failed_count)),
    );
    update_cache_status(registry, "regime", "internal", status, None, None);
    println!("完成。");
    Ok::<(), anyhow::Error>(())
}

async fn refresh_risk_data(
    repo: &dyn repository::Repository,
    ctx: &repository::RepositoryContext,
    _cli: &cli::Cli,
    config: &models::ConfigRoot,
    registry: &mut models::CacheStatusRegistry,
) -> Result<()> {
    print!("- 正在刷新风险因子... ");

    let overlay = {
        let config = config.clone();
        tokio::task::spawn_blocking(move || {
            let market_provider = api::create_market_provider(&config.market, Some("yahoo"));
            let fx_provider = api::create_fx_provider(&config.fx, None);

            engine::risk_overlay::calculate_risk_overlay(
                &config.risk,
                &config.regime,
                market_provider.as_ref(),
                fx_provider.as_ref(),
            )
        })
        .await?
    };

    let cache = models::RiskCache {
        overlay: overlay.clone(),
        fetched_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        status: "正常".to_string(),
    };

    repo.save_risk_cache(ctx, &cache).await?;
    update_cache_status(
        registry,
        "risk",
        "yahoo",
        "正常",
        None,
        Some(overlay.risk_label),
    );
    println!("完成。");
    Ok::<(), anyhow::Error>(())
}

#[allow(clippy::needless_borrow)]
async fn refresh_instrument_data(
    repo: &dyn repository::Repository,
    ctx: &repository::RepositoryContext,
    _cli: &cli::Cli,
    config: &models::ConfigRoot,
    registry: &mut models::CacheStatusRegistry,
) -> Result<()> {
    let instruments = repo.load_instruments(&ctx).await?;
    print!("- 正在刷新标的注册表 ({} 个标的)... ", instruments.len());

    let (mut cache, results) = {
        let config = config.clone();
        let instruments = instruments.clone();
        tokio::task::spawn_blocking(move || {
            let cache = models::InstrumentQuoteCache::default();
            let mut results = Vec::new();
            for i in &instruments {
                if !i.enabled {
                    continue;
                }
                let provider = api::create_instrument_provider(&config.market, Some(&i.provider));
                let q_res = provider.latest(i);
                results.push(q_res);
            }
            (cache, results)
        })
        .await?
    };

    let mut success_count = 0;
    let mut last_date = None;

    for q_res in results {
        match q_res {
            Ok(q) => {
                success_count += 1;
                last_date = Some(q.latest_date.clone());
                cache.entries.push(models::InstrumentQuoteCacheEntry {
                    instrument_id: q.instrument_id,
                    symbol: q.symbol,
                    name_zh: q.name_zh,
                    price: q.latest_price,
                    date: q.latest_date,
                    currency: q.currency,
                    quote_unit: q.quote_unit,
                    provider: q.provider,
                    source: q.source,
                    status: q.status,
                    fetched_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    warning: q.warning,
                });
            }
            Err(e) => {
                eprintln!("\n  ✗ 标的刷新失败: {}", e);
            }
        }
    }

    cache.fetched_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    repo.save_instrument_cache(ctx, &cache).await?;

    update_cache_status(
        registry,
        "instrument",
        "multi",
        "正常",
        last_date,
        Some(format!("成功: {}/{}", success_count, instruments.len())),
    );
    println!("完成。");
    Ok::<(), anyhow::Error>(())
}

#[allow(clippy::needless_borrow)]
async fn refresh_proxy_data(
    repo: &dyn repository::Repository,
    ctx: &repository::RepositoryContext,
    _cli: &cli::Cli,
    config: &models::ConfigRoot,
    registry: &mut models::CacheStatusRegistry,
) -> Result<()> {
    print!("- 正在计算估算净值... ");
    let state = repo.load_state(&ctx).await?;

    let results = {
        let config = config.clone();
        tokio::task::spawn_blocking(move || {
            let market_provider = api::create_market_provider(&config.market, None);
            let fx_provider = api::create_fx_provider(&config.fx, None);

            engine::valuation::calculate_proxy_valuations(
                &config,
                &state,
                market_provider.as_ref(),
                fx_provider.as_ref(),
            )
        })
        .await?
    };

    let cache = models::ProxyValuationCache {
        results,
        fetched_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    repo.save_proxy_cache(ctx, &cache).await?;
    update_cache_status(registry, "proxy", "internal", "正常", None, None);
    println!("完成。");
    Ok::<(), anyhow::Error>(())
}

#[allow(clippy::needless_borrow)]
async fn refresh_daily_data(
    _repo: &dyn repository::Repository,
    _ctx: &repository::RepositoryContext,
    _cli: &cli::Cli,
    _config: &models::ConfigRoot,
    registry: &mut models::CacheStatusRegistry,
) -> Result<()> {
    print!("- 正在刷新今日执行概要... ");
    // For now, daily plan is computed on the fly in web but we mark it as "refreshed"
    // to show it's available. Real caching can be added if needed.
    update_cache_status(registry, "daily", "internal", "正常", None, None);
    println!("完成。");
    Ok::<(), anyhow::Error>(())
}

#[allow(clippy::needless_borrow)]
async fn run_ops_command(
    repo: &dyn repository::Repository,
    ctx: &repository::RepositoryContext,
    cli: &cli::Cli,
    command: &cli::OpsCommands,
) -> Result<()> {
    let config = repo.load_config(&ctx).await?;
    let state = repo.load_state(&ctx).await?;

    match command {
        cli::OpsCommands::Today { date, verbose } => {
            let target_date = date
                .clone()
                .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
            println!("每日操作台 ({})\n", target_date);

            // 1. 数据状态
            println!("1. 数据状态:");
            let registry = repo.load_cache_status(&ctx).await.unwrap_or_default();
            let keys = vec!["fund", "market", "risk", "instrument", "proxy"];
            let mut stale_keys = Vec::new();
            for key in keys {
                let status = registry.statuses.iter().find(|s| s.key == key);
                match status {
                    Some(s) if s.status == "正常" => {
                        if *verbose {
                            println!("   - {:<12}: 正常 ({})", key, s.last_updated_at);
                        }
                    }
                    Some(s) => {
                        println!(
                            "   - {:<12}: {} (警告: {})",
                            key,
                            s.status,
                            s.warning.as_deref().unwrap_or("-")
                        );
                        stale_keys.push(key);
                    }
                    None => {
                        println!("   - {:<12}: 缺失", key);
                        stale_keys.push(key);
                    }
                }
            }
            if stale_keys.is_empty() {
                println!("   [✓] 所有数据已就绪。");
            } else {
                println!("   [!] 部分数据缺失或过期，建议运行: cargo run -- ops refresh");
            }

            // 2. 今日定投
            println!("\n2. 今日定投:");
            let dca_plans = repo.load_plans(&ctx).await?;
            let nav_cache = repo.load_nav_cache(&ctx).await?;
            let dca_preview =
                engine::dca::calculate_dca_preview(&config, &dca_plans, &nav_cache, &target_date);
            let settlements = repo.load_settlements(&ctx).await?;
            let snapshots = repo.load_alipay_snapshots(&ctx).await?;
            let lifecycle = engine::calculate_dca_lifecycle(
                &config,
                &dca_plans,
                &settlements,
                &snapshots,
                &state,
                &nav_cache,
                &target_date,
            );

            println!(
                "   - 应投笔数: {} 笔",
                dca_preview
                    .items
                    .iter()
                    .filter(|i| i.status == "今日应投")
                    .count()
            );
            println!("   - 计划总额: {:.2} CNY", dca_preview.total_due_amount);
            println!(
                "   - 闭环状态: {} 个待确认, {} 个待入账",
                lifecycle.count_waiting_confirmation, lifecycle.count_unapplied
            );

            // 3. 待处理事项
            println!("\n3. 待处理事项:");
            let pending_items: Vec<_> = lifecycle
                .items
                .iter()
                .filter(|i| i.suggested_next_action != "无需处理" && i.lifecycle_status != "已暂停")
                .collect();

            if pending_items.is_empty() {
                println!("   [✓] 暂无需要人工处理的定投事项。");
            } else {
                for i in &pending_items {
                    println!(
                        "   - [{}] {}: {}",
                        i.lifecycle_status, i.asset_id, i.suggested_next_action
                    );
                }
            }

            // 4. 今日执行计划
            println!("\n4. 今日执行计划:");

            let (risk_overlay, regimes) = {
                let config = config.clone();
                tokio::task::spawn_blocking(move || {
                    let market_provider = api::create_market_provider(&config.market, None);
                    let fx_provider = api::create_fx_provider(&config.fx, None);

                    let risk_overlay = engine::risk_overlay::calculate_risk_overlay(
                        &config.risk,
                        &config.regime,
                        market_provider.as_ref(),
                        fx_provider.as_ref(),
                    );

                    // Build regimes
                    let mut regimes = std::collections::HashMap::new();
                    for asset in &config.assets {
                        let symbol_opt = asset
                            .reference_instrument_symbol
                            .clone()
                            .or(asset.reference_index_symbol.clone());
                        if let Some(candles) = symbol_opt.and_then(|s| {
                            market_provider
                                .fetch_daily_candles(&s, config.regime.default_lookback_days)
                                .ok()
                                .map(|c| (s, c))
                        }) {
                            let regime = engine::regime::calculate_market_regime(
                                &candles.0,
                                &candles.1,
                                &config.regime,
                            );
                            regimes.insert(asset.asset_id.clone(), regime);
                        }
                    }
                    (risk_overlay, regimes)
                })
                .await?
            };

            let decision = engine::generate_buy_suggestions(&config, &state, target_date.clone());

            let kelly =
                engine::kelly::calculate_kelly_preview(&config, &decision, &risk_overlay, &regimes);

            let mut latest_snaps = std::collections::HashMap::new();
            for s in &snapshots {
                let entry = latest_snaps.entry(s.asset_id.clone()).or_insert(s.clone());
                if s.snapshot_date >= entry.snapshot_date {
                    *entry = s.clone();
                }
            }
            let mut reconciliation_results = Vec::new();
            for asset in &config.assets {
                if let Some(s) = latest_snaps.get(&asset.asset_id) {
                    reconciliation_results
                        .push(engine::reconciliation::reconcile_asset(&config, &state, s));
                }
            }

            let adjusted = engine::adjusted_decision::calculate_adjusted_decision(
                &config,
                &state,
                &decision,
                &risk_overlay,
                &regimes,
            );

            let plan = engine::daily_plan::generate_daily_execution_plan(
                &config,
                &state,
                target_date.clone(),
                &dca_preview,
                &adjusted,
                &kelly,
                &reconciliation_results,
            );

            println!(
                "   - 最终建议买入: {:.2} CNY",
                plan.total_recommended_amount
            );
            let count_paused = plan
                .items
                .iter()
                .filter(|i| i.status == "暂停执行" || i.status == "等待对账")
                .count();
            if count_paused > 0 {
                println!(
                    "   - 暂停执行资产: {} 个 (原因: 对账不一致或数据缺失)",
                    count_paused
                );
            }
            if !plan.warnings.is_empty() {
                println!("   - 警告提示: {} 条", plan.warnings.len());
            }

            // 5. 风险状态
            println!("\n5. 风险状态:");
            println!("   - 风险评级: {}", risk_overlay.risk_label);
            println!("   - 风险分数: {:.1} / 100", risk_overlay.risk_score);
            if let Some(qqq) = regimes.get("nasdaq_100_fund") {
                println!(
                    "   - 纳指冷热: {} (钟摆分数 {:.1})",
                    qqq.regime_label, qqq.pendulum_score
                );
            }

            // 6. 下一步建议
            println!("\n6. 下一步建议:");
            let mut steps = 0;
            if !stale_keys.is_empty() {
                steps += 1;
                println!(
                    "{}. 运行 cargo run -- ops refresh 刷新所有行情数据。",
                    steps
                );
            }
            for i in pending_items {
                steps += 1;
                println!("{}. {}: {}", steps, i.asset_id, i.suggested_next_action);
            }
            if steps == 0 {
                println!("今日无需额外操作。");
            }
        }
        cli::OpsCommands::Refresh => {
            println!("开始执行全量数据刷新 (ops refresh)...\n");
            run_data_command(
                repo,
                ctx,
                cli,
                &cli::DataCommands::Refresh {
                    all: true,
                    fund: false,
                    market: false,
                    risk: false,
                    instrument: false,
                    proxy: false,
                    daily: false,
                },
            )
            .await?;
        }
        cli::OpsCommands::Status { verbose } => {
            println!("组合简报 (Status)\n");
            let summary = engine::calculate_portfolio_summary(&config, &state);
            let registry = repo.load_cache_status(&ctx).await.unwrap_or_default();

            println!("资产概览:");
            println!(
                "   - 总资产:     {:.2} {}",
                summary.total_asset_value, config.portfolio.base_currency
            );
            println!(
                "   - 权益占比:   {:.2}%",
                (summary.equity_value / summary.total_asset_value) * 100.0
            );
            println!(
                "   - 可用现金:   {:.2} {}",
                summary.available_cash, config.portfolio.base_currency
            );

            println!("\n定投状态:");
            let target_date = Local::now().format("%Y-%m-%d").to_string();
            let dca_plans = repo.load_plans(&ctx).await?;
            let nav_cache = repo.load_nav_cache(&ctx).await?;
            let dca_preview =
                engine::dca::calculate_dca_preview(&config, &dca_plans, &nav_cache, &target_date);
            println!(
                "   - 今日定投:   {:.2} CNY ({} 笔)",
                dca_preview.total_due_amount,
                dca_preview
                    .items
                    .iter()
                    .filter(|i| i.status == "今日应投")
                    .count()
            );

            let settlements = repo.load_settlements(&ctx).await?;
            let snapshots = repo.load_alipay_snapshots(&ctx).await?;
            let lifecycle = engine::calculate_dca_lifecycle(
                &config,
                &dca_plans,
                &settlements,
                &snapshots,
                &state,
                &nav_cache,
                &target_date,
            );
            println!(
                "   - 待处理项:   {} 项 (定投闭环)",
                lifecycle.count_waiting_confirmation
                    + lifecycle.count_unapplied
                    + lifecycle.count_attention_required
            );

            println!("\n市场风险:");
            let risk_cache = repo.load_risk_cache(&ctx).await.unwrap_or(None);
            if let Some(rc) = risk_cache {
                println!("   - 风险等级:   {}", rc.overlay.risk_label);
            } else {
                println!("   - 风险等级:   未知 (请运行 ops refresh)");
            }

            if *verbose {
                println!("\n数据刷新详情:");
                for s in registry.statuses {
                    println!("   - {:<12}: {} ({})", s.key, s.status, s.last_updated_at);
                }
            }
        }
        cli::OpsCommands::Doctor { verbose } => {
            println!("运行诊断程序 (ops doctor)...\n");

            println!("1. 配置文件检查:");
            match repo.load_config(&ctx).await {
                Ok(_) => println!("   [✓] config.toml 格式正确"),
                Err(e) => println!("   [✗] config.toml 加载失败: {}", e),
            }

            println!("\n2. 标的注册表检查:");
            match repo.load_instruments(&ctx).await {
                Ok(insts) => {
                    println!("   [✓] instruments.toml 加载成功 ({} 个标的)", insts.len());
                    let missing_names: Vec<_> = insts
                        .iter()
                        .filter(|i| i.enabled && i.name_zh.is_none())
                        .collect();
                    if !missing_names.is_empty() {
                        println!(
                            "   [!] 警告: 有 {} 个启用标的缺少中文名称",
                            missing_names.len()
                        );
                    }
                }
                Err(e) => println!("   [✗] instruments.toml 加载失败: {}", e),
            }

            println!("\n3. 缓存完整性检查:");
            let registry = repo.load_cache_status(&ctx).await.unwrap_or_default();
            let required_keys = vec!["fund", "market", "risk", "instrument"];
            for key in required_keys {
                if registry
                    .statuses
                    .iter()
                    .any(|s| s.key == key && s.status == "正常")
                {
                    println!("   [✓] {} 缓存正常", key);
                } else {
                    println!("   [✗] {} 缓存缺失或异常", key);
                }
            }

            println!("\n4. 数据文件检查:");
            let files = vec![
                &cli.state,
                &cli.transactions,
                &cli.dca_plans,
                &cli.dca_settlements,
                &cli.alipay_snapshots,
            ];
            for f in files {
                if Path::new(f).exists() {
                    println!("   [✓] 文件存在: {}", f);
                } else {
                    println!("   [!] 缺失(可选): {}", f);
                }
            }

            if *verbose {
                println!("\n详细资产校验:");
                if let Ok(c) = repo.load_config(&ctx).await {
                    for a in c.assets {
                        let status = if a.enabled { "启用" } else { "禁用" };
                        println!("   - {:<20}: {} [{}]", a.asset_id, a.fund_name, status);
                    }
                }
            }
        }
        cli::OpsCommands::Checklist => {
            println!("每日操作清单 (Ops Checklist)\n");
            println!("--- [ 阶段 1: 数据准备 ] ---");
            println!("   [ ] 运行数据刷新: cargo run -- ops refresh");
            println!("   [ ] 检查缓存状态: cargo run -- data cache-status");

            println!("\n--- [ 阶段 2: 定投确认 ] ---");
            let target_date = Local::now().format("%Y-%m-%d").to_string();
            let plans = repo.load_plans(&ctx).await?;
            let settlements = repo.load_settlements(&ctx).await?;
            let snapshots = repo.load_alipay_snapshots(&ctx).await?;
            let nav_cache = repo.load_nav_cache(&ctx).await?;
            let lifecycle = engine::calculate_dca_lifecycle(
                &config,
                &plans,
                &settlements,
                &snapshots,
                &state,
                &nav_cache,
                &target_date,
            );

            if lifecycle.count_waiting_confirmation > 0 {
                println!(
                    "   [ ] 录入定投确认单 ({} 个待办): cargo run -- dca settlement add ...",
                    lifecycle.count_waiting_confirmation
                );
            } else {
                println!("   [✓] 定投确认单已全部录入");
            }

            if lifecycle.count_unapplied > 0 {
                println!(
                    "   [ ] 执行定投确认入账 ({} 个待办): cargo run -- dca settlement apply ...",
                    lifecycle.count_unapplied
                );
            } else {
                println!("   [✓] 定投单已全部入账");
            }

            println!("\n--- [ 阶段 3: 对账与差异 ] ---");
            if lifecycle.count_attention_required > 0 {
                println!(
                    "   [ ] 处理对账不一致 ({} 个资产): cargo run -- reconcile alipay compare-all",
                    lifecycle.count_attention_required
                );
            } else {
                println!("   [✓] 支付宝对账已通过");
            }

            println!("\n--- [ 阶段 4: 今日买入执行 ] ---");
            println!("   [ ] 查看今日执行计划: cargo run -- daily plan");
            println!("   [ ] 手动录入场外交易: cargo run -- tx add buy ...");

            println!("\n(提示: 该命令为只读查询)");
        }
    }

    Ok::<(), anyhow::Error>(())
}

#[allow(clippy::needless_borrow)]
async fn run_report_command(
    repo: &dyn repository::Repository,
    ctx: &repository::RepositoryContext,
    _cli: &cli::Cli,
    command: &cli::ReportCommands,
) -> Result<()> {
    let config = repo.load_config(&ctx).await?;
    let state = repo.load_state(&ctx).await?;

    match command {
        cli::ReportCommands::Daily { date, save } => {
            let target_date = date
                .clone()
                .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
            println!("正在生成每日复盘报告 ({}) ...\n", target_date);

            let plans = repo.load_plans(&ctx).await?;
            let settlements = repo.load_settlements(&ctx).await?;
            let snapshots = repo.load_alipay_snapshots(&ctx).await?;

            let summary = engine::calculate_portfolio_summary(&config, &state);
            let nav_cache = repo.load_nav_cache(&ctx).await?;
            let dca_lifecycle = engine::calculate_dca_lifecycle(
                &config,
                &plans,
                &settlements,
                &snapshots,
                &state,
                &nav_cache,
                &target_date,
            );

            let risk_overlay = {
                let config = config.clone();
                tokio::task::spawn_blocking(move || {
                    let market_provider = api::create_market_provider(&config.market, None);
                    let fx_provider = api::create_fx_provider(&config.fx, None);
                    engine::risk_overlay::calculate_risk_overlay(
                        &config.risk,
                        &config.regime,
                        market_provider.as_ref(),
                        fx_provider.as_ref(),
                    )
                })
                .await?
            };

            let mut latest_snaps = std::collections::HashMap::new();
            for s in &snapshots {
                let entry = latest_snaps.entry(s.asset_id.clone()).or_insert(s.clone());
                if s.snapshot_date >= entry.snapshot_date {
                    *entry = s.clone();
                }
            }
            let mut reconciliation_results = Vec::new();
            for asset in &config.assets {
                if let Some(s) = latest_snaps.get(&asset.asset_id) {
                    reconciliation_results
                        .push(engine::reconciliation::reconcile_asset(&config, &state, s));
                }
            }

            let transactions = repo.load_transactions(&ctx).await?;
            let backend_name = match config.storage.backend {
                models::StorageBackend::Postgres => "postgres",
                _ => "json",
            };
            let extended_summary = engine::report_summary::generate_report_summary(
                &ctx.portfolio_id,
                backend_name,
                &target_date,
                &target_date,
                &transactions,
                &state,
            );

            let report = engine::report::generate_investment_report(
                models::ReportPeriod::Daily,
                &format!("每日复盘报告 - {}", target_date),
                &target_date,
                &target_date,
                Some(summary),
                Some(dca_lifecycle),
                Some(risk_overlay),
                None,
                &reconciliation_results,
                Some(extended_summary),
            );

            let markdown = engine::report::render_report_to_markdown(&report);
            println!("{}", markdown);

            if *save {
                let filename = format!("daily-{}.md", target_date);
                repo.save_markdown_report(&ctx, &markdown, &filename)
                    .await?;
                println!("\n报告已保存。");
            }
        }
        cli::ReportCommands::Weekly { start, end, save } => {
            let target_end = end
                .clone()
                .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
            let target_start = start.clone().unwrap_or_else(|| {
                // Default to 7 days ago
                let end_dt = chrono::NaiveDate::parse_from_str(&target_end, "%Y-%m-%d").unwrap();
                (end_dt - chrono::Duration::days(6))
                    .format("%Y-%m-%d")
                    .to_string()
            });

            println!(
                "正在生成每周复盘报告 ({} 至 {}) ...\n",
                target_start, target_end
            );

            let settlements = repo.load_settlements(&ctx).await?;
            let range_settlements: Vec<_> = settlements
                .iter()
                .filter(|s| s.deduction_date >= target_start && s.deduction_date <= target_end)
                .collect();

            let total_confirmed = range_settlements.iter().map(|s| s.amount).sum::<f64>();
            let summary = engine::calculate_portfolio_summary(&config, &state);

            let transactions = repo.load_transactions(&ctx).await?;
            let backend_name = match config.storage.backend {
                models::StorageBackend::Postgres => "postgres",
                _ => "json",
            };
            let extended_summary = engine::report_summary::generate_report_summary(
                &ctx.portfolio_id,
                backend_name,
                &target_start,
                &target_end,
                &transactions,
                &state,
            );

            let mut report = engine::report::generate_investment_report(
                models::ReportPeriod::Weekly,
                &format!("每周复盘报告 ({} - {})", target_start, target_end),
                &target_start,
                &target_end,
                Some(summary),
                None,
                None,
                None,
                &[],
                Some(extended_summary),
            );

            report.sections.push(models::ReportSection {
                title: "本周定投汇总".to_string(),
                status: "正常".to_string(),
                summary: format!(
                    "本周共完成 {} 笔定投，确认金额 {:.2} CNY。",
                    range_settlements.len(),
                    total_confirmed
                ),
                details: range_settlements
                    .iter()
                    .map(|s| format!("{}: {:.2} CNY ({})", s.asset_id, s.amount, s.deduction_date))
                    .collect(),
                warnings: vec![],
                suggested_actions: vec![],
            });

            let markdown = engine::report::render_report_to_markdown(&report);
            println!("{}", markdown);

            if *save {
                let filename = format!("weekly-{}-{}.md", target_start, target_end);
                repo.save_markdown_report(&ctx, &markdown, &filename)
                    .await?;
                println!("\n报告已保存。");
            }
        }
        cli::ReportCommands::Monthly { month, save } => {
            let target_month = month
                .clone()
                .unwrap_or_else(|| Local::now().format("%Y-%m").to_string());
            println!("正在生成月度复盘报告 ({}) ...\n", target_month);

            let settlements = repo.load_settlements(&ctx).await?;
            let range_settlements: Vec<_> = settlements
                .iter()
                .filter(|s| s.deduction_date.starts_with(&target_month))
                .collect();

            let total_confirmed = range_settlements.iter().map(|s| s.amount).sum::<f64>();
            let summary = engine::calculate_portfolio_summary(&config, &state);

            let target_start = format!("{}-01", target_month);
            let target_end = format!("{}-31", target_month); // Simplified

            let transactions = repo.load_transactions(&ctx).await?;
            let backend_name = match config.storage.backend {
                models::StorageBackend::Postgres => "postgres",
                _ => "json",
            };
            let extended_summary = engine::report_summary::generate_report_summary(
                &ctx.portfolio_id,
                backend_name,
                &target_start,
                &target_end,
                &transactions,
                &state,
            );

            let mut report = engine::report::generate_investment_report(
                models::ReportPeriod::Monthly,
                &format!("月度复盘报告 - {}", target_month),
                &target_start,
                &target_end,
                Some(summary),
                None,
                None,
                None,
                &[],
                Some(extended_summary),
            );

            report.sections.push(models::ReportSection {
                title: "月度定投汇总".to_string(),
                status: "正常".to_string(),
                summary: format!(
                    "本月共完成 {} 笔定投，总金额 {:.2} CNY。",
                    range_settlements.len(),
                    total_confirmed
                ),
                details: vec![format!("累计金额: {:.2} CNY", total_confirmed)],
                warnings: vec![],
                suggested_actions: vec![],
            });

            let markdown = engine::report::render_report_to_markdown(&report);
            println!("{}", markdown);

            if *save {
                let filename = format!("monthly-{}.md", target_month);
                repo.save_markdown_report(&ctx, &markdown, &filename)
                    .await?;
                println!("\n报告已保存。");
            }
        }
        cli::ReportCommands::Portfolio => {
            let summary = engine::calculate_portfolio_summary(&config, &state);
            println!("组合简报 (Portfolio Report)\n");
            println!("总资产: {:.2} CNY", summary.total_asset_value);
            println!("当前现金: {:.2} CNY", summary.cash);
            println!(
                "权益市值: {:.2} CNY (占比 {:.2}%)",
                summary.equity_value,
                (summary.equity_value / summary.total_asset_value) * 100.0
            );
            println!("\n赛道分配:");
            for ss in summary.sector_summaries {
                println!(
                    "- {:<15}: {:>10.2} ({:>6.2}%) | 状态: {}",
                    ss.sector_name,
                    ss.current_value,
                    ss.current_weight * 100.0,
                    ss.status
                );
            }
        }
        cli::ReportCommands::Dca => {
            let plans = repo.load_plans(&ctx).await?;
            let settlements = repo.load_settlements(&ctx).await?;
            let snapshots = repo.load_alipay_snapshots(&ctx).await?;
            let nav_cache = repo.load_nav_cache(&ctx).await?;
            let date = Local::now().format("%Y-%m-%d").to_string();
            let summary = engine::calculate_dca_lifecycle(
                &config,
                &plans,
                &settlements,
                &snapshots,
                &state,
                &nav_cache,
                &date,
            );

            display_dca_lifecycle_summary(&summary, None);
        }
        cli::ReportCommands::Reconcile => {
            let snapshots = repo.load_alipay_snapshots(&ctx).await?;
            println!("对账汇总报告 (Reconciliation Report)\n");

            let mut total_diff = 0.0;
            let mut mismatch_count = 0;

            for asset in &config.assets {
                if !asset.enabled {
                    continue;
                }
                let latest_snap = snapshots
                    .iter()
                    .filter(|s| s.asset_id == asset.asset_id)
                    .max_by_key(|s| s.snapshot_date.clone());

                match latest_snap {
                    Some(snap) => {
                        let res = engine::reconciliation::reconcile_asset(&config, &state, snap);
                        println!(
                            "{:<20}: {} (差异: {:.2})",
                            asset.asset_id, res.status, res.market_value_diff
                        );
                        if res.status != "一致" {
                            mismatch_count += 1;
                            total_diff += res.market_value_diff.abs();
                        }
                    }
                    None => println!("{:<20}: 缺失快照", asset.asset_id),
                }
            }
            println!(
                "\n总结: {} 个资产存在差异，总绝对差异: {:.2} CNY",
                mismatch_count, total_diff
            );
        }
        cli::ReportCommands::Risk => {
            let risk_overlay = {
                let config = config.clone();
                tokio::task::spawn_blocking(move || {
                    let market_provider = api::create_market_provider(&config.market, None);
                    let fx_provider = api::create_fx_provider(&config.fx, None);
                    engine::risk_overlay::calculate_risk_overlay(
                        &config.risk,
                        &config.regime,
                        market_provider.as_ref(),
                        fx_provider.as_ref(),
                    )
                })
                .await?
            };

            println!("风险分析报告 (Risk Report)\n");
            println!("全局风险评分: {:.1} / 100", risk_overlay.risk_score);
            println!("风险等级: {}", risk_overlay.risk_label);
            println!("\n风险解读: {}", risk_overlay.explanation);

            if !risk_overlay.warnings.is_empty() {
                println!("\n警告:");
                for w in risk_overlay.warnings {
                    println!("- {}", w);
                }
            }
        }
        cli::ReportCommands::Snapshot { save } => {
            let snapshot = engine::report::create_portfolio_snapshot(&config, &state);
            println!("组合快照预览 (Snapshot)\n");
            println!("日期: {}", snapshot.date);
            println!("总资产: {:.2}", snapshot.total_assets);
            println!("现金: {:.2}", snapshot.cash);
            println!("权益: {:.2}", snapshot.equity_value);

            if *save {
                let mut snapshots = repo.load_portfolio_snapshots(&ctx).await?;
                snapshots.push(snapshot);
                repo.save_portfolio_snapshots(&ctx, &snapshots).await?;
                println!("\n快照已保存至: {}", repo.get_snapshot_path());
            }
        }
    }

    Ok::<(), anyhow::Error>(())
}

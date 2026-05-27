use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "pendulum-kelly-cli")]
#[command(about = "A local portfolio ledger tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to config.toml
    #[arg(long, global = true, default_value = "data/config.toml")]
    pub config: String,

    /// Path to portfolio_state.json
    #[arg(long, global = true, default_value = "data/portfolio_state.json")]
    pub state: String,

    /// Path to transactions.json
    #[arg(long, global = true, default_value = "data/transactions.json")]
    pub transactions: String,

    /// Path to fund_nav_cache.json
    #[arg(long, global = true, default_value = "data/fund_nav_cache.json")]
    pub cache: String,

    /// Path to market_price_cache.json
    #[arg(long, global = true, default_value = "data/market_price_cache.json")]
    pub market_cache: String,

    /// Path to fx_usd_cnh_cache.json
    #[arg(long, global = true, default_value = "data/fx_usd_cnh_cache.json")]
    pub fx_cache: String,

    /// Path to dca_plans.json
    #[arg(long, global = true, default_value = "data/dca_plans.json")]
    pub dca_plans: String,

    /// Path to alipay_snapshots.json
    #[arg(long, global = true, default_value = "data/alipay_snapshots.json")]
    pub alipay_snapshots: String,

    /// Path to instruments.toml
    #[arg(long, global = true, default_value = "data/instruments.toml")]
    pub instruments: String,

    /// Path to dca_settlements.json
    #[arg(long, global = true, default_value = "data/dca_settlements.json")]
    pub dca_settlements: String,

    /// Path to reconciliation_audit.json
    #[arg(long, global = true, default_value = "data/reconciliation_audit.json")]
    pub reconciliation_audit: String,

    /// Path to dca_settlement_audit.json
    #[arg(long, global = true, default_value = "data/dca_settlement_audit.json")]
    pub dca_settlement_audit: String,

    /// Path to cache_status.json
    #[arg(long, global = true, default_value = "data/cache_status.json")]
    pub cache_status: String,

    /// Path to instrument_quote_cache.json
    #[arg(
        long,
        global = true,
        default_value = "data/instrument_quote_cache.json"
    )]
    pub instrument_cache: String,

    /// Path to risk_cache.json
    #[arg(long, global = true, default_value = "data/risk_cache.json")]
    pub risk_cache: String,

    /// Path to proxy_valuation_cache.json
    #[arg(long, global = true, default_value = "data/proxy_valuation_cache.json")]
    pub proxy_cache: String,

    /// Path to regime_cache.json
    #[arg(long, global = true, default_value = "data/regime_cache.json")]
    pub regime_cache: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// View current holdings
    Holdings {
        #[arg(long)]
        all: bool,

        /// Show proxy estimated values
        #[arg(long)]
        proxy: bool,
    },

    /// Valuation commands
    Valuation {
        #[command(subcommand)]
        command: ValuationCommands,
    },

    /// Mark to market: update valuations
    Mtm,

    /// Portfolio commands
    Portfolio {
        #[command(subcommand)]
        command: PortfolioCommands,
    },

    /// Sector commands
    Sector {
        #[command(subcommand)]
        command: SectorCommands,
    },

    /// Decision commands
    Decision {
        #[command(subcommand)]
        command: DecisionCommands,
    },

    /// Fund commands
    Fund {
        #[command(subcommand)]
        command: FundCommands,
    },

    /// Market data commands
    Market {
        #[command(subcommand)]
        command: MarketCommands,
    },

    /// Asset commands
    Asset {
        #[command(subcommand)]
        command: AssetCommands,
    },

    /// Configuration and system health commands
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Transaction commands
    Tx {
        #[command(subcommand)]
        command: TxCommands,
    },

    /// Report and review commands
    Report {
        #[command(subcommand)]
        command: ReportCommands,
    },

    /// Cash management commands
    Cash {
        #[command(subcommand)]
        command: CashCommands,
    },

    /// Expense commands
    Expense {
        #[command(subcommand)]
        command: ExpenseCommands,
    },

    /// Start the local web UI or run web diagnostics
    Web {
        /// Port to listen on
        #[arg(long, default_value = "8787")]
        port: u16,

        #[command(subcommand)]
        command: Option<WebCommands>,
    },

    /// Data management commands
    Data {
        #[command(subcommand)]
        command: DataCommands,
    },

    /// Daily operations commands
    Ops {
        #[command(subcommand)]
        command: OpsCommands,
    },

    /// FX commands
    Fx {
        #[command(subcommand)]
        command: FxCommands,
    },

    /// Risk commands
    Risk {
        #[command(subcommand)]
        command: RiskCommands,
    },

    /// Kelly sizing preview commands
    Kelly {
        #[command(subcommand)]
        command: KellyCommands,
    },

    /// DCA plan management
    Dca {
        #[command(subcommand)]
        command: DcaCommands,
    },

    /// Reconciliation commands
    Reconcile {
        #[command(subcommand)]
        command: ReconcileCommands,
    },

    /// Market instrument registry commands
    Instrument {
        #[command(subcommand)]
        command: InstrumentCommands,
    },

    /// Manage and preview daily execution plan
    Daily {
        #[command(subcommand)]
        command: DailyCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum ReconcileCommands {
    /// Alipay reconciliation subcommands
    Alipay {
        #[command(subcommand)]
        command: AlipayReconcileCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum AlipayReconcileCommands {
    /// Add an Alipay holding snapshot
    Add {
        #[arg(long)]
        asset_id: String,
        #[arg(long)]
        date: String,
        #[arg(long)]
        market_value: f64,
        #[arg(long)]
        units: Option<f64>,
        #[arg(long)]
        cost_basis: Option<f64>,
        #[arg(long)]
        nav: Option<f64>,
        #[arg(long)]
        nav_date: Option<String>,
        #[arg(long)]
        daily_pnl: Option<f64>,
        #[arg(long)]
        total_pnl: Option<f64>,
        #[arg(long)]
        note: Option<String>,
    },
    /// List Alipay snapshots
    List,
    /// Compare system holding against Alipay snapshot
    Compare {
        #[arg(long)]
        asset_id: String,
        #[arg(long)]
        date: Option<String>,
    },
    /// Compare all assets against latest Alipay snapshots
    CompareAll,
    /// Suggest calibration based on reconciliation
    Suggest {
        #[arg(long)]
        asset_id: String,
    },
    /// Apply calibration to system state
    Apply {
        #[arg(long)]
        snapshot_id: String,
        #[arg(long)]
        confirm: bool,
        #[arg(long, default_value_t = false)]
        allow_calibration_apply: bool,
    },
    /// Remove an Alipay snapshot
    Remove {
        #[arg(long)]
        snapshot_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum DcaCommands {
    /// Add a new DCA plan
    Add {
        #[arg(long)]
        asset_id: String,

        #[arg(long)]
        amount: f64,

        /// Frequency: daily, weekly, monthly
        #[arg(long)]
        frequency: String,

        #[arg(long)]
        start_date: Option<String>,

        #[arg(long)]
        end_date: Option<String>,

        /// 1-7 for weekly (1=Monday)
        #[arg(long)]
        weekday: Option<u32>,

        /// 1-31 for monthly
        #[arg(long)]
        month_day: Option<u32>,

        #[arg(long)]
        note: Option<String>,

        #[arg(long, default_value = "0")]
        priority: i32,
    },

    /// List all DCA plans
    List,

    /// Preview today's due DCA plans
    Preview {
        /// Preview for a specific date (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
    },

    /// Disable a DCA plan
    Disable {
        #[arg(long)]
        plan_id: String,
    },

    /// Enable a DCA plan
    Enable {
        #[arg(long)]
        plan_id: String,
    },

    /// Remove a DCA plan
    Remove {
        #[arg(long)]
        plan_id: String,
    },

    /// Compare DCA total with decision engine versions
    CompareDecision,

    /// Manage DCA settlements (confirmed reality)
    Settlement {
        #[command(subcommand)]
        command: DcaSettlementCommands,
    },

    /// Show DCA lifecycle status for all plans
    Lifecycle {
        /// Date for the lifecycle check (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
        /// Filter by asset ID
        #[arg(long)]
        asset_id: Option<String>,
    },

    /// Show DCA items that need manual attention
    Pending,

    /// Detailed explanation of lifecycle status for an asset
    LifecycleExplain {
        #[arg(long)]
        asset_id: String,
        /// Date for the explanation (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum DcaSettlementCommands {
    /// Add a confirmed DCA settlement
    Add {
        #[arg(long)]
        asset_id: String,
        #[arg(long)]
        amount: f64,
        #[arg(long)]
        confirmed_nav: f64,
        #[arg(long)]
        confirmed_units: f64,
        #[arg(long)]
        deduction_date: String,
        #[arg(long)]
        confirmation_date: String,
        #[arg(long)]
        plan_id: Option<String>,
        #[arg(long)]
        fee: Option<f64>,
        #[arg(long)]
        note: Option<String>,
    },
    /// List DCA settlements
    List,
    /// Preview impact of a settlement
    Preview {
        #[arg(long)]
        settlement_id: String,
    },
    /// Apply a confirmed settlement to holdings
    Apply {
        #[arg(long)]
        settlement_id: String,
        #[arg(long)]
        confirm: bool,
    },
    /// Compare settlement impact with latest Alipay snapshot
    CompareAlipay {
        #[arg(long)]
        settlement_id: String,
    },
    /// Remove a settlement record
    Remove {
        #[arg(long)]
        settlement_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum KellyCommands {
    /// Calculate and preview Kelly-adjusted buy suggestions
    Preview,

    /// Calculate and view Kelly portfolio-level preview
    Portfolio,

    /// Explain Kelly calculation for a specific asset
    Explain {
        #[arg(long)]
        asset_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum FxCommands {
    /// Get USD/CNH exchange rate
    UsdCnh,

    /// View USD/CNH exchange rate history
    UsdCnhHistory {
        #[arg(long, default_value = "30")]
        days: usize,
    },
}

#[derive(Subcommand, Debug)]
pub enum RiskCommands {
    /// View crypto risk basket
    Crypto {
        #[arg(long)]
        symbol: Option<String>,
    },

    /// View risk snapshot
    Snapshot,

    /// View individual risk factors and their current status
    Factors,

    /// Calculate and view global risk overlay analysis
    Overlay,

    /// Provide detailed explanation of the global risk score
    Explain,

    /// View history for a specific risk symbol
    History {
        /// Symbol to lookup (positional, e.g. ^VIX)
        symbol: Option<String>,

        /// Symbol to lookup (named option, e.g. --symbol ^VIX)
        #[arg(long = "symbol")]
        symbol_opt: Option<String>,

        /// Number of days to look back
        #[arg(long, default_value = "250")]
        days: usize,

        /// Optional provider override
        #[arg(long)]
        provider: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum TxCommands {
    /// List all transactions
    List,

    /// Add a transaction
    Add {
        #[command(subcommand)]
        command: TxAddCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum TxAddCommands {
    /// Record a buy transaction
    Buy {
        #[arg(long)]
        asset_id: String,

        #[arg(long)]
        amount: f64,

        #[arg(long)]
        price: f64,

        #[arg(long)]
        date: String,

        #[arg(long, default_value = "")]
        note: String,

        #[arg(long)]
        units: Option<f64>,

        #[arg(long, default_value = "0.0")]
        fee: f64,

        #[arg(long, default_value = "CNY")]
        currency: String,
    },

    /// Record a sell transaction
    Sell {
        #[arg(long)]
        asset_id: String,

        #[arg(long, required_unless_present = "amount")]
        units: Option<f64>,

        #[arg(long)]
        price: f64,

        #[arg(long)]
        date: String,

        #[arg(long, default_value = "")]
        note: String,

        #[arg(long, required_unless_present = "units")]
        amount: Option<f64>,

        #[arg(long, default_value = "0.0")]
        fee: f64,

        #[arg(long, default_value = "CNY")]
        currency: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum CashCommands {
    /// Set manual cash balance
    Set {
        #[arg(long)]
        amount: f64,
    },

    /// Record cash in
    In {
        #[arg(long)]
        amount: f64,

        #[arg(long, default_value = "")]
        note: String,
    },

    /// Record cash out
    Out {
        #[arg(long)]
        amount: f64,

        #[arg(long, default_value = "")]
        note: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum FundCommands {
    /// Lookup a fund by code
    Lookup { fund_code: String },

    /// Validate fund names in config against real data
    Validate,

    /// Sync local fund name with real data for a specific asset
    SyncName {
        #[arg(long)]
        asset_id: String,
    },

    /// Sync all local fund names with real data
    SyncAllNames,
}

#[derive(Subcommand, Debug)]
pub enum MarketCommands {
    /// Lookup latest market price for a symbol
    Lookup {
        symbol: String,
        /// Explicitly specify the market data provider (e.g., yahoo, mock)
        #[arg(long)]
        provider: Option<String>,
    },

    /// View recent market history (daily candles)
    History {
        symbol: String,
        #[arg(long, default_value = "30")]
        days: usize,
        /// Explicitly specify the market data provider (e.g., yahoo, mock)
        #[arg(long)]
        provider: Option<String>,
    },

    /// Analyze market regime (Pendulum Score)
    Regime {
        /// Symbol to analyze (e.g. QQQ)
        symbol: Option<String>,

        /// Analyze a specific asset's reference index
        #[arg(long)]
        asset_id: Option<String>,

        /// Number of days to look back for analysis
        #[arg(long, default_value = "250")]
        days: usize,

        /// Optional provider override
        #[arg(long)]
        provider: Option<String>,
    },

    /// Analyze market regime for all assets with reference indexes
    RegimeAll,

    /// Explain market regime calculation for a symbol
    RegimeExplain {
        /// Symbol to explain
        symbol: String,

        /// Optional provider override
        #[arg(long)]
        provider: Option<String>,
    },

    /// View or set the default market provider
    Provider {
        #[command(subcommand)]
        command: Option<MarketProviderCommands>,
    },
}

#[derive(Subcommand, Debug)]
pub enum MarketProviderCommands {
    /// Set the default market provider (yahoo, mock)
    Set { provider: String },
}

#[derive(Subcommand, Debug)]
pub enum ReportCommands {
    /// Generate a daily review report
    Daily {
        /// Date for the report (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
        /// Save the report to data/reports/
        #[arg(long)]
        save: bool,
    },
    /// Generate a weekly review report
    Weekly {
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start: Option<String>,
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end: Option<String>,
        /// Save the report to data/reports/
        #[arg(long)]
        save: bool,
    },
    /// Generate a monthly review report
    Monthly {
        /// Month (YYYY-MM)
        #[arg(long)]
        month: Option<String>,
        /// Save the report to data/reports/
        #[arg(long)]
        save: bool,
    },
    /// Portfolio status report
    Portfolio,
    /// DCA lifecycle report
    Dca,
    /// Reconciliation summary report
    Reconcile,
    /// Risk and market regime report
    Risk,
    /// Portfolio snapshot commands
    Snapshot {
        /// Save current snapshot to data/portfolio_snapshots.json
        #[arg(long)]
        save: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum DailyCommands {
    /// Show daily execution plan
    Plan {
        /// Date for the plan (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
    },
    /// Compact summary of daily execution plan
    Summary,
    /// Detailed explanation for a specific asset in the daily plan
    Explain {
        #[arg(long)]
        asset_id: String,
    },
    /// Show a practical daily manual checklist
    Checklist,
}

#[derive(Subcommand, Debug)]
pub enum AssetCommands {
    /// List all configured assets
    List,

    /// Add a new asset to config and state
    Add {
        #[arg(long)]
        asset_id: String,

        #[arg(long)]
        fund_code: String,

        #[arg(long)]
        fund_name: Option<String>,

        #[arg(long)]
        sector: String,

        #[arg(long)]
        currency: String,

        #[arg(long)]
        valuation_method: String,

        #[arg(long, default_value = "0")]
        units: f64,

        #[arg(long, default_value = "0")]
        cost_basis: f64,

        /// Allow multiple assets to use the same fund code
        #[arg(long)]
        allow_duplicate_fund_code: bool,
    },

    /// Disable an asset
    Disable {
        #[arg(long)]
        asset_id: String,
    },

    /// Enable an asset
    Enable {
        #[arg(long)]
        asset_id: String,
    },

    /// Remove an asset (disables it)
    Remove {
        #[arg(long)]
        asset_id: String,
    },

    /// Set an asset's sector
    SetSector {
        #[arg(long)]
        asset_id: String,

        #[arg(long)]
        sector: String,
    },

    /// Set an asset's fund code and optionally sync name
    SetFundCode {
        #[arg(long)]
        asset_id: String,

        #[arg(long)]
        fund_code: String,

        /// Keep the existing local name instead of syncing with real data
        #[arg(long)]
        keep_name: bool,

        /// Allow multiple assets to use the same fund code
        #[arg(long)]
        allow_duplicate_fund_code: bool,
    },

    /// Rename an asset (local name only)
    Rename {
        #[arg(long)]
        asset_id: String,

        #[arg(long)]
        fund_name: String,
    },

    /// Validate all assets in config
    Validate,

    /// Set reference index for an asset
    SetReference {
        #[arg(long)]
        asset_id: String,

        #[arg(long)]
        reference_index_name: String,

        #[arg(long)]
        reference_index_symbol: String,

        #[arg(long)]
        market_data_provider: String,

        #[arg(long)]
        reference_index_currency: Option<String>,

        #[arg(long)]
        proxy_fx_pair: Option<String>,

        #[arg(long)]
        use_fx_adjustment: Option<bool>,

        #[arg(long)]
        reference_instrument_id: Option<String>,

        #[arg(long)]
        reference_instrument_symbol: Option<String>,
    },

    /// List asset reference indexes
    ReferenceList,

    /// Repair missing holdings for configured assets
    RepairHoldings,

    /// Validate all reference indexes linked to assets
    ReferenceValidate,

    /// List assets with duplicate fund codes
    Duplicates,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Run a configuration health check
    Doctor,
}

#[derive(Subcommand, Debug)]
pub enum ValuationCommands {
    /// Preview current fund values estimated by reference indexes
    ProxyPreview,

    /// Explain the proxy valuation calculation for a specific asset
    ProxyExplain {
        #[arg(long)]
        asset_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum PortfolioCommands {
    /// Calculate and view portfolio summary
    Summary,
}

#[derive(Subcommand, Debug)]
pub enum SectorCommands {
    /// List all configured sectors
    List,

    /// View sector summary and targets
    Summary,

    /// Set a sector's target weight
    SetTarget {
        #[arg(long)]
        sector_id: String,

        #[arg(long)]
        target_weight: f64,
    },

    /// Add a new sector
    Add {
        #[arg(long)]
        sector_id: String,

        #[arg(long)]
        name: String,

        #[arg(long)]
        asset_class: String,

        #[arg(long)]
        target_weight: f64,

        #[arg(long)]
        priority: i32,
    },

    /// Disable a sector
    Disable {
        #[arg(long)]
        sector_id: String,
    },

    /// Enable a sector
    Enable {
        #[arg(long)]
        sector_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum DecisionCommands {
    /// Output today's buy suggestions based on targets
    Preview,

    /// Explain the rationale behind the buy suggestions
    Explain,

    /// Calculate and preview risk-adjusted buy suggestions
    AdjustedPreview,

    /// Explain adjusted calculation for a specific asset
    AdjustedExplain {
        #[arg(long)]
        asset_id: String,
    },

    /// Compare base and adjusted suggestions
    Compare,
}

#[derive(Subcommand, Debug)]
pub enum ExpenseCommands {
    /// Record an expense
    Add {
        #[arg(long)]
        amount: f64,

        #[arg(long, default_value = "")]
        note: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum InstrumentCommands {
    /// List all configured instruments
    List,
    /// Lookup latest price for an instrument
    Lookup {
        /// Symbol or instrument ID
        symbol: Option<String>,
        #[arg(long)]
        instrument_id: Option<String>,
    },
    /// View price history for an instrument
    History {
        /// Symbol or instrument ID
        symbol: Option<String>,
        #[arg(long)]
        instrument_id: Option<String>,
        #[arg(long, default_value = "30")]
        days: usize,
    },
    /// Add a new instrument to the registry
    Add {
        #[arg(long)]
        instrument_id: String,
        #[arg(long)]
        symbol: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        name_zh: Option<String>,
        #[arg(long)]
        name_en: Option<String>,
        #[arg(long)]
        description_zh: Option<String>,
        #[arg(long)]
        category_zh: Option<String>,
        #[arg(long)]
        display_label: Option<String>,
        #[arg(long)]
        asset_class: String,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        provider_symbol: String,
        #[arg(long)]
        market: Option<String>,
        #[arg(long)]
        currency: String,
        #[arg(long)]
        quote_unit: String,
        #[arg(long)]
        price_unit: String,
        #[arg(long)]
        note: Option<String>,
    },
    /// Disable an instrument
    Disable {
        #[arg(long)]
        instrument_id: String,
    },
    /// Enable an instrument
    Enable {
        #[arg(long)]
        instrument_id: String,
    },
    /// Validate all enabled instruments
    Validate,
    /// Compact watchlist-style snapshot of all instruments
    Snapshot,
}

#[derive(Subcommand, Debug)]
pub enum WebCommands {
    /// Performance and cache diagnostics for web UI
    Doctor,
}

#[derive(Subcommand, Debug)]
pub enum DataCommands {
    /// Refresh provider-backed data
    Refresh {
        /// Refresh all data
        #[arg(long)]
        all: bool,
        /// Refresh fund NAV data
        #[arg(long)]
        fund: bool,
        /// Refresh market price data
        #[arg(long)]
        market: bool,
        /// Refresh risk factor data
        #[arg(long)]
        risk: bool,
        /// Refresh instrument registry data
        #[arg(long)]
        instrument: bool,
        /// Refresh proxy valuation data
        #[arg(long)]
        proxy: bool,
        /// Refresh daily plan data
        #[arg(long)]
        daily: bool,
    },
    /// Show cache freshness status
    CacheStatus,
}

#[derive(Subcommand, Debug)]
pub enum OpsCommands {
    /// Summarize today's investment workflow
    Today {
        /// Date for the summary (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
        /// Show detailed tables
        #[arg(long, short)]
        verbose: bool,
    },
    /// Refresh all provider-backed data caches
    Refresh,
    /// Compact status view of portfolio and environment
    Status {
        /// Show detailed info
        #[arg(long, short)]
        verbose: bool,
    },
    /// Operational readiness check
    Doctor {
        /// Show detailed diagnostic info
        #[arg(long, short)]
        verbose: bool,
    },
    /// Comprehensive daily manual checklist
    Checklist,
}

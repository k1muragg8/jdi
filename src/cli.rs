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

    /// Cash commands
    Cash {
        #[command(subcommand)]
        command: CashCommands,
    },

    /// Expense commands
    Expense {
        #[command(subcommand)]
        command: ExpenseCommands,
    },

    /// Start the local web UI
    Web {
        /// Port to listen on
        #[arg(long, default_value = "8787")]
        port: u16,
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

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
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// View current holdings
    Holdings,

    /// Mark to market: update valuations
    Mtm,

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
pub enum ExpenseCommands {
    /// Record an expense
    Add {
        #[arg(long)]
        amount: f64,

        #[arg(long, default_value = "")]
        note: String,
    },
}

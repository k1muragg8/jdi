use anyhow::{Context, Result};

pub mod cache_status_store;
pub mod cache_store;
pub mod config_store;
pub mod dca_store;
pub mod fx_cache_store;
pub mod instrument_cache_store;
pub mod instrument_store;
pub mod market_cache_store;
pub mod proxy_cache_store;
pub mod reconciliation_store;
pub mod regime_cache_store;
pub mod report_store;
pub mod risk_cache_store;
pub mod snapshot_store;
pub mod state_store;
pub mod transaction_store;
pub mod web_audit_store;

pub use cache_status_store::{load_cache_status, save_cache_status};
pub use cache_store::{load_cache, save_cache};
pub use config_store::{load_config, save_config};
pub use dca_store::{load_dca_plans, save_dca_plans};
pub use fx_cache_store::{load_fx_cache, save_fx_cache};
pub use instrument_cache_store::{load_instrument_cache, save_instrument_cache};
pub use market_cache_store::{load_market_cache, save_market_cache};
pub use proxy_cache_store::{load_proxy_cache, save_proxy_cache};
pub use reconciliation_store::{
    load_alipay_snapshots, load_reconciliation_audits, save_alipay_snapshots,
    save_reconciliation_audits,
};
pub use regime_cache_store::{load_regime_cache, save_regime_cache};
pub use report_store::save_markdown_report;
pub use risk_cache_store::{load_risk_cache, save_risk_cache};
pub use snapshot_store::{load_snapshots, save_snapshots};
pub use state_store::{load_state, save_state};
pub use transaction_store::{load_transactions, save_transactions};

pub fn create_backup<P: AsRef<std::path::Path>>(path: P) -> Result<()> {
    let path_ref = path.as_ref();
    if !path_ref.exists() {
        return Ok(());
    }
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    let extension = path_ref.extension().and_then(|e| e.to_str()).unwrap_or("");
    let backup_path = path_ref.with_extension(format!("{}.bak.{}", timestamp, extension));
    std::fs::copy(path_ref, backup_path).with_context(|| "Failed to create backup")?;
    Ok(())
}

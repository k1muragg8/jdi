//! Action-specific form types.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct AddSnapshotForm {
    pub asset_id: String,
    pub snapshot_date: String,
    pub market_value: f64,
    pub units: Option<f64>,
    pub cost_basis: Option<f64>,
    pub nav: Option<f64>,
    pub nav_date: Option<String>,
    pub total_pnl: Option<f64>,
}

#[derive(Deserialize)]
pub struct DcaAddForm {
    pub asset_id: String,
    pub amount: f64,
    pub frequency: String,
    pub day: Option<u32>,
}

#[derive(Deserialize)]
pub struct AssetFundCodeForm {
    pub asset_id: String,
    pub fund_code: String,
}

#[derive(Deserialize)]
pub struct InstrumentIdForm {
    pub instrument_id: String,
}

#[derive(Deserialize)]
pub struct InstrumentAddForm {
    pub symbol: String,
    pub instrument_id: Option<String>,
    pub name_zh: Option<String>,
    pub asset_class: Option<String>,
    pub provider: Option<String>,
    pub currency: Option<String>,
}

#[derive(Deserialize)]
pub struct ReconcileApplyForm {
    pub snapshot_id: String,
    pub confirm: Option<String>,
}

#[derive(Deserialize)]
pub struct AddSettlementForm {
    pub asset_id: String,
    pub plan_id: Option<String>,
    pub deduction_date: String,
    pub confirmation_date: String,
    pub amount: f64,
    pub confirmed_nav: f64,
    pub confirmed_units: f64,
    pub fee: Option<f64>,
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct SettlementApplyForm {
    pub settlement_id: String,
    pub confirm: String,
}

#[derive(Deserialize)]
pub struct DcaIdForm {
    pub plan_id: String,
}

#[derive(Deserialize)]
pub struct DcaUpdateAmountForm {
    pub plan_id: String,
    pub amount: f64,
}

#[derive(Deserialize)]
pub struct AssetRenameForm {
    pub asset_id: String,
    pub fund_name: String,
}

#[derive(Deserialize)]
pub struct AssetSectorForm {
    pub asset_id: String,
    pub sector: String,
}

#[derive(Deserialize)]
pub struct InstrumentMetadataForm {
    pub instrument_id: String,
    pub name_zh: Option<String>,
    pub display_label: Option<String>,
    pub provider: Option<String>,
    pub provider_symbol: Option<String>,
}

#[derive(Deserialize)]
pub struct CashReverseForm {
    pub tx_id: String,
}

#[derive(Deserialize)]
pub struct AssetAddForm {
    pub fund_name: String,
    pub fund_code: String,
    pub sector: Option<String>,
}

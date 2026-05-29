use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub date: String,
    pub transaction_type: String, // "buy", "sell", "cash_in", "cash_out", "expense", "dividend", "fee"
    pub asset_id: Option<String>,
    pub amount: f64,
    pub units: Option<f64>,
    pub price: Option<f64>,
    pub fee: f64,
    pub currency: String,
    pub note: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub raw_description: String,
}

impl Transaction {
    /// Generates a deterministic fingerprint based on all fields except the ID.
    /// This is useful for deduplication if IDs are not trusted or missing.
    pub fn fingerprint(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.date.hash(&mut hasher);
        self.transaction_type.hash(&mut hasher);
        self.asset_id.hash(&mut hasher);
        self.amount.to_bits().hash(&mut hasher);
        self.units.map(|u| u.to_bits()).hash(&mut hasher);
        self.price.map(|p| p.to_bits()).hash(&mut hasher);
        self.fee.to_bits().hash(&mut hasher);
        self.currency.hash(&mut hasher);
        self.note.hash(&mut hasher);
        self.source.hash(&mut hasher);
        self.raw_description.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

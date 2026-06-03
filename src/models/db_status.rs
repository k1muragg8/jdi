use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DbStatus {
    pub backend: String,
    pub database_url_source: String,
    pub database_name: Option<String>,
    pub schema: Option<String>,
    pub user: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub fallback: bool,
    pub data_dir: Option<String>,
    pub tables: Vec<TableCount>,
    pub migrations_active: bool,
    pub active_portfolio_id: String,
    pub portfolio_records: Vec<TableCount>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TableCount {
    pub name: String,
    pub count: i64,
}

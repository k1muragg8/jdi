use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAdminAudit {
    pub audit_id: String,
    pub timestamp: String,
    pub actor: String,                  // "local_web"
    pub actor_user_id: Option<String>,  // default "local_user"
    pub target_user_id: Option<String>, // default "local_user"
    pub portfolio_id: Option<String>,   // default "default"
    pub role: Option<String>,           // default "owner"
    pub action: String,
    pub target_file: String,
    pub target_id: Option<String>,
    pub old_value_summary: String,
    pub new_value_summary: String,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebAdminAuditLog {
    pub records: Vec<WebAdminAuditRecord>,
}

// Renaming for clarity and requirement consistency
pub type WebAdminAuditRecord = WebAdminAudit;

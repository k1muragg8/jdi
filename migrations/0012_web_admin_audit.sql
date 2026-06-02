-- 0012_web_admin_audit.sql
CREATE TABLE IF NOT EXISTS web_admin_audit_logs (
    id SERIAL PRIMARY KEY,
    audit_id TEXT UNIQUE NOT NULL,
    timestamp TEXT NOT NULL,
    actor TEXT NOT NULL,
    actor_user_id TEXT,
    target_user_id TEXT,
    portfolio_id TEXT,
    role TEXT,
    action TEXT NOT NULL,
    target_file TEXT NOT NULL,
    target_id TEXT,
    old_value_summary TEXT,
    new_value_summary TEXT,
    status TEXT NOT NULL,
    note TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_web_admin_audit_portfolio_id ON web_admin_audit_logs(portfolio_id);
CREATE INDEX idx_web_admin_audit_timestamp ON web_admin_audit_logs(timestamp);

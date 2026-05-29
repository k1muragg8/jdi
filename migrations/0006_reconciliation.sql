CREATE TABLE alipay_snapshots (
    snapshot_id VARCHAR(100) PRIMARY KEY,
    portfolio_id VARCHAR(50) NOT NULL REFERENCES portfolios(id) ON DELETE CASCADE,
    asset_id VARCHAR(50) NOT NULL,
    fund_code VARCHAR(50) NOT NULL,
    fund_name VARCHAR(255) NOT NULL,
    snapshot_date DATE NOT NULL,
    market_value DOUBLE PRECISION NOT NULL,
    units DOUBLE PRECISION,
    cost_basis DOUBLE PRECISION,
    nav DOUBLE PRECISION,
    nav_date DATE,
    daily_pnl DOUBLE PRECISION,
    total_pnl DOUBLE PRECISION,
    source VARCHAR(50) NOT NULL DEFAULT 'alipay',
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alipay_snapshots_asset_date ON alipay_snapshots(asset_id, snapshot_date);

CREATE TABLE reconciliation_audits (
    audit_id VARCHAR(100) PRIMARY KEY,
    portfolio_id VARCHAR(50) NOT NULL REFERENCES portfolios(id) ON DELETE CASCADE,
    snapshot_id VARCHAR(100) REFERENCES alipay_snapshots(snapshot_id) ON DELETE SET NULL,
    asset_id VARCHAR(50) NOT NULL,
    old_units DOUBLE PRECISION NOT NULL,
    new_units DOUBLE PRECISION NOT NULL,
    old_cost_basis DOUBLE PRECISION NOT NULL,
    new_cost_basis DOUBLE PRECISION NOT NULL,
    old_market_value DOUBLE PRECISION NOT NULL,
    new_market_value DOUBLE PRECISION NOT NULL,
    reason VARCHAR(255) NOT NULL,
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

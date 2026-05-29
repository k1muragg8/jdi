CREATE TABLE dca_plans (
    plan_id VARCHAR(100) PRIMARY KEY,
    portfolio_id VARCHAR(50) NOT NULL REFERENCES portfolios(id) ON DELETE CASCADE,
    asset_id VARCHAR(50) NOT NULL,
    fund_code VARCHAR(50) NOT NULL,
    fund_name VARCHAR(255) NOT NULL,
    amount DOUBLE PRECISION NOT NULL,
    currency VARCHAR(10) NOT NULL DEFAULT 'CNY',
    frequency VARCHAR(20) NOT NULL,
    weekday INTEGER,
    month_day INTEGER,
    start_date DATE NOT NULL,
    end_date DATE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    priority INTEGER NOT NULL DEFAULT 0,
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE dca_settlements (
    settlement_id VARCHAR(100) PRIMARY KEY,
    portfolio_id VARCHAR(50) NOT NULL REFERENCES portfolios(id) ON DELETE CASCADE,
    plan_id VARCHAR(100) REFERENCES dca_plans(plan_id) ON DELETE SET NULL,
    asset_id VARCHAR(50) NOT NULL,
    fund_code VARCHAR(50) NOT NULL,
    fund_name VARCHAR(255) NOT NULL,
    scheduled_date DATE,
    deduction_date DATE NOT NULL,
    confirmation_date DATE NOT NULL,
    amount DOUBLE PRECISION NOT NULL,
    confirmed_nav DOUBLE PRECISION NOT NULL,
    confirmed_units DOUBLE PRECISION NOT NULL,
    fee DOUBLE PRECISION DEFAULT 0.0,
    currency VARCHAR(10) NOT NULL DEFAULT 'CNY',
    source VARCHAR(50) NOT NULL DEFAULT 'alipay',
    status VARCHAR(20) NOT NULL,
    applied BOOLEAN NOT NULL DEFAULT FALSE,
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_dca_settlements_asset ON dca_settlements(asset_id);
CREATE INDEX idx_dca_settlements_dates ON dca_settlements(deduction_date, confirmation_date);

CREATE TABLE dca_settlement_audits (
    audit_id VARCHAR(100) PRIMARY KEY,
    portfolio_id VARCHAR(50) NOT NULL REFERENCES portfolios(id) ON DELETE CASCADE,
    settlement_id VARCHAR(100) NOT NULL REFERENCES dca_settlements(settlement_id) ON DELETE CASCADE,
    asset_id VARCHAR(50) NOT NULL,
    old_units DOUBLE PRECISION NOT NULL,
    new_units DOUBLE PRECISION NOT NULL,
    old_cost_basis DOUBLE PRECISION NOT NULL,
    new_cost_basis DOUBLE PRECISION NOT NULL,
    transaction_id VARCHAR(100),
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

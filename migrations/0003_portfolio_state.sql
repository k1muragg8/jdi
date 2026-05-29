CREATE TABLE portfolios (
    id VARCHAR(50) PRIMARY KEY,
    current_cash DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE holdings (
    portfolio_id VARCHAR(50) NOT NULL REFERENCES portfolios(id) ON DELETE CASCADE,
    asset_id VARCHAR(50) NOT NULL,
    fund_code VARCHAR(50) NOT NULL,
    units DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    units_estimated BOOLEAN NOT NULL DEFAULT FALSE,
    cost_basis DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    last_market_value DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    latest_nav DOUBLE PRECISION,
    latest_nav_date DATE,
    latest_nav_source VARCHAR(50),
    latest_nav_status VARCHAR(50),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (portfolio_id, asset_id)
);

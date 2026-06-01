-- Operation Policy table
CREATE TABLE IF NOT EXISTS operation_policies (
    portfolio_id TEXT PRIMARY KEY REFERENCES portfolios(id) ON DELETE CASCADE,
    target_total_investment_amount DOUBLE PRECISION,
    target_equity_weight DOUBLE PRECISION NOT NULL DEFAULT 0.8,
    min_cash_reserve DOUBLE PRECISION NOT NULL DEFAULT 10000.0,
    max_daily_buy_amount DOUBLE PRECISION NOT NULL DEFAULT 3000.0,
    max_single_asset_buy_amount DOUBLE PRECISION NOT NULL DEFAULT 1000.0,
    max_single_asset_weight DOUBLE PRECISION NOT NULL DEFAULT 0.15,
    max_sector_weight DOUBLE PRECISION NOT NULL DEFAULT 0.3,
    dca_auto_pause_when_target_reached BOOLEAN NOT NULL DEFAULT TRUE,
    dca_auto_resume_when_below_target BOOLEAN NOT NULL DEFAULT TRUE,
    dca_resume_threshold DOUBLE PRECISION NOT NULL DEFAULT 0.95,
    dca_pause_threshold DOUBLE PRECISION NOT NULL DEFAULT 1.05,
    kelly_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    max_kelly_fraction DOUBLE PRECISION NOT NULL DEFAULT 0.25,
    pendulum_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    volatility_window_days INTEGER NOT NULL DEFAULT 20,
    risk_overlay_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    market_refresh_interval_seconds INTEGER NOT NULL DEFAULT 180,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Operation Status table
CREATE TABLE IF NOT EXISTS operation_statuses (
    portfolio_id TEXT PRIMARY KEY REFERENCES portfolios(id) ON DELETE CASCADE,
    last_run_at TEXT,
    last_report_json TEXT,
    is_running BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

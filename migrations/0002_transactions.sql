-- Transactions table for primary ledger
CREATE TABLE transactions (
    id VARCHAR(100) PRIMARY KEY,
    portfolio_id VARCHAR(50) NOT NULL,
    transaction_date DATE NOT NULL,
    transaction_type VARCHAR(50) NOT NULL,
    asset_id VARCHAR(100),
    amount DOUBLE PRECISION NOT NULL,
    units DOUBLE PRECISION,
    price DOUBLE PRECISION,
    fee DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    currency VARCHAR(10) NOT NULL,
    note TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_transactions_portfolio_id ON transactions(portfolio_id);
CREATE INDEX idx_transactions_date ON transactions(transaction_date);
CREATE INDEX idx_transactions_asset_id ON transactions(asset_id);

CREATE TABLE IF NOT EXISTS instruments (
    instrument_id VARCHAR(50) PRIMARY KEY,
    symbol VARCHAR(50) NOT NULL,
    display_symbol VARCHAR(50),
    name VARCHAR(255) NOT NULL,
    name_zh VARCHAR(255),
    name_en VARCHAR(255),
    description_zh TEXT,
    category_zh VARCHAR(100),
    display_label VARCHAR(100),
    asset_class VARCHAR(50) NOT NULL,
    provider VARCHAR(50) NOT NULL,
    provider_symbol VARCHAR(50) NOT NULL,
    market VARCHAR(50),
    exchange VARCHAR(50),
    currency VARCHAR(10) NOT NULL,
    quote_unit VARCHAR(20) NOT NULL,
    price_unit VARCHAR(20) NOT NULL,
    timezone VARCHAR(50),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    priority INTEGER NOT NULL DEFAULT 0,
    tags JSONB NOT NULL DEFAULT '[]',
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_instruments_symbol ON instruments(symbol);

CREATE TABLE IF NOT EXISTS cache_instruments (
    instrument_id VARCHAR(50) PRIMARY KEY,
    symbol VARCHAR(50) NOT NULL,
    name_zh VARCHAR(255),
    price DOUBLE PRECISION NOT NULL,
    date DATE NOT NULL,
    currency VARCHAR(10) NOT NULL,
    quote_unit VARCHAR(20) NOT NULL,
    provider VARCHAR(50) NOT NULL,
    source VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    warning TEXT,
    fetched_at TIMESTAMPTZ NOT NULL
);

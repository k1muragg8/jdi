-- 0013_cache_and_pipeline.sql
CREATE TABLE IF NOT EXISTS global_caches (
    cache_key TEXT PRIMARY KEY,
    data_json TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS daily_operation_reports (
    portfolio_id TEXT PRIMARY KEY REFERENCES portfolios(id) ON DELETE CASCADE,
    report_json TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

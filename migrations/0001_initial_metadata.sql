-- Initial metadata table for storing arbitrary application state, similar to cache_statuses
CREATE TABLE IF NOT EXISTS application_metadata (
    key VARCHAR(255) PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

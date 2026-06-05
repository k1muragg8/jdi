-- 0014_web_jobs.sql
-- Persisted web jobs for async long-running operations (daily pipeline, market refresh, etc.)
CREATE TABLE IF NOT EXISTS web_jobs (
    id TEXT PRIMARY KEY,
    portfolio_id TEXT NOT NULL,
    job_type TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    progress_current INTEGER NOT NULL DEFAULT 0,
    progress_total INTEGER NOT NULL DEFAULT 0,
    message TEXT,
    result_json JSONB,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_web_jobs_portfolio ON web_jobs (portfolio_id);
CREATE INDEX IF NOT EXISTS idx_web_jobs_type ON web_jobs (job_type);
CREATE INDEX IF NOT EXISTS idx_web_jobs_status ON web_jobs (status);
CREATE INDEX IF NOT EXISTS idx_web_jobs_created ON web_jobs (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_web_jobs_portfolio_type ON web_jobs (portfolio_id, job_type);

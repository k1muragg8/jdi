-- 1. Users Table
CREATE TABLE IF NOT EXISTS users (
    id VARCHAR(50) PRIMARY KEY,
    email VARCHAR(255) UNIQUE,
    external_id VARCHAR(100) UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert Default User
INSERT INTO users (id, email) 
VALUES ('local_user', 'local_user@localhost') 
ON CONFLICT (id) DO NOTHING;

-- 2. Portfolios update
ALTER TABLE portfolios ADD COLUMN IF NOT EXISTS owner_user_id VARCHAR(50);

-- Ensure the 'default' portfolio exists
INSERT INTO portfolios (id, owner_user_id, current_cash) 
VALUES ('default', 'local_user', 0.0) 
ON CONFLICT (id) DO UPDATE SET owner_user_id = 'local_user' WHERE portfolios.owner_user_id IS NULL;

-- 3. Orphaned data cleanup/adoption
-- Assign any existing orphaned portfolios to the default user
UPDATE portfolios SET owner_user_id = 'local_user' WHERE owner_user_id IS NULL;

-- Adopt orphaned transactions by creating their missing portfolios (mainly for tests)
INSERT INTO portfolios (id, owner_user_id, current_cash)
SELECT DISTINCT portfolio_id, 'local_user', 0.0
FROM transactions
WHERE portfolio_id NOT IN (SELECT id FROM portfolios)
ON CONFLICT DO NOTHING;

-- 4. Foreign Keys and Indexes
-- Use a DO block to safely add constraints without errors if they already exist
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_portfolios_owner') THEN
        ALTER TABLE portfolios ADD CONSTRAINT fk_portfolios_owner FOREIGN KEY (owner_user_id) REFERENCES users(id) ON DELETE SET NULL;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_transactions_portfolio') THEN
        ALTER TABLE transactions ADD CONSTRAINT fk_transactions_portfolio FOREIGN KEY (portfolio_id) REFERENCES portfolios(id) ON DELETE CASCADE;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_portfolios_owner_user_id ON portfolios(owner_user_id);

-- 5. User Portfolio Roles (for future multi-tenant collaboration)
CREATE TABLE IF NOT EXISTS user_portfolio_roles (
    user_id VARCHAR(50) REFERENCES users(id) ON DELETE CASCADE,
    portfolio_id VARCHAR(50) REFERENCES portfolios(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL, -- e.g., 'owner', 'viewer', 'editor'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY(user_id, portfolio_id)
);

-- Insert default role
INSERT INTO user_portfolio_roles (user_id, portfolio_id, role) 
VALUES ('local_user', 'default', 'owner') 
ON CONFLICT (user_id, portfolio_id) DO NOTHING;

-- Add name and description to portfolios
ALTER TABLE portfolios ADD COLUMN IF NOT EXISTS name VARCHAR(100);
ALTER TABLE portfolios ADD COLUMN IF NOT EXISTS description TEXT;
ALTER TABLE portfolios ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Set default name for existing default portfolio
UPDATE portfolios SET name = 'Default Portfolio' WHERE id = 'default' AND name IS NULL;

-- Make name NOT NULL for future entries (after updating existing ones)
-- We'll just leave it nullable for now to be safe, or set a default.
UPDATE portfolios SET name = id WHERE name IS NULL;

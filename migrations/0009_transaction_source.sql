-- Add source and raw_description to transactions table
ALTER TABLE transactions ADD COLUMN source VARCHAR(100) NOT NULL DEFAULT 'manual';
ALTER TABLE transactions ADD COLUMN raw_description TEXT NOT NULL DEFAULT '';

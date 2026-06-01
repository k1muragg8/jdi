-- Add target asset and sector weights to operation_policies
ALTER TABLE operation_policies ADD COLUMN IF NOT EXISTS target_asset_weights_json TEXT;
ALTER TABLE operation_policies ADD COLUMN IF NOT EXISTS target_sector_weights_json TEXT;

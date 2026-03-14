-- Add meta_json column to fields table for storing JSON metadata (e.g., ext_github config)
ALTER TABLE fields ADD COLUMN meta_json TEXT DEFAULT NULL;

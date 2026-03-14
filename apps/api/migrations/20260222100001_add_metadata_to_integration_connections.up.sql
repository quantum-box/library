-- Add metadata column to integration_connections for storing additional data (webhook_url, etc.)
ALTER TABLE `integration_connections`
    ADD COLUMN `metadata` JSON COMMENT 'Additional metadata (webhook_url, etc.)';

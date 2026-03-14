-- Remove metadata column from integration_connections
ALTER TABLE `integration_connections` DROP COLUMN IF EXISTS `metadata`;

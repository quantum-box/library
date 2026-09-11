ALTER TABLE `outbound_deliveries`
    DROP INDEX `idx_outbound_deliveries_due`,
    DROP COLUMN `scanner_claim_expires_at`,
    DROP COLUMN `scanner_claim_owner`;

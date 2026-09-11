ALTER TABLE `outbound_deliveries`
    ADD COLUMN `scanner_claim_owner` VARCHAR(64) NULL,
    ADD COLUMN `scanner_claim_expires_at` TIMESTAMP(6) NULL,
    ADD INDEX `idx_outbound_deliveries_due`
        (`status`, `next_attempt_at`, `id`);

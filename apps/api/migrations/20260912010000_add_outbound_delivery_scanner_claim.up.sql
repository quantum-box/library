ALTER TABLE `outbound_deliveries`
    ADD COLUMN `scanner_claim_owner` VARCHAR(64) NULL
        AFTER `next_attempt_at`,
    ADD COLUMN `scanner_claim_expires_at` TIMESTAMP(6) NULL
        AFTER `scanner_claim_owner`,
    ADD INDEX `idx_outbound_deliveries_due`
        (`status`, `next_attempt_at`, `id`);

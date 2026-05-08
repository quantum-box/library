CREATE TABLE IF NOT EXISTS `oauth_tokens` (
    `id` VARCHAR(32) NOT NULL COMMENT 'Token ID (ULID)',
    `tenant_id` VARCHAR(29) NOT NULL COMMENT 'Tenant ID',
    `provider` VARCHAR(32) NOT NULL COMMENT 'OAuth provider',
    `access_token` TEXT NOT NULL COMMENT 'Encrypted access token',
    `refresh_token` TEXT COMMENT 'Encrypted refresh token',
    `token_type` VARCHAR(32) NOT NULL DEFAULT 'Bearer' COMMENT 'Token type',
    `expires_at` TIMESTAMP NULL COMMENT 'Token expiration time',
    `scope` VARCHAR(255) COMMENT 'OAuth scopes',
    `external_account_id` VARCHAR(255) COMMENT 'External account ID',
    `external_account_name` VARCHAR(255) COMMENT 'External account name',
    `created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    UNIQUE KEY `uk_tenant_provider` (`tenant_id`, `provider`),
    INDEX `idx_tenant_id` (`tenant_id`),
    INDEX `idx_provider` (`provider`),
    INDEX `idx_expires_at` (`expires_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='OAuth tokens for external integrations';

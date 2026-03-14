-- Integration connections for tracking integration status
CREATE TABLE IF NOT EXISTS `integration_connections` (
    `id` VARCHAR(32) NOT NULL COMMENT 'Connection ID (ULID)',
    `tenant_id` VARCHAR(29) NOT NULL COMMENT 'Tenant ID',
    `integration_id` VARCHAR(32) NOT NULL COMMENT 'Integration ID',
    `provider` VARCHAR(32) NOT NULL COMMENT 'Provider name',
    `status` VARCHAR(32) NOT NULL DEFAULT 'active' COMMENT 'Connection status (active, paused, expired, disconnected, error)',
    `external_account_id` VARCHAR(255) COMMENT 'External account ID',
    `external_account_name` VARCHAR(255) COMMENT 'External account name',
    `token_expires_at` TIMESTAMP NULL COMMENT 'Token expiration time',
    `last_synced_at` TIMESTAMP NULL COMMENT 'Last successful sync time',
    `error_message` TEXT COMMENT 'Last error message if status is error',
    `created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT 'Creation time',
    `updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT 'Last update time',
    PRIMARY KEY (`id`),
    UNIQUE KEY `uk_tenant_integration` (`tenant_id`, `integration_id`),
    INDEX `idx_tenant_id` (`tenant_id`),
    INDEX `idx_integration_id` (`integration_id`),
    INDEX `idx_status` (`status`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Integration connections for tenants';

-- OAuth tokens table
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

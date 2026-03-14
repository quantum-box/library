-- Database sync configurations table
CREATE TABLE IF NOT EXISTS database_sync_configs (
    id VARCHAR(26) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(32) NOT NULL,
    data_id VARCHAR(64) NOT NULL,
    provider VARCHAR(32) NOT NULL,
    target_container VARCHAR(255) NOT NULL,
    target_resource VARCHAR(512),
    target_version VARCHAR(128),
    status VARCHAR(32) NOT NULL DEFAULT 'never_synced',
    status_error TEXT,
    last_synced_at DATETIME(6),
    last_result_id VARCHAR(128),
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    
    INDEX idx_sync_configs_tenant_id (tenant_id),
    INDEX idx_sync_configs_data_id (data_id),
    INDEX idx_sync_configs_provider (provider),
    UNIQUE INDEX idx_sync_configs_data_provider (data_id, provider)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;


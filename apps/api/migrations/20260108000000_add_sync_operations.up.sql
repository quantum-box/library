-- Add sync_operations table for API pull synchronization tracking
-- Part of library inbound_sync Phase 1 implementation

CREATE TABLE IF NOT EXISTS sync_operations (
    id VARCHAR(30) PRIMARY KEY COMMENT 'syo_ prefixed ULID',
    endpoint_id VARCHAR(30) NOT NULL COMMENT 'FK to webhook_endpoints',
    operation_type ENUM('webhook', 'initial_sync', 'on_demand_pull', 'scheduled_sync') NOT NULL COMMENT 'Type of sync operation',
    status ENUM('queued', 'running', 'completed', 'failed', 'cancelled') NOT NULL DEFAULT 'queued' COMMENT 'Current status of the operation',
    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT 'When the operation started',
    completed_at DATETIME NULL COMMENT 'When the operation completed',
    stats JSON NULL COMMENT 'Processing statistics (created, updated, deleted, skipped counts)',
    error_message TEXT NULL COMMENT 'Error message if operation failed',
    progress VARCHAR(255) NULL COMMENT 'Progress information (e.g., "10/100 files processed")',

    INDEX idx_sync_operations_endpoint (endpoint_id, started_at DESC),
    INDEX idx_sync_operations_status (status),
    INDEX idx_sync_operations_started (started_at DESC),

    CONSTRAINT fk_sync_operations_endpoint
        FOREIGN KEY (endpoint_id) REFERENCES webhook_endpoints(id)
        ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='API pull synchronization operations log';

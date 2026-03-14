-- Inbound Sync Engine Tables
-- Webhook endpoints, events, and sync states for inbound webhook synchronization

-- Webhook Endpoint Configuration
CREATE TABLE IF NOT EXISTS webhook_endpoints (
    id VARCHAR(30) PRIMARY KEY COMMENT 'whe_ prefixed ULID',
    tenant_id VARCHAR(30) NOT NULL COMMENT 'Organization/tenant ID',
    repository_id VARCHAR(30) NULL COMMENT 'Target Library repository (optional)',
    name VARCHAR(255) NOT NULL COMMENT 'Display name',
    provider VARCHAR(50) NOT NULL COMMENT 'github, linear, hubspot, stripe, notion, airtable, generic',
    config JSON NOT NULL COMMENT 'Provider-specific configuration',
    events JSON NOT NULL COMMENT 'Events to listen for (empty array = all)',
    mapping_config JSON NULL COMMENT 'Property mapping configuration',
    secret_hash VARCHAR(255) NOT NULL COMMENT 'Hashed webhook secret',
    status ENUM('active', 'paused', 'disabled') NOT NULL DEFAULT 'active',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    INDEX idx_webhook_endpoints_tenant (tenant_id),
    INDEX idx_webhook_endpoints_tenant_provider (tenant_id, provider),
    INDEX idx_webhook_endpoints_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Webhook endpoint configurations for receiving external webhooks';

-- Webhook Event Log
CREATE TABLE IF NOT EXISTS webhook_events (
    id VARCHAR(30) PRIMARY KEY COMMENT 'wev_ prefixed ULID',
    endpoint_id VARCHAR(30) NOT NULL COMMENT 'FK to webhook_endpoints',
    provider VARCHAR(50) NOT NULL COMMENT 'Provider type',
    event_type VARCHAR(100) NOT NULL COMMENT 'Event type (e.g., push, issue.updated)',
    payload JSON NOT NULL COMMENT 'Raw webhook payload',
    headers JSON NULL COMMENT 'HTTP headers from webhook request',
    signature_valid BOOLEAN NOT NULL DEFAULT false COMMENT 'Whether signature verification passed',
    processing_status ENUM('pending', 'processing', 'completed', 'failed', 'skipped') NOT NULL DEFAULT 'pending',
    error_message TEXT NULL COMMENT 'Error message if failed',
    retry_count INT NOT NULL DEFAULT 0 COMMENT 'Number of retry attempts',
    next_retry_at DATETIME NULL COMMENT 'Next retry time for pending retries',
    stats JSON NULL COMMENT 'Processing statistics (created, updated, deleted counts)',
    received_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT 'When webhook was received',
    processed_at DATETIME NULL COMMENT 'When processing completed',

    INDEX idx_webhook_events_endpoint (endpoint_id, received_at DESC),
    INDEX idx_webhook_events_status (processing_status, next_retry_at),
    INDEX idx_webhook_events_received (received_at),

    CONSTRAINT fk_webhook_events_endpoint
        FOREIGN KEY (endpoint_id) REFERENCES webhook_endpoints(id)
        ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Log of received webhook events and their processing status';

-- Sync State (External ID <-> Library Data ID mapping)
CREATE TABLE IF NOT EXISTS sync_states (
    id VARCHAR(30) PRIMARY KEY COMMENT 'sst_ prefixed ULID',
    endpoint_id VARCHAR(30) NOT NULL COMMENT 'FK to webhook_endpoints',
    data_id VARCHAR(30) NOT NULL COMMENT 'Library data ID',
    external_id VARCHAR(255) NOT NULL COMMENT 'External service resource ID',
    external_version VARCHAR(100) NULL COMMENT 'External version/ETag for conflict detection',
    local_version VARCHAR(100) NULL COMMENT 'Local version for conflict detection',
    sync_direction ENUM('inbound', 'outbound', 'both') NOT NULL DEFAULT 'inbound',
    last_synced_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    metadata JSON NULL COMMENT 'Additional metadata',

    UNIQUE INDEX idx_sync_states_endpoint_external (endpoint_id, external_id),
    UNIQUE INDEX idx_sync_states_endpoint_data (endpoint_id, data_id),
    INDEX idx_sync_states_last_synced (last_synced_at),

    CONSTRAINT fk_sync_states_endpoint
        FOREIGN KEY (endpoint_id) REFERENCES webhook_endpoints(id)
        ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Tracks synchronization state between external resources and Library data';

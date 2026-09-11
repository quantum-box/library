-- Provider-neutral review, delivery, and durable webhook dispatch lifecycle.
CREATE TABLE IF NOT EXISTS `inbound_change_sets` (
    `id`                         VARCHAR(30)  NOT NULL COMMENT 'ChangeSet ID (ics_)',
    `tenant_id`                  VARCHAR(29)  NOT NULL,
    `binding_id`                 VARCHAR(30)  NOT NULL,
    `data_id`                    VARCHAR(64)  NULL COMMENT 'Library data ID when already linked',
    `external_object_id`         VARCHAR(1024) NOT NULL,
    `external_object_id_hash`    CHAR(64)     NOT NULL,
    `external_revision`          VARCHAR(255) NOT NULL,
    `base_external_revision`     VARCHAR(255) NULL,
    `change_type`                VARCHAR(16)  NOT NULL,
    `payload`                    JSON         NOT NULL COMMENT 'Sanitized provider-neutral proposed content',
    `idempotency_key`            CHAR(64)     NOT NULL,
    `status`                     VARCHAR(16)  NOT NULL DEFAULT 'pending',
    `decision_note`              TEXT         NULL,
    `created_at`                 TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    `decided_at`                 TIMESTAMP(6) NULL,
    PRIMARY KEY (`id`),
    UNIQUE KEY `uk_inbound_change_sets_idempotency`
        (`binding_id`, `idempotency_key`),
    INDEX `idx_inbound_change_sets_review`
        (`tenant_id`, `binding_id`, `status`, `created_at`),
    INDEX `idx_inbound_change_sets_object`
        (`binding_id`, `external_object_id_hash`, `created_at`),
    CONSTRAINT `fk_inbound_change_sets_binding` FOREIGN KEY (`binding_id`)
        REFERENCES `external_sync_bindings` (`id`) ON DELETE CASCADE,
    CONSTRAINT `chk_inbound_change_sets_type` CHECK (
        CAST(`change_type` AS BINARY) IN ('upsert', 'tombstone', 'rename')
    ),
    CONSTRAINT `chk_inbound_change_sets_status` CHECK (
        CAST(`status` AS BINARY) IN ('pending', 'accepted', 'rejected', 'conflict')
    )
)
COMMENT='Provider changes awaiting an explicit Library decision';

CREATE TABLE IF NOT EXISTS `outbound_deliveries` (
    `id`                         VARCHAR(30)  NOT NULL COMMENT 'Delivery ID (odl_)',
    `tenant_id`                  VARCHAR(29)  NOT NULL,
    `binding_id`                 VARCHAR(30)  NOT NULL,
    `data_id`                    VARCHAR(64)  NOT NULL,
    `external_object_id`         VARCHAR(1024) NOT NULL,
    `external_object_id_hash`    CHAR(64)     NOT NULL,
    `library_revision`           VARCHAR(255) NOT NULL,
    `base_external_revision`     VARCHAR(255) NULL,
    `payload`                    JSON         NOT NULL COMMENT 'Immutable delivery snapshot',
    `idempotency_key`            CHAR(64)     NOT NULL,
    `status`                     VARCHAR(16)  NOT NULL DEFAULT 'pending',
    `attempt_count`              INT UNSIGNED NOT NULL DEFAULT 0,
    `next_attempt_at`            TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    `last_error_category`        VARCHAR(64)  NULL,
    `remote_revision`            VARCHAR(255) NULL,
    `delivery_url`               VARCHAR(2048) NULL,
    `created_at`                 TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    `updated_at`                 TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                                             ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (`id`),
    UNIQUE KEY `uk_outbound_deliveries_idempotency`
        (`binding_id`, `idempotency_key`),
    INDEX `idx_outbound_deliveries_dispatch`
        (`tenant_id`, `binding_id`, `status`, `next_attempt_at`),
    INDEX `idx_outbound_deliveries_data`
        (`binding_id`, `data_id`, `created_at`),
    CONSTRAINT `fk_outbound_deliveries_binding` FOREIGN KEY (`binding_id`)
        REFERENCES `external_sync_bindings` (`id`) ON DELETE CASCADE,
    CONSTRAINT `chk_outbound_deliveries_status` CHECK (
        CAST(`status` AS BINARY) IN (
            'pending', 'retrying', 'delivered', 'conflict', 'failed'
        )
    )
)
COMMENT='At-least-once delivery of Library revisions to external objects';

-- The API stores only a digest of the capability. The plaintext capability is
-- handed to a Durable Object, which presents it when invoking the consumer.
CREATE TABLE IF NOT EXISTS `external_sync_dispatch_jobs` (
    `event_id`                   VARCHAR(30)  NOT NULL,
    `capability_hash`            CHAR(64)     NOT NULL,
    `status`                     VARCHAR(16)  NOT NULL DEFAULT 'pending',
    `attempt_count`              INT UNSIGNED NOT NULL DEFAULT 0,
    `next_attempt_at`            TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    `lease_expires_at`           TIMESTAMP(6) NULL,
    `last_error_category`        VARCHAR(64)  NULL,
    `created_at`                 TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    `updated_at`                 TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                                             ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (`event_id`),
    UNIQUE KEY `uk_external_sync_dispatch_capability` (`capability_hash`),
    INDEX `idx_external_sync_dispatch_due`
        (`status`, `next_attempt_at`, `lease_expires_at`),
    CONSTRAINT `fk_external_sync_dispatch_event` FOREIGN KEY (`event_id`)
        REFERENCES `webhook_events` (`id`) ON DELETE CASCADE,
    CONSTRAINT `chk_external_sync_dispatch_status` CHECK (
        CAST(`status` AS BINARY) IN ('pending', 'processing', 'completed', 'failed')
    )
)
COMMENT='Capability-gated durable dispatch from edge alarm to webhook consumer';

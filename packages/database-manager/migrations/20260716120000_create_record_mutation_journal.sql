-- Expand-only journal for versioned Record patch decisions and Database BC
-- domain events. The existing updateData path remains unchanged until the API
-- and apps/web adopt mandatory expected_version and operation_id inputs.

CREATE TABLE record_mutation_operations (
    operation_id VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    tenant_id VARCHAR(29) NOT NULL,
    database_id VARCHAR(29) NOT NULL,
    data_id VARCHAR(31) NOT NULL,
    mutation_kind VARCHAR(8) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    actor_kind VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    actor_id VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    expected_version BIGINT UNSIGNED NOT NULL,
    fingerprint_version SMALLINT UNSIGNED NOT NULL,
    request_fingerprint BINARY(32) NOT NULL,
    decision_kind VARCHAR(8) CHARACTER SET ascii COLLATE ascii_bin NOT NULL
        DEFAULT 'PENDING',
    decision_version SMALLINT UNSIGNED NULL,
    decision_payload JSON NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    decided_at TIMESTAMP(6) NULL,

    PRIMARY KEY (operation_id),
    CONSTRAINT chk_record_operations_mutation_kind CHECK (
        CAST(mutation_kind AS BINARY) IN ('PATCH', 'CREATE', 'DELETE')
    ),
    CONSTRAINT chk_record_operations_actor_kind CHECK (
        CAST(actor_kind AS BINARY) IN ('USER', 'SERVICE_ACCOUNT', 'SYSTEM')
    ),
    CONSTRAINT chk_record_operations_actor_id CHECK (actor_id <> ''),
    CONSTRAINT chk_record_operations_expected_version CHECK (
        expected_version > 0
    ),
    CONSTRAINT chk_record_operations_fingerprint_version CHECK (
        fingerprint_version > 0
    ),
    CONSTRAINT chk_record_operations_decision_kind CHECK (
        CAST(decision_kind AS BINARY) IN (
            'PENDING', 'ACCEPTED', 'CONFLICT', 'REJECTED'
        )
    ),
    CONSTRAINT chk_record_operations_decision_payload CHECK (
        (
            CAST(decision_kind AS BINARY) = 'PENDING'
            AND decision_version IS NULL
            AND decision_payload IS NULL
            AND decided_at IS NULL
        )
        OR (
            CAST(decision_kind AS BINARY) <> 'PENDING'
            AND decision_version IS NOT NULL
            AND decision_version > 0
            AND decision_payload IS NOT NULL
            AND decided_at IS NOT NULL
        )
    ),
    INDEX idx_record_operations_scope (
        tenant_id, database_id, data_id, created_at, operation_id
    )
);

-- Outbox rows intentionally have no FK to objects/data. A pending event must
-- survive aggregate deletion until every consumer reaches a terminal state.
CREATE TABLE domain_outbox_events (
    event_id VARCHAR(31) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    operation_id VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    event_sequence INT UNSIGNED NOT NULL,
    tenant_id VARCHAR(29) NOT NULL,
    database_id VARCHAR(29) NOT NULL,
    aggregate_type VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    aggregate_id VARCHAR(31) NOT NULL,
    aggregate_version BIGINT UNSIGNED NOT NULL,
    event_type VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    payload JSON NOT NULL,
    occurred_at TIMESTAMP(6) NOT NULL,

    PRIMARY KEY (event_id),
    CONSTRAINT uq_domain_outbox_operation_sequence
        UNIQUE (operation_id, event_sequence),
    CONSTRAINT uq_domain_outbox_aggregate_version UNIQUE (
        tenant_id,
        database_id,
        aggregate_type,
        aggregate_id,
        aggregate_version
    ),
    CONSTRAINT chk_domain_outbox_aggregate_type CHECK (
        CAST(aggregate_type AS BINARY) IN ('RECORD')
    ),
    CONSTRAINT chk_domain_outbox_aggregate_version CHECK (
        aggregate_version > 0
    ),
    CONSTRAINT chk_domain_outbox_event_sequence CHECK (
        event_sequence > 0
    ),
    CONSTRAINT fk_domain_outbox_operation FOREIGN KEY (operation_id)
        REFERENCES record_mutation_operations (operation_id)
        ON DELETE RESTRICT,
    INDEX idx_domain_outbox_aggregate_order (
        tenant_id,
        database_id,
        aggregate_type,
        aggregate_id,
        aggregate_version,
        event_id
    ),
    INDEX idx_domain_outbox_occurred (occurred_at, event_id)
);

-- Event lifecycle and per-consumer delivery lifecycle are deliberately
-- separate. Rows are created by a later dispatcher/consumer registration
-- slice; the API Lambda does not spawn a resident poller.
CREATE TABLE domain_outbox_deliveries (
    event_id VARCHAR(31) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    consumer_name VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    state VARCHAR(9) CHARACTER SET ascii COLLATE ascii_bin NOT NULL
        DEFAULT 'PENDING',
    attempt_count INT UNSIGNED NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    lease_owner VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NULL,
    lease_expires_at TIMESTAMP(6) NULL,
    delivered_at TIMESTAMP(6) NULL,
    last_error TEXT NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
        ON UPDATE CURRENT_TIMESTAMP(6),

    PRIMARY KEY (event_id, consumer_name),
    CONSTRAINT chk_domain_outbox_delivery_state CHECK (
        CAST(state AS BINARY) IN (
            'PENDING', 'INFLIGHT', 'DELIVERED', 'DEAD'
        )
    ),
    CONSTRAINT fk_domain_outbox_delivery_event FOREIGN KEY (event_id)
        REFERENCES domain_outbox_events (event_id)
        ON DELETE CASCADE,
    INDEX idx_domain_outbox_delivery_claim (
        consumer_name,
        state,
        next_attempt_at,
        lease_expires_at,
        event_id
    )
);

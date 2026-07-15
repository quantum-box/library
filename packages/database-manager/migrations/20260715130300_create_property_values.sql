-- Canonical PropertyValue storage. This is expand-only: value0..value50 and
-- field_num remain available to the legacy reader/writer until parity and the
-- rollback window have completed.
CREATE TABLE property_values (
    tenant_id VARCHAR(29) NOT NULL,
    database_id VARCHAR(29) NOT NULL,
    data_id VARCHAR(31) NOT NULL,
    property_id VARCHAR(31) NOT NULL,
    type_key VARCHAR(64) NOT NULL,
    type_version SMALLINT UNSIGNED NOT NULL DEFAULT 1,
    value_encoding_version SMALLINT UNSIGNED NOT NULL DEFAULT 1,
    value LONGTEXT NOT NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
        ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, database_id, data_id, property_id),
    CONSTRAINT fk_property_values_tenant_database_data
        FOREIGN KEY (tenant_id, database_id, data_id)
        REFERENCES data (tenant_id, object_id, id)
        ON DELETE CASCADE,
    CONSTRAINT fk_property_values_tenant_database_property
        FOREIGN KEY (tenant_id, database_id, property_id)
        REFERENCES fields (tenant_id, object_id, id)
        ON DELETE CASCADE,
    INDEX idx_property_values_property_scan
        (tenant_id, database_id, property_id, data_id)
);

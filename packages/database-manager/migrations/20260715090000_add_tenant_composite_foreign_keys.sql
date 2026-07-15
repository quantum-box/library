-- Enforce the tenant boundary in the physical schema.
--
-- Keep FOREIGN_KEY_CHECKS enabled. Each new composite foreign key is added
-- before its legacy single-column counterpart is removed so that a failed
-- validation leaves the existing constraint in place.

-- Referenced columns must be backed by an index whose leading columns match
-- the foreign key. The ids remain globally unique primary keys; these unique
-- tenant-leading candidate keys make the tenant part of every reference.
ALTER TABLE objects
    ADD CONSTRAINT uq_objects_tenant_id_id UNIQUE (tenant_id, id);

ALTER TABLE fields
    ADD CONSTRAINT uq_fields_tenant_object_id_id
        UNIQUE (tenant_id, object_id, id),
    ADD CONSTRAINT fk_fields_tenant_object
        FOREIGN KEY (tenant_id, object_id)
        REFERENCES objects (tenant_id, id);

ALTER TABLE data
    ADD CONSTRAINT uq_data_tenant_id_id UNIQUE (tenant_id, id),
    ADD INDEX idx_data_tenant_object_id (tenant_id, object_id),
    ADD CONSTRAINT fk_data_tenant_object
        FOREIGN KEY (tenant_id, object_id)
        REFERENCES objects (tenant_id, id);

-- `indexes.object_id` is the legacy name of the referenced data record id.
ALTER TABLE indexes
    ADD INDEX idx_indexes_tenant_data_id (tenant_id, object_id),
    ADD CONSTRAINT fk_indexes_tenant_data
        FOREIGN KEY (tenant_id, object_id)
        REFERENCES data (tenant_id, id);

ALTER TABLE relationships
    ADD INDEX idx_relationships_tenant_object_field_id
        (tenant_id, object_id, field_id),
    ADD INDEX idx_relationships_tenant_target_object_id
        (tenant_id, target_object_id),
    ADD CONSTRAINT fk_relationships_tenant_object
        FOREIGN KEY (tenant_id, object_id)
        REFERENCES objects (tenant_id, id),
    ADD CONSTRAINT fk_relationships_tenant_target_object
        FOREIGN KEY (tenant_id, target_object_id)
        REFERENCES objects (tenant_id, id),
    ADD CONSTRAINT fk_relationships_tenant_object_field
        FOREIGN KEY (tenant_id, object_id, field_id)
        REFERENCES fields (tenant_id, object_id, id);

-- The stronger constraints are now active and have validated existing rows.
ALTER TABLE fields
    DROP FOREIGN KEY fk_fields_objects;

ALTER TABLE data
    DROP FOREIGN KEY fk_data_objects;

ALTER TABLE indexes
    DROP FOREIGN KEY fk_indexes_data;

ALTER TABLE relationships
    DROP FOREIGN KEY fk_relationships_object_id,
    DROP FOREIGN KEY fk_relationships_target_object_id,
    DROP FOREIGN KEY fk_relationships_field_id;

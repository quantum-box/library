-- `relationships` already stores one row per Relation Property. Keep its
-- physical name and legacy columns during the compatibility window while
-- making the row an explicit RelationDefinition control plane.
--
-- Existing definitions came from a multi-value CSV representation and had no
-- inverse or lifecycle configuration. Preserve those semantics as
-- Many/Many/Restrict with no inverse Property.
ALTER TABLE relationships
    ADD COLUMN forward_cardinality VARCHAR(4) NOT NULL DEFAULT 'MANY',
    ADD COLUMN reverse_cardinality VARCHAR(4) NOT NULL DEFAULT 'MANY',
    ADD COLUMN inverse_field_id VARCHAR(31) NULL,
    ADD COLUMN on_target_delete VARCHAR(8) NOT NULL DEFAULT 'RESTRICT',
    ADD CONSTRAINT chk_relationships_forward_cardinality
        CHECK (forward_cardinality IN ('ONE', 'MANY')),
    ADD CONSTRAINT chk_relationships_reverse_cardinality
        CHECK (reverse_cardinality IN ('ONE', 'MANY')),
    ADD CONSTRAINT chk_relationships_on_target_delete
        CHECK (on_target_delete IN ('RESTRICT', 'NULLIFY')),
    ADD CONSTRAINT uq_relationships_tenant_source_field
        UNIQUE (tenant_id, object_id, field_id),
    ADD CONSTRAINT uq_relationships_tenant_inverse_field
        UNIQUE (tenant_id, target_object_id, inverse_field_id),
    ADD CONSTRAINT fk_relationships_tenant_target_inverse_field
        FOREIGN KEY (tenant_id, target_object_id, inverse_field_id)
        REFERENCES fields (tenant_id, object_id, id);

-- A RelationDefinition is owned by its source Property. Deleting that
-- Property removes only the definition; target Database deletion remains
-- restricted while a definition points at it.
ALTER TABLE relationships
    DROP FOREIGN KEY fk_relationships_tenant_object_field,
    ADD CONSTRAINT fk_relationships_tenant_source_property
        FOREIGN KEY (tenant_id, object_id, field_id)
        REFERENCES fields (tenant_id, object_id, id)
        ON DELETE CASCADE;

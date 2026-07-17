ALTER TABLE relationships
    ADD CONSTRAINT uq_relationships_tenant_inverse_field
        UNIQUE (tenant_id, target_object_id, inverse_field_id);

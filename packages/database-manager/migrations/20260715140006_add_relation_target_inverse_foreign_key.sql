ALTER TABLE relationships
    ADD CONSTRAINT fk_relationships_tenant_target_inverse_field
        FOREIGN KEY (tenant_id, target_object_id, inverse_field_id)
        REFERENCES fields (tenant_id, object_id, id);

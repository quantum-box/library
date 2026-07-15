-- PropertyValue references a record through its complete tenant/database/data
-- scope. The globally unique data id remains the primary key.
ALTER TABLE data
    ADD CONSTRAINT uq_data_tenant_object_id_id
        UNIQUE (tenant_id, object_id, id);

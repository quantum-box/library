-- `field_num` remains the legacy value-column address during the expand and
-- rollback window. Prevent two concurrent schema mutations from sharing it.
--
-- MySQL unique indexes allow multiple NULL values. The generated marker is 1
-- only for an Id property, so at most one Id definition can exist per scoped
-- Database while every non-Id property remains unconstrained by this marker.
-- Keep this column VIRTUAL: TiDB supports indexing virtual generated columns
-- but does not support adding a stored generated column through ALTER TABLE.
ALTER TABLE fields
    ADD COLUMN id_singleton_marker TINYINT UNSIGNED
        GENERATED ALWAYS AS (
            CASE WHEN UPPER(datatype) = 'ID' THEN 1 ELSE NULL END
        ) VIRTUAL,
    ADD CONSTRAINT uq_fields_tenant_object_field_num
        UNIQUE (tenant_id, object_id, field_num),
    ADD CONSTRAINT uq_fields_tenant_object_id_singleton
        UNIQUE (tenant_id, object_id, id_singleton_marker);

-- TiDB supports only one added CHECK constraint per ALTER TABLE statement.
ALTER TABLE property_values
    ADD CONSTRAINT chk_property_values_type_version
        CHECK (type_version > 0);

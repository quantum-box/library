-- TiDB supports only one added CHECK constraint per ALTER TABLE statement.
ALTER TABLE property_values
    ADD CONSTRAINT chk_property_values_encoding_version
        CHECK (value_encoding_version > 0);

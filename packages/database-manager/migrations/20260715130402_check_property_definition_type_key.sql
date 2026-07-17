-- TiDB supports only one added CHECK constraint per ALTER TABLE statement.
ALTER TABLE fields
    ADD CONSTRAINT ck_fields_property_definition_type_key
        CHECK (
            type_key IS NULL
            OR REGEXP_LIKE(
                type_key,
                '^[a-z][a-z0-9]*(_[a-z0-9]+)*$',
                'c'
            )
        );

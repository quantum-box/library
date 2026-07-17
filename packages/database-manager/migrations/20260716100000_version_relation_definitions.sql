-- Version RelationDefinition mutations without changing the legacy physical
-- table or its compatibility columns. Existing definitions become readable
-- V1/generation-1 rows. Only inverse Properties created by the new schema
-- mutation UoW are marked as owned; compatibility inverses remain detached
-- from lifecycle deletion.
--
-- TiDB applies DDL even when a migration later fails. Keep each schema change
-- independently retryable so a partially applied migration cannot block the
-- next deployment.
ALTER TABLE relationships
    ADD COLUMN inverse_owned BOOLEAN NOT NULL DEFAULT FALSE
        AFTER inverse_field_id;

-- Expand phase for optimistic record concurrency.
-- Existing and newly created records begin at version 1. Write-side CAS and
-- version increments are introduced by a later rollout slice.
ALTER TABLE data
    ADD COLUMN record_version BIGINT UNSIGNED NOT NULL DEFAULT 1,
    ADD CONSTRAINT chk_data_record_version_nonzero
        CHECK (record_version > 0);

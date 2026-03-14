-- Add up migration script here
ALTER TABLE repos ADD COLUMN is_public BOOLEAN NOT NULL DEFAULT FALSE;
-- Add down migration script here
ALTER TABLE repos DROP COLUMN is_public;

-- Add down migration script here
-- Revert the addition of org_id column
ALTER TABLE library.repos
DROP COLUMN org_id;

-- Revert the modification of username column
ALTER TABLE library.repos
MODIFY COLUMN username VARCHAR(255);

-- Remove the username column
ALTER TABLE library.repos
DROP COLUMN username;

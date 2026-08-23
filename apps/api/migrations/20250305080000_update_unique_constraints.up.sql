-- Drop existing unique constraint on repos.
-- DROP INDEX, not DROP CONSTRAINT: TiDB resolves DROP CONSTRAINT to CHECK
-- constraints only and answers ERROR 3940 for a UNIQUE key once
-- tidb_enable_check_constraint is ON. MySQL drops the same UNIQUE key
-- through DROP INDEX, so one statement covers both engines.
ALTER TABLE library.repos DROP INDEX uniq_repo_username_on_org_id;
-- Add new unique constraint including platform_id on repos
ALTER TABLE library.repos
ADD CONSTRAINT uniq_repo_username_on_org_id_platform_id UNIQUE (username, org_id, platform_id);
-- Add unique constraint on organizations for username and platform_id
ALTER TABLE library.organizations
ADD CONSTRAINT uniq_org_username_platform_id UNIQUE (username, platform_id);

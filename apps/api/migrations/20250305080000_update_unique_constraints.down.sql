-- Drop new unique constraints (DROP INDEX for TiDB compatibility, see the up migration)
ALTER TABLE library.organizations DROP INDEX uniq_org_username_platform_id;
ALTER TABLE library.repos DROP INDEX uniq_repo_username_on_org_id_platform_id;
-- Restore original unique constraint on repos
ALTER TABLE library.repos
ADD CONSTRAINT uniq_repo_username_on_org_id UNIQUE (username, org_id);

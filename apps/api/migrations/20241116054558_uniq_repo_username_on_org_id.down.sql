-- Add down migration script here
-- DROP INDEX for TiDB compatibility, see 20250305080000_update_unique_constraints.up.sql
ALTER TABLE library.repos
DROP INDEX uniq_repo_username_on_org_id;

-- Add down migration script here
ALTER TABLE library.repos
DROP CONSTRAINT uniq_repo_username_on_org_id;

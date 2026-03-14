-- Remove indexes for platform_id columns
DROP INDEX idx_organizations_platform_id ON library.organizations;
DROP INDEX idx_repos_platform_id ON library.repos;
DROP INDEX idx_policies_platform_id ON library.policies;
DROP INDEX idx_databases_platform_id ON library.databases;
-- Remove platform_id column from organizations table
ALTER TABLE library.organizations DROP COLUMN platform_id;
-- Remove platform_id column from repos table
ALTER TABLE library.repos DROP COLUMN platform_id;
-- Remove platform_id column from policies table
ALTER TABLE library.policies DROP COLUMN platform_id;
-- Remove platform_id column from databases table
ALTER TABLE library.databases DROP COLUMN platform_id;
-- Add platform_id column to organizations table
ALTER TABLE library.organizations
ADD COLUMN platform_id VARCHAR(29) NOT NULL DEFAULT 'tn_01j91h09tpj5ehwbwfwfxpak2b';
-- Add platform_id column to repos table
ALTER TABLE library.repos
ADD COLUMN platform_id VARCHAR(29) NOT NULL DEFAULT 'tn_01j91h09tpj5ehwbwfwfxpak2b';
-- Add platform_id column to policies table
ALTER TABLE library.policies
ADD COLUMN platform_id VARCHAR(29) NOT NULL DEFAULT 'tn_01j91h09tpj5ehwbwfwfxpak2b';
-- Add platform_id column to databases table
ALTER TABLE library.databases
ADD COLUMN platform_id VARCHAR(29) NOT NULL DEFAULT 'tn_01j91h09tpj5ehwbwfwfxpak2b';
-- Add indexes for platform_id columns
CREATE INDEX idx_organizations_platform_id ON library.organizations (platform_id);
CREATE INDEX idx_repos_platform_id ON library.repos (platform_id);
CREATE INDEX idx_policies_platform_id ON library.policies (platform_id);
CREATE INDEX idx_databases_platform_id ON library.databases (platform_id);
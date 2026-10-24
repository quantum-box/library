CREATE TABLE IF NOT EXISTS library_pm_migration_plans (
 id VARCHAR(64) NOT NULL PRIMARY KEY,
 repo_id VARCHAR(64) NOT NULL,
 digest CHAR(64) NOT NULL,
 reviewed_by VARCHAR(255) NOT NULL,
 plan_json JSON NOT NULL,
 status VARCHAR(32) NOT NULL DEFAULT 'reviewed',
 INDEX(repo_id)
);
CREATE TABLE IF NOT EXISTS library_pm_migration_items (
 plan_id VARCHAR(64) NOT NULL,
 resource_key CHAR(64) NOT NULL,
 legacy_data_id VARCHAR(64) NOT NULL,
 original_projection_id VARCHAR(64) NOT NULL,
 operation_id VARCHAR(64) NOT NULL,
 accepted_version VARCHAR(64) NULL,
 status VARCHAR(32) NOT NULL DEFAULT 'pending',
 PRIMARY KEY(plan_id,resource_key)
);

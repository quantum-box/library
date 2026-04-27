-- PLT-942 / ADR-0001: Library MDM Hub Phase 1 (Registry)
-- external system code → Library global_id mapping, tenant-scoped.
CREATE TABLE IF NOT EXISTS `global_id_mapping` (
    `id`            VARCHAR(32)  NOT NULL COMMENT 'Mapping ID (ULID, prefix gim_)',
    `tenant_id`     VARCHAR(29)  NOT NULL COMMENT 'Tenant ID (tn_) — scope isolation',
    `global_id`     VARCHAR(64)  NOT NULL COMMENT 'Library-issued global ID (prefix gid_)',
    `system`        VARCHAR(64)  NOT NULL COMMENT 'Source system name (bakuure / tws / ...)',
    `system_code`   VARCHAR(255) NOT NULL COMMENT 'Code in source system (e.g. BWS-001)',
    `name`          VARCHAR(255) NOT NULL COMMENT 'Display name',
    `created_at`    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at`    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    UNIQUE KEY `uk_tenant_global_id`   (`tenant_id`, `global_id`),
    UNIQUE KEY `uk_tenant_system_code` (`tenant_id`, `system`, `system_code`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Phase 1 Registry: external system code → Library global_id (PLT-942 / ADR-0001)';

-- Provider-neutral external synchronization model.
--
-- `external_scope_hash` is SHA-256 over canonical JSON. It makes binding
-- creation idempotent without exposing GitHub-specific repository/ref/path
-- columns in the common model. Provider adapters validate the JSON payload.
CREATE TABLE IF NOT EXISTS `external_sync_bindings` (
    `id`                  VARCHAR(30)  NOT NULL COMMENT 'Binding ID (esb_)',
    `tenant_id`           VARCHAR(29)  NOT NULL COMMENT 'Tenant ID (tn_)',
    `repo_id`             VARCHAR(29)  NOT NULL COMMENT 'Library repo ID (rp_)',
    `provider`            VARCHAR(32)  NOT NULL,
    `connection_id`       VARCHAR(32) CHARACTER SET utf8mb4
                                      COLLATE utf8mb4_unicode_ci NOT NULL
                                      COMMENT 'Tenant-owned integration connection (con_)',
    `external_scope`      JSON         NOT NULL COMMENT 'Provider-validated, non-secret scope config',
    `external_scope_hash` CHAR(64)     NOT NULL COMMENT 'SHA-256 of canonical external_scope JSON',
    `object_type`         VARCHAR(128) NOT NULL,
    `mapping`             JSON         NOT NULL COMMENT 'Provider-neutral field/content mapping',
    `inbound_policy`      VARCHAR(16)  NOT NULL DEFAULT 'review',
    `outbound_policy`     VARCHAR(16)  NOT NULL DEFAULT 'review',
    `delete_policy`       VARCHAR(32)  NOT NULL DEFAULT 'review_tombstone',
    `status`              VARCHAR(32)  NOT NULL DEFAULT 'active',
    `created_at`          TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at`          TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
                                      ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    UNIQUE KEY `uk_external_sync_binding_scope`
        (`tenant_id`, `repo_id`, `provider`, `external_scope_hash`),
    INDEX `idx_external_sync_bindings_repo`
        (`tenant_id`, `repo_id`, `status`),
    INDEX `idx_external_sync_bindings_connection` (`connection_id`),
    CONSTRAINT `fk_external_sync_bindings_repo` FOREIGN KEY (`repo_id`)
        REFERENCES `repos` (`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_external_sync_bindings_connection` FOREIGN KEY (`connection_id`)
        REFERENCES `integration_connections` (`id`) ON DELETE RESTRICT
)
-- Most columns deliberately inherit the database charset/collation so the
-- repo foreign key stays compatible with the original `repos.id` definition.
-- `connection_id` alone matches the explicit collation on the newer
-- `integration_connections` table.
COMMENT='Repository-level external synchronization configuration';

-- Data lives in the database-manager database in production, so `data_id`
-- cannot have a cross-database foreign key here. Tenant ownership is enforced
-- by repository queries through the parent binding.
CREATE TABLE IF NOT EXISTS `external_object_links` (
    `binding_id`                       VARCHAR(30)  NOT NULL,
    `data_id`                          VARCHAR(64)  NOT NULL COMMENT 'Library data ID (data_)',
    `external_object_id`               VARCHAR(1024) NOT NULL,
    `external_object_id_hash`          CHAR(64)     NOT NULL COMMENT 'SHA-256 of exact external object ID',
    `last_accepted_external_revision`  VARCHAR(255) NULL,
    `last_delivered_library_revision`  VARCHAR(255) NULL,
    `base_content_hash`                CHAR(64)     NULL COMMENT 'Common accepted content SHA-256',
    `created_at`                       TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at`                       TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
                                                   ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (`binding_id`, `data_id`),
    UNIQUE KEY `uk_external_object_links_object`
        (`binding_id`, `external_object_id_hash`),
    CONSTRAINT `fk_external_object_links_binding` FOREIGN KEY (`binding_id`)
        REFERENCES `external_sync_bindings` (`id`) ON DELETE CASCADE
)
COMMENT='Mapping between Library data and provider objects';

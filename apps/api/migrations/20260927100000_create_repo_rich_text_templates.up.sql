-- RichText templates are owned by one Library repository. The body stays
-- serialized as text so clients can round-trip the editor document exactly.
CREATE TABLE IF NOT EXISTS `repo_rich_text_templates` (
    `id`         VARCHAR(30)  NOT NULL COMMENT 'RichText template ID (rtt_)',
    `repo_id`    VARCHAR(29)  NOT NULL COMMENT 'Library repo ID (rp_)',
    `name`       VARCHAR(255) NOT NULL COMMENT 'Template name, unique within its repo',
    `rich_text`  LONGTEXT     NOT NULL COMMENT 'Serialized RichText block document JSON',
    `created_at` TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at` TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
                  ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    UNIQUE KEY `uq_repo_rich_text_templates_repo_name` (`repo_id`, `name`),
    KEY `idx_repo_rich_text_templates_repo` (`repo_id`),
    CONSTRAINT `fk_repo_rich_text_templates_repo` FOREIGN KEY (`repo_id`)
        REFERENCES `repos` (`id`) ON DELETE CASCADE
)
COMMENT='Repo-scoped RichText templates for new Library records';

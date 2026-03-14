-- Collaborative document state storage for real-time editing (Yjs CRDT).
-- The yjs_state column holds the binary-encoded Y.Doc snapshot.
CREATE TABLE IF NOT EXISTS `collaborative_documents` (
    `document_key` VARCHAR(100) NOT NULL,
    `operator_id`  VARCHAR(29)  NOT NULL,
    `yjs_state`    LONGBLOB     NOT NULL,
    `updated_at`   DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    `created_at`   DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (`document_key`)
);

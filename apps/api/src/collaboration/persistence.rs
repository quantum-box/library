/// Y.Doc persistence to MySQL/TiDB.
use async_trait::async_trait;

#[async_trait]
pub trait DocumentPersistence: Send + Sync {
    /// Load the persisted Y.Doc binary state for a document.
    /// Returns `None` if the document has never been persisted.
    async fn load(
        &self,
        document_key: &str,
    ) -> Result<Option<Vec<u8>>, sqlx::Error>;

    /// Save the Y.Doc binary state.
    async fn save(
        &self,
        document_key: &str,
        operator_id: &str,
        state: &[u8],
    ) -> Result<(), sqlx::Error>;
}

/// SQLx-backed persistence using the `collaborative_documents`
/// table.
pub struct SqlxDocumentPersistence {
    pool: std::sync::Arc<sqlx::MySqlPool>,
}

impl SqlxDocumentPersistence {
    pub fn new(pool: std::sync::Arc<sqlx::MySqlPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DocumentPersistence for SqlxDocumentPersistence {
    async fn load(
        &self,
        document_key: &str,
    ) -> Result<Option<Vec<u8>>, sqlx::Error> {
        let row = sqlx::query_scalar!(
            "SELECT yjs_state \
             FROM collaborative_documents \
             WHERE document_key = ?",
            document_key
        )
        .fetch_optional(self.pool.as_ref())
        .await?;
        Ok(row)
    }

    async fn save(
        &self,
        document_key: &str,
        operator_id: &str,
        state: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO collaborative_documents \
             (document_key, operator_id, yjs_state) \
             VALUES (?, ?, ?) \
             ON DUPLICATE KEY UPDATE \
             yjs_state = VALUES(yjs_state), \
             updated_at = CURRENT_TIMESTAMP(6)",
            document_key,
            operator_id,
            state
        )
        .execute(self.pool.as_ref())
        .await?;
        Ok(())
    }
}

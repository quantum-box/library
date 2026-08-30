use std::collections::HashMap;
use std::sync::Arc;

use value_object::TenantId;

use crate::domain::translation::{
    LanguageTag, TranslationRecord, TranslationRepository,
    TranslationScope, TranslationStatus,
};

#[derive(Debug)]
pub struct TranslationRepositoryImpl {
    db: Arc<persistence::Db>,
}

impl TranslationRepositoryImpl {
    pub fn new(db: Arc<persistence::Db>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl TranslationRepository for TranslationRepositoryImpl {
    async fn find_scope(
        &self,
        tenant_id: &TenantId,
        scope: TranslationScope,
        target_lang: &LanguageTag,
    ) -> errors::Result<HashMap<String, TranslationRecord>> {
        let rows = sqlx::query!(
            "SELECT target_id, source_lang, source_hash, translated,
                    status, model, reviewed_by
             FROM translations
             WHERE tenant_id = ? AND scope = ? AND target_lang = ?",
            tenant_id.to_string(),
            scope.as_str(),
            target_lang.as_str(),
        )
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        let mut records = HashMap::with_capacity(rows.len());
        for row in rows {
            // A row we cannot interpret is a bug in an older write, and
            // treating it as absent would silently retranslate and
            // overwrite it. Fail loudly instead.
            let record = TranslationRecord {
                tenant_id: tenant_id.clone(),
                scope,
                target_id: row.target_id.clone(),
                target_lang: target_lang.clone(),
                source_lang: row
                    .source_lang
                    .as_deref()
                    .map(LanguageTag::new)
                    .transpose()?,
                source_hash: row.source_hash,
                translated: row.translated,
                status: row.status.parse::<TranslationStatus>()?,
                model: row.model,
                reviewed_by: row.reviewed_by,
            };
            records.insert(row.target_id, record);
        }
        Ok(records)
    }

    async fn upsert(
        &self,
        record: &TranslationRecord,
    ) -> errors::Result<()> {
        // `reviewed_by IS NULL` guards every mutable column, so a row a
        // person has edited keeps its text, its hash and its status even
        // when the source moves on. The staleness it then reports is
        // true and is what the owner is meant to act on.
        sqlx::query!(
            "INSERT INTO translations
                 (tenant_id, scope, target_id, target_lang, source_lang,
                  source_hash, translated, status, model)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE
                 source_lang = IF(reviewed_by IS NULL,
                                  VALUES(source_lang), source_lang),
                 source_hash = IF(reviewed_by IS NULL,
                                  VALUES(source_hash), source_hash),
                 translated  = IF(reviewed_by IS NULL,
                                  VALUES(translated), translated),
                 status      = IF(reviewed_by IS NULL,
                                  VALUES(status), status),
                 model       = IF(reviewed_by IS NULL,
                                  VALUES(model), model)",
            record.tenant_id.to_string(),
            record.scope.as_str(),
            record.target_id,
            record.target_lang.as_str(),
            record.source_lang.as_ref().map(|tag| tag.to_string()),
            record.source_hash,
            record.translated,
            record.status.as_str(),
            record.model,
        )
        .execute(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        Ok(())
    }
}

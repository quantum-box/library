use std::collections::HashSet;
use std::sync::Arc;

use crate::domain::repo::RepoId;
use crate::domain::translation::{
    GlossaryRepository, GlossaryTerm, LanguageTag, GLOSSARY_ALL_LANGUAGES,
};

#[derive(Debug)]
pub struct GlossaryRepositoryImpl {
    db: Arc<persistence::Db>,
}

impl GlossaryRepositoryImpl {
    pub fn new(db: Arc<persistence::Db>) -> Self {
        Self { db }
    }
}

/// The stored `target_lang` for one term.
fn stored_lang(term: &GlossaryTerm) -> String {
    term.target_lang
        .as_ref()
        .map(|tag| tag.to_string())
        .unwrap_or_else(|| GLOSSARY_ALL_LANGUAGES.to_string())
}

#[async_trait::async_trait]
impl GlossaryRepository for GlossaryRepositoryImpl {
    async fn find_by_repo(
        &self,
        repo_id: &RepoId,
    ) -> errors::Result<Vec<GlossaryTerm>> {
        let rows = sqlx::query!(
            "SELECT term, target_lang, translation
             FROM repo_glossary_terms
             WHERE repo_id = ?
             ORDER BY term, target_lang",
            repo_id.to_string(),
        )
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        rows.into_iter()
            .map(|row| {
                let target_lang = if row.target_lang
                    == GLOSSARY_ALL_LANGUAGES
                {
                    None
                } else {
                    Some(LanguageTag::new(&row.target_lang).map_err(
                        |error| {
                            errors::Error::internal_server_error(format!(
                                "stored glossary language `{}` is not a \
                                 valid language tag: {error}",
                                row.target_lang
                            ))
                        },
                    )?)
                };
                Ok(GlossaryTerm {
                    term: row.term,
                    target_lang,
                    translation: row.translation,
                })
            })
            .collect()
    }

    async fn replace_for_repo(
        &self,
        repo_id: &RepoId,
        terms: &[GlossaryTerm],
    ) -> errors::Result<()> {
        let mut tx = self
            .db
            .pool()
            .begin()
            .await
            .map_err(errors::Error::internal_server_error)?;

        // A whole-glossary replace, unlike the published-language diff:
        // there is no per-row timestamp anyone reads, and a term whose
        // translation changed has to be rewritten anyway.
        let desired: HashSet<(String, String)> = terms
            .iter()
            .map(|term| (term.term.clone(), stored_lang(term)))
            .collect();

        let existing = sqlx::query!(
            "SELECT term, target_lang FROM repo_glossary_terms
             WHERE repo_id = ?",
            repo_id.to_string(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(errors::Error::internal_server_error)?;

        for row in existing {
            if desired
                .contains(&(row.term.clone(), row.target_lang.clone()))
            {
                continue;
            }
            sqlx::query!(
                "DELETE FROM repo_glossary_terms
                 WHERE repo_id = ? AND term = ? AND target_lang = ?",
                repo_id.to_string(),
                row.term,
                row.target_lang,
            )
            .execute(&mut *tx)
            .await
            .map_err(errors::Error::internal_server_error)?;
        }

        for term in terms {
            sqlx::query!(
                "INSERT INTO repo_glossary_terms
                     (repo_id, term, target_lang, translation)
                 VALUES (?, ?, ?, ?)
                 ON DUPLICATE KEY UPDATE
                     translation = VALUES(translation)",
                repo_id.to_string(),
                term.term,
                stored_lang(term),
                term.translation,
            )
            .execute(&mut *tx)
            .await
            .map_err(errors::Error::internal_server_error)?;
        }

        tx.commit()
            .await
            .map_err(errors::Error::internal_server_error)?;

        Ok(())
    }
}

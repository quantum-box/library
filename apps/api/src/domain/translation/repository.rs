//! Persistence ports for the translation feature.

use std::collections::HashMap;

use async_trait::async_trait;
use value_object::TenantId;

use crate::domain::repo::RepoId;

use super::{
    GlossaryEntry, LanguageTag, TranslationScope, TranslationStatus,
};

/// The languages a repo has been published in.
///
/// This is the allow-list consulted before any translation work is
/// scheduled, so it is read on the hot path of every public docs
/// request and written only by a repo administrator.
#[async_trait]
pub trait PublishedLanguageRepository:
    Send + Sync + std::fmt::Debug
{
    /// Languages currently declared for this repo, in a stable order.
    async fn find_by_repo(
        &self,
        repo_id: &RepoId,
    ) -> errors::Result<Vec<LanguageTag>>;

    /// Replaces the declared set with exactly `languages`.
    ///
    /// Removing a language does not delete its cached translations: the
    /// repo simply stops serving them, and re-declaring it later costs
    /// nothing. Deleting the rows would throw away work that is still
    /// valid, and any human review attached to it.
    async fn replace_for_repo(
        &self,
        repo_id: &RepoId,
        languages: &[LanguageTag],
    ) -> errors::Result<()>;
}

/// One cached translation, as stored.
#[derive(Debug, Clone)]
pub struct TranslationRecord {
    pub tenant_id: TenantId,
    pub scope: TranslationScope,
    pub target_id: String,
    pub target_lang: LanguageTag,
    /// `None` when the writing system did not identify a single
    /// language; the model is asked to infer it instead.
    pub source_lang: Option<LanguageTag>,
    pub source_hash: String,
    pub translated: Option<String>,
    pub status: TranslationStatus,
    pub model: Option<String>,
    /// Set once a person edited the translation. Such a row is never
    /// overwritten automatically, however stale it becomes.
    pub reviewed_by: Option<String>,
}

impl TranslationRecord {
    /// Whether this row still answers for `source_hash`.
    pub fn is_current_for(&self, source_hash: &str) -> bool {
        self.source_hash == source_hash && self.status.has_text()
    }
}

#[async_trait]
pub trait TranslationRepository: Send + Sync + std::fmt::Debug {
    /// Every row for one scope and language within a tenant, keyed by
    /// target id.
    ///
    /// Loaded wholesale rather than by id list because the schema-level
    /// scopes are bounded by how many properties a tenant has defined,
    /// and a dynamic `IN` list cannot be checked at compile time. A
    /// record-level scope will need a narrower query.
    async fn find_scope(
        &self,
        tenant_id: &TenantId,
        scope: TranslationScope,
        target_lang: &LanguageTag,
    ) -> errors::Result<HashMap<String, TranslationRecord>>;

    /// Writes a translation, leaving human-reviewed rows untouched.
    async fn upsert(
        &self,
        record: &TranslationRecord,
    ) -> errors::Result<()>;
}

/// The sentinel `target_lang` meaning "every target language".
///
/// Not a valid BCP-47 tag, so it cannot be confused with one, and
/// storable in a primary key in a way `NULL` is not.
pub const GLOSSARY_ALL_LANGUAGES: &str = "*";

/// One glossary row as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryTerm {
    pub term: String,
    /// `None` when the row applies to every target language.
    pub target_lang: Option<LanguageTag>,
    pub translation: String,
}

impl GlossaryTerm {
    /// Whether this row should be injected for `target_lang`.
    pub fn applies_to(&self, target_lang: &LanguageTag) -> bool {
        match &self.target_lang {
            None => true,
            Some(tag) => tag.matches_primary(target_lang),
        }
    }
}

/// Selects and orders the glossary rows for one target language.
///
/// A language-specific row wins over an every-language one for the same
/// term: the general rule is the default, and naming a language is how
/// an owner overrides it.
pub fn glossary_for_language(
    terms: &[GlossaryTerm],
    target_lang: &LanguageTag,
) -> Vec<GlossaryEntry> {
    let mut chosen: std::collections::BTreeMap<&str, &GlossaryTerm> =
        std::collections::BTreeMap::new();

    for term in terms.iter().filter(|t| t.applies_to(target_lang)) {
        let entry = chosen.entry(term.term.as_str());
        match entry {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(term);
            }
            std::collections::btree_map::Entry::Occupied(mut slot) => {
                if slot.get().target_lang.is_none()
                    && term.target_lang.is_some()
                {
                    slot.insert(term);
                }
            }
        }
    }

    chosen
        .into_values()
        .map(|term| GlossaryEntry {
            term: term.term.clone(),
            translation: term.translation.clone(),
        })
        .collect()
}

#[async_trait]
pub trait GlossaryRepository: Send + Sync + std::fmt::Debug {
    async fn find_by_repo(
        &self,
        repo_id: &RepoId,
    ) -> errors::Result<Vec<GlossaryTerm>>;

    /// Replaces the repo's glossary with exactly `terms`.
    async fn replace_for_repo(
        &self,
        repo_id: &RepoId,
        terms: &[GlossaryTerm],
    ) -> errors::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(value: &str) -> LanguageTag {
        LanguageTag::new(value).unwrap()
    }

    fn term(
        name: &str,
        lang: Option<&str>,
        translation: &str,
    ) -> GlossaryTerm {
        GlossaryTerm {
            term: name.into(),
            target_lang: lang.map(tag),
            translation: translation.into(),
        }
    }

    #[test]
    fn an_every_language_row_applies_everywhere() {
        let rows = vec![term("Library", None, "Library")];
        let entries = glossary_for_language(&rows, &tag("ja"));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].translation, "Library");
        assert_eq!(glossary_for_language(&rows, &tag("zh-Hans")).len(), 1);
    }

    #[test]
    fn a_row_for_another_language_is_left_out() {
        let rows = vec![term("Repository", Some("ja"), "リポジトリ")];
        assert!(glossary_for_language(&rows, &tag("en")).is_empty());
        assert_eq!(glossary_for_language(&rows, &tag("ja")).len(), 1);
    }

    #[test]
    fn naming_a_language_overrides_the_general_rule() {
        // The owner says "leave Library alone everywhere, except in
        // Japanese where it is written out".
        let rows = vec![
            term("Library", None, "Library"),
            term("Library", Some("ja"), "ライブラリ"),
        ];
        let japanese = glossary_for_language(&rows, &tag("ja"));
        assert_eq!(japanese.len(), 1, "one entry per term");
        assert_eq!(japanese[0].translation, "ライブラリ");

        let english = glossary_for_language(&rows, &tag("en"));
        assert_eq!(english[0].translation, "Library");
    }

    #[test]
    fn a_regional_target_matches_a_base_language_row() {
        let rows = vec![term("Repository", Some("en"), "Repo")];
        assert_eq!(glossary_for_language(&rows, &tag("en-GB")).len(), 1);
    }

    #[test]
    fn entries_come_back_in_a_stable_order() {
        // The glossary goes into a prompt, and a prompt that reorders
        // itself between runs defeats any upstream caching.
        let rows = vec![
            term("Zeta", None, "Zeta"),
            term("Alpha", None, "Alpha"),
            term("Mid", None, "Mid"),
        ];
        let terms: Vec<String> = glossary_for_language(&rows, &tag("ja"))
            .into_iter()
            .map(|entry| entry.term)
            .collect();
        assert_eq!(terms, vec!["Alpha", "Mid", "Zeta"]);
    }
}

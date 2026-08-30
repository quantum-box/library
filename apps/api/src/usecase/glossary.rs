//! The per-repo glossary.
//!
//! Every entry is injected into every translation prompt, which is what
//! makes it effective and what makes it need a bound: the glossary is
//! paid for on each batch, in tokens, forever.
//!
//! It matters more than it would with a large model. This pipeline runs
//! on a small one, and small models routinely translate a product name
//! or an internal codename as if it were an ordinary noun. On a public
//! repo that shows up as vocabulary drifting from page to page.

use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyForResourceInput, ExecutorAction,
    MultiTenancyAction,
};

use crate::domain::translation::{
    GlossaryRepository, GlossaryTerm, LanguageTag,
};

use super::{ViewRepoInputData, ViewRepoInputPort};

/// Upper bound on glossary size.
///
/// The whole glossary rides along with every batch, so a thousand-entry
/// list would cost more in prompt tokens than the labels it guards.
pub const MAX_GLOSSARY_TERMS: usize = 200;

/// Storage width for a term and its translation.
const MAX_TERM_LENGTH: usize = 255;

pub struct GetGlossaryInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub organization_username: String,
    pub repo_username: String,
}

pub struct SetGlossaryInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub organization_username: String,
    pub repo_username: String,
    /// `(term, target_lang, translation)`, with `target_lang` absent
    /// meaning the entry applies to every language.
    pub terms: Vec<(String, Option<String>, String)>,
}

#[async_trait]
pub trait GetGlossaryInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: &GetGlossaryInputData<'a>,
    ) -> errors::Result<Vec<GlossaryTerm>>;
}

#[async_trait]
pub trait SetGlossaryInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: &SetGlossaryInputData<'a>,
    ) -> errors::Result<Vec<GlossaryTerm>>;
}

/// Validates, normalizes and deduplicates a submitted glossary.
///
/// Exposed separately so the rules can be tested without a repo, an
/// executor or a database in the way.
pub fn normalize_glossary(
    raw: &[(String, Option<String>, String)],
) -> errors::Result<Vec<GlossaryTerm>> {
    if raw.len() > MAX_GLOSSARY_TERMS {
        return Err(errors::Error::bad_request(format!(
            "A repo glossary holds at most {MAX_GLOSSARY_TERMS} terms, \
             but {} were submitted",
            raw.len()
        )));
    }

    let mut terms = Vec::with_capacity(raw.len());
    for (term, lang, translation) in raw {
        let term = term.trim();
        let translation = translation.trim();

        if term.is_empty() {
            return Err(errors::Error::bad_request(
                "A glossary term cannot be empty",
            ));
        }
        if translation.is_empty() {
            return Err(errors::Error::bad_request(format!(
                "The translation for `{term}` cannot be empty; repeat \
                 the term itself to leave it untranslated"
            )));
        }
        if term.chars().count() > MAX_TERM_LENGTH
            || translation.chars().count() > MAX_TERM_LENGTH
        {
            return Err(errors::Error::bad_request(format!(
                "`{term}` exceeds the {MAX_TERM_LENGTH}-character limit \
                 for a glossary entry"
            )));
        }

        let target_lang =
            lang.as_ref().map(LanguageTag::new).transpose()?;

        terms.push(GlossaryTerm {
            term: term.to_string(),
            target_lang,
            translation: translation.to_string(),
        });
    }

    // Sort before dedup so that two submissions of the same glossary
    // store the same rows and produce the same prompt.
    terms.sort_by(|left, right| {
        left.term.cmp(&right.term).then_with(|| {
            left.target_lang
                .as_ref()
                .map(|tag| tag.to_string())
                .cmp(&right.target_lang.as_ref().map(|tag| tag.to_string()))
        })
    });

    let before = terms.len();
    terms.dedup_by(|left, right| {
        left.term == right.term && left.target_lang == right.target_lang
    });
    if terms.len() != before {
        return Err(errors::Error::bad_request(
            "The glossary lists the same term twice for one language; \
             which translation wins would be arbitrary",
        ));
    }

    Ok(terms)
}

#[derive(Debug)]
pub struct GetGlossary {
    view_repo: Arc<dyn ViewRepoInputPort>,
    glossary: Arc<dyn GlossaryRepository>,
}

impl GetGlossary {
    pub fn new(
        view_repo: Arc<dyn ViewRepoInputPort>,
        glossary: Arc<dyn GlossaryRepository>,
    ) -> Self {
        Self {
            view_repo,
            glossary,
        }
    }
}

#[async_trait]
impl GetGlossaryInputPort for GetGlossary {
    #[tracing::instrument(name = "GetGlossary::execute", skip_all)]
    async fn execute<'a>(
        &self,
        input: &GetGlossaryInputData<'a>,
    ) -> errors::Result<Vec<GlossaryTerm>> {
        let repo = self
            .view_repo
            .execute(&ViewRepoInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                organization_username: input.organization_username.clone(),
                repo_username: input.repo_username.clone(),
            })
            .await?
            .repo;

        self.glossary.find_by_repo(repo.id()).await
    }
}

#[derive(Debug)]
pub struct SetGlossary {
    auth_app: Arc<dyn AuthApp>,
    view_repo: Arc<dyn ViewRepoInputPort>,
    glossary: Arc<dyn GlossaryRepository>,
}

impl SetGlossary {
    pub fn new(
        auth_app: Arc<dyn AuthApp>,
        view_repo: Arc<dyn ViewRepoInputPort>,
        glossary: Arc<dyn GlossaryRepository>,
    ) -> Self {
        Self {
            auth_app,
            view_repo,
            glossary,
        }
    }
}

#[async_trait]
impl SetGlossaryInputPort for SetGlossary {
    #[tracing::instrument(name = "SetGlossary::execute", skip_all)]
    async fn execute<'a>(
        &self,
        input: &SetGlossaryInputData<'a>,
    ) -> errors::Result<Vec<GlossaryTerm>> {
        let terms = normalize_glossary(&input.terms)?;

        let repo = self
            .view_repo
            .execute(&ViewRepoInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                organization_username: input.organization_username.clone(),
                repo_username: input.repo_username.clone(),
            })
            .await?
            .repo;

        self.auth_app
            .check_policy_for_resource(&CheckPolicyForResourceInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:UpdateRepo",
                resource_trn: &format!("trn:library:repo:{}", repo.id()),
            })
            .await?;

        self.glossary.replace_for_repo(repo.id(), &terms).await?;

        // Existing translations are deliberately left alone. A glossary
        // change makes them wrong in the owner's judgement, not stale
        // against their source, and silently discarding paid-for and
        // possibly human-reviewed work is not this endpoint's call.
        Ok(terms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(
        entries: &[(&str, Option<&str>, &str)],
    ) -> Vec<(String, Option<String>, String)> {
        entries
            .iter()
            .map(|(term, lang, translation)| {
                (
                    term.to_string(),
                    lang.map(|l| l.to_string()),
                    translation.to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn a_plain_glossary_round_trips() {
        let terms =
            normalize_glossary(&raw(&[("Library", None, "Library")]))
                .unwrap();
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].term, "Library");
        assert_eq!(terms[0].target_lang, None);
    }

    #[test]
    fn whitespace_is_trimmed_from_both_sides_of_an_entry() {
        let terms = normalize_glossary(&raw(&[(
            "  Repository  ",
            Some("ja"),
            "  リポジトリ  ",
        )]))
        .unwrap();
        assert_eq!(terms[0].term, "Repository");
        assert_eq!(terms[0].translation, "リポジトリ");
    }

    #[test]
    fn entries_are_ordered_so_the_prompt_is_stable() {
        let terms = normalize_glossary(&raw(&[
            ("Zeta", None, "Zeta"),
            ("Alpha", None, "Alpha"),
        ]))
        .unwrap();
        assert_eq!(terms[0].term, "Alpha");
        assert_eq!(terms[1].term, "Zeta");
    }

    #[test]
    fn the_same_term_may_differ_per_language() {
        let terms = normalize_glossary(&raw(&[
            ("Library", None, "Library"),
            ("Library", Some("ja"), "ライブラリ"),
        ]))
        .unwrap();
        assert_eq!(terms.len(), 2);
    }

    #[test]
    fn the_same_term_twice_for_one_language_is_rejected() {
        // Picking a winner would be arbitrary, and the owner would not
        // know which one had been kept.
        let error = normalize_glossary(&raw(&[
            ("Library", Some("ja"), "ライブラリ"),
            ("Library", Some("ja"), "書庫"),
        ]));
        assert!(error.is_err());
    }

    #[test]
    fn an_empty_translation_is_rejected_with_the_way_out() {
        let error = normalize_glossary(&raw(&[("Library", None, "  ")]))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("repeat the term itself"),
            "the error must say how to leave a term untranslated: \
             {error}"
        );
    }

    #[test]
    fn an_empty_term_is_rejected() {
        assert!(normalize_glossary(&raw(&[("  ", None, "x")])).is_err());
    }

    #[test]
    fn a_malformed_language_is_rejected() {
        assert!(normalize_glossary(&raw(&[(
            "Library",
            Some("not a language"),
            "Library"
        )]))
        .is_err());
    }

    #[test]
    fn the_glossary_is_bounded_because_it_rides_every_prompt() {
        let too_many: Vec<(String, Option<String>, String)> = (0
            ..MAX_GLOSSARY_TERMS + 1)
            .map(|index| {
                (format!("term{index}"), None, format!("t{index}"))
            })
            .collect();
        assert!(normalize_glossary(&too_many).is_err());
    }

    #[test]
    fn an_empty_glossary_is_allowed() {
        assert_eq!(normalize_glossary(&[]).unwrap(), vec![]);
    }
}

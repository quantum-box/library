//! Declaring the languages a repo is published in.
//!
//! This is the switch the whole translation feature hangs off. A
//! language listed here is one the owner has asked to be translated
//! into and is willing to pay for; a language absent from it is served
//! as the source text and schedules no work at all.
//!
//! Keeping the allow-list explicit is what makes the public docs
//! endpoints safe to expose anonymously: a reader can ask for any
//! `lang` they like, and anything unpublished costs a table lookup
//! rather than a model call.

use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyForResourceInput, ExecutorAction,
    MultiTenancyAction,
};

use crate::domain::translation::{
    LanguageTag, PublishedLanguageRepository,
};

use super::{ViewRepoInputData, ViewRepoInputPort};

/// Upper bound on how many languages one repo may publish.
///
/// Every language multiplies the eager translation cost of the entire
/// repo, so an accidental fifty-entry list is a bill, not a typo worth
/// honouring. The limit is deliberately generous for real use.
pub const MAX_PUBLISHED_LANGUAGES: usize = 20;

pub struct GetPublishedLanguagesInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub organization_username: String,
    pub repo_username: String,
}

pub struct SetPublishedLanguagesInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub organization_username: String,
    pub repo_username: String,
    /// Raw tags as submitted; normalized and deduplicated by the
    /// usecase so that callers need not agree on spelling.
    pub languages: Vec<String>,
}

#[async_trait]
pub trait GetPublishedLanguagesInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: &GetPublishedLanguagesInputData<'a>,
    ) -> errors::Result<Vec<LanguageTag>>;
}

#[async_trait]
pub trait SetPublishedLanguagesInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: &SetPublishedLanguagesInputData<'a>,
    ) -> errors::Result<Vec<LanguageTag>>;
}

/// Normalizes, deduplicates and bounds a submitted language list.
///
/// Exposed separately because the rules are worth testing without a
/// repo, an executor or a database in the way.
pub fn normalize_language_list(
    raw: &[String],
) -> errors::Result<Vec<LanguageTag>> {
    if raw.len() > MAX_PUBLISHED_LANGUAGES {
        return Err(errors::Error::bad_request(format!(
            "A repo may publish at most {MAX_PUBLISHED_LANGUAGES} \
             languages, but {} were requested",
            raw.len()
        )));
    }

    let mut tags = raw
        .iter()
        .map(LanguageTag::new)
        .collect::<Result<Vec<_>, _>>()?;

    // Normalization happens first, so `en` and `EN` collapse into one
    // entry instead of being stored twice and translated twice.
    tags.sort();
    tags.dedup();

    Ok(tags)
}

#[derive(Debug)]
pub struct GetPublishedLanguages {
    view_repo: Arc<dyn ViewRepoInputPort>,
    published_languages: Arc<dyn PublishedLanguageRepository>,
}

impl GetPublishedLanguages {
    pub fn new(
        view_repo: Arc<dyn ViewRepoInputPort>,
        published_languages: Arc<dyn PublishedLanguageRepository>,
    ) -> Self {
        Self {
            view_repo,
            published_languages,
        }
    }
}

#[async_trait]
impl GetPublishedLanguagesInputPort for GetPublishedLanguages {
    #[tracing::instrument(
        name = "GetPublishedLanguages::execute",
        skip_all
    )]
    async fn execute<'a>(
        &self,
        input: &GetPublishedLanguagesInputData<'a>,
    ) -> errors::Result<Vec<LanguageTag>> {
        // Reading the list is exactly as privileged as reading the repo
        // itself, so the visibility rules are borrowed wholesale rather
        // than restated here.
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

        self.published_languages.find_by_repo(repo.id()).await
    }
}

#[derive(Debug)]
pub struct SetPublishedLanguages {
    auth_app: Arc<dyn AuthApp>,
    view_repo: Arc<dyn ViewRepoInputPort>,
    published_languages: Arc<dyn PublishedLanguageRepository>,
}

impl SetPublishedLanguages {
    pub fn new(
        auth_app: Arc<dyn AuthApp>,
        view_repo: Arc<dyn ViewRepoInputPort>,
        published_languages: Arc<dyn PublishedLanguageRepository>,
    ) -> Self {
        Self {
            auth_app,
            view_repo,
            published_languages,
        }
    }
}

#[async_trait]
impl SetPublishedLanguagesInputPort for SetPublishedLanguages {
    #[tracing::instrument(
        name = "SetPublishedLanguages::execute",
        skip_all
    )]
    async fn execute<'a>(
        &self,
        input: &SetPublishedLanguagesInputData<'a>,
    ) -> errors::Result<Vec<LanguageTag>> {
        // Validate before resolving anything: a malformed tag should
        // fail the same way whether or not the repo happens to exist.
        let languages = normalize_language_list(&input.languages)?;

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

        // Publishing a language is a repo setting, so it reuses the
        // repo-update permission rather than introducing an action that
        // would have to be granted separately in Tachyon.
        self.auth_app
            .check_policy_for_resource(&CheckPolicyForResourceInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:UpdateRepo",
                resource_trn: &format!("trn:library:repo:{}", repo.id()),
            })
            .await?;

        self.published_languages
            .replace_for_repo(repo.id(), &languages)
            .await?;

        Ok(languages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(raw: &[&str]) -> Vec<String> {
        raw.iter().map(|s| s.to_string()).collect()
    }

    fn normalized(raw: &[&str]) -> Vec<String> {
        normalize_language_list(&tags(raw))
            .unwrap()
            .iter()
            .map(|tag| tag.to_string())
            .collect()
    }

    #[test]
    fn normalizes_spelling_so_one_language_is_stored_once() {
        assert_eq!(normalized(&["EN", "en", "En"]), vec!["en"]);
        assert_eq!(normalized(&["zh-hans", "ZH-HANS"]), vec!["zh-Hans"]);
    }

    #[test]
    fn keeps_distinct_languages_in_a_deterministic_order() {
        assert_eq!(
            normalized(&["ko", "en", "zh-Hans"]),
            vec!["en", "ko", "zh-Hans"]
        );
        // Order of submission must not change what is stored.
        assert_eq!(
            normalized(&["zh-Hans", "ko", "en"]),
            vec!["en", "ko", "zh-Hans"]
        );
    }

    #[test]
    fn an_empty_list_is_allowed_and_means_publish_nothing() {
        assert_eq!(normalize_language_list(&[]).unwrap(), vec![]);
    }

    #[test]
    fn a_malformed_tag_is_rejected_rather_than_skipped() {
        let error =
            normalize_language_list(&tags(&["en", "not a language"]));
        assert!(error.is_err());
    }

    #[test]
    fn the_list_is_bounded_because_each_language_is_a_bill() {
        let too_many: Vec<String> =
            (0..MAX_PUBLISHED_LANGUAGES + 1).map(dummy_tag).collect();
        assert!(normalize_language_list(&too_many).is_err());

        let at_the_limit: Vec<String> =
            (0..MAX_PUBLISHED_LANGUAGES).map(dummy_tag).collect();
        assert!(normalize_language_list(&at_the_limit).is_ok());
    }

    /// Distinct well-formed two-letter tags, `aa` through `az` and on.
    fn dummy_tag(index: usize) -> String {
        let first = (b'a' + (index / 26) as u8) as char;
        let second = (b'a' + (index % 26) as u8) as char;
        format!("{first}{second}")
    }
}

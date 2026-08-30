//! Translating a repo's short text: schema labels and record names.
//!
//! Tier 1 is the schema -- column headings and select-option labels.
//! A few hundred strings per tenant, translated once, changing how
//! every record reads.
//!
//! Tier 2 is record names. There are far more of them, but on a public
//! docs site they matter more than anything else short of the body: the
//! listing is nothing but record names, so a repo whose schema is
//! translated and whose names are not still presents an untranslated
//! index to every reader arriving from a search engine.
//!
//! Record bodies are Tier 3 and are not touched here -- they need a
//! structure-preserving pass that this batching does not provide.
//!
//! The run is explicit rather than automatic. The API is deployed on
//! Lambda, where a resident worker has nowhere to live, so translation
//! is driven by a request an operator (or a scheduler) makes, and the
//! declaration endpoint stays fast.

use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use database_manager::domain::{Property, PropertyType};
use serde::Serialize;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyForResourceInput, ExecutorAction,
    MultiTenancyAction,
};

use crate::domain::translation::{
    detect_source_language, glossary_for_language,
    has_translatable_schema_labels, is_externally_owned_property,
    source_hash, GlossaryEntry, GlossaryRepository, LanguageTag,
    PublishedLanguageRepository, TranslationBatch, TranslationItem,
    TranslationRecord, TranslationRepository, TranslationScope,
    TranslationStatus, Translator,
};

use super::{
    GetPropertiesInputData, GetPropertiesInputPort, ViewDataListInputData,
    ViewDataListInputPort, ViewRepoInputData, ViewRepoInputPort,
};

/// How many labels go to the model at once.
///
/// Small enough that one malformed response costs little to retry, and
/// that the reply fits comfortably in the completion budget.
const BATCH_SIZE: usize = 40;

/// Records pulled per page while collecting names.
const RECORD_PAGE_SIZE: u32 = 100;

/// Upper bound on records visited in one run.
///
/// A run is a single HTTP request, so it cannot page through a repo of
/// unbounded size. Stopping at a known number and reporting it lets a
/// scheduler call again; silently covering a prefix would read as
/// "everything is translated" when it is not.
const MAX_RECORDS_PER_RUN: usize = 500;

pub struct TranslateRepoInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub organization_username: String,
    pub repo_username: String,
}

/// What one language's pass did.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TranslationOutcome {
    pub language: String,
    /// Labels sent to the model and stored.
    pub translated: usize,
    /// Labels whose cached translation still matched the source.
    pub already_current: usize,
    /// Labels left alone because a person had edited the translation.
    pub human_reviewed: usize,
    /// Set when the source language is the target language, so there
    /// was nothing to do.
    pub skipped_same_language: bool,
}

/// What a whole run did.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RepoTranslationReport {
    pub outcomes: Vec<TranslationOutcome>,
    /// Records whose names were considered this run.
    pub records_scanned: usize,
    /// True when the repo has more records than one run visits, so the
    /// caller knows to run again rather than assuming completion.
    pub records_truncated: bool,
}

#[async_trait]
pub trait TranslateRepoInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: &TranslateRepoInputData<'a>,
    ) -> errors::Result<RepoTranslationReport>;
}

/// The schema labels of one repo, paired with the scope they belong to.
///
/// Pulled out as a free function so the selection rules can be tested
/// without a database or a model.
pub fn collect_schema_labels(
    properties: &[Property],
) -> Vec<(TranslationScope, TranslationItem)> {
    let mut labels = Vec::new();

    for property in properties {
        // Externally synced fields are mirrored from another system;
        // translating them would make the copy disagree with its source
        // on the next round trip.
        if is_externally_owned_property(property.name()) {
            continue;
        }

        labels.push((
            TranslationScope::PropertyDef,
            TranslationItem {
                id: property.id().to_string(),
                text: property.name().to_string(),
            },
        ));

        if !has_translatable_schema_labels(property.property_type()) {
            continue;
        }

        let items = match property.property_type() {
            PropertyType::Select(config) => config.items(),
            PropertyType::MultiSelect(config) => config.items(),
            _ => continue,
        };

        for item in items {
            labels.push((
                TranslationScope::SelectOption,
                TranslationItem {
                    id: item.id().to_string(),
                    text: item.name().to_string(),
                },
            ));
        }
    }

    labels
}

/// The record names of one page, as translatable items.
///
/// No blank-name guard: `Data::name` is a `Text`, which refuses an
/// empty or whitespace-only value at construction, so there is nothing
/// here for such a filter to catch.
pub fn collect_record_names(
    records: &[database_manager::domain::Data],
) -> Vec<(TranslationScope, TranslationItem)> {
    records
        .iter()
        .map(|record| {
            (
                TranslationScope::RecordName,
                TranslationItem {
                    id: record.id().to_string(),
                    text: record.name().to_string(),
                },
            )
        })
        .collect()
}

pub struct TranslateRepo {
    auth_app: Arc<dyn AuthApp>,
    view_repo: Arc<dyn ViewRepoInputPort>,
    get_properties: Arc<dyn GetPropertiesInputPort>,
    view_data_list: Arc<dyn ViewDataListInputPort>,
    published_languages: Arc<dyn PublishedLanguageRepository>,
    glossary: Arc<dyn GlossaryRepository>,
    translations: Arc<dyn TranslationRepository>,
    /// `None` when no model is configured, which turns the feature off
    /// rather than failing a deployment.
    translator: Option<Arc<dyn Translator>>,
    model: Option<String>,
}

impl Debug for TranslateRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranslateRepo")
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

impl TranslateRepo {
    // Each argument is a distinct collaborator wired once at startup,
    // which is how every usecase in this crate is constructed. Grouping
    // them behind a struct would read better and is worth doing across
    // the board, but not for this one constructor alone.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        auth_app: Arc<dyn AuthApp>,
        view_repo: Arc<dyn ViewRepoInputPort>,
        get_properties: Arc<dyn GetPropertiesInputPort>,
        view_data_list: Arc<dyn ViewDataListInputPort>,
        published_languages: Arc<dyn PublishedLanguageRepository>,
        glossary: Arc<dyn GlossaryRepository>,
        translations: Arc<dyn TranslationRepository>,
        translator: Option<Arc<dyn Translator>>,
        model: Option<String>,
    ) -> Self {
        Self {
            auth_app,
            view_repo,
            get_properties,
            view_data_list,
            published_languages,
            glossary,
            translations,
            translator,
            model,
        }
    }
}

#[async_trait]
impl TranslateRepoInputPort for TranslateRepo {
    #[tracing::instrument(name = "TranslateRepo::execute", skip_all)]
    async fn execute<'a>(
        &self,
        input: &TranslateRepoInputData<'a>,
    ) -> errors::Result<RepoTranslationReport> {
        let (Some(translator), Some(model)) =
            (self.translator.as_ref(), self.model.as_ref())
        else {
            // An explicit request deserves an explicit answer: silently
            // doing nothing would look like a successful no-op run.
            return Err(errors::Error::bad_request(
                "translation is not configured; set \
                 LIBRARY_TRANSLATION_MODEL to enable it",
            ));
        };

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

        // Spending money on a repo's behalf is at least as privileged as
        // changing its settings.
        self.auth_app
            .check_policy_for_resource(&CheckPolicyForResourceInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:UpdateRepo",
                resource_trn: &format!("trn:library:repo:{}", repo.id()),
            })
            .await?;

        let languages =
            self.published_languages.find_by_repo(repo.id()).await?;
        if languages.is_empty() {
            return Ok(RepoTranslationReport {
                outcomes: Vec::new(),
                records_scanned: 0,
                records_truncated: false,
            });
        }

        let properties = self
            .get_properties
            .execute(GetPropertiesInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                org_username: input.organization_username.clone(),
                repo_username: input.repo_username.clone(),
            })
            .await?;

        let mut labels = collect_schema_labels(&properties);

        // Tier 2. Paged rather than fetched whole, and bounded, because
        // a run is one HTTP request and a repo can be arbitrarily large.
        let mut records_scanned = 0usize;
        let mut records_truncated = false;
        let mut page = 1u32;
        loop {
            let (records, _properties, paginator) = self
                .view_data_list
                .execute(&ViewDataListInputData {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    org_username: input.organization_username.clone(),
                    repo_username: input.repo_username.clone(),
                    page: Some(page),
                    page_size: Some(RECORD_PAGE_SIZE),
                })
                .await?;

            labels.extend(collect_record_names(&records));
            records_scanned += records.len();

            if records_scanned >= MAX_RECORDS_PER_RUN {
                records_truncated = page < paginator.total_pages;
                break;
            }
            if page >= paginator.total_pages || records.is_empty() {
                break;
            }
            page += 1;
        }

        if labels.is_empty() {
            return Ok(RepoTranslationReport {
                outcomes: Vec::new(),
                records_scanned,
                records_truncated,
            });
        }

        // One detection across everything collected: individual labels
        // and record names are far too short to classify on their own,
        // and a repo is overwhelmingly written in one language.
        let corpus = labels
            .iter()
            .map(|(_, item)| item.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let source_lang = detect_source_language(&corpus);

        // The repo's own organization, not the caller's tenancy
        // context. The anonymous docs read path has no operator to
        // derive a tenant from, and both sides must agree on the key or
        // the read finds nothing. `OperatorId` is an alias of
        // `TenantId`, so this is the same identifier either way.
        let tenant_id = repo.organization_id().clone();

        // Read once per run, not once per language: the rows are the
        // same and the per-language selection is a pure filter.
        let glossary_terms = self.glossary.find_by_repo(repo.id()).await?;
        let mut outcomes = Vec::with_capacity(languages.len());

        for target_lang in &languages {
            if source_lang
                .as_ref()
                .is_some_and(|source| source == target_lang)
            {
                outcomes.push(TranslationOutcome {
                    language: target_lang.to_string(),
                    translated: 0,
                    already_current: 0,
                    human_reviewed: 0,
                    skipped_same_language: true,
                });
                continue;
            }

            let glossary =
                glossary_for_language(&glossary_terms, target_lang);
            let outcome = self
                .translate_one_language(
                    translator.as_ref(),
                    model,
                    &tenant_id,
                    source_lang.as_ref(),
                    target_lang,
                    &labels,
                    &glossary,
                )
                .await?;
            outcomes.push(outcome);
        }

        Ok(RepoTranslationReport {
            outcomes,
            records_scanned,
            records_truncated,
        })
    }
}

impl TranslateRepo {
    #[allow(clippy::too_many_arguments)]
    async fn translate_one_language(
        &self,
        translator: &dyn Translator,
        model: &str,
        tenant_id: &value_object::TenantId,
        source_lang: Option<&LanguageTag>,
        target_lang: &LanguageTag,
        labels: &[(TranslationScope, TranslationItem)],
        glossary: &[GlossaryEntry],
    ) -> errors::Result<TranslationOutcome> {
        let mut already_current = 0usize;
        let mut human_reviewed = 0usize;
        let mut pending: Vec<(TranslationScope, TranslationItem, String)> =
            Vec::new();

        // Derived from the batch rather than hardcoded, so adding a
        // scope upstream needs no change here.
        let mut scopes: Vec<TranslationScope> =
            labels.iter().map(|(scope, _)| *scope).collect();
        scopes.sort_by_key(|scope| scope.as_str());
        scopes.dedup();

        for scope in scopes {
            let existing = self
                .translations
                .find_scope(tenant_id, scope, target_lang)
                .await?;

            for (item_scope, item) in labels {
                if *item_scope != scope {
                    continue;
                }
                let hash = source_hash(&item.text);
                match existing.get(&item.id) {
                    Some(record) if record.reviewed_by.is_some() => {
                        human_reviewed += 1;
                    }
                    Some(record) if record.is_current_for(&hash) => {
                        already_current += 1;
                    }
                    _ => pending.push((scope, item.clone(), hash)),
                }
            }
        }

        let mut translated = 0usize;
        for chunk in pending.chunks(BATCH_SIZE) {
            let items: Vec<TranslationItem> =
                chunk.iter().map(|(_, item, _)| item.clone()).collect();

            let results = translator
                .translate(TranslationBatch {
                    source_lang,
                    target_lang,
                    glossary,
                    items: &items,
                })
                .await?;

            for (scope, item, hash) in chunk {
                let Some(text) = results.get(&item.id) else {
                    // The parser guarantees completeness, so reaching
                    // here means the contract changed underneath us.
                    return Err(errors::Error::internal_server_error(
                        format!("no translation returned for {}", item.id),
                    ));
                };

                self.translations
                    .upsert(&TranslationRecord {
                        tenant_id: tenant_id.clone(),
                        scope: *scope,
                        target_id: item.id.clone(),
                        target_lang: target_lang.clone(),
                        source_lang: source_lang.cloned(),
                        source_hash: hash.clone(),
                        translated: Some(text.clone()),
                        status: TranslationStatus::Fresh,
                        model: Some(model.to_string()),
                        reviewed_by: None,
                    })
                    .await?;
                translated += 1;
            }
        }

        Ok(TranslationOutcome {
            language: target_lang.to_string(),
            translated,
            already_current,
            human_reviewed,
            skipped_same_language: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use database_manager::domain::{
        DatabaseId, PropertyId, SelectItem, SelectItemId, TypeId,
        TypeMultiSelect, TypeSelect,
    };
    use value_object::TenantId;

    fn property(
        id: &str,
        name: &str,
        property_type: PropertyType,
    ) -> Property {
        Property::new(
            &id.parse::<PropertyId>().unwrap(),
            &"tn_01j702qf86pc2j35s0kv0gv3gy".parse::<TenantId>().unwrap(),
            &"db_01hkz3700yt46snfewzpakeyj4"
                .parse::<DatabaseId>()
                .unwrap(),
            name,
            &property_type,
            false,
            0,
        )
    }

    fn option(id: &str, key: &str, name: &str) -> SelectItem {
        SelectItem::new(
            id.parse::<SelectItemId>().unwrap(),
            key.parse().unwrap(),
            name.parse().unwrap(),
        )
    }

    fn ids(
        labels: &[(TranslationScope, TranslationItem)],
        scope: TranslationScope,
    ) -> Vec<String> {
        labels
            .iter()
            .filter(|(item_scope, _)| *item_scope == scope)
            .map(|(_, item)| item.text.clone())
            .collect()
    }

    #[test]
    fn every_property_contributes_its_heading() {
        let labels = collect_schema_labels(&[
            property(
                "prop_01hkz3700yt46snfewzpakeyj4",
                "担当者",
                PropertyType::String,
            ),
            property(
                "prop_01hkz3700yt46snfewzpakeyj5",
                "期限",
                PropertyType::Date,
            ),
        ]);

        // A Date value is never translated, but its column heading is.
        assert_eq!(
            ids(&labels, TranslationScope::PropertyDef),
            vec!["担当者", "期限"]
        );
    }

    #[test]
    fn select_options_are_collected_as_schema_labels() {
        let select = PropertyType::Select(TypeSelect::new(vec![
            option("op_01hkz3700yt46snfewzpakeyj4", "todo", "未着手"),
            option("op_01hkz3700yt46snfewzpakeyj5", "doing", "対応中"),
        ]));
        let labels = collect_schema_labels(&[property(
            "prop_01hkz3700yt46snfewzpakeyj4",
            "状態",
            select,
        )]);

        assert_eq!(
            ids(&labels, TranslationScope::PropertyDef),
            vec!["状態"]
        );
        assert_eq!(
            ids(&labels, TranslationScope::SelectOption),
            vec!["未着手", "対応中"]
        );
    }

    #[test]
    fn multi_select_options_are_collected_too() {
        let multi =
            PropertyType::MultiSelect(TypeMultiSelect::new(vec![option(
                "op_01hkz3700yt46snfewzpakeyj4",
                "urgent",
                "至急",
            )]));
        let labels = collect_schema_labels(&[property(
            "prop_01hkz3700yt46snfewzpakeyj4",
            "タグ",
            multi,
        )]);
        assert_eq!(
            ids(&labels, TranslationScope::SelectOption),
            vec!["至急"]
        );
    }

    #[test]
    fn externally_synced_properties_are_left_out_entirely() {
        // Translating a mirrored field would make the copy disagree
        // with its source on the next sync.
        let labels = collect_schema_labels(&[property(
            "prop_01hkz3700yt46snfewzpakeyj4",
            "ext_github_state",
            PropertyType::Select(TypeSelect::new(vec![option(
                "op_01hkz3700yt46snfewzpakeyj4",
                "open",
                "Open",
            )])),
        )]);
        assert!(labels.is_empty());
    }

    #[test]
    fn a_non_select_property_contributes_no_option_labels() {
        let labels = collect_schema_labels(&[property(
            "prop_01hkz3700yt46snfewzpakeyj4",
            "ID",
            PropertyType::Id(TypeId::default()),
        )]);
        assert_eq!(
            ids(&labels, TranslationScope::SelectOption),
            Vec::<String>::new()
        );
    }

    fn record(id: &str, name: &str) -> database_manager::domain::Data {
        database_manager::domain::Data::new(
            &id.parse::<database_manager::domain::DataId>().unwrap(),
            &"tn_01j702qf86pc2j35s0kv0gv3gy".parse::<TenantId>().unwrap(),
            &"db_01hkz3700yt46snfewzpakeyj4"
                .parse::<DatabaseId>()
                .unwrap(),
            name,
            vec![],
            chrono::Utc::now(),
            chrono::Utc::now(),
        )
        .unwrap()
    }

    #[test]
    fn record_names_are_collected_and_keyed_by_record_id() {
        let records = vec![
            record("data_01hkz3700yt46snfewzpakeyj4", "四半期の売上まとめ"),
            record("data_01hkz3700yt46snfewzpakeyj5", "採用計画"),
        ];
        let collected = collect_record_names(&records);

        assert_eq!(collected.len(), 2);
        assert!(collected
            .iter()
            .all(|(scope, _)| *scope == TranslationScope::RecordName));
        assert_eq!(collected[0].1.id, "data_01hkz3700yt46snfewzpakeyj4");
        assert_eq!(collected[0].1.text, "四半期の売上まとめ");
    }

    #[test]
    fn an_empty_page_collects_nothing() {
        assert!(collect_record_names(&[]).is_empty());
    }

    #[test]
    fn option_ids_are_what_the_translation_is_keyed_by() {
        // Keying on the option id rather than its label is what lets a
        // renamed option keep its translation history.
        let select = PropertyType::Select(TypeSelect::new(vec![option(
            "op_01hkz3700yt46snfewzpakeyj4",
            "todo",
            "未着手",
        )]));
        let labels = collect_schema_labels(&[property(
            "prop_01hkz3700yt46snfewzpakeyj4",
            "状態",
            select,
        )]);
        let option_entry = labels
            .iter()
            .find(|(scope, _)| *scope == TranslationScope::SelectOption)
            .unwrap();
        assert_eq!(option_entry.1.id, "op_01hkz3700yt46snfewzpakeyj4");
    }
}

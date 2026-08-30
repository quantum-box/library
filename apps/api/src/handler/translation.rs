//! Published-language endpoints.
//!
//! Three routes: one anonymous, so a public docs page can offer a
//! language switcher without authenticating, and two authenticated ones
//! for the repo owner who decides what gets published.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path as AxumPath},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::app::LibraryApp;
use crate::handler::library_executor_extractor::LibraryExecutor;
use crate::usecase::{
    GetGlossaryInputData, GetPublishedLanguagesInputData, LibraryOrg,
    SetGlossaryInputData, SetPublishedLanguagesInputData,
    TranslateRepoInputData,
};

/// The languages a repo is published in.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PublishedLanguagesResponse {
    /// Normalized BCP-47 tags, sorted. Empty means the repo is served
    /// only in the language each document was written in.
    pub languages: Vec<String>,
}

/// Replaces the published set with exactly these languages.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SetPublishedLanguagesRequest {
    /// Tags are normalized and deduplicated, so `EN` and `en` are the
    /// same entry. An empty list unpublishes every language.
    pub languages: Vec<String>,
}

/// `GET /docs/:org/:repo/languages`
///
/// Anonymous for public repos, so the docs pages can render a language
/// switcher without a token. Private repos fall back to the usual
/// executor and policy checks.
#[utoipa::path(
    get,
    path = "/docs/{org}/{repo}/languages",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 200, description = "Published languages", body = PublishedLanguagesResponse),
        (status = 403, description = "Private repository access denied"),
        (status = 404, description = "Organization or repository not found")
    ),
    tag = "public-docs"
)]
#[axum::debug_handler]
pub async fn list_doc_languages(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
) -> errors::Result<Json<PublishedLanguagesResponse>> {
    // Anonymous callers carry no operator, so the tenancy context is
    // built from the path the same way the other docs routes do it.
    let library_org = LibraryOrg::with_org(org.clone());

    let languages = library_app
        .get_published_languages
        .execute(&GetPublishedLanguagesInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            organization_username: org,
            repo_username: repo,
        })
        .await?;

    Ok(Json(PublishedLanguagesResponse {
        languages: languages.iter().map(|tag| tag.to_string()).collect(),
    }))
}

/// `GET /v1beta/repos/:org/:repo/languages`
#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/languages",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 200, description = "Published languages", body = PublishedLanguagesResponse),
        (status = 403, description = "Access denied"),
        (status = 404, description = "Organization or repository not found")
    )
)]
#[axum::debug_handler]
pub async fn get_published_languages(
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
) -> errors::Result<Json<PublishedLanguagesResponse>> {
    let languages = library_app
        .get_published_languages
        .execute(&GetPublishedLanguagesInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            organization_username: org,
            repo_username: repo,
        })
        .await?;

    Ok(Json(PublishedLanguagesResponse {
        languages: languages.iter().map(|tag| tag.to_string()).collect(),
    }))
}

/// `PUT /v1beta/repos/:org/:repo/languages`
///
/// Declaring a language is what schedules translation for it, so this
/// route is the only way work enters the pipeline. It requires the same
/// permission as any other repo setting.
///
/// The public docs pages cache a repo's published set briefly, so a
/// change here can take up to a minute to show up there.
#[utoipa::path(
    put,
    path = "/v1beta/repos/{org}/{repo}/languages",
    request_body = SetPublishedLanguagesRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 200, description = "Published languages after the update", body = PublishedLanguagesResponse),
        (status = 400, description = "Malformed language tag, or too many languages"),
        (status = 403, description = "Access denied"),
        (status = 404, description = "Organization or repository not found")
    )
)]
#[axum::debug_handler]
pub async fn set_published_languages(
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(payload): Json<SetPublishedLanguagesRequest>,
) -> errors::Result<Json<PublishedLanguagesResponse>> {
    let languages = library_app
        .set_published_languages
        .execute(&SetPublishedLanguagesInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            organization_username: org,
            repo_username: repo,
            languages: payload.languages,
        })
        .await?;

    Ok(Json(PublishedLanguagesResponse {
        languages: languages.iter().map(|tag| tag.to_string()).collect(),
    }))
}

/// What one language's schema pass did.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TranslationOutcomeResponse {
    pub language: String,
    /// Labels sent to the model and stored on this run.
    pub translated: usize,
    /// Labels whose cached translation still matched the source.
    pub already_current: usize,
    /// Labels left alone because a person had edited the translation.
    pub human_reviewed: usize,
    /// True when the schema is already written in this language.
    pub skipped_same_language: bool,
}

/// Result of a schema translation run.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RunTranslationsResponse {
    pub outcomes: Vec<TranslationOutcomeResponse>,
    /// Records whose names this run considered.
    pub records_scanned: usize,
    /// True when the repo holds more records than one run visits. Call
    /// again rather than reading a finished run into this.
    pub records_truncated: bool,
}

/// `POST /v1beta/repos/:org/:repo/translations/run`
///
/// Translates the repo's schema labels -- column headings and select
/// option labels -- into every published language.
///
/// The run is explicit rather than automatic because the API is
/// deployed on Lambda, where a resident worker has nowhere to live.
/// Declaring a language stays fast; this is what spends the money, so
/// it carries the same permission as changing a repo setting.
#[utoipa::path(
    post,
    path = "/v1beta/repos/{org}/{repo}/translations/run",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 200, description = "Per-language outcome of the run", body = RunTranslationsResponse),
        (status = 400, description = "No translation model is configured"),
        (status = 403, description = "Access denied"),
        (status = 404, description = "Organization or repository not found")
    )
)]
#[axum::debug_handler]
pub async fn run_translations(
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
) -> errors::Result<Json<RunTranslationsResponse>> {
    let report = library_app
        .translate_schema_labels
        .execute(&TranslateRepoInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            organization_username: org,
            repo_username: repo,
        })
        .await?;

    Ok(Json(RunTranslationsResponse {
        records_scanned: report.records_scanned,
        records_truncated: report.records_truncated,
        outcomes: report
            .outcomes
            .into_iter()
            .map(|outcome| TranslationOutcomeResponse {
                language: outcome.language,
                translated: outcome.translated,
                already_current: outcome.already_current,
                human_reviewed: outcome.human_reviewed,
                skipped_same_language: outcome.skipped_same_language,
            })
            .collect(),
    }))
}

/// One fixed term translation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GlossaryTermPayload {
    /// The source term, as an author writes it.
    pub term: String,
    /// BCP-47 tag, or omitted to apply the entry to every language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_lang: Option<String>,
    /// What the term must become. Repeat the term itself to keep it
    /// untranslated.
    pub translation: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GlossaryResponse {
    pub terms: Vec<GlossaryTermPayload>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SetGlossaryRequest {
    /// Replaces the glossary wholesale. An empty list clears it.
    pub terms: Vec<GlossaryTermPayload>,
}

fn glossary_response(
    terms: Vec<crate::domain::translation::GlossaryTerm>,
) -> GlossaryResponse {
    GlossaryResponse {
        terms: terms
            .into_iter()
            .map(|term| GlossaryTermPayload {
                term: term.term,
                target_lang: term.target_lang.map(|tag| tag.to_string()),
                translation: term.translation,
            })
            .collect(),
    }
}

/// `GET /v1beta/repos/:org/:repo/glossary`
#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/glossary",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 200, description = "The repo glossary", body = GlossaryResponse),
        (status = 403, description = "Access denied"),
        (status = 404, description = "Organization or repository not found")
    )
)]
#[axum::debug_handler]
pub async fn get_glossary(
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
) -> errors::Result<Json<GlossaryResponse>> {
    let terms = library_app
        .get_glossary
        .execute(&GetGlossaryInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            organization_username: org,
            repo_username: repo,
        })
        .await?;

    Ok(Json(glossary_response(terms)))
}

/// `PUT /v1beta/repos/:org/:repo/glossary`
///
/// Replaces the glossary. Existing translations are left as they are: a
/// glossary change makes them wrong in the owner's judgement, not stale
/// against their source, and discarding paid-for — possibly reviewed —
/// work is not this endpoint's decision. Re-run the translation to
/// apply the new vocabulary.
#[utoipa::path(
    put,
    path = "/v1beta/repos/{org}/{repo}/glossary",
    request_body = SetGlossaryRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 200, description = "The glossary after the update", body = GlossaryResponse),
        (status = 400, description = "Malformed entry, duplicate term, or too many terms"),
        (status = 403, description = "Access denied"),
        (status = 404, description = "Organization or repository not found")
    )
)]
#[axum::debug_handler]
pub async fn set_glossary(
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(payload): Json<SetGlossaryRequest>,
) -> errors::Result<Json<GlossaryResponse>> {
    let terms = library_app
        .set_glossary
        .execute(&SetGlossaryInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            organization_username: org,
            repo_username: repo,
            terms: payload
                .terms
                .into_iter()
                .map(|entry| {
                    (entry.term, entry.target_lang, entry.translation)
                })
                .collect(),
        })
        .await?;

    Ok(Json(glossary_response(terms)))
}

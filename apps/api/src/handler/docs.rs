//! Public documentation endpoints
//!
//! Serves Library documents as rendered HTML pages or raw Markdown.
//! Public repos are readable without authentication. Private repos use
//! the same optional executor and policy checks as the REST data
//! endpoints.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path as AxumPath, Query},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use pulldown_cmark::{Options, Parser};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::app::LibraryApp;
use crate::domain::translation::{
    detect_source_language, negotiate_from_accept_language,
    resolve_requested_language, source_hash, LanguageChoice, LanguageTag,
    TranslationScope,
};
use crate::handler::library_executor_extractor::LibraryExecutor;
use crate::usecase::library_client_url::data_url;
use crate::usecase::markdown_composer::{
    compose_markdown, compose_markdown_with_ui_url,
};
use crate::usecase::{
    LibraryOrg, ViewDataInputData, ViewDataListInputData, ViewRepoInputData,
};

/// Language selection shared by every docs route.
#[derive(Deserialize, IntoParams, Default)]
#[into_params(parameter_in = Query)]
pub struct DocsLangQuery {
    /// BCP-47 tag. Anything the repo has not published is served as the
    /// source text rather than refused, so a stale link never breaks.
    pub lang: Option<String>,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DocsListQuery {
    /// BCP-47 tag, as on the other docs routes. Carried here rather
    /// than in a second extractor because axum takes one `Query` per
    /// handler.
    pub lang: Option<String>,
    /// 1-origin page number. Defaults to 1.
    #[param(minimum = 1)]
    pub page: Option<u32>,
    /// Number of documents per page. Defaults to 50 and is capped at 100.
    #[param(minimum = 1, maximum = 100)]
    pub page_size: Option<u32>,
}

// ───────────────────────────── Handlers ─────────────────────────────

/// `GET /docs/:org/:repo`
///
/// Lists documents in a repo as an HTML page.
#[utoipa::path(
    get,
    path = "/docs/{org}/{repo}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        DocsListQuery
    ),
    responses(
        (status = 200, description = "Rendered docs list HTML", body = String, content_type = "text/html"),
        (status = 400, description = "Invalid pagination request"),
        (status = 403, description = "Private repository access denied"),
        (status = 404, description = "Organization or repository not found")
    ),
    tag = "public-docs"
)]
#[axum::debug_handler]
pub async fn list_docs(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Query(query): Query<DocsListQuery>,
    headers: axum::http::HeaderMap,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
) -> errors::Result<axum::response::Response> {
    let library_org = LibraryOrg::with_org(org.clone());
    let path = format!("/docs/{org}/{repo}");

    let language = resolve_doc_language(
        &library_app,
        &executor,
        &library_org,
        &org,
        &repo,
        query.lang.as_deref(),
    )
    .await?;

    // Same redirect rule as the document page: the header picks a URL,
    // never a body. Leaving it off here would make the listing and the
    // pages it links to disagree about what a reader gets.
    if query.lang.is_none() {
        if let Some(tag) = negotiate_from_accept_language(
            headers
                .get(axum::http::header::ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok()),
            &language.published,
        ) {
            return Ok(axum::response::Redirect::temporary(&format!(
                "{path}?lang={tag}"
            ))
            .into_response());
        }
    }

    let input = ViewDataListInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org.clone(),
        repo_username: repo.clone(),
        page: Some(query.page.unwrap_or(1)),
        page_size: Some(query.page_size.unwrap_or(50)),
    };

    let (data_list, _properties, paginator) =
        library_app.view_data_list.execute(&input).await?;

    let mut items_html = String::new();
    for data in &data_list {
        items_html.push_str(&format!(
            r#"<li class="doc-item">
                <a href="/docs/{org}/{repo}/{id}">{title}</a>
            </li>"#,
            org = html_escape(&org),
            repo = html_escape(&repo),
            id = html_escape(data.id().as_ref()),
            title = html_escape(
                language
                    .record_names
                    .get(data.id().as_ref())
                    .cloned()
                    .unwrap_or_else(|| data.name().to_string())
                    .as_str()
            ),
        ));
    }

    let pagination = if paginator.total_pages > 1 {
        let mut nav = String::from(r#"<nav class="pagination">"#);
        for p in 1..=paginator.total_pages {
            if p == query.page.unwrap_or(1) {
                nav.push_str(&format!(
                    r#"<span class="current">{p}</span>"#
                ));
            } else {
                nav.push_str(&format!(
                    r#"<a href="/docs/{org}/{repo}?page={p}">{p}</a>"#,
                    org = html_escape(&org),
                    repo = html_escape(&repo),
                ));
            }
        }
        nav.push_str("</nav>");
        nav
    } else {
        String::new()
    };

    // The listing has no body text, so the document titles are the
    // only evidence of what language this repo is written in.
    let titles = data_list
        .iter()
        .map(|data| data.name().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let lang = match language.choice.translated_tag() {
        Some(tag) => tag.to_string(),
        None => document_lang(&titles),
    };
    let link_tags = language_link_tags(&path, &language);
    let langbar = language_bar(&path, &language);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{repo} — Docs</title>
{link_tags}
{STYLE}
</head>
<body>
<header>
    <div class="header-inner">
        <a href="/docs/{org_e}/{repo_e}" class="logo">{org_e}<span class="sep">/</span>{repo_e}</a>
    </div>
</header>
<main>
    {langbar}
    <h1>Documents</h1>
    <p class="meta">{total} documents</p>
    <ul class="doc-list">{items_html}</ul>
    {pagination}
</main>
<footer><p>Powered by <strong>Library</strong></p></footer>
</body>
</html>"#,
        repo = html_escape(&repo),
        org_e = html_escape(&org),
        repo_e = html_escape(&repo),
        total = paginator.total_items,
        STYLE = DOCS_STYLE,
    );

    Ok(docs_response(html, &lang, &language))
}

/// `GET /docs/:org/:repo/:data_id`
///
/// Renders a single document as an HTML page with the Markdown body
/// converted to HTML via pulldown-cmark.
#[utoipa::path(
    get,
    path = "/docs/{org}/{repo}/{data_id}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("data_id" = String, Path, description = "Data ID"),
        DocsLangQuery
    ),
    responses(
        (status = 200, description = "Rendered document HTML", body = String, content_type = "text/html"),
        (status = 403, description = "Private repository access denied"),
        (status = 404, description = "Organization, repository, or data not found")
    ),
    tag = "public-docs"
)]
#[axum::debug_handler]
pub async fn view_doc(
    AxumPath((org, repo, data_id)): AxumPath<(String, String, String)>,
    Query(lang_query): Query<DocsLangQuery>,
    headers: axum::http::HeaderMap,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
) -> errors::Result<axum::response::Response> {
    let library_org = LibraryOrg::with_org(org.clone());
    let path = format!("/docs/{org}/{repo}/{data_id}");

    let language = resolve_doc_language(
        &library_app,
        &executor,
        &library_org,
        &org,
        &repo,
        lang_query.lang.as_deref(),
    )
    .await?;

    // With no explicit choice, an `Accept-Language` match redirects to
    // the canonical URL for that language rather than varying the body:
    // one URL, one language, one cache entry.
    if lang_query.lang.is_none() {
        if let Some(tag) = negotiate_from_accept_language(
            headers
                .get(axum::http::header::ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok()),
            &language.published,
        ) {
            return Ok(axum::response::Redirect::temporary(&format!(
                "{path}?lang={tag}"
            ))
            .into_response());
        }
    }

    let input = ViewDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org.clone(),
        repo_username: repo.clone(),
        data_id: data_id.clone(),
    };

    let (data, properties) = library_app.view_data.execute(&input).await?;
    let properties = apply_property_name_translations(
        &properties,
        &language.property_names,
    );
    let markdown = compose_markdown(&data, &properties);

    // Strip YAML frontmatter before rendering
    let body_md = strip_frontmatter(&markdown);
    let html_body = markdown_to_html(body_md);

    // The declared language is the one actually being served: the
    // requested translation when there is one, otherwise whatever the
    // document is written in.
    let lang = match language.choice.translated_tag() {
        Some(tag) => tag.to_string(),
        None => document_lang(&format!("{}\n{}", data.name(), body_md)),
    };
    let link_tags = language_link_tags(&path, &language);
    let langbar = language_bar(&path, &language);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} — {repo} Docs</title>
{link_tags}
{STYLE}
</head>
<body>
<header>
    <div class="header-inner">
        <a href="/docs/{org_e}/{repo_e}" class="logo">{org_e}<span class="sep">/</span>{repo_e}</a>
    </div>
</header>
<main class="document">
    {langbar}
    <article>{html_body}</article>
    <a href="/docs/{org_e}/{repo_e}" class="back">&larr; Back to documents</a>
</main>
<footer><p>Powered by <strong>Library</strong></p></footer>
</body>
</html>"#,
        title = html_escape(
            language
                .record_names
                .get(data.id().as_ref())
                .cloned()
                .unwrap_or_else(|| data.name().to_string())
                .as_str()
        ),
        repo = html_escape(&repo),
        org_e = html_escape(&org),
        repo_e = html_escape(&repo),
        STYLE = DOCS_STYLE,
    );

    Ok(docs_response(html, &lang, &language))
}

/// `GET /docs/:org/:repo/:data_id/md`
///
/// Returns the raw composed Markdown (with YAML frontmatter). The
/// frontmatter carries `url`, the address of this document in the
/// Library client, alongside `id` and `title`.
#[utoipa::path(
    get,
    path = "/docs/{org}/{repo}/{data_id}/md",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("data_id" = String, Path, description = "Data ID"),
        DocsLangQuery
    ),
    responses(
        (status = 200, description = "Composed Markdown with YAML frontmatter, including the document's `url` in the Library client", body = String, content_type = "text/markdown"),
        (status = 403, description = "Private repository access denied"),
        (status = 404, description = "Organization, repository, or data not found")
    ),
    tag = "public-docs"
)]
#[axum::debug_handler]
pub async fn view_doc_markdown(
    AxumPath((org, repo, data_id)): AxumPath<(String, String, String)>,
    Query(lang_query): Query<DocsLangQuery>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
) -> errors::Result<impl IntoResponse> {
    let library_org = LibraryOrg::with_org(org.clone());

    let language = resolve_doc_language(
        &library_app,
        &executor,
        &library_org,
        &org,
        &repo,
        lang_query.lang.as_deref(),
    )
    .await?;

    let input = ViewDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org.clone(),
        repo_username: repo.clone(),
        data_id: data_id.clone(),
    };

    let (data, properties) = library_app.view_data.execute(&input).await?;
    let properties = apply_property_name_translations(
        &properties,
        &language.property_names,
    );
    let ui_url = data_url(&org, &repo, &data_id);
    let markdown =
        compose_markdown_with_ui_url(&data, &properties, Some(&ui_url));

    // No `Accept-Language` redirect here. This route is read by agents
    // and scripts as much as by browsers, and a 302 on a data endpoint
    // surprises callers that a browser would not notice.
    let content_language = match language.choice.translated_tag() {
        Some(tag) => tag.to_string(),
        None => document_lang(&markdown),
    };

    Ok((
        StatusCode::OK,
        [
            ("Content-Type", "text/markdown; charset=utf-8".to_string()),
            ("Content-Language", content_language),
            (
                "X-Library-Translation",
                language.translation_state().to_string(),
            ),
        ],
        markdown,
    ))
}

// ──────────────────────── Language resolution ───────────────────────

/// Default lifetime of a cached repo context.
///
/// Publishing a language becomes visible to readers within this window.
const DEFAULT_DOCS_REPO_CACHE_TTL_SECS: u64 = 60;

/// Distinct repos held at once. Public traffic concentrates on a few.
const DOCS_REPO_CACHE_CAPACITY: usize = 512;

/// The repo facts every docs request needs before it can pick a
/// language: which repo this is, and what it publishes.
///
/// Cached because resolving them costs an organization lookup and a
/// repo lookup that `view_data` is about to repeat, plus a query for
/// the published set — three round trips added to the anonymous read
/// path, paid even by repos that publish nothing.
///
/// This is not an authorization cache. It holds only identifiers and a
/// language list, and the `view_data` / `view_data_list` call that
/// follows still performs the full visibility check on every request,
/// hit or miss.
#[derive(Debug, Clone)]
struct DocsRepoContext {
    organization_id: value_object::TenantId,
    published: Vec<LanguageTag>,
}

static DOCS_REPO_CONTEXT: once_cell::sync::Lazy<
    crate::ttl_cache::TtlCache<(String, String), DocsRepoContext>,
> = once_cell::sync::Lazy::new(|| {
    crate::ttl_cache::TtlCache::new(
        docs_repo_cache_ttl(),
        DOCS_REPO_CACHE_CAPACITY,
    )
});

/// `DOCS_REPO_CACHE_TTL_SECS` overrides the default, and `0` disables
/// the cache outright — the switch to reach for when a stale entry has
/// to be ruled out without a deploy.
fn docs_repo_cache_ttl() -> std::time::Duration {
    let secs = std::env::var("DOCS_REPO_CACHE_TTL_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_DOCS_REPO_CACHE_TTL_SECS);
    std::time::Duration::from_secs(secs)
}

/// The language a docs response is being served in, with the cached
/// labels needed to render it.
struct DocLanguage {
    /// Everything the repo publishes, for the switcher and `hreflang`.
    published: Vec<LanguageTag>,
    choice: LanguageChoice,
    /// Property definition id -> translated heading.
    property_names: std::collections::HashMap<String, String>,
    /// Record id -> translated name. The listing is nothing but these,
    /// so an untranslated map here is what a reader notices first.
    record_names: std::collections::HashMap<String, String>,
}

impl DocLanguage {
    /// The value for `X-Library-Translation`, which lets an API or
    /// agent caller tell a translation from the original.
    fn translation_state(&self) -> &'static str {
        match self.choice {
            LanguageChoice::Source => "source",
            LanguageChoice::Translated(_) => "fresh",
        }
    }
}

/// Resolves the requested language and loads its schema labels.
///
/// Never fails on an unknown language: the allow-list decides, and
/// anything outside it degrades to the source text.
async fn resolve_doc_language(
    library_app: &LibraryApp,
    executor: &LibraryExecutor,
    library_org: &LibraryOrg,
    org: &str,
    repo: &str,
    requested: Option<&str>,
) -> errors::Result<DocLanguage> {
    let cache_key = (org.to_string(), repo.to_string());
    let context = match DOCS_REPO_CONTEXT.get(&cache_key) {
        Some(hit) => hit,
        None => {
            let repo_entity = library_app
                .view_repo
                .execute(&ViewRepoInputData {
                    executor,
                    multi_tenancy: library_org,
                    organization_username: org.to_string(),
                    repo_username: repo.to_string(),
                })
                .await?
                .repo;
            let context = DocsRepoContext {
                organization_id: repo_entity.organization_id().clone(),
                published: library_app
                    .published_language_repo
                    .find_by_repo(repo_entity.id())
                    .await?,
            };
            DOCS_REPO_CONTEXT.insert(cache_key, context.clone());
            context
        }
    };
    let published = context.published;

    let choice = resolve_requested_language(requested, &published);

    let (property_names, record_names) = match choice.translated_tag() {
        None => Default::default(),
        Some(tag) => {
            let tenant = &context.organization_id;
            (
                translated_texts(
                    library_app,
                    tenant,
                    TranslationScope::PropertyDef,
                    tag,
                )
                .await?,
                translated_texts(
                    library_app,
                    tenant,
                    TranslationScope::RecordName,
                    tag,
                )
                .await?,
            )
        }
    };

    Ok(DocLanguage {
        published,
        choice,
        property_names,
        record_names,
    })
}

/// Loads one scope's cached text, dropping rows that carry none.
///
/// A pending or failed row is absent from the map, which is what makes
/// the read path degrade to the source per item rather than per page.
async fn translated_texts(
    library_app: &LibraryApp,
    tenant: &value_object::TenantId,
    scope: TranslationScope,
    tag: &LanguageTag,
) -> errors::Result<std::collections::HashMap<String, String>> {
    Ok(library_app
        .translation_repo
        .find_scope(tenant, scope, tag)
        .await?
        .into_iter()
        .filter_map(|(target_id, record)| {
            record.translated.map(|text| (target_id, text))
        })
        .collect())
}

/// Rebuilds the property list with translated headings.
///
/// Only the definition name changes; ids, types and configuration are
/// carried through untouched, so nothing downstream can tell the
/// difference apart from the label.
fn apply_property_name_translations(
    properties: &[database_manager::domain::Property],
    names: &std::collections::HashMap<String, String>,
) -> Vec<database_manager::domain::Property> {
    properties
        .iter()
        .map(|property| match names.get(&property.id().to_string()) {
            Some(translated) => {
                database_manager::domain::Property::with_meta_json(
                    property.id(),
                    property.tenant_id(),
                    property.database_id(),
                    translated,
                    property.property_type(),
                    *property.is_indexed(),
                    *property.property_num(),
                    property.meta_json().clone(),
                )
            }
            None => property.clone(),
        })
        .collect()
}

/// Renders the `hreflang` set and canonical link for a docs page.
///
/// Only published languages are advertised. Announcing a language that
/// falls back to the source would tell a search engine a translation
/// exists where it does not.
fn language_link_tags(path: &str, language: &DocLanguage) -> String {
    let mut tags = String::new();
    let canonical = match language.choice.translated_tag() {
        Some(tag) => format!("{path}?lang={tag}"),
        None => path.to_string(),
    };
    tags.push_str(&format!(
        r#"<link rel="canonical" href="{}">"#,
        html_escape(&canonical)
    ));
    // The source is what a reader gets with no preference expressed.
    tags.push_str(&format!(
        r#"<link rel="alternate" hreflang="x-default" href="{}">"#,
        html_escape(path)
    ));
    for tag in &language.published {
        tags.push_str(&format!(
            r#"<link rel="alternate" hreflang="{tag}" href="{}?lang={tag}">"#,
            html_escape(path)
        ));
    }
    tags
}

/// Renders the language switcher and, on a translation, the notice that
/// the text is machine generated.
///
/// Hiding the machine-translation notice would leave a reader who spots
/// an error with no way back to the original.
fn language_bar(path: &str, language: &DocLanguage) -> String {
    if language.published.is_empty() {
        return String::new();
    }

    let mut bar = String::from(r#"<div class="langbar">"#);
    let is_source = language.choice.translated_tag().is_none();
    if is_source {
        bar.push_str(r#"<span class="current">original</span>"#);
    } else {
        bar.push_str(&format!(
            r#"<a href="{}">original</a>"#,
            html_escape(path)
        ));
    }
    for tag in &language.published {
        let selected = language
            .choice
            .translated_tag()
            .is_some_and(|current| current == tag);
        if selected {
            bar.push_str(&format!(r#"<span class="current">{tag}</span>"#));
        } else {
            bar.push_str(&format!(
                r#"<a href="{}?lang={tag}">{tag}</a>"#,
                html_escape(path)
            ));
        }
    }
    bar.push_str("</div>");

    if !is_source {
        bar.push_str(
            r#"<p class="mt-notice">Machine translated. <a href=""#,
        );
        bar.push_str(&html_escape(path));
        bar.push_str(r#"">View the original</a>.</p>"#);
    }

    bar
}

/// Wraps a rendered docs page with the headers a public, cacheable,
/// possibly-translated page needs.
///
/// `Vary: Accept-Language` is deliberately not set. The header only ever
/// drives a redirect, so the body for a given URL never depends on it,
/// and declaring the variance would fragment every CDN entry by browser.
fn docs_response(
    html: String,
    lang: &str,
    language: &DocLanguage,
) -> axum::response::Response {
    use axum::http::header::{
        HeaderValue, CACHE_CONTROL, CONTENT_LANGUAGE, ETAG,
    };

    // A strong validator over the rendered bytes. Conditional GET is not
    // implemented yet, so this only helps caches dedupe; it is correct
    // as far as it goes and costs one hash.
    let etag = format!("\"{}\"", &source_hash(&html)[..32]);
    let state = language.translation_state();

    let mut response = Html(html).into_response();
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(lang) {
        headers.insert(CONTENT_LANGUAGE, value);
    }
    if let Ok(value) = HeaderValue::from_str(&etag) {
        headers.insert(ETAG, value);
    }
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static(
            "public, max-age=300, stale-while-revalidate=86400",
        ),
    );
    // `state` is one of a fixed set of ASCII literals, so this cannot
    // fail; the fallible form avoids a panic path regardless.
    if let Ok(value) = HeaderValue::from_str(state) {
        headers.insert("x-library-translation", value);
    }
    response
}

// ───────────────────────────── Helpers ──────────────────────────────

/// The `lang` attribute served when the text does not identify its own
/// language.
///
/// Latin-script prose could be any of a dozen languages, so the page
/// keeps the value it has always declared rather than guessing.
const UNDETERMINED_DOC_LANG: &str = "en";

/// Resolves the `lang` attribute for a rendered docs page.
///
/// The pages used to declare `en` unconditionally, which mislabelled
/// every Japanese repo. Detection is script-based, so it settles the
/// languages that Library actually mixes and abstains on the rest.
fn document_lang(text: &str) -> String {
    detect_source_language(text)
        .map(|tag| tag.to_string())
        .unwrap_or_else(|| UNDETERMINED_DOC_LANG.to_string())
}

/// Convert Markdown to HTML using pulldown-cmark with common extensions.
fn markdown_to_html(md: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(md, options);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

/// Strip YAML frontmatter (delimited by `---`) from a Markdown string.
fn strip_frontmatter(md: &str) -> &str {
    if !md.starts_with("---") {
        return md;
    }
    // Find the closing `---`
    if let Some(end) = md[3..].find("\n---") {
        let after = end + 3 + 4; // skip past "\n---"
        md[after..].trim_start_matches('\n')
    } else {
        md
    }
}

/// Minimal HTML-entity escaping.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ───────────────────────────── Style ────────────────────────────────

const DOCS_STYLE: &str = r#"<style>
:root {
    --bg: #ffffff;
    --fg: #1a1a2e;
    --muted: #64748b;
    --accent: #6366f1;
    --border: #e2e8f0;
    --surface: #f8fafc;
}
@media (prefers-color-scheme: dark) {
    :root {
        --bg: #0f172a;
        --fg: #e2e8f0;
        --muted: #94a3b8;
        --accent: #818cf8;
        --border: #1e293b;
        --surface: #1e293b;
    }
}
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background: var(--bg);
    color: var(--fg);
    line-height: 1.7;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
}
header {
    border-bottom: 1px solid var(--border);
    padding: 1rem 2rem;
}
.header-inner { max-width: 48rem; margin: 0 auto; }
.logo {
    text-decoration: none;
    color: var(--fg);
    font-weight: 700;
    font-size: 1.1rem;
}
.logo .sep { color: var(--muted); margin: 0 0.15em; }
main {
    max-width: 48rem;
    margin: 0 auto;
    padding: 2rem;
    flex: 1;
    width: 100%;
}
h1 { font-size: 1.75rem; margin-bottom: 0.25rem; }
.meta { color: var(--muted); margin-bottom: 1.5rem; font-size: 0.9rem; }
.doc-list { list-style: none; }
.doc-item { border-bottom: 1px solid var(--border); }
.doc-item a {
    display: block;
    padding: 0.75rem 0;
    text-decoration: none;
    color: var(--accent);
    font-weight: 500;
}
.doc-item a:hover { text-decoration: underline; }
.pagination { margin-top: 2rem; display: flex; gap: 0.5rem; }
.pagination a, .pagination .current {
    display: inline-block;
    padding: 0.3rem 0.7rem;
    border-radius: 4px;
    text-decoration: none;
    font-size: 0.85rem;
}
.pagination a { background: var(--surface); color: var(--accent); }
.pagination .current { background: var(--accent); color: #fff; }
article {
    line-height: 1.8;
}
article h1 { font-size: 2rem; margin: 1.5rem 0 0.75rem; }
article h2 { font-size: 1.5rem; margin: 1.25rem 0 0.5rem; }
article h3 { font-size: 1.25rem; margin: 1rem 0 0.5rem; }
article p { margin-bottom: 1rem; }
article pre {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 1rem;
    overflow-x: auto;
    margin-bottom: 1rem;
    font-size: 0.875rem;
}
article code {
    font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
    font-size: 0.875em;
}
article :not(pre) > code {
    background: var(--surface);
    padding: 0.15em 0.35em;
    border-radius: 3px;
}
article img { max-width: 100%; border-radius: 6px; }
article blockquote {
    border-left: 3px solid var(--accent);
    padding-left: 1rem;
    margin: 1rem 0;
    color: var(--muted);
}
article table {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: 1rem;
}
article th, article td {
    border: 1px solid var(--border);
    padding: 0.5rem 0.75rem;
    text-align: left;
}
article th { background: var(--surface); font-weight: 600; }
article ul, article ol { margin-bottom: 1rem; padding-left: 1.5rem; }
article li { margin-bottom: 0.25rem; }
.langbar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 1rem;
    font-size: 0.8rem;
}
.langbar a, .langbar .current {
    padding: 0.15rem 0.5rem;
    border-radius: 3px;
    text-decoration: none;
}
.langbar a { background: var(--surface); color: var(--accent); }
.langbar a:hover { text-decoration: underline; }
.langbar .current { background: var(--accent); color: #fff; }
.mt-notice {
    font-size: 0.8rem;
    color: var(--muted);
    margin-bottom: 1.5rem;
}
.mt-notice a { color: var(--accent); }
.back {
    display: inline-block;
    margin-top: 2rem;
    color: var(--accent);
    text-decoration: none;
    font-size: 0.9rem;
}
.back:hover { text-decoration: underline; }
footer {
    border-top: 1px solid var(--border);
    padding: 1rem 2rem;
    text-align: center;
    color: var(--muted);
    font-size: 0.8rem;
}
</style>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_frontmatter() {
        let md = "---\nid: data_123\ntitle: Hello\n---\n\n# Hello\n";
        assert_eq!(strip_frontmatter(md), "# Hello\n");
    }

    #[test]
    fn test_strip_frontmatter_no_frontmatter() {
        let md = "# Hello\n";
        assert_eq!(strip_frontmatter(md), "# Hello\n");
    }

    #[test]
    fn test_markdown_to_html() {
        let md = "# Hello\n\nWorld";
        let html = markdown_to_html(md);
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<p>World</p>"));
    }

    #[test]
    fn document_lang_reports_the_language_of_japanese_content() {
        assert_eq!(document_lang("# 設計メモ\n\n本文はこちらです"), "ja");
    }

    #[test]
    fn document_lang_falls_back_for_latin_script_content() {
        // Latin script cannot single out a language, so the page keeps
        // the value it declared before detection existed.
        assert_eq!(
            document_lang("# Design notes\n\nThe body goes here"),
            "en"
        );
    }

    #[test]
    fn document_lang_falls_back_when_there_is_nothing_to_judge() {
        assert_eq!(document_lang(""), "en");
    }

    #[test]
    fn document_lang_emits_a_value_safe_to_inline_unescaped() {
        // The attribute is interpolated into the template without
        // escaping, which is only sound while the tag stays
        // alphanumeric.
        for text in ["設計メモの本文", "이것은 한국어 문서입니다", ""]
        {
            let lang = document_lang(text);
            assert!(
                lang.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "`{lang}` must be safe to inline into the template"
            );
        }
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(
            html_escape("<script>alert('xss')</script>"),
            "&lt;script&gt;alert('xss')&lt;/script&gt;"
        );
    }
}

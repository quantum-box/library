//! Public documentation endpoints
//!
//! Serves documents from public Library repos as rendered HTML pages
//! or raw Markdown. These endpoints require no authentication — access
//! is governed by the repo's `is_public` flag (checked inside the
//! existing ViewData / ViewDataList usecases).

use std::sync::Arc;

use axum::{
    extract::{Extension, Path as AxumPath, Query},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use pulldown_cmark::{Options, Parser};
use serde::Deserialize;

use crate::app::LibraryApp;
use crate::handler::library_executor_extractor::{
    LibraryExecutor, LibraryExecutorKind,
};
use crate::usecase::markdown_composer::compose_markdown;
use crate::usecase::{LibraryOrg, ViewDataInputData, ViewDataListInputData};

/// Anonymous executor for unauthenticated public access.
fn anonymous_executor() -> LibraryExecutor {
    LibraryExecutor {
        inner: LibraryExecutorKind::None,
        original_token: None,
    }
}

#[derive(Deserialize)]
pub struct DocsListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

// ───────────────────────────── Handlers ─────────────────────────────

/// `GET /docs/:org/:repo`
///
/// Lists documents in a public repo as an HTML page.
#[axum::debug_handler]
pub async fn list_docs(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Query(query): Query<DocsListQuery>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
) -> errors::Result<impl IntoResponse> {
    let executor = anonymous_executor();
    let library_org = LibraryOrg::with_org(org.clone());

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
            id = html_escape(&data.id().to_string()),
            title = html_escape(&data.name().to_string()),
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

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{repo} — Docs</title>
{STYLE}
</head>
<body>
<header>
    <div class="header-inner">
        <a href="/docs/{org_e}/{repo_e}" class="logo">{org_e}<span class="sep">/</span>{repo_e}</a>
    </div>
</header>
<main>
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

    Ok(Html(html))
}

/// `GET /docs/:org/:repo/:data_id`
///
/// Renders a single document as an HTML page with the Markdown body
/// converted to HTML via pulldown-cmark.
#[axum::debug_handler]
pub async fn view_doc(
    AxumPath((org, repo, data_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
) -> errors::Result<impl IntoResponse> {
    let executor = anonymous_executor();
    let library_org = LibraryOrg::with_org(org.clone());

    let input = ViewDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org.clone(),
        repo_username: repo.clone(),
        data_id,
    };

    let (data, properties) =
        library_app.view_data.execute(&input).await?;
    let markdown = compose_markdown(&data, &properties);

    // Strip YAML frontmatter before rendering
    let body_md = strip_frontmatter(&markdown);
    let html_body = markdown_to_html(body_md);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} — {repo} Docs</title>
{STYLE}
</head>
<body>
<header>
    <div class="header-inner">
        <a href="/docs/{org_e}/{repo_e}" class="logo">{org_e}<span class="sep">/</span>{repo_e}</a>
    </div>
</header>
<main class="document">
    <article>{html_body}</article>
    <a href="/docs/{org_e}/{repo_e}" class="back">&larr; Back to documents</a>
</main>
<footer><p>Powered by <strong>Library</strong></p></footer>
</body>
</html>"#,
        title = html_escape(&data.name().to_string()),
        repo = html_escape(&repo),
        org_e = html_escape(&org),
        repo_e = html_escape(&repo),
        STYLE = DOCS_STYLE,
    );

    Ok(Html(html))
}

/// `GET /docs/:org/:repo/:data_id/md`
///
/// Returns the raw composed Markdown (with YAML frontmatter).
#[axum::debug_handler]
pub async fn view_doc_markdown(
    AxumPath((org, repo, data_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
) -> errors::Result<impl IntoResponse> {
    let executor = anonymous_executor();
    let library_org = LibraryOrg::with_org(org.clone());

    let input = ViewDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org,
        repo_username: repo,
        data_id,
    };

    let (data, properties) =
        library_app.view_data.execute(&input).await?;
    let markdown = compose_markdown(&data, &properties);

    Ok((
        StatusCode::OK,
        [("Content-Type", "text/markdown; charset=utf-8")],
        markdown,
    ))
}

// ───────────────────────────── Helpers ──────────────────────────────

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
    fn test_html_escape() {
        assert_eq!(
            html_escape("<script>alert('xss')</script>"),
            "&lt;script&gt;alert('xss')&lt;/script&gt;"
        );
    }
}

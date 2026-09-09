//! Anonymous public-document responses for Cloudflare Pages.
use library_worker_common::{
    api_request, bounded_stream, decode, encode, fetch_timeout,
    response_json,
};
use lol_html::{
    element, html_content::ContentType, rewrite_str, RewriteStrSettings,
};
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

const PUBLIC_ORIGIN: &str = "https://planetlibrary.txcloud.app";
const LIMIT: usize = 2 * 1024 * 1024;
// Set by the build script from the same resolved Vite environment as the SPA.
const API_BASE: &str = match option_env!("VITE_LIBRARY_API_BASE_URL") {
    Some(v) => v,
    None => "",
};

type PublicResult<T> = std::result::Result<T, u16>;
#[derive(Deserialize)]
struct Profile {
    name: String,
    username: String,
    #[serde(default, deserialize_with = "nullable_string")]
    description: String,
    is_public: bool,
}
#[derive(Deserialize)]
struct Data {
    id: String,
    name: String,
    #[serde(default)]
    items: Vec<Item>,
}
#[derive(Deserialize)]
struct Item {
    property_id: String,
    value: Value,
}
#[derive(Deserialize)]
struct Listing {
    data: Vec<Data>,
    paginator: Paginator,
}
#[derive(Deserialize)]
struct Paginator {
    total_pages: u32,
}
#[derive(Deserialize)]
struct Property {
    id: String,
    property_type: String,
}

fn nullable_string<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

async fn read_json<T: serde::de::DeserializeOwned>(
    path: &str,
) -> PublicResult<T> {
    if API_BASE.is_empty() {
        return Err(503);
    }
    let headers = Headers::new();
    headers
        .set("accept", "application/json")
        .map_err(|_| 503u16)?;
    let req = api_request(
        &format!("{}/v1beta/repos/{path}", API_BASE.trim_end_matches('/')),
        Method::Get,
        headers,
        None,
    )
    .map_err(|_| 503u16)?;
    let mut response =
        fetch_timeout(req, 8000).await.map_err(|_| 503u16)?;
    match response.status_code() {
        200..=299 => {
            response_json(&mut response, LIMIT).await.map_err(|_| 503)
        }
        401 | 403 | 404 => Err(404),
        _ => Err(503),
    }
}
fn esc(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&#39;".into(),
            _ => c.to_string(),
        })
        .collect()
}
fn replace(source: &str, pattern: &str, replacement: &str) -> String {
    Regex::new(pattern)
        .expect("static regex")
        .replace_all(source, replacement)
        .into_owned()
}
fn body_text(value: &str, format: &str) -> String {
    if format == "richText" {
        return serde_json::from_str::<Value>(value)
            .ok()
            .map(|v| visit(&v, 0).trim().into())
            .unwrap_or_default();
    }
    if format == "html" && value.trim_start().starts_with('<') {
        let value =
            replace(value, r"(?is)<script\b[^>]*>.*?</script\s*>", "");
        let value =
            replace(&value, r"(?is)<style\b[^>]*>.*?</style\s*>", "");
        let mut value = replace(&value, r"<[^>]+>", " ");
        for (pattern, text) in [
            ("(?i)&nbsp;", " "),
            ("(?i)&amp;", "&"),
            ("(?i)&lt;", "<"),
            ("(?i)&gt;", ">"),
            ("(?i)&quot;", "\""),
            ("&#39;", "'"),
        ] {
            value = replace(&value, pattern, text);
        }
        return value.trim().into();
    }
    let value = replace(value, r"!\[([^\]]*)\]\([^)]*\)", "$1");
    let value = replace(&value, r"\[([^\]]+)\]\([^)]*\)", "$1");
    let value =
        replace(&value, r"(?m)^\s{0,3}(?:#{1,6}\s+|>\s?|[-*+]\s+)", "");
    replace(&value, r"[*_`~]", "").trim().into()
}
fn visit(node: &Value, depth: usize) -> String {
    if depth > 40 {
        return String::new();
    }
    match node {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(|v| visit(v, depth + 1)).collect(),
        Value::Object(_) => {
            if let Some(s) = node["text"].as_str() {
                return s.into();
            }
            if node["type"] == "tableContent" {
                return visit(&node["rows"], depth + 1);
            }
            if let Some(cells) = node["cells"].as_array() {
                return cells
                    .iter()
                    .map(|c| visit(c, depth + 1))
                    .collect::<Vec<_>>()
                    .join(" | ")
                    + "\n";
            }
            if node["type"] == "htmlPreview" {
                return node["props"]["source"]
                    .as_str()
                    .map(|s| body_text(s, "html") + "\n")
                    .unwrap_or_default();
            }
            visit(&node["content"], depth + 1)
                + if node["type"] == "link" || node["type"] == "text" {
                    ""
                } else {
                    "\n"
                }
                + &visit(&node["children"], depth + 1)
        }
        _ => String::new(),
    }
}
fn document_text(data: &Data, properties: &[Property]) -> String {
    for (kind, format) in [
        ("RICH_TEXT", "richText"),
        ("MARKDOWN", "markdown"),
        ("HTML", "html"),
    ] {
        if let Some(property) =
            properties.iter().find(|p| p.property_type == kind)
        {
            return data
                .items
                .iter()
                .find(|item| item.property_id == property.id)
                .and_then(|item| item.value[format].as_str())
                .map(|s| body_text(s, format))
                .unwrap_or_default();
        }
    }
    String::new()
}
fn public_path(org: &str, repo: &str, id: Option<&str>) -> String {
    format!(
        "/public/{}/{}{}",
        encode(org),
        encode(repo),
        id.map(|s| format!("/{}", encode(s))).unwrap_or_default()
    )
}
fn route(path: &str) -> Option<Vec<String>> {
    let raw = path
        .strip_prefix("/public/")?
        .strip_suffix('/')
        .unwrap_or(path.strip_prefix("/public/")?);
    let parts = raw.split('/').map(decode).collect::<Option<Vec<_>>>()?;
    if !(2..=3).contains(&parts.len())
        || parts.iter().any(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || s.contains(['/', '\\'])
                || s.chars().any(|c| (c as u32) < 32)
        })
    {
        return None;
    }
    Some(parts)
}
fn response(
    body: String,
    headers: Headers,
    head: bool,
    status: u16,
) -> Result<Response> {
    Ok(if head {
        Response::empty()?
    } else {
        Response::ok(body)?
    }
    .with_headers(headers)
    .with_status(status))
}
async fn shell(env: &Env, url: &Url) -> Result<Response> {
    env.service("ASSETS")?
        .fetch_request(Request::new(url.join("/")?.as_str(), Method::Get)?)
        .await
}
async fn render(
    env: &Env,
    url: &Url,
    parts: &[String],
) -> PublicResult<(String, &'static str)> {
    let org = &parts[0];
    let repo = &parts[1];
    let id = parts.get(2).map(String::as_str);
    let path = format!("{}/{}", encode(org), encode(repo));
    let base_path = public_path(org, repo, None);
    let profile: Profile = read_json(&path).await?;
    if !profile.is_public {
        return Err(404);
    }
    let origin = url.origin().ascii_serialization();
    if id == Some("sitemap.xml") {
        let page_text = url
            .query_pairs()
            .find(|(k, _)| k == "page")
            .map(|(_, v)| v.into_owned());
        let page: u32 = match &page_text {
            Some(s)
                if !s.is_empty()
                    && !s.starts_with('0')
                    && s.len() <= 6
                    && s.bytes().all(|c| c.is_ascii_digit()) =>
            {
                s.parse().map_err(|_| 404u16)?
            }
            Some(_) => return Err(404),
            None => 1,
        };
        let listing: Listing = read_json(&format!(
            "{path}/data-list?page={page}&page_size=100"
        ))
        .await?;
        let pages = listing.paginator.total_pages.max(1);
        if pages > 10000 {
            return Err(503);
        }
        if page > pages {
            return Err(404);
        }
        let loc = |p: String| {
            format!("<loc>{}</loc>", esc(&(origin.clone() + &p)))
        };
        let xml = if page_text.is_none() {
            format!("<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">{}</sitemapindex>", (1..=pages).map(|p| format!("<sitemap>{}</sitemap>", loc(format!("{base_path}/sitemap.xml?page={p}")))).collect::<String>())
        } else {
            let mut entries = if page == 1 {
                format!("<url>{}</url>", loc(base_path))
            } else {
                String::new()
            };
            for data in listing.data {
                entries += &format!(
                    "<url>{}</url>",
                    loc(public_path(org, repo, Some(&data.id)))
                );
            }
            format!("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">{entries}</urlset>")
        };
        return Ok((
            format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>{xml}"),
            "application/xml; charset=utf-8",
        ));
    }
    let site = if profile.name.is_empty() {
        &profile.username
    } else {
        &profile.name
    };
    let (title, text, listing) = if let Some(id) = id {
        let data_path = format!("{path}/data/{}", encode(id));
        let properties_path = format!("{path}/properties");
        let (data, properties): (Data, Vec<Property>) = futures::try_join!(
            read_json(&data_path),
            read_json(&properties_path)
        )?;
        if data.id != id {
            return Err(404);
        }
        let text = document_text(&data, &properties);
        (
            if data.name.is_empty() {
                site.clone()
            } else {
                data.name
            },
            text,
            None,
        )
    } else {
        let listing: Listing =
            read_json(&format!("{path}/data-list?page=1&page_size=100"))
                .await?;
        (site.clone(), profile.description.clone(), Some(listing))
    };
    let description = replace(
        if text.trim().is_empty() {
            &profile.description
        } else {
            text.trim()
        },
        r"\s+",
        " ",
    )
    .trim()
    .chars()
    .take(160)
    .collect::<String>();
    let canonical = origin.clone() + &public_path(org, repo, id);
    let seo_title = if id.is_some() {
        format!("{title} · {site}")
    } else {
        title.clone()
    };
    let mut schema = json!({"@context":"https://schema.org", "@type": if id.is_some() { "TechArticle" } else { "CollectionPage" }, "name":title, "description":description, "url":canonical, "isPartOf":{"@type":"WebSite", "name":site}});
    if id.is_some() {
        schema["headline"] = json!(title);
    }
    let mut head = String::new();
    for (key, name, content) in [
        ("name", "description", description.as_str()),
        (
            "name",
            "robots",
            if origin == PUBLIC_ORIGIN {
                "index, follow"
            } else {
                "noindex, nofollow"
            },
        ),
        ("name", "twitter:card", "summary"),
        ("name", "twitter:title", &seo_title),
        ("name", "twitter:description", &description),
        ("property", "og:title", &seo_title),
        ("property", "og:description", &description),
        (
            "property",
            "og:type",
            if id.is_some() { "article" } else { "website" },
        ),
        ("property", "og:url", &canonical),
        ("property", "og:site_name", site),
    ] {
        head += &format!(
            "<meta data-public-docs-meta {key}=\"{name}\" content=\"{}\">",
            esc(content)
        );
    }
    head += &format!("<link data-public-docs-meta rel=\"canonical\" href=\"{}\"><link data-public-docs-meta rel=\"sitemap\" type=\"application/xml\" href=\"{}/sitemap.xml\"><script type=\"application/ld+json\" data-public-docs-meta data-public-docs-schema>{}</script>", esc(&canonical), esc(&base_path), schema.to_string().replace('<', "\\u003c"));
    let nav = listing
        .map(|l| {
            format!(
                "<nav>{}</nav>",
                l.data
                    .iter()
                    .map(|d| format!(
                        "<p><a href=\"{}\">{}</a></p>",
                        esc(&public_path(org, repo, Some(&d.id))),
                        esc(&d.name)
                    ))
                    .collect::<String>()
            )
        })
        .unwrap_or_default();
    let body = format!("<main style=\"height:100%;overflow:auto;max-width:900px;margin:auto;padding:32px;background:white;color:#253047\"><a href=\"{}\">{}</a><h1>{}</h1><div style=\"white-space:pre-wrap\">{}</div>{nav}</main>", esc(&base_path), esc(site), esc(&title), esc(&text));
    let mut shell = shell(env, url).await.map_err(|_| 503u16)?;
    if !(200..300).contains(&shell.status_code()) {
        return Err(503);
    }
    let shell = String::from_utf8(
        bounded_stream(shell.stream().map_err(|_| 503u16)?, LIMIT)
            .await
            .map_err(|_| 503u16)?,
    )
    .map_err(|_| 503u16)?;
    let html = rewrite_str(
        &shell,
        RewriteStrSettings {
            element_content_handlers: vec![
                element!("title", |el| {
                    el.set_attribute("data-public-docs-title", "")?;
                    el.set_inner_content(&seo_title, ContentType::Text);
                    Ok(())
                }),
                element!("head", |el| {
                    el.append(&head, ContentType::Html);
                    Ok(())
                }),
                element!("#root", |el| {
                    el.set_inner_content(&body, ContentType::Html);
                    Ok(())
                }),
            ],
            ..RewriteStrSettings::default()
        },
    )
    .map_err(|_| 503u16)?;
    Ok((html, "text/html; charset=utf-8"))
}

#[event(fetch)]
pub async fn fetch(
    request: Request,
    env: Env,
    _ctx: Context,
) -> Result<Response> {
    let url = request.url()?;
    let head = request.method() == Method::Head;
    if url.path() == "/robots.txt" {
        let headers = Headers::new();
        headers.set("content-type", "text/plain; charset=utf-8")?;
        return response(
            if url.origin().ascii_serialization() == PUBLIC_ORIGIN {
                "User-agent: *\nAllow: /public/\nDisallow: /\n"
            } else {
                "User-agent: *\nDisallow: /\n"
            }
            .into(),
            headers,
            head,
            200,
        );
    }
    let parts = route(url.path());
    if parts.is_none() && !url.path().starts_with("/public/") {
        return env.service("ASSETS")?.fetch_request(request).await;
    }
    if parts.is_some()
        && !matches!(request.method(), Method::Get | Method::Head)
    {
        let headers = Headers::new();
        headers.set("allow", "GET, HEAD")?;
        return response(String::new(), headers, true, 405);
    }
    let headers = Headers::new();
    headers.set("cache-control", "no-store")?;
    headers.set("content-type", "text/html; charset=utf-8")?;
    if url.origin().ascii_serialization() != PUBLIC_ORIGIN {
        headers.set("x-robots-tag", "noindex, nofollow")?;
    }
    let result = match parts {
        Some(parts) => render(&env, &url, &parts).await,
        None => Err(404),
    };
    match result {
        Ok((html, content_type)) => {
            headers.set("content-type", content_type)?;
            response(html, headers, head, 200)
        }
        Err(status) => {
            headers.set("x-robots-tag", "noindex, nofollow")?;
            if status == 503 {
                headers.set("retry-after", "60")?;
                console_error!("{{\"event\":\"public_docs_unavailable\"}}");
            }
            let shell = shell(&env, &url).await?;
            if head {
                Ok(Response::empty()?
                    .with_status(status)
                    .with_headers(headers))
            } else {
                Ok(shell.with_status(status).with_headers(headers))
            }
        }
    }
}

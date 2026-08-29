//! BlockNote document -> HTML.
//!
//! A direct walk of the block tree, not a Markdown round trip -- going
//! through Markdown would drop what Markdown cannot hold. Notably an empty
//! paragraph renders as `<p><br></p>`, the form BlockNote itself preserves
//! on re-parse, and underline survives (`<u>`), which Markdown loses.

use serde_json::Value;

use super::blocks_of;

/// Render a BlockNote document to HTML.
///
/// Total like [`super::to_markdown`]: never panics, never fails; unknown
/// blocks degrade to their text instead of vanishing. The output is a
/// fragment (no `<html>` wrapper), fit for embedding in a CMS page.
pub fn to_html(document: &Value) -> String {
    render_blocks(blocks_of(document))
}

fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(character),
        }
    }
    out
}

fn escape_attribute(text: &str) -> String {
    escape_text(text).replace('"', "&quot;")
}

/// Escaped for a text node. A newline inside a run is a hard break, and
/// HTML collapses a bare one to a space, so it becomes an explicit `<br>`.
fn escape_inline(text: &str) -> String {
    escape_text(text).replace('\n', "<br>")
}

#[derive(PartialEq, Clone, Copy)]
enum ListKind {
    Bullet,
    Numbered,
    Check,
}

fn list_kind(typ: &str) -> Option<ListKind> {
    match typ {
        "bulletListItem" | "toggleListItem" => Some(ListKind::Bullet),
        "numberedListItem" => Some(ListKind::Numbered),
        "checkListItem" => Some(ListKind::Check),
        _ => None,
    }
}

fn render_blocks(blocks: &[Value]) -> String {
    let mut out = String::new();
    let mut open_list: Option<ListKind> = None;

    for block in blocks {
        let typ = block
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let kind = list_kind(typ);

        if open_list.is_some() && open_list != kind {
            close_list(&mut out, open_list.take());
        }
        if let Some(kind) = kind {
            if open_list.is_none() {
                out.push_str(match kind {
                    ListKind::Bullet | ListKind::Check => "<ul>",
                    ListKind::Numbered => "<ol>",
                });
                open_list = Some(kind);
            }
            render_list_item(&mut out, block, typ, kind);
            continue;
        }

        render_block(&mut out, block, typ);
    }

    close_list(&mut out, open_list);
    out
}

fn close_list(out: &mut String, kind: Option<ListKind>) {
    match kind {
        Some(ListKind::Bullet) | Some(ListKind::Check) => {
            out.push_str("</ul>");
        }
        Some(ListKind::Numbered) => out.push_str("</ol>"),
        None => {}
    }
}

fn render_list_item(
    out: &mut String,
    block: &Value,
    typ: &str,
    kind: ListKind,
) {
    out.push_str("<li>");
    if kind == ListKind::Check {
        let checked = block
            .get("props")
            .and_then(|props| props.get("checked"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        out.push_str(if checked {
            "<input type=\"checkbox\" checked disabled>"
        } else {
            "<input type=\"checkbox\" disabled>"
        });
    }
    let _ = typ;
    out.push_str(&render_inline(block.get("content")));
    let children = children_of(block);
    if !children.is_empty() {
        out.push_str(&render_blocks(children));
    }
    out.push_str("</li>");
}

fn children_of(block: &Value) -> &[Value] {
    block
        .get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// The image block's resize width, rounded to whole pixels.
fn prop_width(block: &Value) -> Option<u64> {
    let width = block
        .get("props")
        .and_then(|props| props.get("previewWidth"))?
        .as_f64()?;
    (width.is_finite() && width > 0.0).then(|| width.round() as u64)
}

/// Splits a `#w=<pixels>` fragment off an image URL, returning the clean
/// URL and the width it carried.
fn split_width_fragment(url: &str) -> (&str, Option<u64>) {
    let Some((src, fragment)) = url.rsplit_once("#w=") else {
        return (url, None);
    };
    match fragment.parse::<u64>() {
        Ok(width) if width > 0 => (src, Some(width)),
        _ => (url, None),
    }
}

fn prop_str<'a>(block: &'a Value, name: &str) -> Option<&'a str> {
    block
        .get("props")
        .and_then(|props| props.get(name))?
        .as_str()
}

fn render_block(out: &mut String, block: &Value, typ: &str) {
    let content = render_inline(block.get("content"));
    let children = children_of(block);

    match typ {
        "paragraph" => {
            if content.is_empty() {
                // The empty paragraph is the point of the type. <p></p> is
                // dropped by parsers (BlockNote's included); <p><br></p>
                // survives.
                out.push_str("<p><br></p>");
            } else {
                out.push_str(&format!("<p>{content}</p>"));
            }
        }
        "heading" => {
            let level = block
                .get("props")
                .and_then(|props| props.get("level"))
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .clamp(1, 6);
            out.push_str(&format!("<h{level}>{content}</h{level}>"));
        }
        "codeBlock" => {
            let language = prop_str(block, "language").unwrap_or("");
            let body = escape_text(&raw_text(block.get("content")));
            if language.is_empty() {
                out.push_str(&format!("<pre><code>{body}</code></pre>"));
            } else {
                out.push_str(&format!(
                    "<pre><code class=\"language-{}\">{body}</code></pre>",
                    escape_attribute(language)
                ));
            }
        }
        "quote" => {
            out.push_str("<blockquote>");
            if !content.is_empty() {
                out.push_str(&format!("<p>{content}</p>"));
            }
            out.push_str(&render_blocks(children));
            out.push_str("</blockquote>");
            return;
        }
        "divider" | "pageBreak" => out.push_str("<hr>"),
        "image" => {
            let url = prop_str(block, "url").unwrap_or_default();
            let caption = prop_str(block, "caption")
                .filter(|value| !value.is_empty())
                .or_else(|| prop_str(block, "name"))
                .unwrap_or_default();
            // The resize width lives either on the block (rich text) or
            // in the `#w=` URL fragment (Markdown dialect); honour both
            // and keep the fragment out of the fetched src.
            let (src, fragment_width) = split_width_fragment(url);
            let width = prop_width(block).or(fragment_width);
            match width {
                Some(width) => out.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\" width=\"{}\">",
                    escape_attribute(src),
                    escape_attribute(caption),
                    width
                )),
                None => out.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\">",
                    escape_attribute(src),
                    escape_attribute(caption)
                )),
            }
        }
        "video" | "audio" | "file" => {
            let url = prop_str(block, "url").unwrap_or_default();
            let name = prop_str(block, "name")
                .filter(|value| !value.is_empty())
                .unwrap_or(typ);
            out.push_str(&format!(
                "<a href=\"{}\">{}</a>",
                escape_attribute(url),
                escape_text(name)
            ));
        }
        "table" => render_table(out, block.get("content")),
        // Rendered live, but inside a sandboxed frame so the embedded
        // document's scripts never touch the page this fragment lands in.
        "htmlPreview" => {
            let source = prop_str(block, "source").unwrap_or_default();
            out.push_str(&format!(
                "<iframe class=\"html-preview\" sandbox=\"allow-scripts\" \
                 srcdoc=\"{}\"></iframe>",
                escape_attribute(source)
            ));
        }
        // Unknown block: keep the text rather than dropping it, same
        // policy as the Markdown renderer.
        _ => {
            if !content.is_empty() {
                out.push_str(&format!("<p>{content}</p>"));
            }
        }
    }

    if !matches!(typ, "quote") {
        let nested = render_blocks(children);
        out.push_str(&nested);
    }
}

fn render_inline(content: Option<&Value>) -> String {
    let Some(content) = content else {
        return String::new();
    };
    if let Some(text) = content.as_str() {
        return escape_inline(text);
    }
    let Some(items) = content.as_array() else {
        return String::new();
    };

    let mut out = String::new();
    for item in items {
        match item.get("type").and_then(Value::as_str) {
            Some("text") => {
                let text =
                    item.get("text").and_then(Value::as_str).unwrap_or("");
                out.push_str(&styled(text, item.get("styles")));
            }
            Some("link") => {
                let href =
                    item.get("href").and_then(Value::as_str).unwrap_or("");
                out.push_str(&format!(
                    "<a href=\"{}\">{}</a>",
                    escape_attribute(href),
                    render_inline(item.get("content"))
                ));
            }
            _ => out.push_str(&render_inline(item.get("content"))),
        }
    }
    out
}

fn styled(text: &str, styles: Option<&Value>) -> String {
    let flag = |name: &str| {
        styles
            .and_then(|value| value.get(name))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };

    let mut out = escape_inline(text);
    // Unlike Markdown, HTML can carry all of these. Colors are still
    // dropped: their values are editor theme keywords, not CSS.
    if flag("code") {
        out = format!("<code>{out}</code>");
    }
    if flag("strike") {
        out = format!("<s>{out}</s>");
    }
    if flag("underline") {
        out = format!("<u>{out}</u>");
    }
    if flag("italic") {
        out = format!("<em>{out}</em>");
    }
    if flag("bold") {
        out = format!("<strong>{out}</strong>");
    }
    out
}

/// Unescaped source text for `<pre>` bodies.
fn raw_text(content: Option<&Value>) -> String {
    let Some(content) = content else {
        return String::new();
    };
    if let Some(text) = content.as_str() {
        return text.to_string();
    }
    let Some(items) = content.as_array() else {
        return String::new();
    };
    items
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect()
}

fn render_table(out: &mut String, content: Option<&Value>) {
    let Some(rows) = content
        .and_then(|value| value.get("rows"))
        .and_then(Value::as_array)
    else {
        return;
    };
    if rows.is_empty() {
        return;
    }

    out.push_str("<table>");
    for (index, row) in rows.iter().enumerate() {
        let Some(cells) = row.get("cells").and_then(Value::as_array) else {
            continue;
        };
        let tag = if index == 0 { "th" } else { "td" };
        out.push_str("<tr>");
        for cell in cells {
            // A cell is an object wrapping its inline content; older
            // documents carry the inline array directly.
            let content = match cell.get("content") {
                Some(content) => render_inline(Some(content)),
                None => render_inline(Some(cell)),
            };
            out.push_str(&format!("<{tag}>{content}</{tag}>"));
        }
        out.push_str("</tr>");
    }
    out.push_str("</table>");
}

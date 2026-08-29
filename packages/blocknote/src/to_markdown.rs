//! BlockNote document -> Markdown.

use serde_json::Value;

use super::blocks_of;
use super::inline::render_inline;

/// Render a BlockNote document to Markdown.
///
/// Total by construction: never panics and never fails. This runs on every
/// read path -- the MCP tools, /docs, /data and the GitHub sync -- where an
/// error would be a user-visible outage, so a malformed or unrecognized
/// document degrades instead.
///
/// The result is a lossy *view*. The stored JSON is authoritative. Notably
/// an empty paragraph has no Markdown representation and is dropped;
/// preserving it is exactly why the document is stored as JSON.
///
/// The output carries no trailing newline; callers append their own.
pub fn to_markdown(document: &Value) -> String {
    render_blocks(blocks_of(document))
}

fn is_list_item(typ: &str) -> bool {
    matches!(
        typ,
        "bulletListItem"
            | "numberedListItem"
            | "checkListItem"
            | "toggleListItem"
    )
}

fn render_blocks(blocks: &[Value]) -> String {
    let mut out = String::new();
    let mut previous: Option<&str> = None;
    let mut ordinal = 0_usize;

    for block in blocks {
        let typ = block
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if typ == "numberedListItem" {
            // A `start` prop rebases the run; later items count on from it.
            match block
                .get("props")
                .and_then(|props| props.get("start"))
                .and_then(Value::as_u64)
            {
                Some(start) => ordinal = start as usize,
                None => ordinal += 1,
            }
        }
        let Some(chunk) = render_block(block, typ, ordinal) else {
            continue;
        };
        if typ != "numberedListItem" {
            ordinal = 0;
        }

        if !out.is_empty() {
            // Runs of the same list type stay tight; a blank line between
            // items would make the list loose, which renders with paragraph
            // spacing.
            let tight = previous == Some(typ) && is_list_item(typ);
            out.push_str(if tight { "\n" } else { "\n\n" });
        }
        out.push_str(&chunk);
        previous = Some(typ);
    }

    out
}

fn children_of(block: &Value) -> &[Value] {
    block
        .get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn prop_str<'a>(block: &'a Value, name: &str) -> Option<&'a str> {
    block
        .get("props")
        .and_then(|props| props.get(name))?
        .as_str()
}

fn indent(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| {
            if line.is_empty() {
                line.to_string()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_block(
    block: &Value,
    typ: &str,
    ordinal: usize,
) -> Option<String> {
    let content = render_inline(block.get("content"));
    let children = children_of(block);
    // Nested blocks have to line up past the parent's marker or CommonMark
    // reads them as a sibling list, which splits a numbered run in two.
    let mut child_indent = "  ".to_string();

    let rendered = match typ {
        "paragraph" => {
            // An empty paragraph is a real blank line in the editor but has
            // no Markdown form. Dropping it keeps the output clean rather
            // than emitting stray whitespace that round-trips badly.
            if content.is_empty() {
                None
            } else {
                Some(content)
            }
        }
        "heading" => {
            let level = block
                .get("props")
                .and_then(|props| props.get("level"))
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .clamp(1, 6) as usize;
            Some(format!("{} {content}", "#".repeat(level)))
        }
        "bulletListItem" | "toggleListItem" => Some(format!("- {content}")),
        "numberedListItem" => {
            let marker = format!("{}. ", ordinal.max(1));
            child_indent = " ".repeat(marker.chars().count());
            Some(format!("{marker}{content}"))
        }
        "checkListItem" => {
            let checked = block
                .get("props")
                .and_then(|props| props.get("checked"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let marker = if checked { "- [x] " } else { "- [ ] " };
            child_indent = " ".repeat(marker.chars().count());
            Some(format!("{marker}{content}"))
        }
        "codeBlock" => {
            let language = prop_str(block, "language").unwrap_or("");
            // Code is literal, so take the raw text rather than the escaped
            // inline rendering. Trailing newlines are dropped because the
            // closing fence supplies one -- a Markdown parser hands back
            // code content with its final newline attached, so keeping it
            // would grow the block by a line on every round trip.
            let body = raw_text(block.get("content"));
            let body = body.trim_end_matches('\n');
            Some(format!("```{language}\n{body}\n```"))
        }
        "quote" => {
            let mut quoted = content;
            let nested = render_blocks(children);
            if !nested.is_empty() {
                if !quoted.is_empty() {
                    quoted.push_str("\n\n");
                }
                quoted.push_str(&nested);
            }
            // Every line needs the marker, blank ones included: an
            // unmarked blank line ends the quote and splits it in two.
            let quoted = quoted
                .lines()
                .map(|line| {
                    if line.is_empty() {
                        ">".to_string()
                    } else {
                        format!("> {line}")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Some(quoted);
        }
        "divider" | "pageBreak" => Some("---".to_string()),
        "image" => {
            let url = prop_str(block, "url").unwrap_or_default();
            let caption = prop_str(block, "caption")
                .filter(|value| !value.is_empty())
                .or_else(|| prop_str(block, "name"))
                .unwrap_or_default();
            Some(format!("![{caption}]({url})"))
        }
        "video" | "audio" | "file" => {
            let url = prop_str(block, "url").unwrap_or_default();
            let name = prop_str(block, "name")
                .filter(|value| !value.is_empty())
                .unwrap_or(typ);
            Some(format!("[{name}]({url})"))
        }
        "table" => render_table(block.get("content")),
        // An embedded HTML document, previewed in a sandboxed frame. The
        // source is the value, so a fenced block is its faithful Markdown
        // view. The `preview` info word is what from_markdown keys on to
        // rebuild this block instead of a plain code block; the leading
        // `html` keeps GitHub's highlighter working.
        "htmlPreview" => {
            let source = prop_str(block, "source").unwrap_or_default();
            let source = source.trim_end_matches('\n');
            Some(format!("```html preview\n{source}\n```"))
        }
        // A block type this binary does not know -- most likely a plugin
        // added in a newer editor. Keep its text as a paragraph rather than
        // letting the content vanish from the GitHub sync.
        _ => {
            if content.is_empty() {
                None
            } else {
                Some(content)
            }
        }
    };

    let nested = render_blocks(children);
    match (rendered, nested.is_empty()) {
        (Some(text), true) => Some(text),
        (Some(text), false) => {
            Some(format!("{text}\n{}", indent(&nested, &child_indent)))
        }
        (None, true) => None,
        (None, false) => Some(nested),
    }
}

/// Unescaped text, for contexts such as fenced code where Markdown syntax
/// is not interpreted.
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

/// A table cell is `{ type: "tableCell", content: [...] }`; older
/// documents carry the inline array directly.
fn render_cell(cell: &Value) -> String {
    match cell.get("content") {
        Some(content) => render_inline(Some(content)),
        None => render_inline(Some(cell)),
    }
}

fn render_table(content: Option<&Value>) -> Option<String> {
    let rows = content?.get("rows")?.as_array()?;
    if rows.is_empty() {
        return None;
    }

    let cells_of = |row: &Value| -> Vec<String> {
        row.get("cells")
            .and_then(Value::as_array)
            .map(|cells| cells.iter().map(render_cell).collect())
            .unwrap_or_default()
    };

    let header = cells_of(&rows[0]);
    if header.is_empty() {
        return None;
    }

    let mut lines = vec![
        format!("| {} |", header.join(" | ")),
        format!("| {} |", vec!["---"; header.len()].join(" | ")),
    ];
    for row in &rows[1..] {
        lines.push(format!("| {} |", cells_of(row).join(" | ")));
    }
    Some(lines.join("\n"))
}

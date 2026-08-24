//! Inline content rendering shared by the Markdown serializer.

use serde_json::Value;

/// Characters that would otherwise be read as Markdown syntax.
const ESCAPED: [char; 6] = ['\\', '`', '*', '_', '[', ']'];

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        // A newline inside a run is a hard break. Emitting it bare would
        // re-parse as a space, so the line join survives one round trip and
        // then collapses; the backslash form is what CommonMark reads back
        // as a break.
        if character == '\n' {
            out.push_str("\\\n");
            continue;
        }
        if ESCAPED.contains(&character) {
            out.push('\\');
        }
        out.push(character);
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

    // `code` short-circuits: a code span is literal, so escaping inside it
    // would show the backslashes. `underline`, `textColor` and
    // `backgroundColor` are dropped -- Markdown has no equivalent, and
    // smuggling them through as raw HTML would corrupt the /docs renderer.
    if flag("code") {
        return format!("`{text}`");
    }

    let mut out = escape(text);
    if flag("strike") {
        out = format!("~~{out}~~");
    }
    if flag("bold") {
        out = format!("**{out}**");
    }
    if flag("italic") {
        out = format!("*{out}*");
    }
    out
}

/// Render a block's `content` to Markdown.
///
/// Tolerates the shorthand where `content` is a bare string, and blocks such
/// as `divider` that carry no `content` key at all.
pub fn render_inline(content: Option<&Value>) -> String {
    let Some(content) = content else {
        return String::new();
    };
    if let Some(text) = content.as_str() {
        return escape(text);
    }
    let Some(items) = content.as_array() else {
        // `table` and other structured contents are handled by the caller;
        // anything else contributes no inline text.
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
                let label = render_inline(item.get("content"));
                let href =
                    item.get("href").and_then(Value::as_str).unwrap_or("");
                out.push_str(&format!("[{label}]({href})"));
            }
            // An inline type from a newer editor: keep its text rather than
            // dropping it.
            _ => out.push_str(&render_inline(item.get("content"))),
        }
    }
    out
}

/// The document's text with no Markdown syntax.
///
/// Used where the value has to be a plain string -- search text and the
/// legacy emptiness check -- so that raw JSON keys never leak into either.
pub fn plain_text(document: &Value) -> String {
    let mut out = String::new();
    collect_plain_text(super::blocks_of(document), &mut out);
    out.trim().to_string()
}

fn collect_plain_text(blocks: &[Value], out: &mut String) {
    for block in blocks {
        collect_inline_text(block.get("content"), out);
        out.push('\n');
        if let Some(children) =
            block.get("children").and_then(Value::as_array)
        {
            collect_plain_text(children, out);
        }
    }
}

fn collect_inline_text(content: Option<&Value>, out: &mut String) {
    let Some(content) = content else { return };
    if let Some(text) = content.as_str() {
        out.push_str(text);
        return;
    }
    if let Some(items) = content.as_array() {
        for item in items {
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                out.push_str(text);
            }
            collect_inline_text(item.get("content"), out);
        }
        return;
    }
    // A wrapper such as `tableCell`, which holds its runs under `content`.
    if content.get("content").is_some() {
        collect_inline_text(content.get("content"), out);
        return;
    }
    // `tableContent` and friends: walk whatever arrays it holds.
    if let Some(rows) = content.get("rows").and_then(Value::as_array) {
        for row in rows {
            if let Some(cells) = row.get("cells").and_then(Value::as_array)
            {
                for cell in cells {
                    collect_inline_text(Some(cell), out);
                    out.push(' ');
                }
            }
        }
    }
}

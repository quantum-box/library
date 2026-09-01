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

/// Where a document walk deposits the text it finds.
///
/// The walk is shared by [`plain_text`], which keeps everything, and
/// [`plain_text_preview`], which stops once it has enough -- so a listing
/// that shows one line of a book-length body never walks the whole book.
trait TextSink {
    fn push(&mut self, text: &str);
    /// A block boundary. Sinks that collapse whitespace treat it as one.
    fn break_block(&mut self);
    /// Whether further text would be discarded, letting the walk stop.
    fn is_full(&self) -> bool {
        false
    }
}

/// Keeps every character, with blocks separated by newlines.
struct FullText(String);

impl TextSink for FullText {
    fn push(&mut self, text: &str) {
        self.0.push_str(text);
    }

    fn break_block(&mut self) {
        self.0.push('\n');
    }
}

/// The document's text with no Markdown syntax.
///
/// Used where the value has to be a plain string -- search text and the
/// legacy emptiness check -- so that raw JSON keys never leak into either.
pub fn plain_text(document: &Value) -> String {
    let mut sink = FullText(String::new());
    collect_blocks(super::blocks_of(document), &mut sink);
    sink.0.trim().to_string()
}

/// A capped, whitespace-collapsed rendering of a document's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPreview {
    /// The leading text, at most the requested number of characters.
    pub text: String,
    /// Whether the document holds more text than `text` shows.
    pub truncated: bool,
}

/// The document's leading text, capped at `limit` characters.
///
/// Runs of whitespace -- including the block boundaries that
/// [`plain_text`] renders as newlines -- collapse to a single space, so the
/// result is a one-line summary fit for a table cell or a list response.
///
/// The walk stops one character past `limit`, which is what makes
/// `truncated` exact: the cap is reached only by a character that did not
/// fit, never by a document that happens to end there.
pub fn plain_text_preview(document: &Value, limit: usize) -> TextPreview {
    let mut sink = PreviewText::new(limit);
    collect_blocks(super::blocks_of(document), &mut sink);
    sink.finish()
}

/// Collapses whitespace and stops one character past the cap.
struct PreviewText {
    out: String,
    /// Counted in characters, because `limit` is a character count and
    /// `out.len()` would cap a Japanese document three times too early.
    chars: usize,
    limit: usize,
    /// Whitespace is held back rather than written, which drops it at both
    /// ends of the preview and collapses every run in between.
    pending_space: bool,
}

impl PreviewText {
    fn new(limit: usize) -> Self {
        Self {
            out: String::new(),
            chars: 0,
            limit,
            pending_space: false,
        }
    }

    fn push_char(&mut self, character: char) {
        self.out.push(character);
        self.chars += 1;
    }

    fn finish(self) -> TextPreview {
        if self.chars > self.limit {
            let text: String = self.out.chars().take(self.limit).collect();
            return TextPreview {
                text: text.trim_end().to_string(),
                truncated: true,
            };
        }
        TextPreview {
            text: self.out,
            truncated: false,
        }
    }
}

impl TextSink for PreviewText {
    fn push(&mut self, text: &str) {
        for character in text.chars() {
            if character.is_whitespace() {
                // Leading whitespace is dropped: nothing to separate yet.
                if self.chars > 0 {
                    self.pending_space = true;
                }
                continue;
            }
            if self.is_full() {
                return;
            }
            if self.pending_space {
                self.pending_space = false;
                self.push_char(' ');
                if self.is_full() {
                    return;
                }
            }
            self.push_char(character);
        }
    }

    fn break_block(&mut self) {
        if self.chars > 0 {
            self.pending_space = true;
        }
    }

    fn is_full(&self) -> bool {
        self.chars > self.limit
    }
}

fn collect_blocks<S: TextSink>(blocks: &[Value], sink: &mut S) {
    for block in blocks {
        if sink.is_full() {
            return;
        }
        // htmlPreview keeps its document in props, not content; without
        // this a body holding only a preview would read as empty and be
        // treated as a cleared value.
        if block.get("type").and_then(Value::as_str) == Some("htmlPreview")
            && let Some(source) = block
                .get("props")
                .and_then(|props| props.get("source"))
                .and_then(Value::as_str)
        {
            sink.push(source);
        }
        collect_inline_text(block.get("content"), sink);
        sink.break_block();
        if let Some(children) =
            block.get("children").and_then(Value::as_array)
        {
            collect_blocks(children, sink);
        }
    }
}

fn collect_inline_text<S: TextSink>(content: Option<&Value>, sink: &mut S) {
    let Some(content) = content else { return };
    if sink.is_full() {
        return;
    }
    if let Some(text) = content.as_str() {
        sink.push(text);
        return;
    }
    if let Some(items) = content.as_array() {
        for item in items {
            if sink.is_full() {
                return;
            }
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                sink.push(text);
            }
            collect_inline_text(item.get("content"), sink);
        }
        return;
    }
    // A wrapper such as `tableCell`, which holds its runs under `content`.
    if content.get("content").is_some() {
        collect_inline_text(content.get("content"), sink);
        return;
    }
    // `tableContent` and friends: walk whatever arrays it holds.
    if let Some(rows) = content.get("rows").and_then(Value::as_array) {
        for row in rows {
            if let Some(cells) = row.get("cells").and_then(Value::as_array)
            {
                for cell in cells {
                    if sink.is_full() {
                        return;
                    }
                    collect_inline_text(Some(cell), sink);
                    sink.push(" ");
                }
            }
        }
    }
}

#[cfg(test)]
mod preview_tests {
    use super::*;
    use serde_json::json;

    fn paragraph(text: &str) -> Value {
        json!({
            "type": "paragraph",
            "content": [{ "type": "text", "text": text }],
        })
    }

    #[test]
    fn a_short_document_is_returned_whole_and_untruncated() {
        let document = json!([paragraph("Hello")]);

        let preview = plain_text_preview(&document, 100);

        assert_eq!(
            preview,
            TextPreview {
                text: "Hello".to_string(),
                truncated: false,
            }
        );
    }

    #[test]
    fn block_boundaries_collapse_into_single_spaces() {
        let document =
            json!([paragraph("one"), paragraph(""), paragraph("two")]);

        let preview = plain_text_preview(&document, 100);

        assert_eq!(preview.text, "one two");
        assert!(!preview.truncated);
    }

    #[test]
    fn a_document_ending_exactly_at_the_limit_is_not_truncated() {
        let document = json!([paragraph("12345")]);

        let preview = plain_text_preview(&document, 5);

        assert_eq!(preview.text, "12345");
        assert!(
            !preview.truncated,
            "the cap was reached by the document ending, not by a dropped character"
        );
    }

    #[test]
    fn one_character_past_the_limit_truncates() {
        let document = json!([paragraph("123456")]);

        let preview = plain_text_preview(&document, 5);

        assert_eq!(preview.text, "12345");
        assert!(preview.truncated);
    }

    #[test]
    fn trailing_whitespace_alone_does_not_truncate() {
        let document = json!([paragraph("12345"), paragraph("")]);

        let preview = plain_text_preview(&document, 5);

        assert_eq!(preview.text, "12345");
        assert!(!preview.truncated);
    }

    #[test]
    fn the_cap_counts_characters_not_bytes() {
        let document = json!([paragraph("あいうえお")]);

        let preview = plain_text_preview(&document, 5);

        assert_eq!(preview.text, "あいうえお");
        assert!(!preview.truncated);
    }

    #[test]
    fn a_preview_cut_mid_space_keeps_no_trailing_space() {
        let document = json!([paragraph("ab cd")]);

        let preview = plain_text_preview(&document, 3);

        assert_eq!(preview.text, "ab");
        assert!(preview.truncated);
    }

    #[test]
    fn a_zero_limit_yields_nothing_but_still_reports_the_body() {
        let document = json!([paragraph("Hello")]);

        let preview = plain_text_preview(&document, 0);

        assert_eq!(preview.text, "");
        assert!(preview.truncated);
    }

    #[test]
    fn an_empty_document_is_not_truncated() {
        let document = json!([]);

        let preview = plain_text_preview(&document, 10);

        assert_eq!(preview.text, "");
        assert!(!preview.truncated);
    }

    #[test]
    fn nested_children_contribute_to_the_preview() {
        let document = json!([{
            "type": "bulletListItem",
            "content": [{ "type": "text", "text": "parent" }],
            "children": [paragraph("child")],
        }]);

        let preview = plain_text_preview(&document, 100);

        assert_eq!(preview.text, "parent child");
    }

    #[test]
    fn the_walk_stops_early_on_a_long_document() {
        // A body far larger than the cap: the assertion is on the result,
        // but the point of the test is that `is_full` ends the walk.
        let blocks: Vec<Value> =
            (0..5_000).map(|_| paragraph("filler text")).collect();
        let document = Value::Array(blocks);

        let preview = plain_text_preview(&document, 20);

        assert_eq!(preview.text.chars().count(), 20);
        assert!(preview.truncated);
    }

    #[test]
    fn plain_text_still_keeps_the_whole_document() {
        let document = json!([paragraph("one"), paragraph("two")]);

        assert_eq!(plain_text(&document), "one\ntwo");
    }
}

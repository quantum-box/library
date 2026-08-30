//! Markdown -> BlockNote document.
//!
//! Markdown carries strictly less than the block tree does, so this
//! direction loses information by nature. What it must never do is lose
//! *text*: an earlier flat implementation dropped tables and raw HTML
//! entirely and concatenated nested list items into a single line, which
//! is why the conversion is built around an explicit container stack now.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde_json::{Map, Value, json};

/// Parse Markdown into a BlockNote document.
///
/// Emits partial blocks -- no `id`, only the props that matter -- because
/// the editor materializes ids and defaults on load.
pub fn from_markdown(markdown: &str) -> Value {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;

    let mut builder = Builder::default();
    for event in Parser::new_ext(markdown, options) {
        builder.handle(event);
    }
    builder.finish()
}

/// The block whose inline content is currently being collected.
#[derive(Clone, Copy)]
enum Leaf {
    Paragraph,
    Heading(u64),
    Code,
}

/// A container that owns child blocks: a list item, a quote, a table cell.
enum Container {
    Item {
        ordered: bool,
        start: Option<u64>,
        checked: Option<bool>,
    },
    Quote,
    TableCell,
}

struct Frame {
    container: Container,
    children: Vec<Value>,
}

#[derive(Default)]
struct Builder {
    /// Completed top-level blocks.
    blocks: Vec<Value>,
    /// Open containers, innermost last.
    frames: Vec<Frame>,
    /// Inline runs for the leaf block currently open.
    inline: Vec<Value>,
    leaf: Option<Leaf>,
    styles: Map<String, Value>,
    link: Option<(String, Vec<Value>)>,
    /// `Some(start)` per open list; `None` means unordered.
    lists: Vec<Option<u64>>,
    /// Only the first item of an ordered list carries its start number.
    list_start_pending: Option<u64>,
    checked: Option<bool>,
    code_language: String,
    /// Rows of the table being read, each a list of cell contents.
    table: Option<Vec<Vec<Value>>>,
    row: Vec<Value>,
    /// Alt text of the image being read.
    image: Option<(String, String)>,
}

impl Builder {
    fn handle(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.push_text(&text),
            // Raw HTML has no block equivalent. Keeping its source as text
            // beats dropping the content, which is what used to happen.
            //
            // A block's trailing newline is separation, not a hard break --
            // leaving it in makes the escaper add a backslash that grows by
            // one on every re-import.
            Event::Html(text) => {
                self.push_text(text.trim_end_matches('\n'))
            }
            Event::InlineHtml(text) => self.push_text(&text),
            Event::Code(text) => {
                let mut styles = Map::new();
                styles.insert("code".into(), Value::Bool(true));
                self.push_run(&text, styles);
            }
            Event::SoftBreak => self.push_text(" "),
            Event::HardBreak => self.push_text("\n"),
            Event::TaskListMarker(checked) => {
                self.checked = Some(checked);
                if let Some(Frame {
                    container: Container::Item { checked: slot, .. },
                    ..
                }) = self.frames.last_mut()
                {
                    *slot = Some(checked);
                }
            }
            Event::Rule => {
                self.flush();
                self.emit(json!({
                    "type": "divider",
                    "props": {},
                    "children": [],
                }));
            }
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.open_leaf(Leaf::Paragraph),
            Tag::Heading { level, .. } => {
                self.open_leaf(Leaf::Heading(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                }))
            }
            Tag::List(start) => {
                self.flush();
                self.lists.push(start);
                self.list_start_pending = start.filter(|start| *start != 1);
            }
            Tag::Item => {
                self.flush();
                let ordered =
                    self.lists.last().copied().flatten().is_some();
                let start = self.list_start_pending.take();
                self.frames.push(Frame {
                    container: Container::Item {
                        ordered,
                        start,
                        checked: None,
                    },
                    children: Vec::new(),
                });
                self.checked = None;
            }
            Tag::BlockQuote(_) => {
                self.flush();
                self.frames.push(Frame {
                    container: Container::Quote,
                    children: Vec::new(),
                });
            }
            Tag::CodeBlock(kind) => {
                self.flush();
                self.code_language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(language) => {
                        language.to_string()
                    }
                    pulldown_cmark::CodeBlockKind::Indented => {
                        String::new()
                    }
                };
                self.leaf = Some(Leaf::Code);
            }
            Tag::Table(_) => {
                self.flush();
                self.table = Some(Vec::new());
            }
            Tag::TableHead | Tag::TableRow => self.row = Vec::new(),
            Tag::TableCell => {
                self.flush();
                self.frames.push(Frame {
                    container: Container::TableCell,
                    children: Vec::new(),
                });
            }
            Tag::Strong => {
                self.styles.insert("bold".into(), Value::Bool(true));
            }
            Tag::Emphasis => {
                self.styles.insert("italic".into(), Value::Bool(true));
            }
            Tag::Strikethrough => {
                self.styles.insert("strike".into(), Value::Bool(true));
            }
            Tag::Link { dest_url, .. } => {
                self.link = Some((dest_url.to_string(), Vec::new()));
            }
            Tag::Image {
                dest_url, title, ..
            } => {
                // The alt text arrives as Text events inside this tag, so
                // collect it rather than reading the (usually empty) title.
                self.image =
                    Some((dest_url.to_string(), title.to_string()));
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::CodeBlock => {
                self.flush()
            }
            TagEnd::Item => self.close_item(),
            TagEnd::BlockQuote(_) => self.close_quote(),
            TagEnd::List(_) => {
                self.lists.pop();
                self.list_start_pending = None;
            }
            TagEnd::TableCell => self.close_table_cell(),
            TagEnd::TableHead | TagEnd::TableRow => {
                if let Some(table) = self.table.as_mut() {
                    table.push(std::mem::take(&mut self.row));
                }
            }
            TagEnd::Table => self.close_table(),
            TagEnd::Strong => {
                self.styles.remove("bold");
            }
            TagEnd::Emphasis => {
                self.styles.remove("italic");
            }
            TagEnd::Strikethrough => {
                self.styles.remove("strike");
            }
            TagEnd::Link => {
                if let Some((href, content)) = self.link.take() {
                    self.inline.push(json!({
                        "type": "link",
                        "href": href,
                        "content": content,
                    }));
                }
            }
            TagEnd::Image => {
                if let Some((url, caption)) = self.image.take() {
                    // Alt text landed in `inline`; prefer it over the title.
                    let alt = plain_runs(&self.inline);
                    self.inline.clear();
                    let caption =
                        if alt.is_empty() { caption } else { alt };
                    self.leaf = None;
                    // The Markdown dialect carries a resize width in a
                    // `#w=` fragment; store it as the block's own width
                    // so a rich text document needs no decoding later.
                    let (src, width) = split_width_fragment(&url);
                    let props = match width {
                        Some(width) => json!({
                            "url": src,
                            "caption": caption,
                            "previewWidth": width,
                        }),
                        None => {
                            json!({ "url": url, "caption": caption })
                        }
                    };
                    self.emit(json!({
                        "type": "image",
                        "props": props,
                        "children": [],
                    }));
                }
            }
            _ => {}
        }
    }

    fn open_leaf(&mut self, leaf: Leaf) {
        self.flush();
        self.leaf = Some(leaf);
    }

    fn push_text(&mut self, text: &str) {
        let styles = self.styles.clone();
        self.push_run(text, styles);
    }

    fn push_run(&mut self, text: &str, styles: Map<String, Value>) {
        if text.is_empty() {
            return;
        }
        // Text outside any block still belongs somewhere -- raw HTML and
        // table text arrive with no Paragraph around them.
        if self.leaf.is_none() && self.image.is_none() {
            self.leaf = Some(Leaf::Paragraph);
        }
        let run = json!({
            "type": "text",
            "text": text,
            "styles": Value::Object(styles),
        });
        match self.link.as_mut() {
            Some((_, content)) => content.push(run),
            None => self.inline.push(run),
        }
    }

    /// Put a finished block where the innermost container wants it.
    fn emit(&mut self, block: Value) {
        match self.frames.last_mut() {
            Some(frame) => frame.children.push(block),
            None => self.blocks.push(block),
        }
    }

    /// Close the leaf block under construction, if any.
    fn flush(&mut self) {
        let Some(leaf) = self.leaf.take() else {
            self.inline.clear();
            return;
        };
        let content = std::mem::take(&mut self.inline);

        let block = match leaf {
            Leaf::Paragraph => json!({
                "type": "paragraph",
                "props": {},
                "content": content,
                "children": [],
            }),
            Leaf::Heading(level) => json!({
                "type": "heading",
                "props": { "level": level },
                "content": content,
                "children": [],
            }),
            Leaf::Code => {
                let language = std::mem::take(&mut self.code_language);
                if is_html_preview(&language) {
                    // The parser hands back the body with its final newline
                    // attached; keeping it would grow the source by a line
                    // on every round trip.
                    let source = plain_runs(&content);
                    let source = source.trim_end_matches('\n');
                    json!({
                        "type": "htmlPreview",
                        "props": { "source": source },
                        "children": [],
                    })
                } else {
                    json!({
                        "type": "codeBlock",
                        "props": { "language": language },
                        "content": content,
                        "children": [],
                    })
                }
            }
        };
        self.emit(block);
    }

    fn close_item(&mut self) {
        self.flush();
        let Some(frame) = self.frames.pop() else {
            return;
        };
        let Container::Item {
            ordered,
            start,
            checked,
        } = frame.container
        else {
            return;
        };

        // A list item's own text arrives as the first paragraph inside it;
        // BlockNote holds it as the item's inline content instead.
        let mut children = frame.children;
        let content = hoist_leading_paragraph(&mut children);

        let mut props = Map::new();
        let typ = match (checked, ordered) {
            (Some(checked), _) => {
                props.insert("checked".into(), Value::Bool(checked));
                "checkListItem"
            }
            (None, true) => {
                if let Some(start) = start {
                    props.insert("start".into(), json!(start));
                }
                "numberedListItem"
            }
            (None, false) => "bulletListItem",
        };

        self.emit(json!({
            "type": typ,
            "props": Value::Object(props),
            "content": content,
            "children": children,
        }));
    }

    fn close_quote(&mut self) {
        self.flush();
        let Some(frame) = self.frames.pop() else {
            return;
        };
        let mut children = frame.children;
        let content = hoist_leading_paragraph(&mut children);
        self.emit(json!({
            "type": "quote",
            "props": {},
            "content": content,
            "children": children,
        }));
    }

    fn close_table_cell(&mut self) {
        self.flush();
        let Some(frame) = self.frames.pop() else {
            return;
        };
        let mut children = frame.children;
        let content = hoist_leading_paragraph(&mut children);
        self.row.push(json!({
            "type": "tableCell",
            "content": content,
            "props": {},
        }));
    }

    fn close_table(&mut self) {
        let Some(rows) = self.table.take() else {
            return;
        };
        if rows.is_empty() {
            return;
        }
        let rows: Vec<Value> = rows
            .into_iter()
            .map(|cells| json!({ "cells": cells }))
            .collect();
        self.emit(json!({
            "type": "table",
            "props": {},
            "content": { "type": "tableContent", "rows": rows },
            "children": [],
        }));
    }

    fn finish(mut self) -> Value {
        self.flush();
        while !self.frames.is_empty() {
            // Unbalanced markdown; close what is open so nothing is lost.
            match self.frames.last().map(|frame| &frame.container) {
                Some(Container::Item { .. }) => self.close_item(),
                Some(Container::Quote) => self.close_quote(),
                Some(Container::TableCell) => self.close_table_cell(),
                None => break,
            }
        }
        if self.blocks.is_empty() {
            // An empty array is the "clear this property" sentinel, so an
            // empty document has to be one empty paragraph instead.
            return json!([{
                "type": "paragraph",
                "props": {},
                "content": [],
                "children": [],
            }]);
        }
        Value::Array(self.blocks)
    }
}

/// Take the leading paragraph's inline content, leaving the rest as
/// children. Containers in BlockNote carry inline content of their own.
fn hoist_leading_paragraph(children: &mut Vec<Value>) -> Value {
    let is_paragraph = children
        .first()
        .and_then(|block| block.get("type"))
        .and_then(Value::as_str)
        == Some("paragraph");
    if !is_paragraph {
        return json!([]);
    }
    let first = children.remove(0);
    first.get("content").cloned().unwrap_or_else(|| json!([]))
}

/// A fence whose info string marks an htmlPreview block, the form
/// `to_markdown` writes: `html` first so highlighters still work, plus the
/// word `preview`. A plain `html` fence stays a code block.
fn is_html_preview(info: &str) -> bool {
    let mut words = info.split_whitespace();
    words.next() == Some("html") && words.any(|word| word == "preview")
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

fn plain_runs(runs: &[Value]) -> String {
    runs.iter()
        .filter_map(|run| run.get("text").and_then(Value::as_str))
        .collect()
}

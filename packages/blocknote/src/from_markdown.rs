//! Markdown -> BlockNote document.
//!
//! Deliberately a common subset. Markdown carries strictly less than the
//! block tree does, so this direction is lossy by nature; it exists so that
//! Markdown arriving from GitHub can be opened in the editor at all.

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

#[derive(Default)]
struct Builder {
    blocks: Vec<Value>,
    /// Inline runs accumulated for the block currently being built.
    inline: Vec<Value>,
    styles: Map<String, Value>,
    link: Option<(String, Vec<Value>)>,
    /// The block type to emit when the current container closes.
    pending: Option<PendingBlock>,
    list_kinds: Vec<Option<u64>>,
    checked: Option<bool>,
    in_code: bool,
    code_language: String,
    quote_depth: usize,
}

#[derive(Clone)]
enum PendingBlock {
    Paragraph,
    Heading(usize),
    ListItem,
    Code,
}

impl Builder {
    fn handle(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) | Event::Html(text) => {
                self.push_text(&text);
            }
            Event::Code(text) => {
                let mut styles = Map::new();
                styles.insert("code".into(), Value::Bool(true));
                self.push_run(&text, styles);
            }
            Event::SoftBreak | Event::HardBreak => self.push_text("\n"),
            Event::TaskListMarker(checked) => self.checked = Some(checked),
            Event::Rule => {
                self.flush();
                self.blocks.push(json!({
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
            Tag::Paragraph => self.pending = Some(PendingBlock::Paragraph),
            Tag::Heading { level, .. } => {
                self.pending = Some(PendingBlock::Heading(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                }));
            }
            Tag::List(start) => self.list_kinds.push(start),
            Tag::Item => {
                self.pending = Some(PendingBlock::ListItem);
                self.checked = None;
            }
            Tag::CodeBlock(kind) => {
                self.flush();
                self.in_code = true;
                self.code_language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(language) => {
                        language.to_string()
                    }
                    pulldown_cmark::CodeBlockKind::Indented => {
                        String::new()
                    }
                };
                self.pending = Some(PendingBlock::Code);
            }
            Tag::BlockQuote(_) => self.quote_depth += 1,
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
                self.flush();
                self.blocks.push(json!({
                    "type": "image",
                    "props": {
                        "url": dest_url.to_string(),
                        "caption": title.to_string(),
                    },
                    "children": [],
                }));
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::Item
            | TagEnd::CodeBlock => self.flush(),
            TagEnd::List(_) => {
                self.list_kinds.pop();
            }
            TagEnd::BlockQuote(_) => {
                self.quote_depth = self.quote_depth.saturating_sub(1);
            }
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
            _ => {}
        }
    }

    fn push_text(&mut self, text: &str) {
        let styles = self.styles.clone();
        self.push_run(text, styles);
    }

    fn push_run(&mut self, text: &str, styles: Map<String, Value>) {
        if text.is_empty() {
            return;
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

    /// Emit the block under construction, if any.
    fn flush(&mut self) {
        let Some(pending) = self.pending.take() else {
            self.inline.clear();
            return;
        };
        let content = std::mem::take(&mut self.inline);

        let block = match pending {
            PendingBlock::Paragraph => json!({
                "type": "paragraph",
                "props": {},
                "content": content,
                "children": [],
            }),
            PendingBlock::Heading(level) => json!({
                "type": "heading",
                "props": { "level": level },
                "content": content,
                "children": [],
            }),
            PendingBlock::ListItem => self.list_item(content),
            PendingBlock::Code => {
                self.in_code = false;
                let language = std::mem::take(&mut self.code_language);
                json!({
                    "type": "codeBlock",
                    "props": { "language": language },
                    "content": content,
                    "children": [],
                })
            }
        };

        // A quote in Markdown wraps blocks; the editor's quote holds inline
        // content, so nest the rendered block underneath one.
        if self.quote_depth > 0 {
            self.blocks.push(json!({
                "type": "quote",
                "props": {},
                "content": block
                    .get("content")
                    .cloned()
                    .unwrap_or_else(|| json!([])),
                "children": [],
            }));
            return;
        }
        self.blocks.push(block);
    }

    fn list_item(&mut self, content: Vec<Value>) -> Value {
        if let Some(checked) = self.checked.take() {
            return json!({
                "type": "checkListItem",
                "props": { "checked": checked },
                "content": content,
                "children": [],
            });
        }
        match self.list_kinds.last().copied().flatten() {
            Some(_) => json!({
                "type": "numberedListItem",
                "props": {},
                "content": content,
                "children": [],
            }),
            None => json!({
                "type": "bulletListItem",
                "props": {},
                "content": content,
                "children": [],
            }),
        }
    }

    fn finish(mut self) -> Value {
        self.flush();
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

//! What survives a Markdown round trip.
//!
//! These exist because the first implementation lost text silently: tables
//! and raw HTML converted to nothing, and nested list items came back
//! concatenated into one line ("- outerinnerdeeper"). None of that was
//! visible from the golden fixtures, which only covered documents the
//! editor itself produced.
//!
//! Two properties are asserted:
//!   - text is never lost, whatever else the conversion gives up
//!   - the conversion is idempotent, so importing twice does not keep
//!     rewriting a document

use blocknote::{from_markdown, plain_text, to_markdown};

/// Markdown that survives a round trip unchanged.
const EXACT: &[(&str, &str)] = &[
    ("paragraphs", "one\n\ntwo"),
    ("heading levels", "# h1\n\n## h2\n\n### h3"),
    ("bullet list", "- a\n- b"),
    ("nested list", "- outer\n  - inner\n    - deeper"),
    ("numbered list", "1. one\n2. two"),
    ("numbered start", "5. five\n6. six"),
    ("task list", "- [x] done\n- [ ] todo"),
    ("table", "| a | b |\n| --- | --- |\n| 1 | 2 |"),
    (
        "table with more rows",
        "| h1 | h2 |\n| --- | --- |\n| a | b |\n| c | d |",
    ),
    ("fenced code", "```rust\nfn main() {}\n```"),
    ("bare code", "```\nplain\n```"),
    ("emphasis", "**bold** and *italic* and ~~struck~~"),
    ("inline code", "a `span` here"),
    ("link", "[label](https://example.com)"),
    ("image", "![cap](https://example.com/a.png)"),
    ("divider", "before\n\n---\n\nafter"),
    ("hard break", "line1\\\nline2"),
    ("raw html", "<div class=\"x\">hi</div>"),
    (
        "document",
        "# H\n\ntext **bold**\n\n| a |\n| --- |\n| 1 |\n\nafter",
    ),
];

#[test]
fn representative_markdown_round_trips_unchanged() {
    for (name, markdown) in EXACT {
        let rendered = to_markdown(&from_markdown(markdown));
        assert_eq!(
            &rendered, markdown,
            "{name} did not round trip unchanged"
        );
    }
}

/// Markdown that is allowed to change shape, as long as the words survive.
const NORMALIZED: &[(&str, &str)] = &[
    ("loose list", "- one\n\n- two"),
    ("quote", "> one\n>\n> two"),
    ("setext heading", "Title\n====="),
    ("indented code", "    indented\n"),
    ("footnote", "text[^1]\n\n[^1]: note"),
    ("html comment", "<!-- note -->\n\nvisible"),
    ("autolink", "<https://example.com>"),
    ("nested quote", "> outer\n>\n> > inner"),
    ("mixed nesting", "1. one\n   - sub\n2. two"),
];

#[test]
fn conversion_is_idempotent() {
    for (name, markdown) in EXACT.iter().chain(NORMALIZED) {
        let once = to_markdown(&from_markdown(markdown));
        let twice = to_markdown(&from_markdown(&once));
        assert_eq!(once, twice, "{name} keeps changing on re-import");
    }
}

#[test]
fn no_word_is_dropped_by_the_conversion() {
    // The failure that motivated this file was silent: the output was
    // shorter, not wrong-looking. Checking that every word survives catches
    // a dropped block even when the surrounding syntax changes.
    let cases: &[(&str, &[&str])] = &[
        ("| a | b |\n| --- | --- |\n| 1 | 2 |", &["a", "b", "1", "2"]),
        (
            "- outer\n  - inner\n    - deeper",
            &["outer", "inner", "deeper"],
        ),
        ("<div>kept</div>", &["kept"]),
        ("> quoted\n\npara", &["quoted", "para"]),
        (
            "# Title\n\n| x |\n| --- |\n| cell |\n\ntail",
            &["Title", "x", "cell", "tail"],
        ),
        ("1. one\n   - sub\n2. two", &["one", "sub", "two"]),
    ];

    for (markdown, words) in cases {
        let document = from_markdown(markdown);
        let text = plain_text(&document);
        for word in *words {
            assert!(
                text.contains(word),
                "{word:?} was dropped converting {markdown:?} (got {text:?})"
            );
        }
    }
}

#[test]
fn nested_list_items_stay_separate() {
    // The regression this file is named for: three items came back as
    // "outerinnerdeeper", a single run of concatenated words.
    let document = from_markdown("- outer\n  - inner\n    - deeper");
    let blocks = document.as_array().expect("array of blocks");

    assert_eq!(
        blocks.len(),
        1,
        "the outer item is the only top-level block"
    );
    let outer = &blocks[0];
    assert_eq!(outer["type"], "bulletListItem");
    assert_eq!(outer["content"][0]["text"], "outer");

    let inner = &outer["children"][0];
    assert_eq!(inner["content"][0]["text"], "inner");
    assert_eq!(inner["children"][0]["content"][0]["text"], "deeper");
}

#[test]
fn a_table_becomes_a_table_block_not_a_paragraph() {
    let document = from_markdown("| a | b |\n| --- | --- |\n| 1 | 2 |");
    let blocks = document.as_array().expect("array of blocks");

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0]["type"], "table");
    let rows = blocks[0]["content"]["rows"].as_array().expect("table rows");
    assert_eq!(rows.len(), 2, "header row plus one body row");
    assert_eq!(rows[0]["cells"][0]["content"][0]["text"], "a");
    assert_eq!(rows[1]["cells"][1]["content"][0]["text"], "2");
}

#[test]
fn html_carries_what_markdown_cannot() {
    use blocknote::to_html;

    // Each of these is a case Markdown gives up on. The HTML view exists
    // precisely so CMS output does not have to.
    let document = from_markdown("| a | b |\n| --- | --- |\n| 1 | 2 |");
    let html = to_html(&document);
    assert!(html.contains("<table>"), "table lost in html: {html}");
    assert!(html.contains("<th>a</th>"), "header cell lost: {html}");
    assert!(html.contains("<td>2</td>"), "body cell lost: {html}");

    let document = serde_json::json!([
        { "type": "paragraph",
          "content": [{ "type": "text", "text": "u",
                        "styles": { "underline": true } }] },
        { "type": "paragraph", "content": [] },
        { "type": "paragraph",
          "content": [{ "type": "text", "text": "a\nb", "styles": {} }] },
    ]);
    let html = to_html(&document);
    // Underline and the empty paragraph both survive here and both are
    // dropped by the markdown view.
    assert!(html.contains("<u>u</u>"), "underline lost: {html}");
    assert!(html.contains("<p><br></p>"), "empty paragraph lost: {html}");
    assert!(html.contains("a<br>b"), "hard break lost: {html}");
}

#[test]
fn nesting_survives_three_levels_in_html() {
    let html = blocknote::to_html(&from_markdown(
        "- outer\n  - inner\n    - deeper",
    ));
    assert_eq!(
        html,
        "<ul><li>outer<ul><li>inner<ul><li>deeper</li></ul></li></ul></li></ul>"
    );
}

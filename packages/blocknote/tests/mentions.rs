use blocknote::{plain_text, plain_text_preview, to_html, to_markdown};
use serde_json::json;

#[test]
fn mentions_survive_all_read_formats() {
    let document = json!([{"type": "paragraph", "content": [
        {"type": "userMention", "props": {"userId": "u1", "name": "山田 <太郎>"}}
    ]}]);
    assert_eq!(plain_text(&document), "@山田 <太郎>");
    assert_eq!(plain_text_preview(&document, 100).text, "@山田 <太郎>");
    assert!(to_markdown(&document).contains("@山田 <太郎>"));
    assert!(to_html(&document).contains("@山田 &lt;太郎&gt;"));
}

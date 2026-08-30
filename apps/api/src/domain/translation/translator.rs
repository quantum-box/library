//! The translation port and the rules for talking to a model.
//!
//! Everything here is pure: prompt construction and response parsing
//! are the two places a cheap model most often goes wrong, so they are
//! kept out of the network adapter where they would be untestable.
//!
//! The pipeline deliberately runs on a small, inexpensive model. That
//! choice buys enough headroom to translate eagerly, and it is paid for
//! with strictness: the model is asked for a fixed JSON shape, and
//! anything that does not come back in that shape is rejected rather
//! than patched up. A cheap model will happily prepend "Here is the
//! translation:" or drop an entry, and neither may reach a reader.

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use async_trait::async_trait;
use serde::Deserialize;

use super::LanguageTag;

/// A term whose translation is fixed by the repo owner.
///
/// Small models are markedly worse than large ones at inferring that a
/// word is a product name rather than a noun, so the glossary is not a
/// nicety here -- it is what keeps a repo's vocabulary stable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryEntry {
    pub term: String,
    pub translation: String,
}

/// One string to translate, identified so the response can be matched
/// back without relying on ordering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationItem {
    pub id: String,
    pub text: String,
}

/// A batch of strings sharing a target language.
#[derive(Debug, Clone)]
pub struct TranslationBatch<'a> {
    /// `None` when detection could not settle the source language; the
    /// model is then asked to infer it.
    pub source_lang: Option<&'a LanguageTag>,
    pub target_lang: &'a LanguageTag,
    pub glossary: &'a [GlossaryEntry],
    pub items: &'a [TranslationItem],
}

/// Translates batches of short strings.
#[async_trait]
pub trait Translator: Send + Sync + Debug {
    /// Returns a translation for every item, keyed by item id.
    ///
    /// Implementations must not return partial results: a batch either
    /// yields every id or fails.
    async fn translate(
        &self,
        batch: TranslationBatch<'_>,
    ) -> errors::Result<HashMap<String, String>>;
}

/// A translation longer than this multiple of its source is treated as
/// the model having added commentary rather than translated.
///
/// Schema labels are short, and expansion between languages is real but
/// bounded; a tenfold label is prose that escaped the format.
const MAX_LENGTH_RATIO: usize = 10;

/// Short strings expand proportionally more, so ratio checks only apply
/// once there is enough source text for the ratio to mean anything.
const MIN_LENGTH_FOR_RATIO_CHECK: usize = 8;

/// The instruction sent as the system message.
pub fn system_prompt() -> String {
    [
        "You translate short user-interface labels for a structured \
         data application.",
        "Reply with JSON only, in exactly this shape: \
         {\"translations\":[{\"id\":\"...\",\"text\":\"...\"}]}.",
        "Include every id you were given, exactly once, and no other \
         id.",
        "Translate only the label text. Do not add notes, quotation \
         marks, explanations, or alternatives.",
        "Preserve capitalization style and any surrounding punctuation.",
        "If a label is a proper noun, an identifier, or a code, leave \
         it unchanged.",
    ]
    .join(" ")
}

/// Builds the user message for one batch.
pub fn build_translation_prompt(batch: &TranslationBatch<'_>) -> String {
    let mut prompt = String::new();

    match batch.source_lang {
        Some(source) => prompt.push_str(&format!(
            "Translate from {} to {}.\n",
            source.as_str(),
            batch.target_lang.as_str()
        )),
        None => prompt.push_str(&format!(
            "Translate to {}. Infer the source language from the \
             text.\n",
            batch.target_lang.as_str()
        )),
    }

    if !batch.glossary.is_empty() {
        prompt.push_str(
            "\nUse these translations exactly wherever the term \
             appears:\n",
        );
        for entry in batch.glossary {
            prompt.push_str(&format!(
                "- {} => {}\n",
                entry.term, entry.translation
            ));
        }
    }

    prompt.push_str("\nLabels:\n");
    // Serialized as JSON so that a label containing a newline or a
    // colon cannot be mistaken for part of the surrounding format.
    for item in batch.items {
        prompt.push_str(&format!(
            "{}\n",
            serde_json::json!({ "id": item.id, "text": item.text })
        ));
    }

    prompt
}

#[derive(Debug, Deserialize)]
struct TranslationEnvelope {
    translations: Vec<TranslationEntry>,
}

#[derive(Debug, Deserialize)]
struct TranslationEntry {
    id: String,
    text: String,
}

/// Parses and validates a model response against the batch it answers.
///
/// Rejects the response outright on anything unexpected. Salvaging a
/// partial answer would publish a half-translated schema, which reads
/// as a bug to every visitor; failing leaves the source text in place,
/// which merely reads as untranslated.
pub fn parse_translation_response(
    raw: &str,
    items: &[TranslationItem],
) -> errors::Result<HashMap<String, String>> {
    let body = strip_code_fence(raw.trim());

    let envelope: TranslationEnvelope = serde_json::from_str(body)
        .map_err(|error| {
            errors::Error::internal_server_error(format!(
                "translation response was not the expected JSON: {error}"
            ))
        })?;

    let expected: HashMap<&str, &TranslationItem> =
        items.iter().map(|item| (item.id.as_str(), item)).collect();

    let mut seen: HashSet<String> = HashSet::new();
    let mut translated = HashMap::new();

    for entry in envelope.translations {
        let Some(source) = expected.get(entry.id.as_str()) else {
            return Err(errors::Error::internal_server_error(format!(
                "translation response contained unknown id `{}`",
                entry.id
            )));
        };

        if !seen.insert(entry.id.clone()) {
            return Err(errors::Error::internal_server_error(format!(
                "translation response repeated id `{}`",
                entry.id
            )));
        }

        let text = entry.text.trim();
        if text.is_empty() {
            return Err(errors::Error::internal_server_error(format!(
                "translation for `{}` was empty",
                entry.id
            )));
        }

        if source.text.chars().count() >= MIN_LENGTH_FOR_RATIO_CHECK
            && text.chars().count()
                > source.text.chars().count() * MAX_LENGTH_RATIO
        {
            return Err(errors::Error::internal_server_error(format!(
                "translation for `{}` is {} characters against a \
                 {}-character source, which reads as commentary rather \
                 than a label",
                entry.id,
                text.chars().count(),
                source.text.chars().count()
            )));
        }

        translated.insert(entry.id.clone(), text.to_string());
    }

    if translated.len() != items.len() {
        let missing: Vec<&str> = items
            .iter()
            .map(|item| item.id.as_str())
            .filter(|id| !translated.contains_key(*id))
            .collect();
        return Err(errors::Error::internal_server_error(format!(
            "translation response omitted {} of {} labels: {}",
            missing.len(),
            items.len(),
            missing.join(", ")
        )));
    }

    Ok(translated)
}

/// Removes a ```json fence if the model wrapped its JSON in one.
///
/// Asking for JSON only does not reliably stop a small model from
/// formatting it as a code block, and that is a presentation quirk
/// rather than a wrong answer.
fn strip_code_fence(body: &str) -> &str {
    let Some(rest) = body.strip_prefix("```") else {
        return body;
    };
    let rest = rest.strip_prefix("json").unwrap_or(rest);
    let rest = rest.trim_start_matches(['\n', '\r']);
    rest.strip_suffix("```").unwrap_or(rest).trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<TranslationItem> {
        vec![
            TranslationItem {
                id: "fl_1".into(),
                text: "担当者".into(),
            },
            TranslationItem {
                id: "op_1".into(),
                text: "対応中".into(),
            },
        ]
    }

    fn tag(value: &str) -> LanguageTag {
        LanguageTag::new(value).unwrap()
    }

    #[test]
    fn accepts_a_well_formed_response() {
        let raw = r#"{"translations":[
            {"id":"fl_1","text":"Assignee"},
            {"id":"op_1","text":"In progress"}
        ]}"#;
        let parsed = parse_translation_response(raw, &items()).unwrap();
        assert_eq!(parsed["fl_1"], "Assignee");
        assert_eq!(parsed["op_1"], "In progress");
    }

    #[test]
    fn accepts_json_wrapped_in_a_code_fence() {
        let raw = "```json\n{\"translations\":[{\"id\":\"fl_1\",\"text\":\"Assignee\"},{\"id\":\"op_1\",\"text\":\"In progress\"}]}\n```";
        let parsed = parse_translation_response(raw, &items()).unwrap();
        assert_eq!(parsed["fl_1"], "Assignee");
    }

    #[test]
    fn rejects_a_response_that_omits_a_label() {
        // Half a translated schema is worse than none: the reader sees
        // a bug rather than an untranslated field.
        let raw = r#"{"translations":[{"id":"fl_1","text":"Assignee"}]}"#;
        let error = parse_translation_response(raw, &items()).unwrap_err();
        assert!(error.to_string().contains("op_1"));
    }

    #[test]
    fn rejects_an_id_that_was_never_sent() {
        let raw = r#"{"translations":[
            {"id":"fl_1","text":"Assignee"},
            {"id":"op_1","text":"In progress"},
            {"id":"fl_9","text":"Invented"}
        ]}"#;
        assert!(parse_translation_response(raw, &items()).is_err());
    }

    #[test]
    fn rejects_a_repeated_id() {
        let raw = r#"{"translations":[
            {"id":"fl_1","text":"Assignee"},
            {"id":"fl_1","text":"Owner"},
            {"id":"op_1","text":"In progress"}
        ]}"#;
        assert!(parse_translation_response(raw, &items()).is_err());
    }

    #[test]
    fn rejects_an_empty_translation() {
        let raw = r#"{"translations":[
            {"id":"fl_1","text":"   "},
            {"id":"op_1","text":"In progress"}
        ]}"#;
        assert!(parse_translation_response(raw, &items()).is_err());
    }

    #[test]
    fn rejects_prose_that_escaped_the_label_format() {
        // The classic small-model failure: a helpful preamble.
        let long = "Here is the translation of the label you provided: \
                    Assignee. Let me know if you need anything else!";
        let raw = format!(
            r#"{{"translations":[
                {{"id":"fl_1","text":"{long}"}},
                {{"id":"op_1","text":"In progress"}}
            ]}}"#
        );
        let sources = vec![
            TranslationItem {
                id: "fl_1".into(),
                text: "担当者のフルネーム".into(),
            },
            TranslationItem {
                id: "op_1".into(),
                text: "対応中".into(),
            },
        ];
        assert!(parse_translation_response(&raw, &sources).is_err());
    }

    #[test]
    fn allows_normal_expansion_on_a_short_label() {
        // Two characters of Japanese becoming a longer English word is
        // ordinary, so the ratio check must not fire on short sources.
        let sources = vec![TranslationItem {
            id: "fl_1".into(),
            text: "状態".into(),
        }];
        let raw = r#"{"translations":[{"id":"fl_1","text":"Status"}]}"#;
        assert!(parse_translation_response(raw, &sources).is_ok());
    }

    #[test]
    fn rejects_a_response_that_is_not_json_at_all() {
        assert!(parse_translation_response(
            "I cannot help with that.",
            &items()
        )
        .is_err());
    }

    #[test]
    fn the_prompt_names_both_languages_and_every_label() {
        let target = tag("en");
        let source = tag("ja");
        let items = items();
        let prompt = build_translation_prompt(&TranslationBatch {
            source_lang: Some(&source),
            target_lang: &target,
            glossary: &[],
            items: &items,
        });
        assert!(prompt.contains("from ja to en"));
        assert!(prompt.contains("担当者"));
        assert!(prompt.contains("op_1"));
    }

    #[test]
    fn the_prompt_asks_the_model_to_infer_an_unknown_source() {
        let target = tag("en");
        let items = items();
        let prompt = build_translation_prompt(&TranslationBatch {
            source_lang: None,
            target_lang: &target,
            glossary: &[],
            items: &items,
        });
        assert!(prompt.contains("Infer the source language"));
    }

    #[test]
    fn the_prompt_carries_the_glossary_when_there_is_one() {
        let target = tag("en");
        let items = items();
        let glossary = vec![GlossaryEntry {
            term: "Library".into(),
            translation: "Library".into(),
        }];
        let prompt = build_translation_prompt(&TranslationBatch {
            source_lang: None,
            target_lang: &target,
            glossary: &glossary,
            items: &items,
        });
        assert!(prompt.contains("Library => Library"));
    }

    #[test]
    fn a_label_containing_json_syntax_cannot_break_the_prompt() {
        let target = tag("en");
        let items = vec![TranslationItem {
            id: "fl_1".into(),
            text: "\"quoted\": {value}\nsecond line".into(),
        }];
        let prompt = build_translation_prompt(&TranslationBatch {
            source_lang: None,
            target_lang: &target,
            glossary: &[],
            items: &items,
        });
        // The label is emitted as escaped JSON, so its newline does not
        // become a new line of the prompt format.
        assert!(prompt.contains("\\nsecond line"));
    }
}

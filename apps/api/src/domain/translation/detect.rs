//! Source-language detection.
//!
//! The pipeline needs to know what language a record was written in
//! before it can decide whether a translation is required at all.
//! Calling a model for that on every write would be absurd: for the
//! languages Library actually mixes, the writing system settles the
//! question outright. Kana appear in Japanese and nowhere else; Hangul
//! in Korean and nowhere else; unaccompanied Han characters in Chinese.
//!
//! Scripts that several languages share -- the Latin alphabet above all
//! -- are deliberately reported as undetermined rather than guessed at.
//! A French document must not be labelled English, so the caller
//! decides what to do with `None`.

use super::LanguageTag;

/// A kana character has to make up at least this share of the CJK text
/// before the document is called Japanese.
///
/// A Chinese document quoting a Japanese title picks up a few kana
/// without ceasing to be Chinese; a Japanese document is never
/// kana-free for long.
const KANA_SHARE_FOR_JAPANESE: f32 = 0.05;

/// Scripts are only conclusive once there is enough text to look at.
const MIN_SIGNIFICANT_CHARS: usize = 4;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ScriptCounts {
    kana: usize,
    han: usize,
    hangul: usize,
    cyrillic: usize,
    latin: usize,
}

impl ScriptCounts {
    fn total(&self) -> usize {
        self.kana + self.han + self.hangul + self.cyrillic + self.latin
    }
}

/// Detects the language a piece of user text is written in.
///
/// Returns `None` when the writing system does not identify a single
/// language -- an empty string, a fragment too short to judge, or text
/// in a script that many languages share.
pub fn detect_source_language(text: &str) -> Option<LanguageTag> {
    let counts = count_scripts(text);

    if counts.total() < MIN_SIGNIFICANT_CHARS {
        return None;
    }

    // Hangul is unique to Korean, but a Korean text also carries Han
    // characters, so compare the two rather than testing for presence.
    if counts.hangul > 0 && counts.hangul >= counts.han {
        return tag("ko");
    }

    let cjk = counts.kana + counts.han;
    if cjk > 0 {
        let kana_share = counts.kana as f32 / cjk as f32;
        if kana_share >= KANA_SHARE_FOR_JAPANESE {
            return tag("ja");
        }
        if counts.han > 0 {
            return tag("zh");
        }
    }

    if counts.cyrillic > counts.latin {
        return tag("ru");
    }

    // Latin-script text: shared by too many languages to call.
    None
}

fn count_scripts(text: &str) -> ScriptCounts {
    let mut counts = ScriptCounts::default();

    for character in text.chars() {
        match character as u32 {
            // Hiragana and Katakana, plus halfwidth katakana.
            0x3040..=0x309F | 0x30A0..=0x30FF | 0xFF66..=0xFF9D => {
                counts.kana += 1
            }
            // CJK unified ideographs and extension A.
            0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF => {
                counts.han += 1
            }
            // Hangul syllables, jamo and compatibility jamo.
            0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF => {
                counts.hangul += 1
            }
            0x0400..=0x04FF => counts.cyrillic += 1,
            // Everything else that is a letter is treated as Latin
            // script, accented forms included. Punctuation, digits and
            // whitespace are ignored so that Markdown syntax cannot
            // outvote the prose.
            _ if character.is_alphabetic() => counts.latin += 1,
            _ => {}
        }
    }

    counts
}

fn tag(value: &str) -> Option<LanguageTag> {
    // These literals are valid by construction; a failure here is a
    // programming error rather than bad input.
    LanguageTag::new(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detected(text: &str) -> Option<String> {
        detect_source_language(text).map(|tag| tag.to_string())
    }

    #[test]
    fn kana_identifies_japanese() {
        assert_eq!(
            detected("これは日本語のドキュメントです"),
            Some("ja".into())
        );
        assert_eq!(detected("ひらがなだけ"), Some("ja".into()));
        assert_eq!(detected("カタカナダケ"), Some("ja".into()));
    }

    #[test]
    fn han_without_kana_identifies_chinese() {
        assert_eq!(detected("这是一个中文文档"), Some("zh".into()));
        assert_eq!(detected("這是一個中文文件"), Some("zh".into()));
    }

    #[test]
    fn a_stray_kana_does_not_make_a_chinese_document_japanese() {
        // One kana in a long run of Han stays below the share needed to
        // flip the verdict.
        let chinese_quoting_japanese =
            "这是一个很长的中文文档我们在这里引用了一个日文标题の";
        assert_eq!(detected(chinese_quoting_japanese), Some("zh".into()));
    }

    #[test]
    fn hangul_identifies_korean() {
        assert_eq!(detected("이것은 한국어 문서입니다"), Some("ko".into()));
    }

    #[test]
    fn cyrillic_identifies_russian() {
        assert_eq!(detected("это русский документ"), Some("ru".into()));
    }

    #[test]
    fn latin_script_is_undetermined_rather_than_guessed() {
        // Calling any of these English would mislabel two of them.
        assert_eq!(detected("This is an English document"), None);
        assert_eq!(detected("Ceci est un document francais"), None);
        assert_eq!(detected("Dies ist ein deutsches Dokument"), None);
    }

    #[test]
    fn text_too_short_to_judge_is_undetermined() {
        assert_eq!(detected(""), None);
        assert_eq!(detected("   "), None);
        assert_eq!(detected("# 1. -- []()"), None);
        assert_eq!(detected("あ"), None);
    }

    #[test]
    fn markdown_punctuation_and_code_do_not_count_as_letters() {
        // The Japanese prose decides it, not the surrounding syntax.
        let markdown = "# 見出し\n\n```rust\nlet x = 1;\n```\n\n本文です\n";
        assert_eq!(detected(markdown), Some("ja".into()));
    }

    #[test]
    fn japanese_mixed_with_latin_technical_terms_stays_japanese() {
        assert_eq!(
            detected("Library の RichText は yjs で共同編集する"),
            Some("ja".into())
        );
    }
}

//! Deciding which language a public docs request is served in.
//!
//! Two rules shape this. First, the published set is an allow-list, not
//! a preference: a reader may ask for any language, and anything the
//! owner has not published is served as the source rather than refused.
//! A public page must not 404 because someone edited a query string.
//!
//! Second, `?lang=` is authoritative and `Accept-Language` is only a
//! hint. The header varies per browser and would shatter a CDN cache
//! keyed on it, and a header-selected page cannot be linked to or
//! indexed separately. The header therefore chooses a redirect target,
//! never the response body.

use super::LanguageTag;

/// What language a request resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageChoice {
    /// Serve the document as written.
    Source,
    /// Serve the cached translation for this language.
    Translated(LanguageTag),
}

impl LanguageChoice {
    /// The tag to advertise, if the response is a translation.
    pub fn translated_tag(&self) -> Option<&LanguageTag> {
        match self {
            Self::Source => None,
            Self::Translated(tag) => Some(tag),
        }
    }
}

/// Resolves an explicit `?lang=` value against the published set.
///
/// An unparseable or unpublished tag falls back to the source; it is
/// never an error. Matching is on the primary subtag, so a reader
/// asking for `en-GB` is served a repo published in `en`.
pub fn resolve_requested_language(
    requested: Option<&str>,
    published: &[LanguageTag],
) -> LanguageChoice {
    let Some(raw) = requested else {
        return LanguageChoice::Source;
    };

    let Ok(tag) = LanguageTag::new(raw) else {
        return LanguageChoice::Source;
    };

    published
        .iter()
        .find(|candidate| candidate.matches_primary(&tag))
        .map(|candidate| LanguageChoice::Translated(candidate.clone()))
        .unwrap_or(LanguageChoice::Source)
}

/// Picks the best published language for an `Accept-Language` header.
///
/// Returns `None` when nothing matches, when the header is absent, or
/// when the reader explicitly prefers the source. The caller redirects
/// to `?lang=` with the result rather than varying the body, so that
/// one URL always means one language.
pub fn negotiate_from_accept_language(
    header: Option<&str>,
    published: &[LanguageTag],
) -> Option<LanguageTag> {
    let header = header?;
    if published.is_empty() {
        return None;
    }

    let mut ranked: Vec<(f32, LanguageTag)> = Vec::new();

    for part in header.split(',') {
        let mut pieces = part.split(';');
        let raw = pieces.next()?.trim();
        if raw.is_empty() || raw == "*" {
            continue;
        }

        // `q=` is the only parameter that matters here; anything else
        // is ignored rather than treated as a parse failure.
        let quality = pieces
            .find_map(|piece| {
                piece
                    .trim()
                    .strip_prefix("q=")
                    .and_then(|value| value.parse::<f32>().ok())
            })
            .unwrap_or(1.0);

        // q=0 means "explicitly not this one".
        if quality <= 0.0 {
            continue;
        }

        if let Ok(tag) = LanguageTag::new(raw) {
            ranked.push((quality, tag));
        }
    }

    // Stable sort on quality alone keeps header order as the tie-break,
    // which is what the reader's browser intended.
    ranked.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    ranked.into_iter().find_map(|(_, tag)| {
        published
            .iter()
            .find(|candidate| candidate.matches_primary(&tag))
            .cloned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn published(tags: &[&str]) -> Vec<LanguageTag> {
        tags.iter()
            .map(|tag| LanguageTag::new(*tag).unwrap())
            .collect()
    }

    fn tag(value: &str) -> LanguageTag {
        LanguageTag::new(value).unwrap()
    }

    #[test]
    fn a_published_language_is_served_as_a_translation() {
        assert_eq!(
            resolve_requested_language(
                Some("en"),
                &published(&["en", "zh-Hans"])
            ),
            LanguageChoice::Translated(tag("en"))
        );
    }

    #[test]
    fn no_request_means_the_source() {
        assert_eq!(
            resolve_requested_language(None, &published(&["en"])),
            LanguageChoice::Source
        );
    }

    #[test]
    fn an_unpublished_language_falls_back_instead_of_failing() {
        // A public page must not break because of a query string.
        assert_eq!(
            resolve_requested_language(Some("ko"), &published(&["en"])),
            LanguageChoice::Source
        );
    }

    #[test]
    fn a_malformed_tag_falls_back_rather_than_erroring() {
        assert_eq!(
            resolve_requested_language(
                Some("not a language"),
                &published(&["en"])
            ),
            LanguageChoice::Source
        );
    }

    #[test]
    fn a_regional_request_matches_the_published_base_language() {
        assert_eq!(
            resolve_requested_language(Some("en-GB"), &published(&["en"])),
            LanguageChoice::Translated(tag("en"))
        );
    }

    #[test]
    fn accept_language_picks_the_highest_quality_published_match() {
        assert_eq!(
            negotiate_from_accept_language(
                Some("ko;q=0.9, en;q=0.8, ja;q=0.5"),
                &published(&["ja", "en"])
            ),
            Some(tag("en"))
        );
    }

    #[test]
    fn accept_language_respects_header_order_when_quality_ties() {
        assert_eq!(
            negotiate_from_accept_language(
                Some("ja, en"),
                &published(&["en", "ja"])
            ),
            Some(tag("ja"))
        );
    }

    #[test]
    fn accept_language_ignores_an_explicitly_refused_language() {
        assert_eq!(
            negotiate_from_accept_language(
                Some("en;q=0, ja"),
                &published(&["en", "ja"])
            ),
            Some(tag("ja"))
        );
    }

    #[test]
    fn accept_language_matches_on_the_primary_subtag() {
        assert_eq!(
            negotiate_from_accept_language(
                Some("en-US,en;q=0.9"),
                &published(&["en"])
            ),
            Some(tag("en"))
        );
    }

    #[test]
    fn accept_language_yields_nothing_when_none_is_published() {
        assert_eq!(
            negotiate_from_accept_language(
                Some("ko,de"),
                &published(&["en", "ja"])
            ),
            None
        );
        assert_eq!(negotiate_from_accept_language(Some("en"), &[]), None);
        assert_eq!(
            negotiate_from_accept_language(None, &published(&["en"])),
            None
        );
    }

    #[test]
    fn accept_language_survives_a_wildcard_and_junk() {
        assert_eq!(
            negotiate_from_accept_language(
                Some("*, !!!, en"),
                &published(&["en"])
            ),
            Some(tag("en"))
        );
    }
}

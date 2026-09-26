//! Shared validation for human-edited catalog draft fields.

const MAX_ALIASES: usize = 64;
const MAX_TEXT_LEN: usize = 255;

pub fn validate_required_text(
    field: &str,
    raw: &str,
) -> errors::Result<String> {
    validate_optional_text(field, Some(raw))?.ok_or_else(|| {
        errors::Error::invalid(format!("{field} is required"))
    })
}

pub fn validate_optional_text(
    field: &str,
    raw: Option<&str>,
) -> errors::Result<Option<String>> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    if value.chars().count() > MAX_TEXT_LEN {
        return Err(errors::Error::invalid(format!(
            "{field} is longer than {MAX_TEXT_LEN} characters"
        )));
    }
    Ok(Some(value.to_string()))
}

pub fn validate_aliases(raw: Option<&str>) -> errors::Result<Vec<String>> {
    let mut aliases: Vec<String> = raw
        .unwrap_or("")
        .lines()
        .map(|line| validate_optional_text("alias", Some(line)))
        .collect::<errors::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    aliases.sort();
    aliases.dedup();
    if aliases.len() > MAX_ALIASES {
        return Err(errors::Error::invalid(format!(
            "more than {MAX_ALIASES} aliases"
        )));
    }
    Ok(aliases)
}

pub fn validate_attribute_review_status(
    raw: Option<&str>,
) -> errors::Result<&'static str> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        Some("unreviewed") | None => Ok("unreviewed"),
        Some("reviewed") => Ok("reviewed"),
        Some(other) => Err(errors::Error::invalid(format!(
            "unknown attribute_review_status: {other}"
        ))),
    }
}

pub fn validate_display_order(raw: Option<&str>) -> errors::Result<i32> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => value.parse::<i32>().map_err(|_| {
            errors::Error::invalid(format!(
                "display_order must be an integer: {value:?}"
            ))
        }),
        None => Ok(0),
    }
}

pub fn validate_default_display(raw: Option<&str>) -> errors::Result<bool> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        Some("true") => Ok(true),
        Some("false") | None => Ok(false),
        Some(other) => Err(errors::Error::invalid(format!(
            "default_display must be true or false: {other:?}"
        ))),
    }
}

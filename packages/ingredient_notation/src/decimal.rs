//! COM-860: exact decimal values for nutrient amounts and refuse rates.
//!
//! Library has no built-in Decimal property, so draft records carry amounts
//! as strings. This type is the single place that validates and normalizes
//! them. Values never go through `f64`, so `0.1` stays `0.1` and a
//! re-import produces the same canonical text.

use std::fmt;

/// Longest accepted input, including the decimal point.
const MAX_LEN: usize = 38;

/// A non-negative decimal in canonical text form.
///
/// Canonical form has no leading zeros in the integer part (except a lone
/// `0`), no trailing zeros in the fraction, and no exponent. `"012.50"`
/// normalizes to `"12.5"` and `"0.0"` to `"0"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedDecimal(String);

impl NormalizedDecimal {
    pub fn parse(raw: &str) -> errors::Result<Self> {
        let s = raw.trim();
        if s.is_empty() {
            return Err(errors::Error::invalid("decimal value is empty"));
        }
        if s.len() > MAX_LEN {
            return Err(errors::Error::invalid(format!(
                "decimal value is longer than {MAX_LEN} characters: {s}"
            )));
        }

        let (int_part, frac_part) = match s.split_once('.') {
            Some((i, f)) => (i, Some(f)),
            None => (s, None),
        };
        let is_digits = |p: &str| {
            !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())
        };
        if !is_digits(int_part) || frac_part.is_some_and(|f| !is_digits(f))
        {
            return Err(errors::Error::invalid(format!(
                "not a non-negative decimal: {s}"
            )));
        }

        let int_trimmed = int_part.trim_start_matches('0');
        let int_canonical = if int_trimmed.is_empty() {
            "0"
        } else {
            int_trimmed
        };
        let frac_trimmed =
            frac_part.map(|f| f.trim_end_matches('0')).unwrap_or("");

        Ok(if frac_trimmed.is_empty() {
            Self(int_canonical.to_string())
        } else {
            Self(format!("{int_canonical}.{frac_trimmed}"))
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0 == "0"
    }

    /// Integer part compared as a number, without floating point.
    fn exceeds_integer(&self, max: u64) -> bool {
        let (int_part, frac_part) = match self.0.split_once('.') {
            Some((i, f)) => (i, Some(f)),
            None => (self.0.as_str(), None),
        };
        match int_part.parse::<u64>() {
            Ok(v) => v > max || (v == max && frac_part.is_some()),
            Err(_) => true,
        }
    }

    /// Validate a percentage in `0..=100`.
    pub fn parse_percentage(raw: &str) -> errors::Result<Self> {
        let value = Self::parse(raw)?;
        if value.exceeds_integer(100) {
            return Err(errors::Error::invalid(format!(
                "percentage must be between 0 and 100: {}",
                value
            )));
        }
        Ok(value)
    }
}

impl fmt::Display for NormalizedDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case("0", "0")]
    #[case("0.0", "0")]
    #[case("000", "0")]
    #[case("0.1", "0.1")]
    #[case("012.50", "12.5")]
    #[case("100", "100")]
    #[case(" 3.140 ", "3.14")]
    #[case("0.0001", "0.0001")]
    fn normalizes(#[case] raw: &str, #[case] expected: &str) {
        assert_eq!(
            NormalizedDecimal::parse(raw).unwrap().as_str(),
            expected
        );
    }

    #[rstest]
    #[case("")]
    #[case("-1")]
    #[case("1e3")]
    #[case("1,000")]
    #[case(".5")]
    #[case("5.")]
    #[case("Tr")]
    #[case("(0.1)")]
    #[case("1.2.3")]
    fn rejects(#[case] raw: &str) {
        assert!(NormalizedDecimal::parse(raw).is_err(), "{raw}");
    }

    #[test]
    fn rejects_overlong_input() {
        let raw = "1".repeat(MAX_LEN + 1);
        assert!(NormalizedDecimal::parse(&raw).is_err());
    }

    #[rstest]
    #[case("0", true)]
    #[case("35.5", true)]
    #[case("100", true)]
    #[case("100.0", true)]
    #[case("100.01", false)]
    #[case("101", false)]
    fn percentage_range(#[case] raw: &str, #[case] ok: bool) {
        assert_eq!(NormalizedDecimal::parse_percentage(raw).is_ok(), ok);
    }
}

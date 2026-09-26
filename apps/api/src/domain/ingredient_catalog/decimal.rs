//! COM-860: exact decimal values for nutrient amounts and refuse rates.
//!
//! Library has no built-in Decimal property, so draft records carry amounts
//! as strings. This type is the single place that validates and normalizes
//! them. Values never go through `f64`, so `0.1` stays `0.1` and a
//! re-import produces the same canonical text.

pub use ingredient_notation::NormalizedDecimal;

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
        let raw = "1".repeat(39);
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

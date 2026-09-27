//! COM-860: how a nutrient value was recorded in its source.
//!
//! The Standard Tables of Food Composition in Japan use notation such as
//! `-`, `0`, `Tr`, `(0)`, `(Tr)` and `(12.3)`. These carry different
//! meanings, so none of them may collapse into a plain `0` or an empty
//! cell. A nutrient with no value record at all is reported as
//! `NotListed` by the read API; it is never stored.

pub use ingredient_notation::NutrientValueStatus;

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case("12.3", NutrientValueStatus::Measured, Some("12.3"))]
    #[case("(12.30)", NutrientValueStatus::Estimated, Some("12.3"))]
    #[case("0", NutrientValueStatus::Zero, Some("0"))]
    #[case("(0)", NutrientValueStatus::EstimatedZero, Some("0"))]
    #[case("Tr", NutrientValueStatus::Trace, None)]
    #[case("(Tr)", NutrientValueStatus::EstimatedTrace, None)]
    #[case("-", NutrientValueStatus::NotMeasured, None)]
    #[case("0.1", NutrientValueStatus::Measured, Some("0.1"))]
    fn reads_source_notation(
        #[case] notation: &str,
        #[case] status: NutrientValueStatus,
        #[case] amount: Option<&str>,
    ) {
        let (got_status, got_amount) =
            NutrientValueStatus::from_notation(notation).unwrap();
        assert_eq!(got_status, status);
        assert_eq!(got_amount.as_ref().map(|a| a.as_str()), amount);
    }

    #[rstest]
    #[case("")]
    #[case("*")]
    #[case("(-)")]
    #[case("12.3*")]
    fn rejects_unknown_notation(#[case] notation: &str) {
        assert!(NutrientValueStatus::from_notation(notation).is_err());
    }

    #[test]
    fn trace_and_not_measured_never_store_an_amount() {
        for status in [
            NutrientValueStatus::Trace,
            NutrientValueStatus::EstimatedTrace,
            NutrientValueStatus::NotMeasured,
        ] {
            assert_eq!(status.validate_amount(None).unwrap(), None);
            assert!(status.validate_amount(Some("0")).is_err());
        }
    }

    #[test]
    fn zero_statuses_store_zero_and_reject_other_amounts() {
        let zero = NutrientValueStatus::Zero;
        assert_eq!(
            zero.validate_amount(None).unwrap().unwrap().as_str(),
            "0"
        );
        assert_eq!(
            zero.validate_amount(Some("0.00"))
                .unwrap()
                .unwrap()
                .as_str(),
            "0"
        );
        assert!(zero.validate_amount(Some("0.1")).is_err());
    }

    #[test]
    fn measured_requires_an_amount() {
        let measured = NutrientValueStatus::Measured;
        assert!(measured.validate_amount(None).is_err());
        assert!(measured.validate_amount(Some(" ")).is_err());
        assert_eq!(
            measured
                .validate_amount(Some("0.10"))
                .unwrap()
                .unwrap()
                .as_str(),
            "0.1"
        );
    }

    #[test]
    fn not_listed_is_read_only() {
        assert!(NutrientValueStatus::NotListed
            .validate_amount(None)
            .is_err());
    }

    #[test]
    fn round_trips_through_text() {
        for status in NutrientValueStatus::STORED {
            assert_eq!(
                status.as_str().parse::<NutrientValueStatus>().unwrap(),
                status
            );
        }
    }
}

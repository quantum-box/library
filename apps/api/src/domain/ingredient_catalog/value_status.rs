//! COM-860: how a nutrient value was recorded in its source.
//!
//! The Standard Tables of Food Composition in Japan use notation such as
//! `-`, `0`, `Tr`, `(0)`, `(Tr)` and `(12.3)`. These carry different
//! meanings, so none of them may collapse into a plain `0` or an empty
//! cell. A nutrient with no value record at all is reported as
//! `NotListed` by the read API; it is never stored.

use std::fmt;
use std::str::FromStr;

use super::NormalizedDecimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NutrientValueStatus {
    /// Analytical value, e.g. `12.3`.
    Measured,
    /// Estimated value, written as `(12.3)`.
    Estimated,
    /// Source `0`: below one tenth of the minimum reportable amount or
    /// not detected. Not the same as "not measured".
    Zero,
    /// Estimated zero, written as `(0)`.
    EstimatedZero,
    /// Trace amount, written as `Tr`.
    Trace,
    /// Estimated trace amount, written as `(Tr)`.
    EstimatedTrace,
    /// Not measured, written as `-`.
    NotMeasured,
    /// The release has no value for this ingredient and nutrient. Only
    /// produced by reads; drafts must not store it.
    NotListed,
}

impl NutrientValueStatus {
    pub const STORED: [Self; 7] = [
        Self::Measured,
        Self::Estimated,
        Self::Zero,
        Self::EstimatedZero,
        Self::Trace,
        Self::EstimatedTrace,
        Self::NotMeasured,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::Estimated => "estimated",
            Self::Zero => "zero",
            Self::EstimatedZero => "estimated_zero",
            Self::Trace => "trace",
            Self::EstimatedTrace => "estimated_trace",
            Self::NotMeasured => "not_measured",
            Self::NotListed => "not_listed",
        }
    }

    /// Whether a stored value with this status carries a numeric amount.
    pub fn has_amount(&self) -> bool {
        matches!(
            self,
            Self::Measured
                | Self::Estimated
                | Self::Zero
                | Self::EstimatedZero
        )
    }

    /// Check an amount against this status and return the amount to
    /// store. Zero statuses always store `0`; statuses without an amount
    /// reject one, so `Tr` can never be saved as a number.
    pub fn validate_amount(
        &self,
        amount: Option<&str>,
    ) -> errors::Result<Option<NormalizedDecimal>> {
        let amount = amount.map(str::trim).filter(|s| !s.is_empty());
        match self {
            Self::NotListed => Err(errors::Error::invalid(
                "value_status not_listed cannot be stored",
            )),
            Self::Measured | Self::Estimated => {
                let raw = amount.ok_or_else(|| {
                    errors::Error::invalid(format!(
                        "value_status {} requires an amount",
                        self
                    ))
                })?;
                Ok(Some(NormalizedDecimal::parse(raw)?))
            }
            Self::Zero | Self::EstimatedZero => match amount {
                None => Ok(Some(NormalizedDecimal::parse("0")?)),
                Some(raw) => {
                    let value = NormalizedDecimal::parse(raw)?;
                    if !value.is_zero() {
                        return Err(errors::Error::invalid(format!(
                            "value_status {} requires amount 0, got {}",
                            self, value
                        )));
                    }
                    Ok(Some(value))
                }
            },
            Self::Trace | Self::EstimatedTrace | Self::NotMeasured => {
                match amount {
                    None => Ok(None),
                    Some(raw) => Err(errors::Error::invalid(format!(
                        "value_status {} must not have an amount, got {}",
                        self, raw
                    ))),
                }
            }
        }
    }

    /// Read the source notation of the food composition tables.
    ///
    /// Returns the status and the amount to store. Footnote marks and
    /// other decorations are the importer's job to strip first; anything
    /// not recognised is an error rather than a guess.
    pub fn from_notation(
        notation: &str,
    ) -> errors::Result<(Self, Option<NormalizedDecimal>)> {
        let s = notation.trim();
        let (inner, estimated) =
            match s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
                Some(inner) => (inner.trim(), true),
                None => (s, false),
            };
        let status = match (inner, estimated) {
            ("-", false) => Self::NotMeasured,
            ("Tr", false) => Self::Trace,
            ("Tr", true) => Self::EstimatedTrace,
            (_, _) => {
                let value =
                    NormalizedDecimal::parse(inner).map_err(|_| {
                        errors::Error::invalid(format!(
                            "unrecognised nutrient notation: {notation}"
                        ))
                    })?;
                let status = match (value.is_zero(), estimated) {
                    (true, false) => Self::Zero,
                    (true, true) => Self::EstimatedZero,
                    (false, false) => Self::Measured,
                    (false, true) => Self::Estimated,
                };
                return Ok((status, Some(value)));
            }
        };
        Ok((status, None))
    }
}

impl fmt::Display for NutrientValueStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for NutrientValueStatus {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        Self::STORED
            .into_iter()
            .chain([Self::NotListed])
            .find(|status| status.as_str() == s)
            .ok_or_else(|| {
                errors::Error::invalid(format!("unknown value_status: {s}"))
            })
    }
}

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

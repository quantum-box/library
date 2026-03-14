use rust_decimal::prelude::*;
use std::fmt;
use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

/// TODO: add English documentation
///
/// 1 NanoDollar = $0.000000001 (10^-9 USD)
/// 1 USD = 1,000,000,000 NanoDollars
///
/// TODO: add English documentation
/// TODO: add English documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NanoDollar(i64);

impl NanoDollar {
    /// TODO: add English documentation
    pub const CONVERSION_FACTOR: i64 = 1_000_000_000;

    /// TODO: add English documentation
    pub const ZERO: Self = Self(0);

    /// TODO: add English documentation
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    /// TODO: add English documentation
    pub fn from_usd(usd: Decimal) -> Result<Self, NanoDollarError> {
        let nano = usd * Decimal::from(Self::CONVERSION_FACTOR);

        // TODO: add English comment
        if nano > Decimal::from(i64::MAX) {
            return Err(NanoDollarError::Overflow(usd));
        }

        // TODO: add English comment
        if nano < Decimal::ZERO {
            return Err(NanoDollarError::NegativeValue(usd));
        }

        Ok(Self(nano.round().to_i64().unwrap_or(0)))
    }

    /// TODO: add English documentation
    pub fn to_usd(&self) -> Decimal {
        Decimal::from(self.0) / Decimal::from(Self::CONVERSION_FACTOR)
    }

    /// TODO: add English documentation
    pub fn to_usd_f64(&self) -> Option<f64> {
        self.to_usd().to_f64()
    }

    /// TODO: add English documentation
    pub fn from_jpy(yen: i64) -> Self {
        const NANODOLLARS_PER_JPY: i64 = 1_000_000;
        Self(yen.saturating_mul(NANODOLLARS_PER_JPY))
    }

    /// TODO: add English documentation
    pub fn to_jpy_f64(&self) -> f64 {
        const NANODOLLARS_PER_JPY: f64 = 1_000_000.0;
        self.0 as f64 / NANODOLLARS_PER_JPY
    }

    /// TODO: add English documentation
    /// TODO: add English documentation
    #[deprecated(note = "Use to_usd() directly after migration")]
    pub fn to_credits(&self) -> Decimal {
        self.to_usd() * Decimal::from(100)
    }

    /// TODO: add English documentation
    pub fn value(&self) -> i64 {
        self.0
    }

    /// TODO: add English documentation
    /// TODO: add English documentation
    pub fn to_cents(&self) -> i64 {
        const NANODOLLARS_PER_CENT: i64 = 10_000_000;
        (self.0 + NANODOLLARS_PER_CENT - 1) / NANODOLLARS_PER_CENT
    }

    /// TODO: add English documentation
    pub fn to_stripe_cents(&self) -> i64 {
        self.to_cents()
    }

    /// TODO: add English documentation
    pub fn from_usd_cents(cents: i64) -> Self {
        const NANODOLLARS_PER_CENT: i64 = 10_000_000;
        Self(cents.saturating_mul(NANODOLLARS_PER_CENT))
    }

    /// TODO: add English documentation
    pub fn from_stripe_cents(cents: i64) -> Self {
        Self::from_usd_cents(cents)
    }

    /// TODO: add English documentation
    /// TODO: add English documentation
    pub fn from_payment_internal(value: i64) -> Self {
        // TODO: add English comment
        Self(value * 1_000_000)
    }

    /// TODO: add English documentation
    /// TODO: add English documentation
    pub fn from_catalog_internal(value: i64) -> Self {
        // TODO: add English comment
        Self(value * 10_000)
    }

    /// TODO: add English documentation
    pub fn format_human_readable(&self) -> String {
        let usd = self.to_usd();
        if usd >= Decimal::from(1) {
            format!("${usd:.2}")
        } else if usd >= Decimal::from_str_exact("0.01").unwrap() {
            format!("${usd:.4}")
        } else if usd >= Decimal::from_str_exact("0.0001").unwrap() {
            format!("${usd:.6}")
        } else {
            format!("${usd:.9}")
        }
    }
}

impl fmt::Display for NanoDollar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} nanodollars", self.0)
    }
}

// TODO: add English comment
impl Add for NanoDollar {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign for NanoDollar {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl Sub for NanoDollar {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign for NanoDollar {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

impl Mul<i64> for NanoDollar {
    type Output = Self;

    fn mul(self, rhs: i64) -> Self::Output {
        Self(self.0.saturating_mul(rhs))
    }
}

// TODO: add English comment
impl Default for NanoDollar {
    fn default() -> Self {
        Self::ZERO
    }
}

// TODO: add English comment
impl serde::Serialize for NanoDollar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // TODO: add English comment
        serializer.serialize_i64(self.0)
    }
}

// TODO: add English comment
impl<'de> serde::Deserialize<'de> for NanoDollar {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, Visitor};

        struct NanoDollarVisitor;

        impl Visitor<'_> for NanoDollarVisitor {
            type Value = NanoDollar;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str(
                    "an integer or float representing nanodollars or USD",
                )
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(NanoDollar::new(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if value > i64::MAX as u64 {
                    return Err(E::custom(format!(
                        "Value {value} exceeds i64::MAX"
                    )));
                }
                Ok(NanoDollar::new(value as i64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if value < 0.0 {
                    return Err(E::custom(format!(
                        "Negative value not allowed: {value}"
                    )));
                }

                // TODO: add English comment
                let nanodollars = (value
                    * NanoDollar::CONVERSION_FACTOR as f64)
                    .round() as i64;
                Ok(NanoDollar::new(nanodollars))
            }
        }

        deserializer.deserialize_any(NanoDollarVisitor)
    }
}

// TODO: add English comment
#[derive(Debug, thiserror::Error)]
pub enum NanoDollarError {
    #[error("Value overflow: ${0} exceeds maximum nanodollar value")]
    Overflow(Decimal),

    #[error("Negative value not allowed: ${0}")]
    NegativeValue(Decimal),

    #[error(
        "Precision loss: ${0} cannot be represented exactly in nanodollars"
    )]
    PrecisionLoss(Decimal),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_usd_conversion() {
        // $1 = 1,000,000,000 nanodollars
        let one_dollar = NanoDollar::from_usd(dec!(1)).unwrap();
        assert_eq!(one_dollar.value(), 1_000_000_000);
        assert_eq!(one_dollar.to_usd(), dec!(1));

        // $0.01 = 10,000,000 nanodollars
        let one_cent = NanoDollar::from_usd(dec!(0.01)).unwrap();
        assert_eq!(one_cent.value(), 10_000_000);
        assert_eq!(one_cent.to_cents(), 1);

        // Gemini Flash-Lite: $0.0000001 = 100 nanodollars
        let flash_lite = NanoDollar::from_usd(dec!(0.0000001)).unwrap();
        assert_eq!(flash_lite.value(), 100);
    }

    #[test]
    fn test_credit_conversion() {
        // $10 = 1000 credits = 10,000,000,000 nanodollars
        let ten_dollars = NanoDollar::from_usd(dec!(10)).unwrap();
        assert_eq!(ten_dollars.to_usd(), dec!(10));
    }

    #[test]
    fn test_migration_helpers() {
        // TODO: add English comment
        let payment_value = NanoDollar::from_payment_internal(5);
        assert_eq!(payment_value.value(), 5_000_000);
        assert_eq!(payment_value.to_usd(), dec!(0.005));

        // TODO: add English comment
        let catalog_value = NanoDollar::from_catalog_internal(5);
        assert_eq!(catalog_value.value(), 50_000);
        assert_eq!(catalog_value.to_usd(), dec!(0.00005));
    }

    #[test]
    fn test_cents_conversion() {
        // TODO: add English comment
        let one_cent = NanoDollar::from_usd(dec!(0.01)).unwrap();
        assert_eq!(one_cent.to_cents(), 1);

        // TODO: add English comment
        let one_and_half = NanoDollar::from_usd(dec!(0.015)).unwrap();
        assert_eq!(one_and_half.to_cents(), 2);

        // TODO: add English comment
        let small = NanoDollar::from_usd(dec!(0.001)).unwrap();
        assert_eq!(small.to_cents(), 1);
    }

    #[test]
    fn test_arithmetic() {
        let a = NanoDollar::new(1000);
        let b = NanoDollar::new(500);

        assert_eq!((a + b).value(), 1500);
        assert_eq!((a - b).value(), 500);
        assert_eq!((a * 2).value(), 2000);
    }

    #[test]
    fn test_human_readable_format() {
        assert_eq!(
            NanoDollar::from_usd(dec!(100))
                .unwrap()
                .format_human_readable(),
            "$100.00"
        );
        assert_eq!(
            NanoDollar::from_usd(dec!(1.5))
                .unwrap()
                .format_human_readable(),
            "$1.50"
        );
        assert_eq!(
            NanoDollar::from_usd(dec!(0.05))
                .unwrap()
                .format_human_readable(),
            "$0.0500"
        );
        assert_eq!(
            NanoDollar::from_usd(dec!(0.0001))
                .unwrap()
                .format_human_readable(),
            "$0.000100"
        );
        assert_eq!(
            NanoDollar::from_usd(dec!(0.000000001))
                .unwrap()
                .format_human_readable(),
            "$0.000000001"
        );
    }

    #[test]
    fn test_to_usd_f64() {
        let nano = NanoDollar::from_usd(dec!(2.75)).unwrap();
        let usd = nano.to_usd_f64().unwrap();
        assert!((usd - 2.75).abs() < 1e-12);
    }

    #[test]
    fn test_jpy_helpers() {
        let yen = 4321;
        let nano = NanoDollar::from_jpy(yen);
        assert_eq!(nano.value(), 4_321_000_000);
        assert!((nano.to_jpy_f64() - 4321.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_error_cases() {
        // TODO: add English comment
        assert!(matches!(
            NanoDollar::from_usd(dec!(-1)),
            Err(NanoDollarError::NegativeValue(_))
        ));

        // TODO: add English comment
        let huge = Decimal::from(i64::MAX) * dec!(2);
        assert!(matches!(
            NanoDollar::from_usd(huge),
            Err(NanoDollarError::Overflow(_))
        ));
    }

    #[test]
    fn test_deserialize_from_float() {
        // TODO: add English comment
        let json = r#"1.5"#;
        let nanodollar: NanoDollar = serde_json::from_str(json).unwrap();

        // 1.5 USD = 1,500,000,000 nanodollars
        assert_eq!(nanodollar.value(), 1_500_000_000);
        assert_eq!(nanodollar.to_usd(), dec!(1.5));
    }

    #[test]
    fn test_deserialize_from_integer() {
        // TODO: add English comment
        let json = r#"15000"#;
        let nanodollar: NanoDollar = serde_json::from_str(json).unwrap();

        // 15000 nanodollars = $0.000015
        assert_eq!(nanodollar.value(), 15000);
        assert_eq!(nanodollar.to_usd(), dec!(0.000015));
    }

    #[test]
    fn test_deserialize_usage_rate_structure() {
        // TODO: add English comment
        let json = r#"{
            "rate_type": "prompt_tokens",
            "unit": "token",
            "rate_per_unit": 1.5,
            "description": "プロンプトトークンの使用料",
            "minimum_units": 1
        }"#;

        #[derive(serde::Deserialize)]
        struct TestUsageRate {
            rate_per_unit: NanoDollar,
        }

        let rate: TestUsageRate = serde_json::from_str(json).unwrap();
        // 1.5 USD = 1,500,000,000 nanodollars
        assert_eq!(rate.rate_per_unit.value(), 1_500_000_000);
    }
}

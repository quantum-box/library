use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

use crate::{NanoDollar, NanoDollarError};

/// USD value object with nanodollar precision.
///
/// - `USD::from_cents(100)` → $1.00
/// - `USD::from_nanodollars(1_000_000_000)` → $1.00
/// - Supports conversion to payment units and nanodollars.
///
/// ```rust
/// use rust_decimal::Decimal;
/// use std::str::FromStr;
/// use value_object::USD;
///
/// let price = USD::from_decimal(Decimal::from_str("1.99").unwrap()).unwrap();
/// let free = USD::ZERO;
///
/// let nanodollars = price.to_nanodollars();
/// let payment_units = price.to_payment_units();
///
/// let total = price + USD::from_cents(50).unwrap(); // $1.99 + $0.50 = $2.49
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
)]
pub struct USD(Decimal);

/// Error type for USD conversions and validation.
#[derive(Debug, Clone, PartialEq)]
pub enum USDError {
    /// Value must be non-negative.
    NegativeValue(Decimal),
    /// Value overflowed allowable range.
    Overflow(Decimal),
    /// Input was not a valid numeric representation.
    InvalidValue(String),
}

impl fmt::Display for USDError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            USDError::NegativeValue(value) => {
                write!(f, "Negative USD value not allowed: {value}")
            }
            USDError::Overflow(value) => {
                write!(f, "USD value overflow: {value}")
            }
            USDError::InvalidValue(msg) => {
                write!(f, "Invalid USD value: {msg}")
            }
        }
    }
}

impl std::error::Error for USDError {}

impl USD {
    /// TODO: add English documentation
    pub const ZERO: Self = Self(Decimal::ZERO);

    /// TODO: add English documentation
    pub fn from_decimal(value: Decimal) -> Result<Self, USDError> {
        if value < Decimal::ZERO {
            return Err(USDError::NegativeValue(value));
        }

        // TODO: add English comment
        let max_usd = Decimal::from(999_999_999i64);
        if value > max_usd {
            return Err(USDError::Overflow(value));
        }

        Ok(Self(value))
    }

    /// TODO: add English documentation
    ///
    /// TODO: add English documentation
    /// ```rust
    /// use value_object::USD;
    ///
    /// let dollar = USD::from_cents(150).unwrap(); // $1.50
    /// ```
    pub fn from_cents(cents: i64) -> Result<Self, USDError> {
        if cents < 0 {
            return Err(USDError::NegativeValue(Decimal::from(cents)));
        }

        let usd_value = Decimal::from(cents) / Decimal::from(100);
        Self::from_decimal(usd_value)
    }

    /// TODO: add English documentation
    pub fn from_nanodollars(nanodollars: NanoDollar) -> Self {
        let usd_value = Decimal::from(nanodollars.value())
            / Decimal::from(NanoDollar::CONVERSION_FACTOR);
        // TODO: add English comment
        Self::from_decimal(usd_value).unwrap()
    }

    /// TODO: add English documentation
    pub fn value(&self) -> Decimal {
        self.0
    }

    /// TODO: add English documentation
    pub fn to_nanodollars(&self) -> Result<NanoDollar, NanoDollarError> {
        NanoDollar::from_usd(self.0)
    }

    /// TODO: add English documentation
    /// TODO: add English documentation
    pub fn to_payment_units(&self) -> i64 {
        (self.0 * Decimal::from(1000)).round().to_i64().unwrap_or(0)
    }

    /// TODO: add English documentation
    /// 1 cent = $0.01
    pub fn to_stripe_cents(&self) -> i64 {
        (self.0 * Decimal::from(100)).round().to_i64().unwrap_or(0)
    }

    /// TODO: add English documentation
    pub fn format_human_readable(&self) -> String {
        format!("${:.2}", self.0)
    }

    /// TODO: add English documentation
    pub fn to_string_precise(&self) -> String {
        self.0.to_string()
    }
}

// TODO: add English comment
impl Add for USD {
    type Output = Result<Self, USDError>;

    fn add(self, rhs: Self) -> Self::Output {
        Self::from_decimal(self.0 + rhs.0)
    }
}

impl AddAssign<USD> for USD {
    fn add_assign(&mut self, rhs: USD) {
        if let Ok(result) = self.add(rhs) {
            *self = result;
        }
        // TODO: add English comment
    }
}

impl Sub for USD {
    type Output = Result<Self, USDError>;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::from_decimal(self.0 - rhs.0)
    }
}

impl SubAssign<USD> for USD {
    fn sub_assign(&mut self, rhs: USD) {
        if let Ok(result) = self.sub(rhs) {
            *self = result;
        }
        // TODO: add English comment
    }
}

impl Mul<Decimal> for USD {
    type Output = Result<Self, USDError>;

    fn mul(self, rhs: Decimal) -> Self::Output {
        Self::from_decimal(self.0 * rhs)
    }
}

// TODO: add English comment
impl fmt::Display for USD {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${:.2}", self.0)
    }
}

// TODO: add English comment
impl TryFrom<Decimal> for USD {
    type Error = USDError;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        Self::from_decimal(value)
    }
}

impl From<USD> for Decimal {
    fn from(usd: USD) -> Self {
        usd.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_usd_creation() {
        let usd =
            USD::from_decimal(Decimal::from_str("10.50").unwrap()).unwrap();
        assert_eq!(usd.to_string(), "$10.50");
    }

    #[test]
    fn test_cents_conversion() {
        let usd = USD::from_cents(150).unwrap();
        assert_eq!(usd.value(), Decimal::from_str("1.50").unwrap());
        assert_eq!(usd.to_stripe_cents(), 150);
    }

    #[test]
    fn test_payment_units_conversion() {
        let usd =
            USD::from_decimal(Decimal::from_str("1.234").unwrap()).unwrap();
        assert_eq!(usd.to_payment_units(), 1234); // $1.234 * 1000 = 1234
    }

    #[test]
    fn test_nanodollar_conversion() {
        let usd =
            USD::from_decimal(Decimal::from_str("0.001").unwrap()).unwrap();
        let nanodollars = usd.to_nanodollars().unwrap();
        assert_eq!(nanodollars.value(), 1_000_000); // $0.001 = 1,000,000 nanodollars
    }

    #[test]
    fn test_negative_value_error() {
        let result = USD::from_decimal(Decimal::from_str("-1.00").unwrap());
        assert!(matches!(result, Err(USDError::NegativeValue(_))));
    }

    #[test]
    fn test_arithmetic_operations() {
        let a = USD::from_cents(100).unwrap(); // $1.00
        let b = USD::from_cents(50).unwrap(); // $0.50

        let sum = (a + b).unwrap();
        assert_eq!(sum.to_stripe_cents(), 150); // $1.50

        let diff = (a - b).unwrap();
        assert_eq!(diff.to_stripe_cents(), 50); // $0.50
    }

    #[test]
    fn test_zero_constant() {
        assert_eq!(USD::ZERO.value(), Decimal::ZERO);
        assert_eq!(USD::ZERO.to_string(), "$0.00");
    }
}

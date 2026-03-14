use crate::NanoDollar;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

const NANODOLLARS_PER_USD_CENT: i64 = 10_000_000;
const MAX_SAFE_CENTS: i64 = i64::MAX / NANODOLLARS_PER_USD_CENT;
const MIN_SAFE_CENTS: i64 = -MAX_SAFE_CENTS;

/// USD cents value object for integer-based currency handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UsdCents(i64);

impl UsdCents {
    /// Zero value constant.
    pub const ZERO: Self = Self(0);

    /// Creates a new instance from raw cents value.
    pub const fn new(raw_cents: i64) -> Self {
        Self(raw_cents)
    }

    /// Returns the underlying cents value.
    pub const fn raw(&self) -> i64 {
        self.0
    }

    fn clamp(raw: i64) -> i64 {
        raw.clamp(MIN_SAFE_CENTS, MAX_SAFE_CENTS)
    }

    /// Adds another `UsdCents`, saturating within the safe range.
    pub fn saturating_add(self, other: Self) -> Self {
        Self::new(Self::clamp(self.0.saturating_add(other.0)))
    }

    /// Subtracts another `UsdCents`, saturating within the safe range.
    pub fn saturating_sub(self, other: Self) -> Self {
        Self::new(Self::clamp(self.0.saturating_sub(other.0)))
    }

    /// Returns the absolute value within the safe range.
    pub fn abs(self) -> Self {
        Self::new(Self::clamp(self.0.abs()))
    }

    /// Saturating negation within the safe range.
    pub fn saturating_neg(self) -> Self {
        Self::new(Self::clamp(self.0.saturating_neg()))
    }

    /// Returns the prepaid credit portion (always >= 0).
    pub fn credit(self) -> Self {
        let cents = self.0.saturating_neg().max(0);
        Self::new(cents.min(MAX_SAFE_CENTS))
    }

    /// Returns the outstanding (owed amount) portion (always >= 0).
    pub fn outstanding(self) -> Self {
        let cents = self.0.max(0);
        Self::new(cents.min(MAX_SAFE_CENTS))
    }

    /// Converts the cents value into NanoDollar.
    pub fn to_nano_dollar(self) -> NanoDollar {
        NanoDollar::from_usd_cents(self.0)
    }

    /// Converts the cents value into USD as floating point dollars.
    pub fn to_usd(self) -> f64 {
        self.0 as f64 / 100.0
    }

    /// Converts the Stripe balance representation into signed NanoDollar balance.
    ///
    /// TODO: add English documentation
    /// TODO: add English documentation
    pub fn to_balance_nanodollar(self) -> NanoDollar {
        if self.0 <= 0 {
            self.credit().to_nano_dollar()
        } else {
            let outstanding = self.outstanding().to_nano_dollar();
            NanoDollar::new(-outstanding.value())
        }
    }
}

impl From<i64> for UsdCents {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}

impl From<UsdCents> for i64 {
    fn from(value: UsdCents) -> Self {
        value.raw()
    }
}

impl From<NanoDollar> for UsdCents {
    fn from(value: NanoDollar) -> Self {
        Self::new(value.to_cents().clamp(MIN_SAFE_CENTS, MAX_SAFE_CENTS))
    }
}

impl From<UsdCents> for NanoDollar {
    fn from(value: UsdCents) -> Self {
        value.to_nano_dollar()
    }
}

impl Add for UsdCents {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.saturating_add(rhs)
    }
}

impl AddAssign for UsdCents {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.saturating_add(rhs);
    }
}

impl Sub for UsdCents {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.saturating_sub(rhs)
    }
}

impl SubAssign for UsdCents {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.saturating_sub(rhs);
    }
}

impl Neg for UsdCents {
    type Output = Self;

    fn neg(self) -> Self::Output {
        self.saturating_neg()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credit_returns_positive_for_negative_raw_value() {
        let cents = UsdCents::new(-5_000);
        assert_eq!(cents.credit(), UsdCents::new(5_000));
        assert_eq!(
            cents.credit().to_nano_dollar(),
            NanoDollar::from_usd_cents(5_000)
        );
    }

    #[test]
    fn credit_returns_zero_for_positive_value() {
        let cents = UsdCents::new(2_000);
        assert_eq!(cents.credit(), UsdCents::ZERO);
    }

    #[test]
    fn outstanding_returns_positive_for_positive_value() {
        let cents = UsdCents::new(12_345);
        assert_eq!(cents.outstanding(), UsdCents::new(12_345));
        assert_eq!(
            cents.outstanding().to_nano_dollar(),
            NanoDollar::from_usd_cents(12_345)
        );
    }

    #[test]
    fn outstanding_returns_zero_for_negative_value() {
        let cents = UsdCents::new(-12_345);
        assert_eq!(cents.outstanding(), UsdCents::ZERO);
    }

    #[test]
    fn clamps_to_max_safe_cents() {
        let overflow_candidate = MAX_SAFE_CENTS + 10_000;

        let credit = UsdCents::new(-overflow_candidate).credit();
        assert_eq!(credit, UsdCents::new(MAX_SAFE_CENTS));
        assert_eq!(
            credit.to_nano_dollar(),
            NanoDollar::from_usd_cents(MAX_SAFE_CENTS)
        );

        let outstanding = UsdCents::new(overflow_candidate).outstanding();
        assert_eq!(outstanding, UsdCents::new(MAX_SAFE_CENTS));
        assert_eq!(
            outstanding.to_nano_dollar(),
            NanoDollar::from_usd_cents(MAX_SAFE_CENTS)
        );
    }

    #[test]
    fn zero_value_behaves_as_expected() {
        let cents = UsdCents::ZERO;
        assert_eq!(cents.credit(), UsdCents::ZERO);
        assert_eq!(cents.outstanding(), UsdCents::ZERO);
        assert_eq!(cents.to_nano_dollar(), NanoDollar::ZERO);
    }

    #[test]
    fn add_and_subtract_use_safe_ranges() {
        let a = UsdCents::new(1_000);
        let b = UsdCents::new(2_500);
        assert_eq!((a + b).raw(), 3_500);
        assert_eq!((a - b).raw(), -1_500);

        let max = UsdCents::new(MAX_SAFE_CENTS);
        assert_eq!((max + b).raw(), MAX_SAFE_CENTS);
    }

    #[test]
    fn converts_from_and_to_nanodollar() {
        let usd_cents = UsdCents::new(12_345);
        let nano: NanoDollar = usd_cents.into();
        assert_eq!(
            nano.value(),
            NanoDollar::from_usd_cents(12_345).value()
        );

        let cents_from_nano: UsdCents = nano.into();
        assert_eq!(cents_from_nano.raw(), 12_345);
    }
}

#![allow(dead_code)]

use num::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::ops::Add;
use std::str::FromStr;

use crate::JPY;

#[derive(Debug, Clone)]
pub struct Money<T> {
    amount: f64,
    currency: PhantomData<T>,
}

impl<T> Money<T> {
    pub fn new<N: ToPrimitive>(amount: N) -> Self {
        let f = amount.to_f64().unwrap();
        Self {
            amount: f,
            currency: PhantomData::<T>,
        }
    }

    pub fn value(&self) -> f64 {
        self.amount
    }
}

impl<T> Add for Money<T> {
    type Output = Money<T>;

    fn add(self, other: Money<T>) -> Self::Output {
        Self::new(self.amount + other.amount)
    }
}

/// ```rust
/// use value_object::{Money, JPY};
///
/// let amount = Money::<JPY>::new(100);
/// assert_eq!(amount.to_string(), "100");
///
/// let amount = Money::<JPY>::new(1.1);
/// assert_eq!(amount.to_string(), "1.1");
/// ```
impl std::fmt::Display for Money<JPY> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.amount)
    }
}

/// Determines if amounts are equal when currencies are the same
/// ```rust
/// use value_object::{Money, JPY};
///
/// let jpy_1 = Money::<JPY>::new(1);
/// let jpy_2 = Money::<JPY>::new(1);
/// assert!(jpy_1 == jpy_2);
/// ```
impl<T> PartialEq for Money<T> {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(&self.currency)
            == std::mem::discriminant(&other.currency)
            && (self.amount - other.amount).abs() < f64::EPSILON
    }
}

/// ```rust
/// use value_object::{Money, JPY};
///
/// let amount = "100".parse::<Money<JPY>>().expect("Invalid amount");
/// assert_eq!(amount.value(), 100.0);
///
/// let amount = "0".parse::<Money<JPY>>().expect("Invalid amount");
/// assert_eq!(amount.value(), 0.0);
///
/// let amount = "1.100000000000000000000000000000".parse::<Money<JPY>>().expect("Invalid amount");
/// assert_eq!(amount.value(), 1.1);
/// ```
impl FromStr for Money<JPY> {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let amount = s.parse::<f64>().map_err(|e| {
            errors::Error::invalid(format!("Money parse error: {e}"))
        })?;
        Ok(Self::new(amount))
    }
}

#[test]
fn test_phantom_money() {
    use crate::JPY;
    let jpy_1 = Money::<JPY>::new(1);
    let jpy_2 = Money::<JPY>::new(2);

    let result = jpy_1 + jpy_2; // TODO: add English comment
    assert_eq!(result, Money::<JPY>::new(3));
    // TODO: add English comment
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoneyAmount {
    pub nanodollar: crate::NanoDollar,
}

impl MoneyAmount {
    pub const ZERO: Self = Self {
        nanodollar: crate::NanoDollar::ZERO,
    };

    pub fn new(nanodollar: crate::NanoDollar) -> Self {
        Self { nanodollar }
    }
}

impl Default for MoneyAmount {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Add for MoneyAmount {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self::new(self.nanodollar + other.nanodollar)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MoneySummary {
    pub revenue: crate::NanoDollar,
    pub cost: crate::NanoDollar,
    pub stripe_fees: crate::NanoDollar,
    pub profit: crate::NanoDollar,
    pub margin_percent: Decimal,
}

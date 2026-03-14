use errors::Error;
use std::str::FromStr;

use num::ToPrimitive;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub struct Amount(u64);

impl Amount {
    pub fn new<N: ToPrimitive>(value: N) -> errors::Result<Self> {
        Ok(Self(
            value
                .to_u64()
                .ok_or(errors::Error::business_logic("Invalid amount"))?,
        ))
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl FromStr for Amount {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let amount = s.parse::<u64>()?;
        Ok(Self(amount))
    }
}

/// ```rust
/// use value_object::Amount;
///
/// let input: u32 = 100;
/// let amount = Amount::try_from(input).expect("Invalid amount");
/// assert_eq!(amount.value(), 100);
/// ```
impl TryFrom<u32> for Amount {
    type Error = errors::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// ```rust
/// use value_object::Amount;
///
/// let input: f32 = 100.0;
/// let amount = Amount::try_from(input).expect("Invalid amount");
/// assert_eq!(amount.value(), 100);
/// ```
impl TryFrom<f32> for Amount {
    type Error = errors::Error;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// ```rust
/// use value_object::Amount;
///
/// let input: f64 = 100.0;
/// let amount = Amount::try_from(input).expect("Invalid amount");
/// assert_eq!(amount.value(), 100);
/// ```
impl TryFrom<f64> for Amount {
    type Error = errors::Error;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

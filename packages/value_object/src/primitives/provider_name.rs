use std::fmt;
use std::str::FromStr;

use errors::Error;

/// # Short name
///
/// TODO: add English documentation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortName(String);

impl FromStr for ShortName {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Error::business_logic(
                "ShortName must not be empty",
            ));
        }
        if s.len() > 20 {
            return Err(Error::business_logic(
                "ShortName must be less than 20 characters",
            ));
        }
        Ok(Self(s.to_string()))
    }
}

impl fmt::Display for ShortName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

use errors::Error;
use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoneNumber {
    value: String,
}

impl FromStr for PhoneNumber {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Error::invalid("Phone number is empty"));
        }
        Ok(Self {
            value: s.to_string(),
        })
    }
}

impl Display for PhoneNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<String> for PhoneNumber {
    fn from(s: String) -> Self {
        Self { value: s }
    }
}

use errors::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SkuCode(pub String);

impl SkuCode {
    pub fn new(code: &str) -> errors::Result<Self> {
        Ok(Self(code.to_string()))
    }
}

impl fmt::Display for SkuCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::convert::TryInto<SkuCode> for &str {
    type Error = Error;

    fn try_into(self) -> Result<SkuCode, Self::Error> {
        SkuCode::new(self)
    }
}

impl std::convert::TryInto<SkuCode> for String {
    type Error = Error;

    fn try_into(self) -> Result<SkuCode, Self::Error> {
        SkuCode::new(&self)
    }
}

impl std::str::FromStr for SkuCode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct JanCode(pub String);

impl JanCode {
    pub fn new(code: &str) -> errors::Result<Self> {
        Ok(Self(code.to_string()))
    }
}

impl fmt::Display for JanCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::convert::TryInto<JanCode> for &str {
    type Error = Error;

    fn try_into(self) -> Result<JanCode, Self::Error> {
        JanCode::new(self)
    }
}

impl std::convert::TryInto<JanCode> for String {
    type Error = Error;

    fn try_into(self) -> Result<JanCode, Self::Error> {
        JanCode::new(&self)
    }
}

impl std::str::FromStr for JanCode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

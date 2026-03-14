//! Re-export ID types from value_object for type
//! compatibility with the rest of the workspace.

pub use value_object::{
    Identifier, OperatorId, PlatformId, ServiceAccountId,
    TenantId, UserId,
};

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Policy identifier.
#[derive(
    Clone,
    Eq,
    PartialEq,
    Hash,
    Ord,
    PartialOrd,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct PolicyId(String);

impl PolicyId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PolicyId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "PolicyId({})", self.0)
    }
}

impl fmt::Display for PolicyId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PolicyId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl From<String> for PolicyId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for PolicyId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for PolicyId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for PolicyId {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

/// Public API key identifier.
#[derive(
    Clone,
    Eq,
    PartialEq,
    Hash,
    Ord,
    PartialOrd,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct PublicApiKeyId(String);

impl PublicApiKeyId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PublicApiKeyId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "PublicApiKeyId({})", self.0)
    }
}

impl fmt::Display for PublicApiKeyId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PublicApiKeyId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl From<String> for PublicApiKeyId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for PublicApiKeyId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Public API key value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PublicApiKeyValue(String);

impl PublicApiKeyValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PublicApiKeyValue {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PublicApiKeyValue {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

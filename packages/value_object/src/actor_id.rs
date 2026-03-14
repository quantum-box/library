use crate::{ServiceAccountId, UserId};
use serde::{
    de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// Identifier for an actor that can own chat resources.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ActorId {
    User(UserId),
    ServiceAccount(ServiceAccountId),
}

impl ActorId {
    pub fn as_str(&self) -> &str {
        match self {
            Self::User(id) => id.as_str(),
            Self::ServiceAccount(id) => id.as_str(),
        }
    }

    pub fn is_user(&self) -> bool {
        matches!(self, Self::User(_))
    }

    pub fn is_service_account(&self) -> bool {
        matches!(self, Self::ServiceAccount(_))
    }

    pub fn as_user(&self) -> Option<&UserId> {
        match self {
            Self::User(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_service_account(&self) -> Option<&ServiceAccountId> {
        match self {
            Self::ServiceAccount(id) => Some(id),
            _ => None,
        }
    }
}

impl From<UserId> for ActorId {
    fn from(value: UserId) -> Self {
        Self::User(value)
    }
}

impl From<&UserId> for ActorId {
    fn from(value: &UserId) -> Self {
        Self::User(value.clone())
    }
}

impl From<ServiceAccountId> for ActorId {
    fn from(value: ServiceAccountId) -> Self {
        Self::ServiceAccount(value)
    }
}

impl From<&ServiceAccountId> for ActorId {
    fn from(value: &ServiceAccountId) -> Self {
        Self::ServiceAccount(value.clone())
    }
}

impl Display for ActorId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ActorId {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if UserId::prefixes()
            .iter()
            .any(|prefix| s.starts_with(prefix))
        {
            return Ok(Self::User(UserId::new(s)?));
        }
        if ServiceAccountId::prefixes()
            .iter()
            .any(|prefix| s.starts_with(prefix))
        {
            return Ok(Self::ServiceAccount(ServiceAccountId::new(s)?));
        }
        Err(errors::Error::parse_error(format!(
            "unsupported actor id prefix: {s}"
        )))
    }
}

impl Serialize for ActorId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ActorId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_str(&value).map_err(DeError::custom)
    }
}

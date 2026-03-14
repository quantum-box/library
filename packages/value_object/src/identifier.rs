//! # Identifier
//!
//! TODO: add English documentation
//!

use derive_getters::Getters;
use errors::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Display, str::FromStr};

/// # Identifier
///
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Getters)]
pub struct Identifier {
    value: String,
}

impl Serialize for Identifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.value)
    }
}

impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Identifier::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl FromStr for Identifier {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 3 {
            return Err(Error::type_error(
                "username is too short. must be 3 or more characters",
            ));
        }
        if s.len() > 40 {
            return Err(Error::type_error(
                "username is too long. must be 40 or less characters",
            ));
        }
        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::type_error(
                "invalid character. username must be alphanumeric, hyphen, underscore",
            ));
        }
        // Validate start/end (no hyphens or underscores)
        if s.starts_with(['-', '_']) || s.ends_with(['-', '_']) {
            return Err(Error::type_error(
                "username cannot start or end with hyphens or underscores",
            ));
        }
        // Validate consecutive hyphens/underscores
        if s.contains("--") || s.contains("__") {
            return Err(Error::type_error(
                "username cannot contain consecutive hyphens or underscores",
            ));
        }
        Ok(Identifier {
            value: s.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::*;

    #[rstest]
    #[case("hello_world123", Ok(()))]
    #[case("HELLO_WORLD", Ok(()))]
    #[case(
        "hello world",
        Err("TypeError: invalid character. username must be alphanumeric, hyphen, underscore")
    )]
    #[case("hello-world", Ok(()))]
    #[case(
        "1hello_world",
        Ok(())
    )]
    #[case(
        "_hello_world",
        Err("TypeError: username cannot start or end with hyphens or underscores")
    )]
    #[case(
        "hello_world_1234567890123456789012345678901234567890",
        Err("TypeError: username is too long. must be 40 or less characters")
    )]
    #[case(
        "camelCase",
        Ok(())
    )]
    fn test_identifier_from_str(
        #[case] input: &str,
        #[case] expected: Result<(), &str>,
    ) {
        let result: Result<Identifier, Error> = Identifier::from_str(input);
        match expected {
            Ok(_) => assert!(result.is_ok(), "Expected Ok, got Err"),
            Err(expected_msg) => {
                assert!(result.is_err(), "Expected Err, got Ok");
                assert_eq!(result.unwrap_err().to_string(), expected_msg);
            }
        }
    }

    #[test]
    fn test_identifier_serialize() {
        let identifier = Identifier::from_str("hello_world").unwrap();
        let serialized = serde_json::to_string(&identifier).unwrap();
        assert_eq!(serialized, "\"hello_world\"");
    }

    #[test]
    fn test_identifier_deserialize() {
        let identifier = Identifier::from_str("hello_world").unwrap();
        let serialized = serde_json::to_string(&identifier).unwrap();
        let deserialized: Identifier =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, identifier);
    }
}

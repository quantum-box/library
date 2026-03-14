//! # ModelName
//!
//! TODO: add English documentation
//!

use derive_getters::Getters;
use errors::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Display, str::FromStr};

/// # ModelName
///
/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Getters)]
pub struct ModelName {
    value: String,
}

impl ModelName {
    /// TODO: add English documentation
    pub fn new(name: impl Into<String>) -> Result<Self, Error> {
        let value = name.into().trim().to_string();

        if value.is_empty() {
            return Err(Error::type_error("Model name cannot be empty"));
        }

        if value.len() > 100 {
            return Err(Error::type_error(
                "Model name is too long. Must be 100 characters or less",
            ));
        }

        Ok(Self { value })
    }

    /// TODO: add English documentation
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl Serialize for ModelName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.value)
    }
}

impl<'de> Deserialize<'de> for ModelName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ModelName::new(s).map_err(serde::de::Error::custom)
    }
}

impl Display for ModelName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl FromStr for ModelName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<String> for ModelName {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ModelName {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::*;

    #[rstest]
    #[case("gpt-4", Ok("gpt-4"))]
    #[case("claude-3-opus-20240229", Ok("claude-3-opus-20240229"))]
    #[case("  gpt-4  ", Ok("gpt-4"))] // TODO: add English comment
    #[case("", Err("TypeError: Model name cannot be empty"))]
    #[case("   ", Err("TypeError: Model name cannot be empty"))] // TODO: add English comment
    #[case(&"a".repeat(101), Err("TypeError: Model name is too long. Must be 100 characters or less"))]
    fn test_model_name_new(
        #[case] input: &str,
        #[case] expected: Result<&str, &str>,
    ) {
        let result = ModelName::new(input);
        match expected {
            Ok(expected_value) => {
                assert!(
                    result.is_ok(),
                    "Expected Ok, got Err: {:?}",
                    result
                );
                assert_eq!(result.unwrap().as_str(), expected_value);
            }
            Err(expected_msg) => {
                assert!(result.is_err(), "Expected Err, got Ok");
                assert_eq!(result.unwrap_err().to_string(), expected_msg);
            }
        }
    }

    #[test]
    fn test_model_name_serialize() {
        let model_name = ModelName::new("gpt-4").unwrap();
        let serialized = serde_json::to_string(&model_name).unwrap();
        assert_eq!(serialized, "\"gpt-4\"");
    }

    #[test]
    fn test_model_name_deserialize() {
        let json = "\"claude-3-opus\"";
        let model_name: ModelName = serde_json::from_str(json).unwrap();
        assert_eq!(model_name.as_str(), "claude-3-opus");
    }

    #[test]
    fn test_model_name_deserialize_error() {
        let json = "\"\""; // TODO: add English comment
        let result: Result<ModelName, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}

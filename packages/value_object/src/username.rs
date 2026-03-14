use std::fmt;
use std::str::FromStr;

use crate::Identifier;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Username(String);

impl Username {
    pub fn new(value: &str) -> errors::Result<Self> {
        // Validate length (3-40 characters)
        if value.len() < 3 || value.len() > 40 {
            return Err(errors::Error::type_error(
                "Username must be between 3 and 40 characters",
            ));
        }

        // Validate characters (alphanumeric, hyphens, underscores)
        if !value
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(errors::Error::type_error(
                "Username may only contain alphanumeric characters, hyphens, or underscores",
            ));
        }

        // Validate start/end (no hyphens or underscores)
        if value.starts_with(['-', '_']) || value.ends_with(['-', '_']) {
            return Err(errors::Error::type_error(
                "Username cannot start or end with hyphens or underscores",
            ));
        }

        // Validate consecutive hyphens/underscores
        if value.contains("--") || value.contains("__") {
            return Err(errors::Error::type_error(
                "Username cannot contain consecutive hyphens or underscores",
            ));
        }

        Ok(Self(value.to_string()))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl FromStr for Username {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&Identifier> for Username {
    fn from(value: &Identifier) -> Self {
        value.to_string().parse().unwrap()
    }
}

impl From<&Username> for Identifier {
    fn from(value: &Username) -> Self {
        value.to_string().parse().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_usernames() {
        let valid_cases = vec![
            "abc",
            "user123",
            "hello-world",
            "test_user",
            "a-b-c",
            "x_y_z",
            "Dev1",
            "user-name_123",
        ];

        for case in valid_cases {
            assert!(Username::new(case).is_ok(), "Failed for: {}", case);
        }
    }

    #[test]
    fn test_invalid_usernames() {
        let invalid_cases = vec![
            ("ab", "too short"),
            ("-abc", "starts with hyphen"),
            ("abc-", "ends with hyphen"),
            ("_abc", "starts with underscore"),
            ("abc_", "ends with underscore"),
            ("ab--c", "consecutive hyphens"),
            ("ab__c", "consecutive underscores"),
            ("abc@def", "invalid character"),
            ("user name", "contains space"),
            ("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "too long"),
        ];

        for (case, reason) in invalid_cases {
            assert!(
                Username::new(case).is_err(),
                "Should fail for {}: {}",
                case,
                reason
            );
        }
    }
}

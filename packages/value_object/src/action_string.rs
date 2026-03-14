use errors::{Error as ErrorsError, Result};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// Action string representation in "context:name" format
///
/// Represents an action identifier used for authorization and policy checks.
/// The format is "context:name" where context is the domain (e.g., "auth", "order")
/// and name is the specific action (e.g., "CreateUser", "GetProduct").
///
/// # Examples
///
/// ```rust
/// use value_object::ActionString;
///
/// // Create using new_unchecked (no validation, use with care)
/// let action = ActionString::new_unchecked("auth:CreateUser");
///
/// // Create using try_from with validation
/// let action = ActionString::try_from("auth:CreateUser").unwrap();
///
/// // Get the string representation
/// println!("{}", action.as_str());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionString {
    value: String,
    context: String,
    name: String,
}

impl ActionString {
    /// TODO: add English documentation
    ///
    /// # Safety
    ///
    /// TODO: add English documentation
    /// TODO: add English documentation
    ///
    /// # Arguments
    ///
    /// TODO: add English documentation
    ///
    /// # Returns
    ///
    /// TODO: add English documentation
    pub fn new_unchecked(value: &str) -> Self {
        let parts: Vec<&str> = value.split(":").collect();
        debug_assert_eq!(
            parts.len(),
            2,
            "ActionString should have exactly one ':' separator"
        );
        debug_assert!(!parts[0].is_empty(), "Context should not be empty");
        debug_assert!(!parts[1].is_empty(), "Name should not be empty");

        Self {
            value: value.to_string(),
            context: parts[0].to_string(),
            name: parts[1].to_string(),
        }
    }

    /// TODO: add English documentation
    ///
    /// # Returns
    ///
    /// TODO: add English documentation
    pub fn context(&self) -> &str {
        &self.context
    }

    /// TODO: add English documentation
    ///
    /// # Returns
    ///
    /// TODO: add English documentation
    pub fn name(&self) -> &str {
        &self.name
    }

    /// TODO: add English documentation
    ///
    /// # Returns
    ///
    /// TODO: add English documentation
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// TODO: add English documentation
    ///
    /// # Arguments
    ///
    /// TODO: add English documentation
    ///
    /// # Returns
    ///
    /// TODO: add English documentation
    ///
    /// # Example
    ///
    /// ```rust
    /// # use value_object::ActionString;
    /// let action = ActionString::new_unchecked("auth:CreateUser");
    /// assert!(action.is_context("auth"));
    /// assert!(!action.is_context("order"));
    /// ```
    pub fn is_context(&self, context: &str) -> bool {
        self.context == context
    }

    /// TODO: add English documentation
    fn create_type_error(message: &str) -> ErrorsError {
        errors::type_error(message)
    }
}

impl TryFrom<&str> for ActionString {
    type Error = ErrorsError;

    /// TODO: add English documentation
    ///
    /// # Arguments
    ///
    /// TODO: add English documentation
    ///
    /// # Returns
    ///
    /// TODO: add English documentation
    ///
    /// # Errors
    ///
    /// TODO: add English documentation
    /// TODO: add English documentation
    /// TODO: add English documentation
    /// TODO: add English documentation
    fn try_from(value: &str) -> Result<Self> {
        if value.is_empty() {
            return Err(Self::create_type_error(
                "Action format error: empty string",
            ));
        }

        let parts: Vec<&str> = value.split(":").collect();

        if parts.len() != 2 {
            if parts.len() > 2 {
                return Err(Self::create_type_error(
                    "Action format error: multiple ':' found in action string",
                ));
            } else {
                return Err(Self::create_type_error(&format!(
                    "Action format error: expected 'context:name' format, got '{value}'"
                )));
            }
        }

        if parts[0].is_empty() {
            return Err(Self::create_type_error(
                "Action format error: context cannot be empty",
            ));
        }

        if parts[1].is_empty() {
            return Err(Self::create_type_error(
                "Action format error: name cannot be empty",
            ));
        }

        Ok(Self {
            value: value.to_string(),
            context: parts[0].to_string(),
            name: parts[1].to_string(),
        })
    }
}

impl TryFrom<String> for ActionString {
    type Error = ErrorsError;

    fn try_from(value: String) -> Result<Self> {
        ActionString::try_from(value.as_str())
    }
}

impl Display for ActionString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl AsRef<str> for ActionString {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl From<ActionString> for String {
    fn from(action: ActionString) -> String {
        action.value
    }
}

/// Macro for creating ActionString with compile-time validation
///
/// Creates an ActionString from a literal string, validating the format at compile time.
/// The format must be "context:name" with exactly one colon separator.
///
/// # Examples
///
/// ```rust
/// use value_object::action;
///
/// // Valid action strings
/// let valid_action = action!("auth:CreateUser");
/// let another_valid = action!("order:GetProduct");
///
/// // The following would cause compile-time errors:
/// // action!("");              // empty string
/// // action!("invalid");       // missing colon
/// // action!(":CreateUser");   // empty context
/// // action!("auth:");         // empty name
/// // action!("a:b:c");         // multiple colons
/// ```
#[macro_export]
macro_rules! action {
    ($action:literal) => {{
        // TODO: add English comment
        const ACTION: &str = $action;
        const _: () = {
            let bytes = ACTION.as_bytes();
            let mut colon_found = false;
            let mut colon_count = 0;
            let mut colon_pos = 0;

            // TODO: add English comment
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b':' {
                    colon_count += 1;
                    if colon_count == 1 {
                        colon_pos = i;
                        colon_found = true;
                    } else {
                        panic!("Action format error: multiple ':' found in action string");
                    }
                }
                i += 1;
            }

            if !colon_found {
                panic!("Action format error: expected 'context:name' format");
            }

            if colon_pos == 0 {
                panic!("Action format error: context cannot be empty");
            }

            if colon_pos + 1 >= bytes.len() {
                panic!("Action format error: name cannot be empty");
            }
        };

        $crate::ActionString::new_unchecked(ACTION)
    }};
}

/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
///
/// TODO: add English documentation
///
/// ```rust
/// use value_object::action_str;
///
/// let action_string: &str = action_str!("auth::CreateUser");
/// ```
#[macro_export]
macro_rules! action_str {
    ($action:literal) => {{
        // TODO: add English comment
        const ACTION: &str = $action;
        const _: () = {
            let bytes = ACTION.as_bytes();
            let mut double_colon_found = false;
            let mut i = 0;

            // TODO: add English comment
            while i < bytes.len() - 1 {
                if bytes[i] == b':' && bytes[i + 1] == b':' {
                    if double_colon_found {
                        panic!("Action format error: multiple '::' found in action string");
                    }
                    if i == 0 {
                        panic!("Action format error: context cannot be empty");
                    }
                    if i + 2 >= bytes.len() {
                        panic!("Action format error: name cannot be empty");
                    }
                    double_colon_found = true;
                    i += 2;
                } else {
                    i += 1;
                }
            }

            if !double_colon_found {
                panic!("Action format error: expected 'context::name' format");
            }
        };

        ACTION
    }};
}

/// Macro for creating auth context ActionString
///
/// Shorthand for creating an ActionString with "auth:" prefix.
///
/// # Examples
///
/// ```rust
/// use value_object::auth_action;
///
/// let action = auth_action!("CreateUser");
/// assert_eq!(action.as_str(), "auth:CreateUser");
/// ```
#[macro_export]
macro_rules! auth_action {
    ($name:literal) => {
        $crate::ActionString::new_unchecked(concat!("auth:", $name))
    };
}

/// TODO: add English documentation
#[macro_export]
macro_rules! order_action {
    ($name:literal) => {
        $crate::ActionString::new_unchecked(concat!("order:", $name))
    };
}

/// TODO: add English documentation
#[macro_export]
macro_rules! llms_action {
    ($name:literal) => {
        $crate::ActionString::new_unchecked(concat!("llms:", $name))
    };
}

/// TODO: add English documentation
#[macro_export]
macro_rules! payment_action {
    ($name:literal) => {
        $crate::ActionString::new_unchecked(concat!("payment:", $name))
    };
}

/// TODO: add English documentation
#[macro_export]
macro_rules! auth_action_str {
    ($name:literal) => {
        concat!("auth:", $name)
    };
}

/// TODO: add English documentation
#[macro_export]
macro_rules! order_action_str {
    ($name:literal) => {
        concat!("order:", $name)
    };
}

/// TODO: add English documentation
#[macro_export]
macro_rules! llms_action_str {
    ($name:literal) => {
        concat!("llms:", $name)
    };
}

/// TODO: add English documentation
#[macro_export]
macro_rules! payment_action_str {
    ($name:literal) => {
        concat!("payment:", $name)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_unchecked() {
        let action = ActionString::new_unchecked("auth:CreateUser");
        assert_eq!(action.context(), "auth");
        assert_eq!(action.name(), "CreateUser");
        assert_eq!(action.as_str(), "auth:CreateUser");
    }

    #[test]
    fn test_try_from_valid() {
        let action = ActionString::try_from("auth:CreateUser").unwrap();
        assert_eq!(action.context(), "auth");
        assert_eq!(action.name(), "CreateUser");
        assert_eq!(action.as_str(), "auth:CreateUser");
    }

    #[test]
    fn test_try_from_errors() {
        // TODO: add English comment
        let result = ActionString::try_from("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty string"));

        // TODO: add English comment
        let result = ActionString::try_from("invalid");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expected 'context:name' format"));

        // TODO: add English comment
        let result = ActionString::try_from(":CreateUser");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("context cannot be empty"));

        // TODO: add English comment
        let result = ActionString::try_from("auth:");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("name cannot be empty"));

        // TODO: add English comment
        let result = ActionString::try_from("auth:user:create");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("multiple ':' found"));
    }

    #[test]
    fn test_is_context() {
        let action = ActionString::new_unchecked("auth:CreateUser");
        assert!(action.is_context("auth"));
        assert!(!action.is_context("order"));
        assert!(!action.is_context(""));
    }

    #[test]
    fn test_display() {
        let action = ActionString::new_unchecked("auth:CreateUser");
        assert_eq!(format!("{}", action), "auth:CreateUser");
    }

    #[test]
    fn test_conversions() {
        let action = ActionString::new_unchecked("auth:CreateUser");

        // AsRef<str>
        let s: &str = action.as_ref();
        assert_eq!(s, "auth:CreateUser");

        // Into<String>
        let string: String = action.into();
        assert_eq!(string, "auth:CreateUser");
    }

    #[test]
    fn test_edge_cases() {
        // TODO: add English comment
        let action = ActionString::try_from("a:b").unwrap();
        assert_eq!(action.context(), "a");
        assert_eq!(action.name(), "b");

        // TODO: add English comment
        let action =
            ActionString::try_from("auth_service:Create_User").unwrap();
        assert_eq!(action.context(), "auth_service");
        assert_eq!(action.name(), "Create_User");

        // TODO: add English comment
        let action = ActionString::try_from("auth2:CreateUser123").unwrap();
        assert_eq!(action.context(), "auth2");
        assert_eq!(action.name(), "CreateUser123");
    }

    #[test]
    fn test_valid_action_formats() {
        // TODO: add English comment
        assert_eq!(action!("auth:CreateUser").as_str(), "auth:CreateUser");
        assert_eq!(
            action!("order:GetProduct").as_str(),
            "order:GetProduct"
        );
        assert_eq!(
            action!("llms:ExecuteAgent").as_str(),
            "llms:ExecuteAgent"
        );
        assert_eq!(
            action!("payment:CreateBillingInformation").as_str(),
            "payment:CreateBillingInformation"
        );
    }

    #[test]
    fn test_action_string_properties() {
        // TODO: add English comment
        let action = action!("auth:CreateUser");
        assert_eq!(action.context(), "auth");
        assert_eq!(action.name(), "CreateUser");
        assert!(action.is_context("auth"));
        assert!(!action.is_context("order"));
    }

    #[test]
    fn test_auth_action_helper() {
        assert_eq!(auth_action!("CreateUser").as_str(), "auth:CreateUser");
        assert_eq!(
            auth_action!("UpdatePolicy").as_str(),
            "auth:UpdatePolicy"
        );

        // TODO: add English comment
        let action = auth_action!("CreateUser");
        assert_eq!(action.context(), "auth");
        assert_eq!(action.name(), "CreateUser");
    }

    #[test]
    fn test_other_context_helpers() {
        assert_eq!(
            order_action!("CreateProduct").as_str(),
            "order:CreateProduct"
        );
        assert_eq!(
            llms_action!("ExecuteAgent").as_str(),
            "llms:ExecuteAgent"
        );
        assert_eq!(
            payment_action!("CreateSetupIntent").as_str(),
            "payment:CreateSetupIntent"
        );

        // TODO: add English comment
        assert!(order_action!("CreateProduct").is_context("order"));
        assert!(llms_action!("ExecuteAgent").is_context("llms"));
        assert!(payment_action!("CreateSetupIntent").is_context("payment"));
    }

    #[test]
    fn test_helper_macros_with_edge_cases() {
        // TODO: add English comment
        assert_eq!(auth_action!("a").as_str(), "auth:a"); // TODO: add English comment
        assert_eq!(
            auth_action!("Create_User").as_str(),
            "auth:Create_User"
        ); // TODO: add English comment
        assert_eq!(
            auth_action!("CreateUser123").as_str(),
            "auth:CreateUser123"
        ); // TODO: add English comment
        assert_eq!(
            order_action!("GetProduct_v2").as_str(),
            "order:GetProduct_v2"
        ); // TODO: add English comment
        assert_eq!(
            llms_action!("Execute_Agent_GPT4").as_str(),
            "llms:Execute_Agent_GPT4"
        ); // TODO: add English comment
        assert_eq!(
            payment_action!("CreateBillingInfo").as_str(),
            "payment:CreateBillingInfo"
        ); // TODO: add English comment
    }

    #[test]
    fn test_backward_compatibility_str_macros() {
        // TODO: add English comment
        assert_eq!(auth_action_str!("CreateUser"), "auth:CreateUser");
        assert_eq!(
            order_action_str!("CreateProduct"),
            "order:CreateProduct"
        );
        assert_eq!(llms_action_str!("ExecuteAgent"), "llms:ExecuteAgent");
        assert_eq!(
            payment_action_str!("CreateSetupIntent"),
            "payment:CreateSetupIntent"
        );
    }

    #[test]
    fn test_action_str_macro() {
        // TODO: add English comment
        assert_eq!(action_str!("auth::CreateUser"), "auth::CreateUser");
        assert_eq!(action_str!("order::GetProduct"), "order::GetProduct");
    }
}

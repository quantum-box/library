pub mod email;
// pub mod error;
pub mod macros;
#[cfg(feature = "sqlx")]
pub mod mysql;
pub mod traits;

pub use email::*;

#[cfg(feature = "graphql")]
pub mod graphql;

pub fn upper_camel_to_upper_snake(s: &str) -> String {
    let mut result = String::new();
    let iter = s.chars().peekable();

    for c in iter {
        // TODO: add English comment
        if c.is_uppercase() && !result.is_empty() {
            result.push('_');
        }

        result.push(c.to_ascii_uppercase());
    }

    result
}

pub trait Entity {
    type ID: Clone + Eq + PartialEq;
    fn id(&self) -> Self::ID;
}

#![allow(dead_code)]

// use crate::error::Error;
pub use crate::{def_id, def_id_serde_impls};
pub use smol_str;
pub use ulid::Ulid;

#[macro_export]
macro_rules! def_id_serde_impls {
    ($struct_name:ident) => {
        impl serde::Serialize for $struct_name {
            fn serialize<S>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                self.as_str().serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $struct_name {
            fn deserialize<D>(
                deserializer: D,
            ) -> std::result::Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                let s: String =
                    serde::Deserialize::deserialize(deserializer)?;
                s.parse::<Self>().map_err(::serde::de::Error::custom)
            }
        }
    };
    ($struct_name:ident, _) => {};
}

#[macro_export]
macro_rules! def_id {
    ($struct_name:ident: String) => {
        #[derive(Clone, Eq, PartialEq, Hash)]
        pub struct $struct_name(smol_str::SmolStr);

        impl std::fmt::Debug for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl $struct_name {
            /// Extracts a string slice containing the entire id.
            #[inline(always)]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl $struct_name {
            #[inline(always)]
            pub fn new(s: &str) -> errors::Result<Self> {
                Ok($struct_name(s.to_lowercase().into()))
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                let s = Ulid::new().to_string().to_lowercase();
                Self::new(&s).unwrap()
            }
        }

        impl PartialEq<str> for $struct_name {
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }

        impl PartialEq<&str> for $struct_name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<String> for $struct_name {
            fn eq(&self, other: &String) -> bool {
                self.as_str() == other
            }
        }

        impl PartialOrd for $struct_name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $struct_name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.as_str().cmp(other.as_str())
            }
        }

        impl AsRef<str> for $struct_name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        // impl crate::params::AsCursor for $struct_name {}

        impl std::ops::Deref for $struct_name {
            type Target = str;

            fn deref(&self) -> &str {
                self.as_str()
            }
        }


        impl std::fmt::Display for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0.to_lowercase(), f)
            }
        }

        impl std::str::FromStr for $struct_name {
            type Err = errors::ParseIdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok($struct_name(s.to_lowercase().into()))
            }
        }

        impl serde::Serialize for $struct_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: serde::ser::Serializer
            {
                self.as_str().serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $struct_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: serde::de::Deserializer<'de>
            {
                let s: String = serde::Deserialize::deserialize(deserializer)?;
                s.parse::<Self>().map_err(::serde::de::Error::custom)
            }
        }
    };

    ($struct_name:ident, $prefix:literal $(| $alt_prefix:literal)* $(, { $generate_hint:tt })?) => {
        /// An id for the corresponding object type.
        ///
        /// This type _typically_ will not allocate and
        /// therefore is usually cheaply clonable.
        #[derive(Clone, Eq, PartialEq, Hash)]
        // #[cfg_attr(feature = "axum", derive(utoipa::ToSchema))]
        // #[cfg_attr(feature = "axum", value_type = String)]
        pub struct $struct_name(smol_str::SmolStr);

        impl std::fmt::Debug for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl $struct_name {
            pub fn new(s: &str) -> errors::Result<Self> {
                use std::str::FromStr;
                Ok(Self::from_str(s).unwrap())
            }

            /// The prefix of the id type (e.g. `cus_` for a `CustomerId`).
            #[inline(always)]
            #[deprecated(note = "Please use prefixes or is_valid_prefix")]
            pub fn prefix() -> &'static str {
                $prefix
            }

            /// The valid prefixes of the id type (e.g. [`ch_`, `py_`\ for a `ChargeId`).
            #[inline(always)]
            pub fn prefixes() -> &'static [&'static str] {
                &[$prefix$(, $alt_prefix)*]
            }

            /// Extracts a string slice containing the entire id.
            #[inline(always)]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            /// Check is provided prefix would be a valid prefix for id's of this type
            pub fn is_valid_prefix(prefix: &str) -> bool {
                prefix == $prefix $( || prefix == $alt_prefix )*
            }
        }

        impl PartialEq<str> for $struct_name {
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }

        impl PartialEq<&str> for $struct_name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<String> for $struct_name {
            fn eq(&self, other: &String) -> bool {
                self.as_str() == other
            }
        }

        impl PartialOrd for $struct_name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $struct_name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.as_str().cmp(other.as_str())
            }
        }

        impl AsRef<str> for $struct_name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        // impl crate::params::AsCursor for $struct_name {}

        impl std::ops::Deref for $struct_name {
            type Target = str;

            fn deref(&self) -> &str {
                self.as_str()
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                // prefix_[ulid]
                let s = format!("{}{}", $prefix, Ulid::new().to_string().to_lowercase());
                $struct_name(s.into())
            }
        }

        impl std::fmt::Display for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0.to_lowercase(), f)
            }
        }

        impl std::str::FromStr for $struct_name {
            type Err = errors::ParseIdError;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                if !s.starts_with($prefix) $(
                    && !s.starts_with($alt_prefix) && !s.is_empty()
                )* {
                    // N.B. For debugging
                    eprintln!("ParseError: {} (expected: {:?}) for {}", s, $prefix, stringify!($struct_name));

                    Err(errors::ParseIdError {
                        typename: stringify!($struct_name),
                        expected: stringify!(id to start with $prefix $(or $alt_prefix)*),
                    })
                } else {
                    Ok($struct_name(s.to_lowercase().into()))
                }
            }
        }

        impl From<String> for $struct_name {
            fn from(s: String) -> Self {
                use std::str::FromStr;
                Self::from_str(&s).unwrap()
            }
        }

        def_id_serde_impls!($struct_name $(, $generate_hint )*);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    #[test]
    fn test_serialize() {
        def_id!(InvoiceId, "in_");
        #[derive(Deserialize)]
        struct WithInvoiceId {
            id: InvoiceId,
        }

        for body in [json!({"id": "in_aaaa"})] {
            let deser: WithInvoiceId =
                serde_json::from_value(body).expect("Could not deser");
            assert_eq!(deser.id, "in_aaaa".parse::<InvoiceId>().unwrap());
        }
    }

    #[test]
    fn test_parse_charge() {
        def_id!(ChargeId, "ch_" | "py_");
        assert!("ch_123".parse::<ChargeId>().is_ok());
        assert!("py_123".parse::<ChargeId>().is_ok());
        let bad_parse = "zz_123".parse::<ChargeId>();
        assert!(bad_parse.is_err());
        if let Err(err) = bad_parse {
            assert_eq!(
                format!("{}", err),
                "invalid `ChargeId`, expected id to start with \"ch_\" or \"py_\""
            );
        }
    }

    #[test]
    fn test_parse_from_ulid() {
        def_id!(ChargeTestId, "ch_");
        let id: ChargeTestId =
            "ch_01hqz0p20x0bkghgzp2ajp7970".parse().unwrap();
        assert_eq!(
            id.to_string(),
            "ch_01hqz0p20x0bkghgzp2ajp7970".to_string()
        );
    }

    #[test]
    fn test_serialize_id() {
        use serde::Serialize;

        def_id!(SelId, "sel_");

        #[derive(Serialize)]
        struct WithSelId {
            pub id: SelId,
        }

        let with_id = WithSelId {
            id: "sel_01jc0aph0v6rwx3h9caxepm800".parse::<SelId>().unwrap(),
        };
        let serialized = serde_json::to_string(&with_id).unwrap();
        assert_eq!(
            serialized,
            r#"{"id":"sel_01jc0aph0v6rwx3h9caxepm800"}"#
        );
    }

    #[test]
    fn test_deserilize_id() {
        use serde::Deserialize;

        def_id!(DelId, "del_");

        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct WithDelId {
            pub id: DelId,
        }

        let with_id = WithDelId {
            id: "del_01jc0aph0v6rwx3h9caxepm800".parse::<DelId>().unwrap(),
        };
        let deserialized: WithDelId = serde_json::from_str(
            r#"{"id":"del_01jc0aph0v6rwx3h9caxepm800"}"#,
        )
        .unwrap();

        assert_eq!(deserialized, with_id);
    }

    #[test]
    fn test_debug_output() {
        // TODO: add English comment
        def_id!(TestId, "test_");
        let id =
            "test_01jc0aph0v6rwx3h9caxepm800".parse::<TestId>().unwrap();
        assert_eq!(format!("{:?}", id), "test_01jc0aph0v6rwx3h9caxepm800");

        // TODO: add English comment
        def_id!(StringId: String);
        let string_id = StringId::new("simple_string").unwrap();
        assert_eq!(format!("{:?}", string_id), "simple_string");
    }
}

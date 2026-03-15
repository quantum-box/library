#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_return)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::empty_docs)]
#![allow(clippy::into_iter_on_ref)]
#![allow(clippy::to_string_trait_impl)]

pub extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate serde_repr;
extern crate url;

pub mod apis;
pub mod models;

#[cfg(feature = "auth")]
pub mod auth;

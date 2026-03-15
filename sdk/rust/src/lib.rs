#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_return)]

pub extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate serde_repr;
extern crate url;

pub mod apis;
pub mod models;

#[cfg(feature = "auth")]
pub mod auth;

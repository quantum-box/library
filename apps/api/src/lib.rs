#![allow(unused_imports)]
#![allow(dead_code)]

mod app;
pub mod collaboration;
mod config;
pub mod database_layout;
mod db_pool_metrics;
pub mod domain;
mod error;
pub mod handler;
mod interface_adapter;
pub mod migrations;
pub mod oauth_bootstrap;
mod photon_engine;
mod router;
pub mod sdk_auth;
mod sentry_context;
pub mod ttl_cache;
pub mod usecase;
pub use crate::domain::LIBRARY_TENANT;

pub use app::LibraryApp;
pub use config::Config;
pub use database_layout::DatabaseLayout;
use handler::graphql;
pub use router::router;

pub fn codegen() {
    // TODO: add English comment
    use async_graphql::{EmptySubscription, Schema};
    let schema = Schema::build(
        graphql::Query::default(),
        graphql::Mutation::default(),
        EmptySubscription,
    )
    .finish();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect(
        "This program should be run as part of a Cargo build script",
    );
    let mut file =
        std::fs::File::create(format!("{manifest_dir}/schema.graphql"))
            .expect("Failed to create schema.graphql");

    use std::io::Write;
    file.write_all(schema.clone().sdl().as_bytes())
        .expect("Failed to write schema.graphql");

    // TODO: add English comment
    if let Err(e) = handler::openapi::codegen() {
        eprintln!("Failed to generate OpenAPI document: {e:?}");
    }

    println!("Schema generated");
}

use dotenvy::dotenv;

pub mod app;
mod bootstrap;
pub mod collaboration;
pub mod config;
mod domain;
pub mod error;
pub mod handler;
mod interface_adapter;
mod router;
pub mod sdk_auth;
mod usecase;
pub use domain::LIBRARY_TENANT;

use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let config = config::Config::parse();
    config.validate_for_server_startup()?;
    if config.environment == "development" {
        println!("{config:#?}");
    }
    bootstrap::run_api(config).await
}

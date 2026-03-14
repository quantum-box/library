use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use crate::DEFAULT_FILTER;

pub fn init_simple_tracing() {
    let filter_layer = EnvFilter::new(DEFAULT_FILTER.join(","));
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_line_number(true)
                .with_filter(
                    tracing_subscriber::filter::LevelFilter::DEBUG,
                ),
        )
        .init();
}

pub fn init_simple_tracing_with_env() {
    let filter_layer = EnvFilter::from_env("RUST_LOG");
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
}

pub fn init_debug_tracing() {
    let filter_layer = EnvFilter::new("DEBUG");
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer())
        .init();
    tracing::debug!("debug tracing initialized");
}

pub fn init_simple_info_tracing() {
    let filter_layer = EnvFilter::new("INFO");
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
    tracing::info!("info tracing initialized");
}

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use crate::DEFAULT_FILTER;

/// Installs `subscriber` as the global default, ignoring the error raised
/// when one is already installed.
///
/// Binaries that run on AWS Lambda share a process across warm
/// invocations, so a second `init()` would panic with
/// `SetGlobalDefaultError`. Treating the second call as a no-op keeps the
/// first subscriber in place instead of aborting the runtime.
fn try_init_global<S>(subscriber: S)
where
    S: SubscriberInitExt,
{
    let _ = subscriber.try_init();
}

pub fn init_simple_tracing() {
    let filter_layer = EnvFilter::new(DEFAULT_FILTER.join(","));
    try_init_global(
        tracing_subscriber::registry().with(filter_layer).with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_line_number(true)
                .with_filter(
                    tracing_subscriber::filter::LevelFilter::DEBUG,
                ),
        ),
    );
}

pub fn init_simple_tracing_with_env() {
    let filter_layer = EnvFilter::from_env("RUST_LOG");
    try_init_global(
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(tracing_subscriber::fmt::layer().pretty()),
    );
}

pub fn init_debug_tracing() {
    let filter_layer = EnvFilter::new("DEBUG");
    try_init_global(
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(tracing_subscriber::fmt::layer()),
    );
    tracing::debug!("debug tracing initialized");
}

pub fn init_simple_info_tracing() {
    let filter_layer = EnvFilter::new("INFO");
    try_init_global(
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(tracing_subscriber::fmt::layer().pretty()),
    );
    tracing::info!("info tracing initialized");
}

//! # Telemetry & Tracing Configuration
//!
//! This module provides centralized configuration for logging, tracing,
//! and OpenTelemetry integration.
//!
//! ## Features
//!
//! - **Structured Logging**: JSON-formatted logs in production, human-readable in development
//! - **OpenTelemetry Integration**: OTLP export for distributed tracing (development only)
//! - **Sentry Integration**: Error tracking for production environments
//! - **Environment-aware Configuration**: Automatic switching based on `environment` setting
//! - **HTTP Tracing** (with `axum` feature): Request ID generation and propagation
//!
//! ## Log Levels
//!
//! - `ERROR`: Server errors (5xx), unexpected failures
//! - `WARN`: Client errors (4xx), recoverable issues
//! - `INFO`: Significant business events
//! - `DEBUG`: Detailed debugging information
//!
//! ## HTTP Tracing (axum feature)
//!
//! Enable the `axum` feature to get HTTP tracing utilities:
//!
//! ```toml
//! telemetry = { path = "...", features = ["axum"] }
//! ```
//!
//! Then use the provided layers:
//!
//! ```rust,ignore
//! use telemetry::http::{
//!     create_request_id_layer,
//!     create_trace_layer,
//!     create_propagate_request_id_layer,
//! };
//!
//! let app = Router::new()
//!     .route("/", get(handler))
//!     .layer(create_propagate_request_id_layer())
//!     .layer(create_trace_layer())
//!     .layer(create_request_id_layer());
//! ```
//!
//! ## Best Practices
//!
//! See `docs/src/architecture/structured-logging-guidelines.md` for detailed guidelines on:
//! - Using structured fields instead of string interpolation
//! - Standard field names (request_id, user_id, operator_id, etc.)
//! - Request ID propagation
//! - Log query examples for CloudWatch and Datadog

mod simple;
pub use simple::*;

#[cfg(feature = "axum")]
pub mod http;

use opentelemetry::sdk::metrics::controllers::BasicController;
use opentelemetry_otlp::WithExportConfig;
use sentry_tracing::EventFilter;
use std::collections::HashMap;
use tracing::Instrument;
// use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[derive(Debug)]
pub struct TracingConfig<'a> {
    pub environment: &'a str,
    pub crate_name: &'static str,
    pub filter: Option<Vec<&'a str>>,
    pub otel_endpoint: Option<String>,
    pub insi: Option<bool>,
    /// Enable OpenTelemetry tracing. If None, reads from OTEL_ENABLED env var.
    /// For AWS Lambda with ADOT layer, set OTEL_ENABLED=true and
    /// OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
    pub otel_enabled: Option<bool>,
    /// Sampling rate for traces (0.0 - 1.0). Default is 1.0 (100%).
    /// For production, consider 0.1 (10%) or lower to reduce costs.
    pub otel_sampling_rate: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtlpExporterProtocol {
    Grpc,
    HttpProtobuf,
}

impl OtlpExporterProtocol {
    fn from_env() -> Self {
        match std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL") {
            Ok(raw) => match raw.trim().to_ascii_lowercase().as_str() {
                "http" | "http/protobuf" | "http_proto" | "http-proto" => {
                    OtlpExporterProtocol::HttpProtobuf
                }
                _ => OtlpExporterProtocol::Grpc,
            },
            Err(_) => OtlpExporterProtocol::Grpc,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OtlpExporterConfig {
    pub protocol: OtlpExporterProtocol,
    pub endpoint: String,
    pub headers: HashMap<String, String>,
    pub resource_attributes: Vec<opentelemetry::KeyValue>,
}

impl OtlpExporterConfig {
    fn from_env(service_name: &str, otel_endpoint: Option<String>) -> Self {
        let protocol = OtlpExporterProtocol::from_env();
        let endpoint = match protocol {
            OtlpExporterProtocol::Grpc => {
                otel_endpoint.unwrap_or_else(|| {
                    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                        .unwrap_or_else(|_| "http://localhost:4317".into())
                })
            }
            OtlpExporterProtocol::HttpProtobuf => otel_endpoint
                .or_else(|| {
                    std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").ok()
                })
                .or_else(|| {
                    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()
                })
                .unwrap_or_else(|| {
                    "http://localhost:4318/v1/traces".into()
                }),
        };
        let headers = parse_key_value_list_env(&[
            "OTEL_EXPORTER_OTLP_TRACES_HEADERS",
            "OTEL_EXPORTER_OTLP_HEADERS",
        ]);
        let mut resource_attributes =
            parse_resource_attributes("OTEL_RESOURCE_ATTRIBUTES");
        if !resource_attributes
            .iter()
            .any(|attr| attr.key.as_str() == "service.name")
        {
            resource_attributes.push(opentelemetry::KeyValue::new(
                "service.name",
                service_name.to_string(),
            ));
        }

        Self {
            protocol,
            endpoint,
            headers,
            resource_attributes,
        }
    }

    fn resource(self) -> opentelemetry::sdk::Resource {
        opentelemetry::sdk::Resource::new(self.resource_attributes)
    }
}

fn parse_key_value_list_env(keys: &[&str]) -> HashMap<String, String> {
    for key in keys {
        if let Ok(raw) = std::env::var(key) {
            let parsed = parse_key_value_list(&raw);
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    HashMap::new()
}

fn parse_resource_attributes(key: &str) -> Vec<opentelemetry::KeyValue> {
    std::env::var(key)
        .ok()
        .map(|raw| {
            parse_key_value_list(&raw)
                .into_iter()
                .map(|(key, value)| {
                    opentelemetry::KeyValue::new(key, value)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_key_value_list(raw: &str) -> HashMap<String, String> {
    raw.split(',')
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                return None;
            }
            let (key, value) = trimmed.split_once('=')?;
            let key = key.trim();
            if key.is_empty() {
                return None;
            }
            Some((key.to_string(), value.trim().to_string()))
        })
        .collect()
}

impl Default for TracingConfig<'_> {
    fn default() -> Self {
        let crate_name = env!("CARGO_PKG_NAME");
        TracingConfig {
            environment: "development",
            crate_name,
            filter: None,
            otel_endpoint: None,
            insi: None,
            otel_enabled: None,
            otel_sampling_rate: None,
        }
    }
}

pub fn init_tracing(
    TracingConfig {
        environment,
        crate_name,
        filter,
        otel_endpoint,
        insi,
        otel_enabled,
        otel_sampling_rate,
    }: TracingConfig,
) {
    if environment == "production" {
        init_production_tracing(TracingConfig {
            environment,
            crate_name,
            filter,
            otel_endpoint,
            insi,
            otel_enabled,
            otel_sampling_rate,
        });
        return;
    }
    // Check if OTel is enabled via config or environment variable
    let otel_enabled = otel_enabled.unwrap_or_else(|| {
        std::env::var("OTEL_ENABLED")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true) // Default to true for backward compatibility
    });

    let rust_log = std::env::var("RUST_LOG").ok().unwrap_or("".into());
    let filter_str = filter.unwrap_or(vec![rust_log.as_str()]).join(",");
    let ansi_enabled = insi.unwrap_or(true);

    if otel_enabled {
        init_development_tracing_with_otel(
            crate_name,
            environment,
            &filter_str,
            ansi_enabled,
            otel_endpoint,
        );
    } else {
        init_development_tracing_without_otel(
            environment,
            &filter_str,
            ansi_enabled,
        );
    }
}

fn init_development_tracing_with_otel(
    crate_name: &'static str,
    environment: &str,
    filter_str: &str,
    ansi_enabled: bool,
    otel_endpoint: Option<String>,
) {
    let sentry_layer = if environment == "production" {
        sentry_tracing::layer().event_filter(|md| match md.level() {
            &tracing::Level::ERROR => EventFilter::Event,
            _ => EventFilter::Ignore,
        })
    } else {
        sentry_tracing::layer().event_filter(|_| EventFilter::Ignore)
    };

    let filter_layer = EnvFilter::new(filter_str);
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(ansi_enabled)
        .with_level(true)
        .with_line_number(true)
        .with_file(true);

    // Configure otel exporter.
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter().tonic().with_endpoint(
                otel_endpoint
                    .unwrap_or("http://otel-collector:4317".into()),
            ),
        )
        .with_trace_config(
            opentelemetry::sdk::trace::config()
                .with_sampler(opentelemetry::sdk::trace::Sampler::AlwaysOn)
                .with_id_generator(
                    opentelemetry::sdk::trace::RandomIdGenerator::default(),
                )
                .with_resource(opentelemetry::sdk::Resource::new(vec![
                    opentelemetry::KeyValue::new(
                        "service.name",
                        crate_name,
                    ),
                ])),
        )
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("Not running in tokio runtime");

    let otel_trace_layer =
        tracing_opentelemetry::layer().with_tracer(tracer);
    let otel_metrics_layer = tracing_opentelemetry::MetricsLayer::new(
        build_metrics_controller(),
    );

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(sentry_layer)
        .with(otel_trace_layer)
        .with(otel_metrics_layer)
        .with(fmt_layer)
        .init();

    tracing::info!("tracing initialized");
}

fn init_development_tracing_without_otel(
    environment: &str,
    filter_str: &str,
    ansi_enabled: bool,
) {
    let sentry_layer = if environment == "production" {
        sentry_tracing::layer().event_filter(|md| match md.level() {
            &tracing::Level::ERROR => EventFilter::Event,
            _ => EventFilter::Ignore,
        })
    } else {
        sentry_tracing::layer().event_filter(|_| EventFilter::Ignore)
    };

    let filter_layer = EnvFilter::new(filter_str);
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(ansi_enabled)
        .with_level(true)
        .with_line_number(true)
        .with_file(true);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(sentry_layer)
        .with(fmt_layer)
        .init();

    tracing::info!("tracing initialized (OTEL disabled)");
}

pub fn init_production_tracing(
    TracingConfig {
        environment,
        crate_name,
        filter,
        otel_endpoint,
        insi,
        otel_enabled,
        otel_sampling_rate,
    }: TracingConfig,
) {
    // Check if OTel is enabled via config or environment variable
    let otel_enabled = otel_enabled.unwrap_or_else(|| {
        std::env::var("OTEL_ENABLED")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
    });

    let rust_log = std::env::var("RUST_LOG").ok().unwrap_or("".into());
    let filter_str = filter.unwrap_or(vec![rust_log.as_str()]).join(",");
    let ansi_enabled = insi.unwrap_or(false);
    let is_production = environment == "production";

    if otel_enabled {
        init_production_tracing_with_otel(
            crate_name,
            &filter_str,
            ansi_enabled,
            is_production,
            otel_endpoint,
            otel_sampling_rate,
        );
    } else {
        init_production_tracing_without_otel(
            &filter_str,
            ansi_enabled,
            is_production,
        );
    }
}

fn init_production_tracing_with_otel(
    crate_name: &'static str,
    filter_str: &str,
    ansi_enabled: bool,
    is_production: bool,
    otel_endpoint: Option<String>,
    otel_sampling_rate: Option<f64>,
) {
    let filter_layer = EnvFilter::new(filter_str);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_level(false)
        .with_ansi(ansi_enabled)
        .with_line_number(true)
        .with_file(true);

    // Sampling rate (default 10% for production cost efficiency)
    let sampling_rate = otel_sampling_rate.unwrap_or_else(|| {
        std::env::var("OTEL_TRACES_SAMPLER_ARG")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.1)
    });

    let sampler = if sampling_rate >= 1.0 {
        opentelemetry::sdk::trace::Sampler::AlwaysOn
    } else if sampling_rate <= 0.0 {
        opentelemetry::sdk::trace::Sampler::AlwaysOff
    } else {
        opentelemetry::sdk::trace::Sampler::TraceIdRatioBased(sampling_rate)
    };

    // Service name from env or config
    let service_name = std::env::var("OTEL_SERVICE_NAME")
        .unwrap_or_else(|_| crate_name.to_string());
    let exporter_config =
        OtlpExporterConfig::from_env(&service_name, otel_endpoint);

    eprintln!(
        "Initializing OpenTelemetry: protocol={:?}, header_count={}, sampling_rate={}, service={}",
        exporter_config.protocol,
        exporter_config.headers.len(),
        sampling_rate,
        service_name
    );

    match std::env::var("OTEL_PROPAGATORS")
        .unwrap_or_else(|_| "tracecontext".to_string())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "xray" => opentelemetry::global::set_text_map_propagator(
            opentelemetry_aws::trace::XrayPropagator::default(),
        ),
        _ => opentelemetry::global::set_text_map_propagator(
            opentelemetry::sdk::propagation::TraceContextPropagator::new(),
        ),
    }

    let trace_config = opentelemetry::sdk::trace::config()
        .with_sampler(sampler)
        .with_id_generator(
            // Use X-Ray ID generator for AWS X-Ray compatible trace IDs
            opentelemetry::sdk::trace::XrayIdGenerator::default(),
        )
        .with_resource(exporter_config.clone().resource());

    let tracer = match exporter_config.protocol {
        OtlpExporterProtocol::Grpc => opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(&exporter_config.endpoint),
            )
            .with_trace_config(trace_config)
            .install_batch(opentelemetry::runtime::Tokio),
        OtlpExporterProtocol::HttpProtobuf => {
            opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .http()
                        .with_endpoint(&exporter_config.endpoint)
                        .with_headers(exporter_config.headers),
                )
                .with_trace_config(trace_config)
                .install_batch(opentelemetry::runtime::Tokio)
        }
    }
    .expect("Failed to initialize OpenTelemetry tracer");

    let otel_trace_layer =
        tracing_opentelemetry::layer().with_tracer(tracer);

    let sentry_layer = if is_production {
        sentry_tracing::layer().event_filter(|md| match md.level() {
            &tracing::Level::ERROR => EventFilter::Event,
            _ => EventFilter::Ignore,
        })
    } else {
        sentry_tracing::layer().event_filter(|_| EventFilter::Ignore)
    };

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(otel_trace_layer)
        .with(sentry_layer)
        .with(fmt_layer)
        .init();

    tracing::info!("production tracing initialized with OpenTelemetry");
}

fn init_production_tracing_without_otel(
    filter_str: &str,
    ansi_enabled: bool,
    is_production: bool,
) {
    let filter_layer = EnvFilter::new(filter_str);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_level(false)
        .with_ansi(ansi_enabled)
        .with_line_number(true)
        .with_file(true);

    let sentry_layer = if is_production {
        sentry_tracing::layer().event_filter(|md| match md.level() {
            &tracing::Level::ERROR => EventFilter::Event,
            _ => EventFilter::Ignore,
        })
    } else {
        sentry_tracing::layer().event_filter(|_| EventFilter::Ignore)
    };

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(sentry_layer)
        .with(fmt_layer)
        .init();

    tracing::info!("production tracing initialized (OTel disabled)");
}

pub const DEFAULT_FILTER: [&str; 14] = [
    "tonic=off",
    "rustls=off",
    "hyper=off",
    "mobc=off",
    "reqwest::connect=off",
    "h2=off",
    "tower=off",
    "aide=off",
    "aws_runtime=off",
    "aws_smithy_runtime=off",
    "ureq=off",
    "aws_config=off",
    "sqlx=off",
    "DEBUG",
];

pub use sentry::ClientInitGuard;

/// Initializes the Sentry client and returns its guard.
///
/// The returned [`ClientInitGuard`] flushes pending events and shuts
/// down the Sentry client when it is dropped. Callers must keep the
/// guard alive for the full lifetime of the process (or Lambda
/// invocation); dropping it immediately — e.g. binding to `_` — will
/// silently discard every event forwarded by `sentry_tracing`,
/// because the `sentry_tracing::layer()` bridge no-ops once the
/// client is shut down.
#[must_use = "ClientInitGuard must be held for the full runtime lifetime; dropping it immediately will silently discard Sentry events forwarded by sentry_tracing"]
pub fn init_sentry(dsn: &str) -> sentry::ClientInitGuard {
    sentry::init((
        dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ))
}

// https://github.com/open-telemetry/opentelemetry-rust/blob/d4b9befea04bcc7fc19319a6ebf5b5070131c486/examples/basic-otlp/src/main.rs#L35-L52
fn build_metrics_controller() -> BasicController {
    let otel_exporter_otlp_endpoint =
        std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .unwrap_or("http://otel-collector:4317".into());
    opentelemetry_otlp::new_pipeline()
        .metrics(
            opentelemetry::sdk::metrics::selectors::simple::histogram(Vec::new()),
            opentelemetry::sdk::export::metrics::aggregation::cumulative_temporality_selector(),
            opentelemetry::runtime::Tokio,
        )
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otel_exporter_otlp_endpoint),
        )
        .build()
        .expect("Failed to build metrics controller")
}

#[allow(dead_code)]
pub async fn start() {
    let user = "ymgyt";

    operation()
        .instrument(tracing::info_span!("operation", %user))
        .await;
    operation_2()
        .instrument(tracing::info_span!("operation_2"))
        .await;
}

#[allow(dead_code)]
async fn operation() {
    // trace
    // https://docs.rs/tracing-opentelemetry/latest/tracing_opentelemetry/struct.MetricsLayer.html#usage
    tracing::info!(
        ops = "xxx",
        counter.ops_count = 10,
        "successfully completed"
    );
}

#[allow(dead_code)]
async fn operation_2() {
    tracing::info!(arg = "xyz", "fetch resources...");
    tracing::warn!("something went wrong");
}

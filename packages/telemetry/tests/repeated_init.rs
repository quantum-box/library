//! Regression coverage for repeated tracing initialization.
//!
//! AWS Lambda reuses one process across warm invocations, so a binary that
//! initializes tracing per invocation calls `set_global_default` more than
//! once. That used to panic and kill the runtime, which is what broke the
//! `lambda-library-api-migrate` pre-deploy hook.
//!
//! Each integration test file is its own binary, so the global subscriber
//! state exercised here stays isolated from other tests.

#[test]
fn init_debug_tracing_can_be_called_repeatedly() {
    telemetry::init_debug_tracing();
    telemetry::init_debug_tracing();
    telemetry::init_debug_tracing();
}

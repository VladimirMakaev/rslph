//! Fake Claude test infrastructure.
//!
//! This module provides a fake Claude binary and scenario builder for E2E testing.

pub mod config;
pub mod prebuilt;
pub mod scenario;
pub mod stream_json;

pub use config::{FakeClaudeConfig, InvocationConfig, TokenConfig};
pub use scenario::{FakeClaudeHandle, ScenarioBuilder};
pub use stream_json::StreamEventOutput;

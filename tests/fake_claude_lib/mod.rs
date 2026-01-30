//! Fake Claude test infrastructure.
//!
//! This module provides a fake Claude binary and scenario builder for E2E testing.

pub mod config;
pub mod prebuilt;
pub mod scenario;
pub mod stream_json;

pub use config::FakeClaudeConfig;
pub use scenario::{FakeClaudeHandle, ScenarioBuilder};

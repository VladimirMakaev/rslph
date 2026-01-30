//! Fake Claude test infrastructure.
//!
//! This module provides a fake Claude binary and scenario builder for E2E testing.

pub mod config;
pub mod prebuilt;
pub mod scenario;
pub mod stream_json;

// Re-export commonly used types for convenience
// These are used in test files but clippy doesn't track cross-crate usage properly
#[allow(unused_imports)]
pub use config::FakeClaudeConfig;
#[allow(unused_imports)]
pub use scenario::{FakeClaudeHandle, ScenarioBuilder};

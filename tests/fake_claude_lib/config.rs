//! Configuration types for fake Claude binary.
//!
//! These types are serialized by the test harness and deserialized
//! by the fake Claude binary to determine response behavior.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::stream_json::StreamEventOutput;

/// Token usage configuration for an invocation.
///
/// Allows tests to specify deterministic token values for reproducible results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfig {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

impl Default for TokenConfig {
    fn default() -> Self {
        Self {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        }
    }
}

/// Configuration for the fake Claude binary.
#[derive(Debug, Serialize, Deserialize)]
pub struct FakeClaudeConfig {
    /// Responses for each invocation (0-indexed).
    pub invocations: Vec<InvocationConfig>,

    /// Path to the invocation counter file.
    pub counter_path: PathBuf,
}

/// Configuration for a single invocation of fake Claude.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InvocationConfig {
    /// Events to output for this invocation.
    pub events: Vec<StreamEventOutput>,

    /// Raw lines to output before events (for malformed output testing).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_lines: Vec<String>,

    /// Stderr lines to output before events (for testing stderr handling).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stderr_lines: Vec<String>,

    /// Initial delay before outputting any events (milliseconds).
    /// Use this to simulate a slow startup that causes timeout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_delay_ms: Option<u64>,

    /// Delay between events in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,

    /// Crash (exit 1) after outputting this many events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crash_after_events: Option<usize>,

    /// Exit code to return (defaults to 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,

    /// Token configuration for this invocation's responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_config: Option<TokenConfig>,

    /// When true, actually execute Write and Bash tool_use events.
    /// This creates real files and runs real commands for E2E testing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_tools: Option<bool>,
}

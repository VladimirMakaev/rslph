//! Configuration types for fake Claude binary.
//!
//! These types are serialized by the test harness and deserialized
//! by the fake Claude binary to determine response behavior.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::stream_json::StreamEventOutput;

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

    /// Delay between events in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,

    /// Crash (exit 1) after outputting this many events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crash_after_events: Option<usize>,

    /// Exit code to return (defaults to 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

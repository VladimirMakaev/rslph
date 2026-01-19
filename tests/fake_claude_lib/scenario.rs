//! Scenario builder for configuring fake Claude responses.
//!
//! Provides a fluent API for setting up deterministic test scenarios.

use std::path::PathBuf;
use tempfile::TempDir;

use super::config::{FakeClaudeConfig, InvocationConfig};
use super::stream_json::StreamEventOutput;

/// Builder for configuring fake Claude scenarios.
pub struct ScenarioBuilder {
    /// Completed invocation configurations.
    invocations: Vec<InvocationConfig>,

    /// Current invocation being configured.
    current_invocation: InvocationConfig,

    /// Temporary directory for config and counter files.
    temp_dir: TempDir,
}

impl ScenarioBuilder {
    /// Create a new scenario builder.
    pub fn new() -> Self {
        Self {
            invocations: Vec::new(),
            current_invocation: InvocationConfig::default(),
            temp_dir: TempDir::new().expect("Failed to create temp directory"),
        }
    }

    /// Add a text response to the current invocation.
    ///
    /// This adds a system init event followed by the assistant text response.
    pub fn respond_with_text(mut self, text: &str) -> Self {
        // Add system init event first (like real Claude CLI)
        self.current_invocation
            .events
            .push(StreamEventOutput::system_init());

        // Add assistant text response
        self.current_invocation
            .events
            .push(StreamEventOutput::assistant_text(text));

        // Add result event
        self.current_invocation
            .events
            .push(StreamEventOutput::result(0.001));

        self
    }

    /// Add a raw event to the current invocation.
    pub fn add_event(mut self, event: StreamEventOutput) -> Self {
        self.current_invocation.events.push(event);
        self
    }

    /// Set delay between events for current invocation.
    pub fn with_delay_ms(mut self, delay: u64) -> Self {
        self.current_invocation.delay_ms = Some(delay);
        self
    }

    /// Configure the current invocation to crash after N events.
    pub fn crash_after(mut self, count: usize) -> Self {
        self.current_invocation.crash_after_events = Some(count);
        self
    }

    /// Finalize current invocation and start configuring the next one.
    pub fn next_invocation(mut self) -> Self {
        self.invocations
            .push(std::mem::take(&mut self.current_invocation));
        self
    }

    /// Build the scenario and return a handle.
    pub fn build(mut self) -> FakeClaudeHandle {
        // Push current invocation if it has events
        if !self.current_invocation.events.is_empty() {
            self.invocations.push(self.current_invocation);
        }

        let config_path = self.temp_dir.path().join("fake_claude_config.json");
        let counter_path = self.temp_dir.path().join("fake_claude_counter");

        // Initialize counter to 0
        std::fs::write(&counter_path, "0").expect("Failed to write counter file");

        let config = FakeClaudeConfig {
            invocations: self.invocations,
            counter_path: counter_path.clone(),
        };

        // Write config file
        let config_json = serde_json::to_string_pretty(&config).expect("Failed to serialize config");
        std::fs::write(&config_path, config_json).expect("Failed to write config file");

        // Get path to fake_claude binary
        let executable_path = get_fake_claude_path();

        FakeClaudeHandle {
            executable_path,
            config_path,
            counter_path,
            _temp_dir: self.temp_dir,
        }
    }
}

impl Default for ScenarioBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to a configured fake Claude scenario.
pub struct FakeClaudeHandle {
    /// Path to the fake_claude binary.
    pub executable_path: PathBuf,

    /// Path to the configuration file.
    pub config_path: PathBuf,

    /// Path to the invocation counter file.
    counter_path: PathBuf,

    /// Keep temp directory alive while handle exists.
    _temp_dir: TempDir,
}

impl FakeClaudeHandle {
    /// Get the number of times fake Claude has been invoked.
    pub fn invocation_count(&self) -> usize {
        std::fs::read_to_string(&self.counter_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    }

    /// Get environment variables to set when running rslph.
    ///
    /// Returns tuples of (key, value) for:
    /// - FAKE_CLAUDE_CONFIG: Path to config file
    /// - RSLPH_CLAUDE_PATH: Path to fake_claude binary (to override real claude)
    pub fn env_vars(&self) -> Vec<(&'static str, String)> {
        vec![
            ("FAKE_CLAUDE_CONFIG", self.config_path.to_string_lossy().into_owned()),
            ("RSLPH_CLAUDE_PATH", self.executable_path.to_string_lossy().into_owned()),
        ]
    }
}

/// Get the path to the fake_claude test binary.
fn get_fake_claude_path() -> PathBuf {
    // Try using CARGO_BIN_EXE environment variable (set during cargo test)
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_fake_claude") {
        return PathBuf::from(path);
    }

    // Fallback: construct path from manifest directory
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());

    // During tests, binary is in target/debug/deps or target/debug
    let base = PathBuf::from(manifest_dir);

    // Check common locations
    let candidates = [
        base.join("target/debug/fake_claude"),
        base.join("target/debug/deps/fake_claude"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    // Return default path (will fail at runtime if not found)
    base.join("target/debug/fake_claude")
}

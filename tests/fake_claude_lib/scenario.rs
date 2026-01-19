//! Scenario builder for configuring fake Claude responses.
//!
//! Provides a fluent API for setting up deterministic test scenarios.

use serde_json::json;
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

    /// Add Read tool use to current invocation.
    pub fn uses_read(mut self, path: &str) -> Self {
        let event = StreamEventOutput::tool_use(
            "Read",
            json!({
                "file_path": path
            }),
        );
        self.current_invocation.events.push(event);
        self
    }

    /// Add Write tool use to current invocation.
    pub fn uses_write(mut self, path: &str, content: &str) -> Self {
        let event = StreamEventOutput::tool_use(
            "Write",
            json!({
                "file_path": path,
                "content": content
            }),
        );
        self.current_invocation.events.push(event);
        self
    }

    /// Add Edit tool use to current invocation.
    pub fn uses_edit(mut self, path: &str, old_string: &str, new_string: &str) -> Self {
        let event = StreamEventOutput::tool_use(
            "Edit",
            json!({
                "file_path": path,
                "old_string": old_string,
                "new_string": new_string
            }),
        );
        self.current_invocation.events.push(event);
        self
    }

    /// Add Bash tool use to current invocation.
    pub fn uses_bash(mut self, command: &str) -> Self {
        let event = StreamEventOutput::tool_use(
            "Bash",
            json!({
                "command": command
            }),
        );
        self.current_invocation.events.push(event);
        self
    }

    /// Add generic tool use (for less common tools or custom testing).
    pub fn uses_tool(mut self, name: &str, input: serde_json::Value) -> Self {
        let event = StreamEventOutput::tool_use(name, input);
        self.current_invocation.events.push(event);
        self
    }

    /// Send raw JSON line (for malformed output testing).
    ///
    /// Raw lines are output before events. Use this for testing
    /// how the parser handles malformed or unexpected output.
    pub fn send_raw(mut self, raw_json: &str) -> Self {
        self.current_invocation.raw_lines.push(raw_json.to_string());
        self
    }

    /// Configure exit code for this invocation.
    ///
    /// By default, fake Claude exits with 0. Use this to test
    /// error handling when Claude CLI fails.
    pub fn with_exit_code(mut self, code: i32) -> Self {
        self.current_invocation.exit_code = Some(code);
        self
    }

    /// Alias for with_delay_ms for API consistency.
    pub fn with_delay(self, delay_ms: u64) -> Self {
        self.with_delay_ms(delay_ms)
    }

    /// Finalize current invocation and start configuring the next one.
    pub fn next_invocation(mut self) -> Self {
        self.invocations
            .push(std::mem::take(&mut self.current_invocation));
        self
    }

    /// Build the scenario and return a handle.
    pub fn build(mut self) -> FakeClaudeHandle {
        // Push current invocation if it has content (events or raw_lines)
        if !self.current_invocation.events.is_empty()
            || !self.current_invocation.raw_lines.is_empty()
        {
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

    // Check common locations (exact paths first)
    let candidates = [
        base.join("target/debug/fake_claude"),
        base.join("target/debug/deps/fake_claude"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    // Try finding binary with hash suffix in deps directory
    // The test binary is named like fake_claude-7d73059a19867aac
    let deps_dir = base.join("target/debug/deps");
    if deps_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&deps_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Match fake_claude-HASH (executable, not .d or .o files)
                    if name.starts_with("fake_claude-")
                        && !name.contains('.')
                        && path.is_file()
                    {
                        // Verify it's executable on Unix
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            if let Ok(metadata) = path.metadata() {
                                if metadata.permissions().mode() & 0o111 != 0 {
                                    return path;
                                }
                            }
                        }
                        #[cfg(not(unix))]
                        {
                            return path;
                        }
                    }
                }
            }
        }
    }

    // Return default path (will fail at runtime if not found)
    base.join("target/debug/fake_claude")
}

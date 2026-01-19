# Phase 7: E2E Testing Framework - Research

**Researched:** 2026-01-19 (updated)
**Domain:** End-to-end testing infrastructure with fake Claude simulation
**Confidence:** MEDIUM

## Summary

This research investigates the technical foundations for building a comprehensive E2E testing framework for rslph. The framework requires a fake Claude process that outputs stream-json format matching the real Claude CLI, workspace fixtures, and TUI integration testing.

Key findings:
1. Claude CLI's stream-json format uses JSONL with event types: user, assistant, system, result, summary - already implemented in `src/subprocess/stream_json.rs`
2. **All-Rust approach** recommended: fake-claude as a Rust binary, tests using ratatui-testlib for TUI testing
3. ratatui-testlib (v0.1.0, Dec 2025) provides TuiTestHarness for PTY-based testing with wait_for() and send_text() APIs
4. Rust tests can share types with main crate (StreamJsonEvent, content block types)

**Primary recommendation:** Use Rust for all testing - fake-claude as a test binary, ratatui-testlib for TUI E2E tests, TestBackend for widget unit tests.

## Standard Stack

### Rust Testing Stack (All-Rust Approach)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui-testlib | 0.1.0 | PTY-based TUI testing | TuiTestHarness spawns app, wait_for screen content |
| ratatui TestBackend | 0.30+ | Widget unit testing | Built into ratatui, renders to buffer |
| tempfile | 3.x | Temp directory management | Already in dev-dependencies |
| assert_cmd | 2.x | CLI binary testing | Standard for Rust CLI testing |
| assert_fs | 1.x | Filesystem fixtures | Pairs with assert_cmd |
| insta | 2.x | Snapshot testing | Visual TUI regression testing |
| serde_json | 1.x | JSON generation | Already in dependencies |

**Installation:**
```bash
cargo add --dev ratatui-testlib assert_cmd assert_fs insta
```

## Architecture Patterns

### Recommended Project Structure (All-Rust)
```
tests/
  fake_claude/              # Fake Claude test binary
    mod.rs                  # Module with builder API
    scenario.rs             # Scenario builder pattern
    stream_json.rs          # Re-use/extend main crate types
  fake_claude.rs            # Binary entry point (uses tests/fake_claude/)
  e2e/
    mod.rs                  # Test module
    test_basic_loop.rs      # Basic loop scenarios
    test_edge_cases.rs      # Timeout, crash, malformed
    test_multi_invoke.rs    # Multi-invocation scenarios
    test_tui.rs             # TUI tests with ratatui-testlib
    helpers.rs              # Assertion helpers
    fixtures.rs             # Workspace fixtures

src/                        # Existing Rust TUI tests (inline)
  tui/
    widgets/
      progress_bar.rs       # Contains #[cfg(test)] mod tests
      status_bar.rs         # Contains #[cfg(test)] mod tests
```

### Pattern 1: Fake Claude as Rust Binary

**What:** A separate test binary that acts as fake Claude CLI
**When to use:** All E2E tests needing deterministic Claude output
**Why Rust:** Shares types with main crate, single ecosystem, no Python dependency

```rust
// tests/fake_claude.rs - Binary entry point
use std::env;
use std::fs;
use std::io::{self, Write};

fn main() {
    // Read scenario config from file path in env var
    let config_path = env::var("FAKE_CLAUDE_CONFIG")
        .expect("FAKE_CLAUDE_CONFIG must be set");
    let config: FakeClaudeConfig = serde_json::from_str(
        &fs::read_to_string(config_path).unwrap()
    ).unwrap();

    // Track invocation count
    let invocation = increment_invocation_counter(&config.counter_path);

    // Get response for this invocation
    if let Some(response) = config.invocations.get(invocation - 1) {
        for event in &response.events {
            // Apply delay if configured
            if let Some(delay) = response.delay_ms {
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            // Output stream-json line
            println!("{}", serde_json::to_string(&event).unwrap());
            io::stdout().flush().unwrap();

            // Simulate crash if configured
            if response.crash_after_events == Some(events_output) {
                std::process::exit(1);
            }
        }
    }

    std::process::exit(0);
}
```

### Pattern 2: Scenario Builder API in Rust

**What:** Fluent builder pattern for configuring fake Claude behavior
**When to use:** Test setup for E2E scenarios

```rust
// tests/fake_claude/scenario.rs
use std::path::PathBuf;
use tempfile::TempDir;

pub struct ScenarioBuilder {
    invocations: Vec<InvocationConfig>,
    current_invocation: Option<InvocationConfig>,
    temp_dir: TempDir,
}

impl ScenarioBuilder {
    pub fn new() -> Self {
        Self {
            invocations: vec![],
            current_invocation: Some(InvocationConfig::default()),
            temp_dir: TempDir::new().unwrap(),
        }
    }

    /// Add text response to current invocation
    pub fn respond_with_text(mut self, text: &str) -> Self {
        let inv = self.current_invocation.as_mut().unwrap();
        inv.events.push(StreamJsonEvent::assistant_text(text));
        self
    }

    /// Add Read tool use to current invocation
    pub fn uses_read(mut self, path: &str) -> Self {
        let inv = self.current_invocation.as_mut().unwrap();
        inv.events.push(StreamJsonEvent::tool_use("Read", json!({
            "file_path": path
        })));
        self
    }

    /// Add Write tool use to current invocation
    pub fn uses_write(mut self, path: &str, content: &str) -> Self {
        let inv = self.current_invocation.as_mut().unwrap();
        inv.events.push(StreamJsonEvent::tool_use("Write", json!({
            "file_path": path,
            "content": content
        })));
        self
    }

    /// Add Bash tool use
    pub fn uses_bash(mut self, command: &str) -> Self {
        let inv = self.current_invocation.as_mut().unwrap();
        inv.events.push(StreamJsonEvent::tool_use("Bash", json!({
            "command": command
        })));
        self
    }

    /// Add Edit tool use
    pub fn uses_edit(mut self, path: &str, old: &str, new: &str) -> Self {
        let inv = self.current_invocation.as_mut().unwrap();
        inv.events.push(StreamJsonEvent::tool_use("Edit", json!({
            "file_path": path,
            "old_string": old,
            "new_string": new
        })));
        self
    }

    /// Start configuring next invocation
    pub fn next_invocation(mut self) -> Self {
        if let Some(inv) = self.current_invocation.take() {
            self.invocations.push(inv);
        }
        self.current_invocation = Some(InvocationConfig::default());
        self
    }

    /// Configure delay between events (ms)
    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.current_invocation.as_mut().unwrap().delay_ms = Some(delay_ms);
        self
    }

    /// Configure crash after N events
    pub fn crash_after(mut self, n: usize) -> Self {
        self.current_invocation.as_mut().unwrap().crash_after_events = Some(n);
        self
    }

    /// Build and return path to fake executable
    pub fn build(mut self) -> FakeClaudeHandle {
        // Finalize current invocation
        if let Some(inv) = self.current_invocation.take() {
            self.invocations.push(inv);
        }

        // Write config file
        let config_path = self.temp_dir.path().join("config.json");
        let counter_path = self.temp_dir.path().join("invocation_count");

        let config = FakeClaudeConfig {
            invocations: self.invocations,
            counter_path: counter_path.clone(),
        };

        std::fs::write(&config_path, serde_json::to_string(&config).unwrap()).unwrap();

        // Return handle with paths
        FakeClaudeHandle {
            executable_path: get_fake_claude_binary_path(),
            config_path,
            counter_path,
            _temp_dir: self.temp_dir, // Keep alive
        }
    }
}

pub struct FakeClaudeHandle {
    pub executable_path: PathBuf,
    pub config_path: PathBuf,
    counter_path: PathBuf,
    _temp_dir: TempDir,
}

impl FakeClaudeHandle {
    /// Get invocation count for assertions
    pub fn invocation_count(&self) -> usize {
        std::fs::read_to_string(&self.counter_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    }

    /// Environment variables to pass to rslph
    pub fn env_vars(&self) -> Vec<(&str, &Path)> {
        vec![
            ("FAKE_CLAUDE_CONFIG", self.config_path.as_path()),
        ]
    }
}
```

### Pattern 3: TUI Testing with ratatui-testlib

**What:** PTY-based TUI integration testing
**When to use:** Testing actual TUI behavior, keyboard interaction, visual output

```rust
// tests/e2e/test_tui.rs
use ratatui_testlib::TuiTestHarness;
use std::time::Duration;

#[tokio::test]
async fn test_tui_shows_iteration_progress() {
    // Set up fake Claude
    let fake = ScenarioBuilder::new()
        .respond_with_text("Working on task 1...")
        .uses_write("PROGRESS.md", "- [x] Task 1\n- [ ] Task 2")
        .build();

    // Set up workspace
    let workspace = WorkspaceBuilder::new()
        .with_progress_file("- [ ] Task 1\n- [ ] Task 2")
        .build();

    // Create TUI test harness (80x24 terminal)
    let mut harness = TuiTestHarness::new(80, 24).unwrap();

    // Spawn rslph with fake Claude
    harness.spawn_with_env(
        &get_rslph_binary_path(),
        &["build", "--claude-path", fake.executable_path.to_str().unwrap()],
        workspace.path(),
        fake.env_vars(),
    ).unwrap();

    // Wait for iteration indicator
    harness.wait_for(|state| {
        state.contents().contains("Iteration 1")
    }, Duration::from_secs(5)).await.unwrap();

    // Verify progress shows in TUI
    harness.wait_for(|state| {
        state.contents().contains("Task 1")
    }, Duration::from_secs(2)).await.unwrap();

    // Send quit key
    harness.send_text("q").unwrap();

    // Wait for exit
    harness.wait_for_exit(Duration::from_secs(2)).await.unwrap();
}

#[tokio::test]
async fn test_tui_scroll_keybindings() {
    let fake = ScenarioBuilder::new()
        .respond_with_text("Line 1\nLine 2\nLine 3\n...")  // Long output
        .build();

    let workspace = WorkspaceBuilder::new()
        .with_progress_file("- [ ] Task 1")
        .build();

    let mut harness = TuiTestHarness::new(80, 10).unwrap(); // Small height

    harness.spawn_with_env(
        &get_rslph_binary_path(),
        &["build", "--claude-path", fake.executable_path.to_str().unwrap()],
        workspace.path(),
        fake.env_vars(),
    ).unwrap();

    // Wait for content
    harness.wait_for(|state| {
        state.contents().contains("Line 1")
    }, Duration::from_secs(5)).await.unwrap();

    // Scroll down
    harness.send_text("j").unwrap();

    // Verify scroll happened (Line 1 may be off-screen)
    tokio::time::sleep(Duration::from_millis(100)).await;
    let state = harness.current_state();

    // Press q to quit
    harness.send_text("q").unwrap();
}
```

### Pattern 4: Workspace Fixture Builder

**What:** Isolated workspace with git and config for testing
**When to use:** All E2E tests needing file system isolation

```rust
// tests/e2e/fixtures.rs
use tempfile::TempDir;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct WorkspaceBuilder {
    temp_dir: TempDir,
    init_git: bool,
    config: Option<String>,
    progress_content: Option<String>,
    source_files: Vec<(PathBuf, String)>,
}

impl WorkspaceBuilder {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().unwrap(),
            init_git: true,
            config: None,
            progress_content: None,
            source_files: vec![],
        }
    }

    pub fn with_progress_file(mut self, content: &str) -> Self {
        self.progress_content = Some(content.to_string());
        self
    }

    pub fn with_source_file(mut self, path: &str, content: &str) -> Self {
        self.source_files.push((PathBuf::from(path), content.to_string()));
        self
    }

    pub fn with_config(mut self, config_toml: &str) -> Self {
        self.config = Some(config_toml.to_string());
        self
    }

    pub fn without_git(mut self) -> Self {
        self.init_git = false;
        self
    }

    pub fn build(self) -> Workspace {
        let path = self.temp_dir.path();

        // Initialize git if requested
        if self.init_git {
            Command::new("git")
                .args(["init"])
                .current_dir(path)
                .output()
                .expect("Failed to init git");

            Command::new("git")
                .args(["config", "user.email", "test@test.com"])
                .current_dir(path)
                .output()
                .expect("Failed to set git email");

            Command::new("git")
                .args(["config", "user.name", "Test"])
                .current_dir(path)
                .output()
                .expect("Failed to set git name");
        }

        // Write config
        let config_dir = path.join(".rslph");
        std::fs::create_dir_all(&config_dir).unwrap();
        let config_content = self.config.unwrap_or_else(|| {
            "[rslph]\nclaude_path = \"claude\"\n".to_string()
        });
        std::fs::write(config_dir.join("config.toml"), &config_content).unwrap();

        // Write progress file
        if let Some(content) = self.progress_content {
            std::fs::write(path.join("PROGRESS.md"), &content).unwrap();
        }

        // Write source files
        for (rel_path, content) in self.source_files {
            let full_path = path.join(&rel_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(full_path, content).unwrap();
        }

        Workspace {
            _temp_dir: self.temp_dir,
            path: path.to_path_buf(),
        }
    }
}

pub struct Workspace {
    _temp_dir: TempDir, // Keep alive
    path: PathBuf,
}

impl Workspace {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read_file(&self, rel_path: &str) -> String {
        std::fs::read_to_string(self.path.join(rel_path)).unwrap()
    }

    pub fn file_exists(&self, rel_path: &str) -> bool {
        self.path.join(rel_path).exists()
    }
}
```

### Pattern 5: Verifier Helpers

```rust
// tests/e2e/helpers.rs

/// Assert that a task is marked complete in progress file
pub fn assert_task_complete(workspace: &Workspace, task_pattern: &str) {
    let content = workspace.read_file("PROGRESS.md");
    let pattern = format!("- [x] {}", task_pattern);
    assert!(
        content.contains(&pattern),
        "Expected task '{}' to be complete in PROGRESS.md:\n{}",
        task_pattern, content
    );
}

/// Assert that RALPH_DONE marker exists
pub fn assert_ralph_done(workspace: &Workspace) {
    let content = workspace.read_file("PROGRESS.md");
    assert!(
        content.contains("RALPH_DONE"),
        "Expected RALPH_DONE in PROGRESS.md:\n{}",
        content
    );
}

/// Assert file contains content
pub fn assert_file_contains(workspace: &Workspace, path: &str, expected: &str) {
    let content = workspace.read_file(path);
    assert!(
        content.contains(expected),
        "Expected '{}' in {}:\n{}",
        expected, path, content
    );
}

/// Assert git commit exists with message pattern
pub fn assert_git_commit_exists(workspace: &Workspace, message_pattern: &str) {
    let output = std::process::Command::new("git")
        .args(["log", "--oneline", "-1"])
        .current_dir(workspace.path())
        .output()
        .expect("Failed to run git log");

    let log = String::from_utf8_lossy(&output.stdout);
    assert!(
        log.contains(message_pattern),
        "Expected commit with '{}' in git log:\n{}",
        message_pattern, log
    );
}
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Temp directories | Manual tempfile | tempfile::TempDir | Automatic cleanup, RAII |
| PTY testing | Manual pty crate | ratatui-testlib | wait_for, send_text API |
| Stream-json generation | New types | Extend existing StreamJsonEvent | Type reuse |
| Binary path finding | Hardcoded paths | cargo_test_binary crate | Cross-platform |
| CLI assertions | Raw process output | assert_cmd | Better error messages |

## Common Pitfalls

### Pitfall 1: Stream-JSON Format Mismatch
**What goes wrong:** Fake Claude outputs JSON that doesn't match real Claude CLI format
**How to avoid:** Reuse types from `src/subprocess/stream_json.rs`

### Pitfall 2: Test Binary Not Built
**What goes wrong:** Tests fail because fake_claude binary doesn't exist
**How to avoid:** Use `cargo test --all-targets` or build helper function

### Pitfall 3: Invocation Counter Race Condition
**What goes wrong:** Parallel tests share invocation counter file
**How to avoid:** Use unique counter file per test (in TempDir)

### Pitfall 4: Terminal Size Mismatch
**What goes wrong:** TUI tests pass locally, fail in CI
**How to avoid:** Always use explicit dimensions in TuiTestHarness::new(80, 24)

### Pitfall 5: Async Test Timeouts
**What goes wrong:** Tests hang waiting for TUI content
**How to avoid:** Use reasonable timeouts in wait_for(), add tokio::time::timeout wrapper

## ratatui-testlib API Reference

From docs.rs/ratatui-testlib (v0.1.0):

```rust
// Create harness with terminal dimensions
let mut harness = TuiTestHarness::new(80, 24)?;

// Spawn application
harness.spawn(cmd)?;

// Wait for screen content condition
harness.wait_for(|state| state.contents().contains("Welcome"), Duration::from_secs(5))?;

// Send text input
harness.send_text("hello")?;

// Get current screen state
let state = harness.current_state();
let contents = state.contents(); // Full screen as string

// Wait for exit
harness.wait_for_exit(Duration::from_secs(2)).await?;
```

## Code Examples

### Stream-JSON Event Types (from existing code)

```rust
// Source: src/subprocess/stream_json.rs (existing implementation)

// Text response
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hello"}],"model":"claude-opus-4.5","stop_reason":"end_turn"}}

// Tool use
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"tool1","name":"Read","input":{"file_path":"/tmp/test"}}],"stop_reason":"tool_use"}}

// Thinking block
{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"Let me analyze..."}]}}
```

## Cargo.toml Dev Dependencies

```toml
[dev-dependencies]
ratatui-testlib = "0.1"
assert_cmd = "2"
assert_fs = "1"
tempfile = "3"
insta = "2"
tokio = { version = "1", features = ["full", "test-util"] }

[[test]]
name = "fake_claude"
path = "tests/fake_claude.rs"
```

## Sources

### Primary (HIGH confidence)
- src/subprocess/stream_json.rs - Existing stream-json implementation
- [ratatui-testlib docs](https://docs.rs/ratatui-testlib/latest/ratatui_testlib/) - TuiTestHarness API
- [ratatui TestBackend](https://docs.rs/ratatui/0.30/ratatui/backend/struct.TestBackend.html) - Widget testing
- [tempfile docs](https://docs.rs/tempfile/latest/tempfile/) - TempDir API

### Secondary (MEDIUM confidence)
- [ratatui-testlib lib.rs](https://lib.rs/crates/ratatui-testlib) - v0.1.0, Dec 2025
- [assert_cmd docs](https://docs.rs/assert_cmd/latest/assert_cmd/) - CLI testing patterns

## Metadata

**Confidence breakdown:**
- Rust test structure: HIGH - Standard cargo patterns
- Stream-json reuse: HIGH - Types already exist in crate
- ratatui-testlib: MEDIUM - v0.1.0 is new but API is straightforward
- Builder pattern: HIGH - Standard Rust pattern

**Research date:** 2026-01-19 (updated for all-Rust approach)
**Valid until:** 2026-02-19 (30 days)

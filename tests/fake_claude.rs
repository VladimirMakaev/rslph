//! Fake Claude CLI binary for E2E testing.
//!
//! This binary outputs deterministic stream-json responses based on configuration.
//! It reads configuration from FAKE_CLAUDE_CONFIG environment variable.

mod fake_claude_lib;

use fake_claude_lib::config::FakeClaudeConfig;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

fn main() {
    let config_path = env::var("FAKE_CLAUDE_CONFIG")
        .expect("FAKE_CLAUDE_CONFIG env var must be set");

    let config: FakeClaudeConfig = serde_json::from_str(
        &fs::read_to_string(&config_path)
            .expect("Failed to read config file"),
    )
    .expect("Failed to parse config");

    // Read and increment counter
    let invocation = increment_counter(&config.counter_path);

    // Get response for this invocation (0-indexed)
    let Some(inv_config) = config.invocations.get(invocation) else {
        // Silent pass-through for unconfigured invocations
        return;
    };

    // Output raw lines first (for malformed output tests)
    for raw in &inv_config.raw_lines {
        println!("{}", raw);
        io::stdout().flush().unwrap();
    }

    for (i, event) in inv_config.events.iter().enumerate() {
        if let Some(delay) = inv_config.delay_ms {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }

        println!("{}", serde_json::to_string(&event).unwrap());
        io::stdout().flush().unwrap();

        if inv_config.crash_after_events == Some(i + 1) {
            std::process::exit(1);
        }
    }

    // Exit with configured code or 0
    std::process::exit(inv_config.exit_code.unwrap_or(0));
}

/// Increment the invocation counter and return the previous value.
fn increment_counter(path: &Path) -> usize {
    let current = fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    fs::write(path, format!("{}", current + 1)).ok();
    current
}

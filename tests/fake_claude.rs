//! Fake Claude CLI binary for E2E testing.
//!
//! This binary outputs deterministic stream-json responses based on configuration.
//! It reads configuration from FAKE_CLAUDE_CONFIG environment variable.
//!
//! When `execute_tools: true` is set in the config, this binary will actually
//! execute Write and Bash tool_use events, making it suitable for E2E tests
//! that need the fake Claude to produce real artifacts.

mod fake_claude_lib;

use fake_claude_lib::config::FakeClaudeConfig;
use fake_claude_lib::stream_json::{MessageContentOutput, StreamEventOutput};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn main() {
    let config_path = match env::var("FAKE_CLAUDE_CONFIG") {
        Ok(path) => path,
        Err(_) => {
            // Exit gracefully when not configured (e.g., when cargo runs this as a test target)
            return;
        }
    };

    let config: FakeClaudeConfig = serde_json::from_str(
        &fs::read_to_string(&config_path).expect("Failed to read config file"),
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

    // Output stderr lines (for testing stderr handling)
    for stderr_line in &inv_config.stderr_lines {
        eprintln!("{}", stderr_line);
        io::stderr().flush().unwrap();
    }

    // Apply initial delay before outputting any events
    if let Some(initial_delay) = inv_config.initial_delay_ms {
        std::thread::sleep(std::time::Duration::from_millis(initial_delay));
    }

    for (i, event) in inv_config.events.iter().enumerate() {
        if let Some(delay) = inv_config.delay_ms {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }

        println!("{}", serde_json::to_string(&event).unwrap());
        io::stdout().flush().unwrap();

        // Execute tools if configured to do so
        if inv_config.execute_tools.unwrap_or(false) {
            execute_tools_in_event(event);
        }

        if inv_config.crash_after_events == Some(i + 1) {
            std::process::exit(1);
        }
    }

    // Exit with configured code or 0
    std::process::exit(inv_config.exit_code.unwrap_or(0));
}

/// Execute Write and Bash tool_use events found in an assistant message.
fn execute_tools_in_event(event: &StreamEventOutput) {
    // Only process assistant messages
    if event.event_type != "assistant" {
        return;
    }

    let Some(ref message) = event.message else {
        return;
    };

    // Get content blocks
    let blocks = match &message.content {
        MessageContentOutput::Blocks(blocks) => blocks,
        MessageContentOutput::Text(_) => return,
    };

    for block in blocks {
        if block.block_type != "tool_use" {
            continue;
        }

        let Some(ref name) = block.name else {
            continue;
        };
        let Some(ref input) = block.input else {
            continue;
        };

        match name.as_str() {
            "Write" => {
                if let (Some(file_path), Some(content)) = (
                    input.get("file_path").and_then(|v| v.as_str()),
                    input.get("content").and_then(|v| v.as_str()),
                ) {
                    // Create parent directories if needed
                    if let Some(parent) = Path::new(file_path).parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if let Err(e) = fs::write(file_path, content) {
                        eprintln!("fake_claude: Write failed for {}: {}", file_path, e);
                    }
                }
            }
            "Edit" => {
                if let (Some(file_path), Some(old_string), Some(new_string)) = (
                    input.get("file_path").and_then(|v| v.as_str()),
                    input.get("old_string").and_then(|v| v.as_str()),
                    input.get("new_string").and_then(|v| v.as_str()),
                ) {
                    // Read file, replace old_string with new_string, write back
                    if let Ok(content) = fs::read_to_string(file_path) {
                        let new_content = content.replacen(old_string, new_string, 1);
                        if let Err(e) = fs::write(file_path, new_content) {
                            eprintln!("fake_claude: Edit failed for {}: {}", file_path, e);
                        }
                    }
                }
            }
            "Bash" => {
                if let Some(command) = input.get("command").and_then(|v| v.as_str()) {
                    // Execute bash command
                    let _ = Command::new("sh").arg("-c").arg(command).status();
                }
            }
            _ => {}
        }
    }
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

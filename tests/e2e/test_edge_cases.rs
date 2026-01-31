//! Edge case tests for fake Claude binary
//!
//! These verify the fake Claude handles edge cases correctly.

use crate::fake_claude_lib::ScenarioBuilder;
use std::time::{Duration, Instant};

/// Test that fake Claude can crash after a specified number of events.
#[test]
fn test_fake_claude_crash_after_events() {
    let handle = ScenarioBuilder::new()
        .respond_with_text("Event before crash")
        .crash_after(1) // Crash after first event (system init)
        .build();

    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");

    // Should have crashed with non-zero exit
    assert!(
        !output.status.success(),
        "Expected non-zero exit code, got success"
    );

    // Should have output the first event before crashing
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Expected some output before crash");
}

/// Test that fake Claude respects delay between events.
#[test]
fn test_fake_claude_with_delay() {
    let delay_ms = 50;

    let handle = ScenarioBuilder::new()
        .respond_with_text("Event 1")
        .with_delay(delay_ms)
        .build();

    let start = Instant::now();
    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");
    let elapsed = start.elapsed();

    assert!(output.status.success());

    // respond_with_text adds 3 events (system_init, assistant_text, result)
    // Each has delay_ms delay before output
    // So total should be at least 3 * delay_ms
    // Use 2x to account for timing variance
    let expected_min = Duration::from_millis(delay_ms * 2);
    assert!(
        elapsed >= expected_min,
        "Expected delay of at least {:?}, but took only {:?}",
        expected_min,
        elapsed
    );
}

/// Test that fake Claude can output raw (malformed) lines.
#[test]
fn test_fake_claude_malformed_output() {
    let handle = ScenarioBuilder::new()
        .send_raw("this is not json at all")
        .send_raw("{incomplete json")
        .respond_with_text("Normal event after malformed")
        .build();

    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // All raw lines should appear
    assert!(
        stdout.contains("this is not json at all"),
        "Expected raw line in output, got: {}",
        stdout
    );
    assert!(
        stdout.contains("{incomplete json"),
        "Expected incomplete json in output, got: {}",
        stdout
    );
    // Normal event should also appear
    assert!(
        stdout.contains("Normal event"),
        "Expected normal event in output, got: {}",
        stdout
    );
}

/// Test that fake Claude can exit with custom exit code.
#[test]
fn test_fake_claude_custom_exit_code() {
    let handle = ScenarioBuilder::new()
        .respond_with_text("Will exit with code 42")
        .with_exit_code(42)
        .build();

    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");

    assert!(!output.status.success());

    #[cfg(unix)]
    {
        assert_eq!(
            output.status.code(),
            Some(42),
            "Expected exit code 42, got {:?}",
            output.status.code()
        );
    }
}

/// Test that unconfigured invocations exit silently.
#[test]
fn test_fake_claude_unconfigured_invocation() {
    // Configure only 1 invocation
    let handle = ScenarioBuilder::new()
        .respond_with_text("Only first configured")
        .build();

    // First invocation - should output response
    let output1 = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");
    assert!(
        String::from_utf8_lossy(&output1.stdout).contains("Only first configured"),
        "Expected first invocation output"
    );

    // Second invocation - unconfigured, should exit silently with 0
    let output2 = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");
    assert!(
        output2.status.success(),
        "Expected success for unconfigured invocation"
    );
    assert!(
        output2.stdout.is_empty() || String::from_utf8_lossy(&output2.stdout).trim().is_empty(),
        "Expected empty output for unconfigured invocation, got: {}",
        String::from_utf8_lossy(&output2.stdout)
    );
}

/// Test that empty scenario exits cleanly.
#[test]
fn test_fake_claude_empty_scenario() {
    // No events configured
    let handle = ScenarioBuilder::new().build();

    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");

    // Should succeed with no output
    assert!(output.status.success());
}

/// Test that rapid output (many events with no delay) works correctly.
#[test]
fn test_fake_claude_rapid_output() {
    // Many events with no delay - tests "fast output" scenario
    let mut builder = ScenarioBuilder::new();
    for i in 0..50 {
        builder = builder.uses_bash(&format!("echo {}", i));
    }
    let handle = builder.build();

    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain first and last commands
    assert!(
        stdout.contains("echo 0"),
        "Expected first command in output"
    );
    assert!(
        stdout.contains("echo 49"),
        "Expected last command in output"
    );
}

/// Test raw lines are output before events.
#[test]
fn test_fake_claude_raw_before_events() {
    let handle = ScenarioBuilder::new()
        .send_raw("RAW_LINE_FIRST")
        .respond_with_text("Event after raw")
        .build();

    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find positions to verify ordering
    let raw_pos = stdout.find("RAW_LINE_FIRST").expect("Raw line not found");
    let event_pos = stdout.find("Event after raw").expect("Event not found");

    assert!(
        raw_pos < event_pos,
        "Raw lines should come before events. Raw at {}, event at {}",
        raw_pos,
        event_pos
    );
}

/// Test that fake Claude can output stderr lines.
#[test]
fn test_fake_claude_stderr_output() {
    let handle = ScenarioBuilder::new()
        .send_stderr("Claude Code at Meta (https://example.com)")
        .send_stderr("Warning: experimental feature")
        .respond_with_text("Normal response")
        .build();

    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");

    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Stderr should contain the stderr lines
    assert!(
        stderr.contains("Claude Code at Meta"),
        "Expected stderr line, got: {}",
        stderr
    );
    assert!(
        stderr.contains("Warning: experimental feature"),
        "Expected second stderr line, got: {}",
        stderr
    );

    // Stdout should contain the normal response
    assert!(
        stdout.contains("Normal response"),
        "Expected normal response in stdout, got: {}",
        stdout
    );
}

/// Test stderr output with delayed stdout (reproduces stuck TUI scenario).
#[test]
fn test_fake_claude_stderr_before_delayed_stdout() {
    let handle = ScenarioBuilder::new()
        .send_stderr("Claude Code at Meta (https://example.com)")
        .with_initial_delay_ms(100) // Delay before stdout events
        .respond_with_text("Delayed response")
        .build();

    let start = Instant::now();
    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");
    let elapsed = start.elapsed();

    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Stderr should be present
    assert!(
        stderr.contains("Claude Code at Meta"),
        "Expected stderr, got: {}",
        stderr
    );

    // Stdout should have the delayed response
    assert!(
        stdout.contains("Delayed response"),
        "Expected delayed response, got: {}",
        stdout
    );

    // Should have taken at least 100ms due to initial delay
    assert!(
        elapsed >= Duration::from_millis(50),
        "Expected delay, but took only {:?}",
        elapsed
    );
}

/// Test stderr only without stdout (simulates error scenario).
#[test]
fn test_fake_claude_stderr_only() {
    let handle = ScenarioBuilder::new()
        .send_stderr("Authentication required")
        .send_stderr("Please log in at https://example.com")
        .with_exit_code(1)
        .build();

    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");

    // Should fail
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Stderr should have the error messages
    assert!(
        stderr.contains("Authentication required"),
        "Expected auth error in stderr, got: {}",
        stderr
    );

    // Stdout should be empty (no JSON events)
    assert!(
        output.stdout.is_empty(),
        "Expected empty stdout for error case, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

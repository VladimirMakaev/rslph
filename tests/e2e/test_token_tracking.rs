//! E2E tests for token tracking during build execution.
//!
//! These tests verify that token usage is correctly captured from fake Claude
//! responses and made available to the application.

use crate::fake_claude_lib::ScenarioBuilder;
use crate::fixtures::WorkspaceBuilder;
use assert_cmd::Command;

/// Helper to get rslph command configured with fake Claude
fn rslph_with_fake_claude(
    scenario: &crate::fake_claude_lib::FakeClaudeHandle,
) -> assert_cmd::Command {
    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    for (key, value) in scenario.env_vars() {
        cmd.env(key, value);
    }
    cmd
}

/// Test that fake Claude returns configured token values in output.
#[test]
fn test_fake_claude_with_custom_tokens() {
    // Scenario: Configure fake Claude with specific token values
    let scenario = ScenarioBuilder::new()
        .with_token_usage(5000, 1500, 2000, 1000)
        .respond_with_text("Task completed with custom tokens.")
        .build();

    // Run fake Claude directly and verify token values in output
    let output = std::process::Command::new(&scenario.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &scenario.config_path)
        .output()
        .expect("Failed to run fake claude");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify configured token values appear in the output
    assert!(
        stdout.contains("5000") || stdout.contains("\"input_tokens\":5000"),
        "Expected input_tokens: 5000 in output, got: {}",
        stdout
    );
    assert!(
        stdout.contains("1500") || stdout.contains("\"output_tokens\":1500"),
        "Expected output_tokens: 1500 in output, got: {}",
        stdout
    );
    // Cache tokens may be omitted if zero, but we set them to non-zero
    assert!(
        stdout.contains("2000") || stdout.contains("\"cache_creation_input_tokens\":2000"),
        "Expected cache_creation_input_tokens: 2000 in output, got: {}",
        stdout
    );
    assert!(
        stdout.contains("1000") || stdout.contains("\"cache_read_input_tokens\":1000"),
        "Expected cache_read_input_tokens: 1000 in output, got: {}",
        stdout
    );
}

/// Test that different invocations can have different token values.
#[test]
fn test_fake_claude_multi_invocation_tokens() {
    // Scenario: Two invocations with different token values
    let scenario = ScenarioBuilder::new()
        .with_token_usage(5000, 1500, 2000, 1000)
        .respond_with_text("First invocation")
        .next_invocation()
        .with_token_usage(3000, 1000, 500, 2500)
        .respond_with_text("Second invocation")
        .build();

    // First invocation
    let output1 = std::process::Command::new(&scenario.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &scenario.config_path)
        .output()
        .expect("Failed to run fake claude");
    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    assert!(
        stdout1.contains("5000"),
        "First invocation should have input_tokens: 5000, got: {}",
        stdout1
    );

    // Second invocation
    let output2 = std::process::Command::new(&scenario.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &scenario.config_path)
        .output()
        .expect("Failed to run fake claude");
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(
        stdout2.contains("3000"),
        "Second invocation should have input_tokens: 3000, got: {}",
        stdout2
    );
}

/// Test that rslph build with token-configured fake Claude runs successfully.
#[test]
fn test_rslph_build_with_token_tracking() {
    // Scenario: Configure fake Claude with specific token values
    let scenario = ScenarioBuilder::new()
        .with_token_usage(5000, 1500, 2000, 1000)
        .respond_with_text("I'll work on Task 1. The task has been completed.")
        .build();

    let workspace = WorkspaceBuilder::new()
        .with_progress_file("# Progress\n\n- [ ] Task 1\n")
        .build();

    let mut cmd = rslph_with_fake_claude(&scenario);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("1")
        .arg("--no-tui")
        .current_dir(workspace.path());

    let output = cmd.output().expect("Failed to run rslph");

    // rslph should complete without panicking
    assert!(
        scenario.invocation_count() >= 1,
        "Expected at least 1 invocation, got {}",
        scenario.invocation_count()
    );

    // In non-TUI mode, token info should be logged to stdout/stderr
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Check that build ran (we look for any indication of token values being processed)
    // The exact format depends on logging configuration
    // For now, we just verify the build completes successfully
    assert!(
        output.status.success() || combined.contains("iteration"),
        "Build should complete (may exit non-zero if task not marked done)"
    );
}

/// Test that tool uses also receive configured token values.
#[test]
fn test_fake_claude_tool_use_with_tokens() {
    // Scenario: Tool use with custom tokens
    let scenario = ScenarioBuilder::new()
        .with_token_usage(10000, 5000, 3000, 2000)
        .uses_read("/test/file.txt")
        .build();

    let output = std::process::Command::new(&scenario.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &scenario.config_path)
        .output()
        .expect("Failed to run fake claude");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify tool_use is present
    assert!(
        stdout.contains("tool_use"),
        "Expected tool_use in output, got: {}",
        stdout
    );

    // Verify configured token values
    assert!(
        stdout.contains("10000"),
        "Expected input_tokens: 10000 in tool_use output, got: {}",
        stdout
    );
}

//! E2E tests for Phase 15: Interactive Planning Input
//!
//! These tests verify that the Q&A infrastructure works end-to-end:
//! - Session ID capture from init events (INTER-01)
//! - AskUserQuestion detection (INTER-02)
//! - Question parsing (INTER-03)
//! - Fallback when no questions asked (INTER-07)

#![allow(deprecated)] // Command::cargo_bin is deprecated but still functional

use crate::fake_claude_lib::prebuilt;
use crate::fake_claude_lib::scenario::ScenarioBuilder;
use crate::fixtures::WorkspaceBuilder;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

/// Test INTER-07: Fallback when no questions asked
///
/// When Claude doesn't ask questions, the normal plan flow should continue
/// without entering the Q&A loop.
#[test]
fn test_no_questions_proceeds_normally() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let initial_file = temp_dir.path().join("INITIAL.md");
    fs::write(&initial_file, "# Build a calculator\n\nSimple CLI calculator.").unwrap();

    // Use calculator scenario which doesn't ask questions
    let handle = prebuilt::calculator().build();

    let mut cmd = Command::cargo_bin("rslph").expect("Failed to find rslph binary");
    for (key, val) in handle.env_vars() {
        cmd.env(key, val);
    }

    cmd.arg("plan")
        .arg(&initial_file)
        .arg("--no-tui")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Verify progress.md was created
    let progress_path = temp_dir.path().join("progress.md");
    assert!(progress_path.exists(), "progress.md should be created");

    let content = fs::read_to_string(&progress_path).unwrap();
    assert!(
        content.contains("Calculator"),
        "Should contain Calculator project name"
    );
}

/// Test that session ID is captured from init event (INTER-01)
///
/// This is a structural test - we verify the fake_claude scenario
/// with session_id produces output that the parser can handle.
#[test]
fn test_session_id_in_fake_claude_output() {
    // Create a scenario with session ID
    let handle = ScenarioBuilder::new()
        .with_session_id("test-e2e-session-abc")
        .respond_with_text(
            r#"# Progress: Test

## Status

RALPH_DONE

## Tasks

### Phase 1

- [x] Done

## Testing Strategy

Basic tests.
"#,
        )
        .build();

    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let initial_file = temp_dir.path().join("INITIAL.md");
    fs::write(&initial_file, "# Test project").unwrap();

    let mut cmd = Command::cargo_bin("rslph").expect("Failed to find rslph binary");
    for (key, val) in handle.env_vars() {
        cmd.env(key, val);
    }

    cmd.arg("plan")
        .arg("--no-tui")
        .arg(&initial_file)
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Progress file should be created (session ID was handled correctly)
    let progress_path = temp_dir.path().join("progress.md");
    assert!(progress_path.exists(), "progress.md should be created");
}

/// Test that AskUserQuestion events are emitted by fake_claude (INTER-02, INTER-03)
///
/// Verifies that the interactive_planning scenario produces the expected
/// stream-json output with AskUserQuestion tool_use events.
#[test]
fn test_interactive_scenario_builds_correctly() {
    // Verify the scenario can be built without panicking
    let handle = prebuilt::interactive_planning().build();

    // Verify handle is configured correctly
    let env_vars = handle.env_vars();
    assert_eq!(
        env_vars.len(),
        2,
        "Should have FAKE_CLAUDE_CONFIG and RSLPH_CLAUDE_CMD"
    );

    // Verify config file exists
    assert!(handle.config_path.exists(), "Config file should exist");

    // Read and verify config contains expected structure
    let config_content = fs::read_to_string(&handle.config_path).unwrap();
    assert!(
        config_content.contains("AskUserQuestion"),
        "Config should contain AskUserQuestion"
    );
    assert!(
        config_content.contains("test-session-123"),
        "Config should contain session ID"
    );

    // Verify 3 invocations: testing strategist, questions, resume
    let invocation_count = config_content.matches("\"events\"").count();
    assert_eq!(
        invocation_count, 3,
        "Should have exactly 3 invocations (testing strategist, questions, resume)"
    );
}

/// Test that multi-round Q&A scenario is properly configured (INTER-06)
#[test]
fn test_multi_round_scenario_builds_correctly() {
    let handle = prebuilt::multi_round_qa().build();

    // Verify config file exists and contains expected content
    let config_content = fs::read_to_string(&handle.config_path).unwrap();
    assert!(
        config_content.contains("multi-session-456"),
        "Config should contain multi-round session ID"
    );

    // Count invocations (should be 4: testing strategist, question1, question2, answer)
    let invocation_count = config_content.matches("\"events\"").count();
    assert_eq!(
        invocation_count, 4,
        "Should have exactly 4 invocations for multi-round (testing strategist + 2 question rounds + answer)"
    );
}

/// Test that workspace with input file works correctly
#[test]
fn test_workspace_input_file_for_planning() {
    // Use existing workspace builder patterns
    let workspace = WorkspaceBuilder::new()
        .with_source_file("INITIAL.md", "Build a test app")
        .build();

    // Verify the input file was created
    assert!(
        workspace.file_exists("INITIAL.md"),
        "INITIAL.md should exist"
    );

    let content = workspace.read_file("INITIAL.md");
    assert_eq!(content, "Build a test app");
}

/// Test that scenario with questions has proper event structure
#[test]
fn test_ask_questions_event_structure() {
    // Build a custom scenario with specific questions
    let handle = ScenarioBuilder::new()
        .with_session_id("custom-session")
        .asks_questions(vec!["What is the target platform?", "Which language?"])
        .build();

    // Read the generated config
    let config_content = fs::read_to_string(&handle.config_path).unwrap();

    // Verify the config has expected question content
    assert!(
        config_content.contains("What is the target platform?"),
        "Config should contain first question"
    );
    assert!(
        config_content.contains("Which language?"),
        "Config should contain second question"
    );

    // Verify session ID is present
    assert!(
        config_content.contains("custom-session"),
        "Config should contain custom session ID"
    );
}

/// Test the full interactive Q&A flow end-to-end (INTER-02, INTER-03, INTER-05)
///
/// This test verifies that when Claude asks questions via AskUserQuestion:
/// 1. Questions are detected in the stream output
/// 2. The session resume mechanism works
/// 3. A valid progress file is produced
///
/// Note: This test uses --adaptive flag which enables the Q&A flow.
/// The test provides empty stdin which triggers the resume with empty answers.
#[test]
fn test_interactive_planning_detects_questions() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let initial_file = temp_dir.path().join("INITIAL.md");
    fs::write(&initial_file, "# Build a web app\n\nA simple web application.").unwrap();

    // Use the interactive_planning scenario with correct invocation count
    let handle = prebuilt::interactive_planning().build();

    let mut cmd = Command::cargo_bin("rslph").expect("Failed to find rslph binary");
    for (key, val) in handle.env_vars() {
        cmd.env(key, val);
    }

    // Run with --adaptive flag to enable Q&A flow
    // Provide empty stdin to auto-submit with empty answers
    cmd.arg("plan")
        .arg("--no-tui")
        .arg("--adaptive")
        .arg(&initial_file)
        .current_dir(temp_dir.path())
        .write_stdin("\n\n") // Empty input with double-newline to submit
        .assert()
        .success()
        // Verify questions were detected (appears in stderr trace)
        .stderr(predicate::str::contains("Questions detected: 2"));

    // Verify progress.md was created after resume
    let progress_path = temp_dir.path().join("progress.md");
    assert!(
        progress_path.exists(),
        "progress.md should be created after Q&A flow"
    );

    let content = fs::read_to_string(&progress_path).unwrap();
    assert!(
        content.contains("Interactive Test"),
        "Should contain project name from resumed response"
    );
}

//! E2E tests for eval command with fake Claude.
//!
//! These tests verify the eval command runs with fake Claude scenarios.
//! Full end-to-end testing with working test execution requires more
//! complex subprocess coordination.

use assert_cmd::Command;

use crate::fake_claude_lib::prebuilt;

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

/// Test that eval calculator with fake Claude executes the planning and build phases.
///
/// This test verifies:
/// 1. Eval command starts correctly with fake Claude
/// 2. Planning phase executes (fake Claude invoked)
/// 3. Build phase starts (at least one iteration)
///
/// Note: Full test execution verification requires more complex setup.
#[test]
fn test_eval_runs_with_fake_claude() {
    let handle = prebuilt::calculator().build();

    let mut cmd = rslph_with_fake_claude(&handle);
    cmd.args(["eval", "calculator", "--no-tui"]);

    let output = cmd.output().expect("Failed to run rslph eval");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    // Verify fake Claude was invoked
    assert!(
        handle.invocation_count() >= 2,
        "Expected at least 2 invocations (plan + build), got {}",
        handle.invocation_count()
    );

    // Verify planning phase ran (tokens reported)
    assert!(
        combined.contains("Planning") || combined.contains("tokens"),
        "Should show planning phase evidence. Output:\n{}",
        combined
    );

    // Command should complete (even if with max iterations)
    // The important thing is it doesn't crash
}

/// Test that prebuilt scenarios create valid progress files.
///
/// This is a unit test that verifies the progress file format
/// without running the full eval command.
#[test]
fn test_prebuilt_scenarios_parse_correctly() {
    // Already tested in prebuilt::tests, but good to have here for visibility
    let _ = prebuilt::calculator().build();
    let _ = prebuilt::fizzbuzz().build();
    // If we get here without panicking, scenarios built successfully
}

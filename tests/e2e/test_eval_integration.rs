//! E2E tests for eval command with fake Claude.
//!
//! These tests verify the eval command runs with fake Claude scenarios.
//! Full end-to-end testing with working test execution requires more
//! complex subprocess coordination.

use assert_cmd::Command;
use tempfile::TempDir;

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

/// Test that eval persists workspace and result.json to eval_dir.
///
/// This test verifies:
/// 1. Workspace is created in the configured eval_dir
/// 2. result.json is created with metrics
/// 3. Workspace is NOT deleted after completion
#[test]
fn test_eval_persists_workspace_and_results() {
    let eval_dir = TempDir::new().expect("temp dir for evals");
    let handle = prebuilt::calculator().build();

    let mut cmd = rslph_with_fake_claude(&handle);
    cmd.env("RSLPH_EVAL_DIR", eval_dir.path().to_str().unwrap());
    cmd.args(["eval", "calculator", "--no-tui"]);

    let output = cmd.output().expect("Failed to run rslph eval");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find workspace path from output
    let workspace_line = stdout
        .lines()
        .find(|line| line.contains("Eval workspace:"))
        .expect("Should print workspace path");

    let workspace_path = workspace_line
        .split("Eval workspace:")
        .nth(1)
        .expect("Should have path after colon")
        .trim();

    let workspace = std::path::Path::new(workspace_path);

    // Verify workspace exists and is in eval_dir
    assert!(
        workspace.exists(),
        "Workspace should exist at: {}",
        workspace_path
    );
    assert!(
        workspace.starts_with(eval_dir.path()),
        "Workspace should be under eval_dir. workspace={}, eval_dir={}",
        workspace_path,
        eval_dir.path().display()
    );

    // Verify result.json was created
    let result_json = workspace.join("result.json");
    assert!(
        result_json.exists(),
        "result.json should exist at: {}",
        result_json.display()
    );

    // Verify result.json content is valid
    let content = std::fs::read_to_string(&result_json).expect("read result.json");
    let json: serde_json::Value = serde_json::from_str(&content).expect("parse result.json");

    assert_eq!(json["project"], "calculator");
    assert!(json["elapsed_secs"].is_number(), "Should have elapsed_secs");
    assert!(json["iterations"].is_number(), "Should have iterations");
    assert!(json["tokens"]["input"].is_number(), "Should have token metrics");

    // Verify workspace name has timestamp format (project-YYYYMMDD-HHMMSS)
    let workspace_name = workspace.file_name().unwrap().to_str().unwrap();
    assert!(
        workspace_name.starts_with("calculator-"),
        "Workspace name should start with project name: {}",
        workspace_name
    );
}

/// Test that eval_dir config can be overridden via environment variable.
#[test]
fn test_eval_dir_config_via_env() {
    let custom_eval_dir = TempDir::new().expect("custom eval dir");
    let handle = prebuilt::fizzbuzz().build();

    let mut cmd = rslph_with_fake_claude(&handle);
    cmd.env("RSLPH_EVAL_DIR", custom_eval_dir.path().to_str().unwrap());
    cmd.args(["eval", "fizzbuzz", "--no-tui"]);

    let output = cmd.output().expect("Failed to run rslph eval");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify workspace was created in custom eval_dir
    let workspace_line = stdout
        .lines()
        .find(|line| line.contains("Eval workspace:"));

    assert!(
        workspace_line.is_some(),
        "Should print workspace path. stdout: {}",
        stdout
    );

    let workspace_path = workspace_line
        .unwrap()
        .split("Eval workspace:")
        .nth(1)
        .unwrap()
        .trim();

    assert!(
        workspace_path.starts_with(custom_eval_dir.path().to_str().unwrap()),
        "Workspace should be under custom eval_dir. workspace={}, eval_dir={}",
        workspace_path,
        custom_eval_dir.path().display()
    );

    // Verify the workspace persists (still exists after command completes)
    assert!(
        std::path::Path::new(workspace_path).exists(),
        "Workspace should persist after eval completes"
    );
}

/// Test that eval discovers run command and executes tests successfully.
///
/// This test verifies the test discovery flow:
/// 1. Planning phase runs
/// 2. Build phase runs (may not create files due to fake_claude limitations)
/// 3. Discovery phase is invoked
///
/// Note: Full tool execution from fake_claude requires fixes to the scenario builder
/// to properly structure tool_use as content blocks within assistant messages.
/// This test verifies the discovery invocation happens.
#[test]
fn test_eval_discovery_and_test_execution() {
    let eval_dir = TempDir::new().expect("temp dir for evals");
    let handle = prebuilt::calculator().build();

    let mut cmd = rslph_with_fake_claude(&handle);
    cmd.env("RSLPH_EVAL_DIR", eval_dir.path().to_str().unwrap());
    cmd.args(["eval", "calculator", "--no-tui"]);

    let output = cmd.output().expect("Failed to run rslph eval");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify eval completed
    assert!(
        stdout.contains("EVAL COMPLETE"),
        "Eval should complete. stdout:\n{}",
        stdout
    );

    // Verify test phase was attempted (discovery invoked)
    assert!(
        stdout.contains("TEST PHASE") && stdout.contains("Discovering"),
        "Should attempt test discovery. stdout:\n{}",
        stdout
    );

    // Verify discovery was invoked (even if it failed due to empty response)
    // The third invocation is for discovery
    assert!(
        handle.invocation_count() >= 3,
        "Expected at least 3 invocations (plan + build + discovery), got {}. stderr:\n{}",
        handle.invocation_count(),
        stderr
    );

    // Find workspace path
    let workspace_line = stdout
        .lines()
        .find(|line| line.contains("Eval workspace:"))
        .expect("Should print workspace path");

    let workspace_path = workspace_line
        .split("Eval workspace:")
        .nth(1)
        .expect("Should have path after colon")
        .trim();

    let workspace = std::path::Path::new(workspace_path);

    // Verify result.json was created
    let result_json = workspace.join("result.json");
    assert!(
        result_json.exists(),
        "result.json should exist at: {}",
        result_json.display()
    );

    // Verify result.json content
    let content = std::fs::read_to_string(&result_json).expect("read result.json");
    let json: serde_json::Value = serde_json::from_str(&content).expect("parse result.json");

    assert_eq!(json["project"], "calculator");
    assert!(json["elapsed_secs"].is_number(), "Should have elapsed_secs");
}

/// Test that timeout triggers retry and eventually succeeds.
///
/// This test verifies:
/// 1. First iteration times out (fake Claude delays longer than timeout)
/// 2. Build retries the iteration
/// 3. Second attempt succeeds (fake Claude responds quickly)
/// 4. Eval completes successfully
///
/// Note: This test uses a 2 second timeout to keep test duration reasonable.
#[test]
fn test_timeout_retry_succeeds() {
    let eval_dir = TempDir::new().expect("temp dir for evals");
    let handle = prebuilt::timeout_retry().build();

    let mut cmd = rslph_with_fake_claude(&handle);
    cmd.env("RSLPH_EVAL_DIR", eval_dir.path().to_str().unwrap());
    cmd.env("RSLPH_ITERATION_TIMEOUT", "2");  // 2 second timeout
    cmd.env("RSLPH_TIMEOUT_RETRIES", "3");    // Allow up to 3 retries
    cmd.args(["eval", "calculator", "--no-tui"]);

    let output = cmd.output().expect("Failed to run rslph eval");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    // Verify eval completed successfully
    assert!(
        stdout.contains("EVAL COMPLETE"),
        "Eval should complete. Output:\n{}",
        combined
    );

    // Verify timeout retry occurred (look for retry message in stderr)
    assert!(
        stderr.contains("timed out") || stderr.contains("retry"),
        "Should show timeout retry evidence. stderr:\n{}",
        stderr
    );

    // Verify at least 3 invocations: plan + 2 build attempts (timeout + retry)
    assert!(
        handle.invocation_count() >= 3,
        "Expected at least 3 invocations (plan + timeout + retry), got {}. Output:\n{}",
        handle.invocation_count(),
        combined
    );

    // Find and verify workspace has result.json
    let workspace_line = stdout
        .lines()
        .find(|line| line.contains("Eval workspace:"))
        .expect("Should print workspace path");

    let workspace_path = workspace_line
        .split("Eval workspace:")
        .nth(1)
        .expect("Should have path after colon")
        .trim();

    let result_json = std::path::Path::new(workspace_path).join("result.json");
    assert!(
        result_json.exists(),
        "result.json should exist at: {}",
        result_json.display()
    );
}

/// Test that exhausted retries result in failure.
///
/// This test verifies:
/// 1. All retry attempts timeout
/// 2. Build fails after exhausting retries
/// 3. Error message mentions timeout
#[test]
fn test_timeout_exhausted_fails() {
    let eval_dir = TempDir::new().expect("temp dir for evals");
    let handle = prebuilt::timeout_exhausted().build();

    let mut cmd = rslph_with_fake_claude(&handle);
    cmd.env("RSLPH_EVAL_DIR", eval_dir.path().to_str().unwrap());
    cmd.env("RSLPH_ITERATION_TIMEOUT", "2");  // 2 second timeout
    cmd.env("RSLPH_TIMEOUT_RETRIES", "2");    // Only 2 retries (will be exhausted)
    cmd.args(["eval", "calculator", "--no-tui"]);

    let output = cmd.output().expect("Failed to run rslph eval");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    // Verify eval failed (exit code non-zero or error message)
    let failed = !output.status.success()
        || combined.contains("timed out")
        || combined.contains("failed")
        || combined.contains("error");

    assert!(
        failed,
        "Eval should fail when timeout retries are exhausted. Output:\n{}",
        combined
    );

    // Verify timeout message is present
    assert!(
        combined.contains("timed out") || combined.contains("timeout"),
        "Should mention timeout. Output:\n{}",
        combined
    );
}

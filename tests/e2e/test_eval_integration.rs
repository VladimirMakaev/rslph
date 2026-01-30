//! E2E tests for eval command with fake Claude.
//!
//! These tests verify the eval command runs with fake Claude scenarios.
//! Full end-to-end testing with working test execution requires more
//! complex subprocess coordination.

#![allow(deprecated)] // Command::cargo_bin is deprecated but still functional

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
    assert!(
        json["tokens"]["input"].is_number(),
        "Should have token metrics"
    );

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
    let workspace_line = stdout.lines().find(|line| line.contains("Eval workspace:"));

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
    cmd.env("RSLPH_ITERATION_TIMEOUT", "2"); // 2 second timeout
    cmd.env("RSLPH_TIMEOUT_RETRIES", "3"); // Allow up to 3 retries
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
    cmd.env("RSLPH_ITERATION_TIMEOUT", "2"); // 2 second timeout
    cmd.env("RSLPH_TIMEOUT_RETRIES", "2"); // Only 2 retries (will be exhausted)
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

/// Test that retest command re-runs tests on an existing workspace.
///
/// This test verifies:
/// 1. Eval runs and creates a workspace with result.json
/// 2. Retest command runs on the workspace
/// 3. Retest invokes discovery and test execution
/// 4. result.json is updated with new test results
#[test]
fn test_retest_reruns_tests_on_existing_workspace() {
    let eval_dir = TempDir::new().expect("temp dir for evals");
    let handle = prebuilt::calculator().build();

    // Step 1: Run eval to create workspace
    let mut eval_cmd = rslph_with_fake_claude(&handle);
    eval_cmd.env("RSLPH_EVAL_DIR", eval_dir.path().to_str().unwrap());
    eval_cmd.args(["eval", "calculator", "--no-tui"]);

    let eval_output = eval_cmd.output().expect("Failed to run rslph eval");
    let eval_stdout = String::from_utf8_lossy(&eval_output.stdout);

    // Verify eval completed
    assert!(
        eval_stdout.contains("EVAL COMPLETE"),
        "Eval should complete. stdout:\n{}",
        eval_stdout
    );

    // Extract workspace path from eval output
    let workspace_line = eval_stdout
        .lines()
        .find(|line| line.contains("Eval workspace:"))
        .expect("Should print workspace path");

    let workspace_path = workspace_line
        .split("Eval workspace:")
        .nth(1)
        .expect("Should have path after colon")
        .trim();

    // Verify result.json exists
    let result_json_path = std::path::Path::new(workspace_path).join("result.json");
    assert!(
        result_json_path.exists(),
        "result.json should exist after eval at: {}",
        result_json_path.display()
    );

    // Read initial result.json to compare later
    let initial_content = std::fs::read_to_string(&result_json_path).expect("read initial result");
    let initial_json: serde_json::Value =
        serde_json::from_str(&initial_content).expect("parse initial result");

    // Track invocation count before retest
    let invocations_before = handle.invocation_count();

    // Step 2: Run retest command on the workspace
    let mut retest_cmd = rslph_with_fake_claude(&handle);
    retest_cmd.env("RSLPH_EVAL_DIR", eval_dir.path().to_str().unwrap());
    retest_cmd.args(["retest", workspace_path]);

    let retest_output = retest_cmd.output().expect("Failed to run rslph retest");
    let retest_stdout = String::from_utf8_lossy(&retest_output.stdout);
    let retest_stderr = String::from_utf8_lossy(&retest_output.stderr);

    // Verify retest completed
    assert!(
        retest_stdout.contains("RETEST COMPLETE"),
        "Retest should complete. stdout:\n{}\nstderr:\n{}",
        retest_stdout,
        retest_stderr
    );

    // Verify retest invoked at least one more Claude call (for discovery)
    assert!(
        handle.invocation_count() > invocations_before,
        "Retest should invoke Claude for discovery. Before: {}, After: {}",
        invocations_before,
        handle.invocation_count()
    );

    // Verify test phase ran
    assert!(
        retest_stdout.contains("TEST PHASE"),
        "Retest should run test phase. stdout:\n{}",
        retest_stdout
    );

    // Verify result.json was updated (still valid JSON with same project)
    let updated_content = std::fs::read_to_string(&result_json_path).expect("read updated result");
    let updated_json: serde_json::Value =
        serde_json::from_str(&updated_content).expect("parse updated result");

    assert_eq!(
        updated_json["project"], initial_json["project"],
        "Project name should be preserved"
    );
    assert_eq!(
        updated_json["iterations"], initial_json["iterations"],
        "Iteration count should be preserved (retest doesn't change it)"
    );
}

/// Test that retest fails gracefully for non-existent workspace.
#[test]
fn test_retest_fails_for_nonexistent_workspace() {
    let handle = prebuilt::calculator().build();

    let mut cmd = rslph_with_fake_claude(&handle);
    cmd.args(["retest", "/nonexistent/workspace/path"]);

    let output = cmd.output().expect("Failed to run rslph retest");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify command failed
    assert!(
        !output.status.success(),
        "Retest should fail for nonexistent workspace"
    );

    // Verify error message mentions the path doesn't exist
    assert!(
        stderr.contains("does not exist") || stderr.contains("not exist"),
        "Error should mention path doesn't exist. stderr:\n{}",
        stderr
    );
}

/// Test that retest fails for workspace missing result.json.
#[test]
fn test_retest_fails_for_missing_result_json() {
    let workspace = TempDir::new().expect("temp workspace");
    let handle = prebuilt::calculator().build();

    let mut cmd = rslph_with_fake_claude(&handle);
    cmd.args(["retest", workspace.path().to_str().unwrap()]);

    let output = cmd.output().expect("Failed to run rslph retest");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify command failed
    assert!(
        !output.status.success(),
        "Retest should fail when result.json is missing"
    );

    // Verify error message mentions result.json
    assert!(
        stderr.contains("result.json"),
        "Error should mention result.json. stderr:\n{}",
        stderr
    );
}

// =============================================================================
// Multi-Trial E2E Tests
// =============================================================================

/// Test that --trials flag appears in eval help output.
#[test]
fn test_eval_trials_flag_help() {
    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    cmd.args(["eval", "--help"]);

    let output = cmd.output().expect("Failed to run rslph eval --help");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify --trials flag is documented
    assert!(
        stdout.contains("--trials"),
        "Help should document --trials flag. stdout:\n{}",
        stdout
    );

    // Verify the doc comment about number of trials
    assert!(
        stdout.contains("Number of"),
        "Help should describe trial count. stdout:\n{}",
        stdout
    );
}

/// Test that --trials with invalid value shows appropriate error.
#[test]
fn test_eval_trials_invalid_value() {
    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    cmd.args(["eval", "calculator", "--trials", "abc"]);

    let output = cmd.output().expect("Failed to run rslph eval");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify command failed
    assert!(
        !output.status.success(),
        "Eval should fail with invalid --trials value"
    );

    // Verify error message mentions invalid value
    assert!(
        stderr.contains("invalid value") || stderr.contains("Invalid value"),
        "Error should mention invalid value. stderr:\n{}",
        stderr
    );
}

/// Test that --trials with zero value is handled appropriately.
///
/// With trials=0, clap should reject it or eval should run no trials.
/// Either behavior is acceptable as long as it doesn't crash.
#[test]
fn test_eval_trials_zero() {
    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    cmd.args(["eval", "calculator", "--trials", "0"]);

    let output = cmd.output().expect("Failed to run rslph eval");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Either: command fails with validation error, or succeeds with 0 trials
    // The important thing is it doesn't crash unexpectedly
    if output.status.success() {
        // If it succeeds, it should mention running 0 trials or complete immediately
        // This is acceptable behavior
        assert!(
            stdout.contains("Running 0") || stdout.contains("complete") || stdout.is_empty(),
            "Zero trials should either complete immediately or show no output. stdout:\n{}",
            stdout
        );
    } else {
        // If it fails, error should mention validation/trials
        assert!(
            stderr.contains("trials") || stderr.contains("value") || stderr.contains("invalid"),
            "Error should mention trials validation. stderr:\n{}",
            stderr
        );
    }
}

// =============================================================================
// Compare Command E2E Tests
// =============================================================================

/// Test that compare help output documents required arguments.
#[test]
fn test_compare_help() {
    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    cmd.args(["compare", "--help"]);

    let output = cmd.output().expect("Failed to run rslph compare --help");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify file1 and file2 arguments are documented
    assert!(
        stdout.contains("file1") || stdout.contains("FILE1"),
        "Help should document file1 argument. stdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("file2") || stdout.contains("FILE2"),
        "Help should document file2 argument. stdout:\n{}",
        stdout
    );

    // Verify Compare is documented
    assert!(
        stdout.contains("Compare") || stdout.contains("compare"),
        "Help should describe compare command. stdout:\n{}",
        stdout
    );
}

/// Test that compare fails gracefully with missing file.
#[test]
fn test_compare_missing_file() {
    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    cmd.args([
        "compare",
        "/nonexistent/file1.json",
        "/nonexistent/file2.json",
    ]);

    let output = cmd.output().expect("Failed to run rslph compare");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify command failed
    assert!(
        !output.status.success(),
        "Compare should fail with missing files"
    );

    // Verify error message mentions file issue
    assert!(
        stderr.contains("Failed to read")
            || stderr.contains("nonexistent")
            || stderr.contains("file"),
        "Error should mention file issue. stderr:\n{}",
        stderr
    );
}

/// Test that compare fails when missing required arguments.
#[test]
fn test_compare_missing_args() {
    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    cmd.args(["compare"]);

    let output = cmd.output().expect("Failed to run rslph compare");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify command failed
    assert!(
        !output.status.success(),
        "Compare should fail when missing required arguments"
    );

    // Verify error message mentions required arguments
    assert!(
        stderr.contains("required") || stderr.contains("FILE1") || stderr.contains("file1"),
        "Error should mention required arguments. stderr:\n{}",
        stderr
    );
}

/// Test that compare succeeds with valid JSON files.
#[test]
fn test_compare_valid_files() {
    let temp_dir = TempDir::new().expect("temp dir");

    // Create valid multi-trial result JSON files
    let json1 = r#"{
  "project": "test1",
  "timestamp": "2026-01-21",
  "trial_count": 1,
  "trials": [{
    "trial_num": 1,
    "elapsed_secs": 10.0,
    "iterations": 1,
    "tokens": {"input": 100, "output": 50, "cache_creation": 0, "cache_read": 0},
    "test_results": {"passed": 5, "total": 10, "pass_rate": 50.0},
    "workspace_path": "/tmp/test1"
  }],
  "statistics": {
    "pass_rate": {"mean": 50.0, "variance": 0.0, "std_dev": 0.0, "min": 50.0, "max": 50.0, "count": 1},
    "elapsed_secs": {"mean": 10.0, "variance": 0.0, "std_dev": 0.0, "min": 10.0, "max": 10.0, "count": 1},
    "total_input_tokens": {"mean": 100.0, "variance": 0.0, "std_dev": 0.0, "min": 100.0, "max": 100.0, "count": 1},
    "total_output_tokens": {"mean": 50.0, "variance": 0.0, "std_dev": 0.0, "min": 50.0, "max": 50.0, "count": 1},
    "iterations": {"mean": 1.0, "variance": 0.0, "std_dev": 0.0, "min": 1.0, "max": 1.0, "count": 1}
  }
}"#;

    let json2 = r#"{
  "project": "test2",
  "timestamp": "2026-01-22",
  "trial_count": 1,
  "trials": [{
    "trial_num": 1,
    "elapsed_secs": 8.0,
    "iterations": 1,
    "tokens": {"input": 80, "output": 40, "cache_creation": 0, "cache_read": 0},
    "test_results": {"passed": 7, "total": 10, "pass_rate": 70.0},
    "workspace_path": "/tmp/test2"
  }],
  "statistics": {
    "pass_rate": {"mean": 70.0, "variance": 0.0, "std_dev": 0.0, "min": 70.0, "max": 70.0, "count": 1},
    "elapsed_secs": {"mean": 8.0, "variance": 0.0, "std_dev": 0.0, "min": 8.0, "max": 8.0, "count": 1},
    "total_input_tokens": {"mean": 80.0, "variance": 0.0, "std_dev": 0.0, "min": 80.0, "max": 80.0, "count": 1},
    "total_output_tokens": {"mean": 40.0, "variance": 0.0, "std_dev": 0.0, "min": 40.0, "max": 40.0, "count": 1},
    "iterations": {"mean": 1.0, "variance": 0.0, "std_dev": 0.0, "min": 1.0, "max": 1.0, "count": 1}
  }
}"#;

    let file1 = temp_dir.path().join("result1.json");
    let file2 = temp_dir.path().join("result2.json");

    std::fs::write(&file1, json1).expect("write file1");
    std::fs::write(&file2, json2).expect("write file2");

    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    cmd.args(["compare", file1.to_str().unwrap(), file2.to_str().unwrap()]);

    let output = cmd.output().expect("Failed to run rslph compare");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify command succeeded
    assert!(
        output.status.success(),
        "Compare should succeed with valid files. stderr:\n{}",
        stderr
    );

    // Verify output contains expected content
    assert!(
        stdout.contains("Comparing results"),
        "Output should contain 'Comparing results'. stdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Pass Rate"),
        "Output should contain 'Pass Rate'. stdout:\n{}",
        stdout
    );
}

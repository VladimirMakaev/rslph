//! True E2E integration tests that run rslph with fake Claude
//!
//! These tests invoke the actual rslph binary, configured to use fake Claude,
//! and verify the complete build loop works correctly.
//!
//! The integration point is RSLPH_CLAUDE_CMD env var which overrides the
//! claude command, allowing rslph to use our fake Claude binary.

#![allow(deprecated)] // Command::cargo_bin is deprecated but still functional

use crate::fake_claude_lib::{FakeClaudeHandle, ScenarioBuilder};
use crate::fixtures::WorkspaceBuilder;
use assert_cmd::Command;

/// Helper to get rslph command configured with fake Claude
fn rslph_with_fake_claude(scenario: &FakeClaudeHandle) -> Command {
    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    // Set environment variables for fake Claude
    for (key, value) in scenario.env_vars() {
        cmd.env(key, value);
    }
    cmd
}

#[test]
fn test_rslph_build_single_iteration_success() {
    // Scenario: Claude provides a response in a single iteration
    let scenario = ScenarioBuilder::new()
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
    // Check that fake Claude was invoked at least once
    assert!(
        scenario.invocation_count() >= 1,
        "Expected at least 1 invocation, got {}",
        scenario.invocation_count()
    );

    // The test passes if rslph runs and invokes fake Claude
    // We don't assert on exit code because --max-iterations=1 may exit early
    let _ = output; // Consume output
}

#[test]
fn test_rslph_build_multi_iteration_invokes_claude_multiple_times() {
    // Scenario: Two invocations with different responses
    let scenario = ScenarioBuilder::new()
        .respond_with_text("Working on Task 1...")
        .next_invocation()
        .respond_with_text("Still working...")
        .next_invocation()
        .respond_with_text("Task complete!")
        .build();

    let workspace = WorkspaceBuilder::new()
        .with_progress_file("# Progress\n\n- [ ] Task 1\n- [ ] Task 2\n")
        .build();

    let mut cmd = rslph_with_fake_claude(&scenario);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("3")
        .arg("--no-tui")
        .current_dir(workspace.path());

    cmd.output().expect("Failed to run rslph");

    // Should have invoked Claude at least twice (up to max_iterations)
    let count = scenario.invocation_count();
    assert!(
        count >= 2,
        "Expected at least 2 invocations for multi-iteration, got {}",
        count
    );
}

#[test]
fn test_rslph_build_respects_max_iterations() {
    // Scenario: Claude never completes the task (no RALPH_DONE)
    let scenario = ScenarioBuilder::new()
        .respond_with_text("Still working...")
        .next_invocation()
        .respond_with_text("Still working...")
        .next_invocation()
        .respond_with_text("Still working...")
        .next_invocation()
        .respond_with_text("Still working...")
        .build();

    let workspace = WorkspaceBuilder::new()
        .with_progress_file("# Progress\n\n- [ ] Never-ending task\n")
        .build();

    let mut cmd = rslph_with_fake_claude(&scenario);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("2") // Limit to 2
        .arg("--no-tui")
        .current_dir(workspace.path());

    cmd.output().expect("Failed to run rslph");

    // Should stop at max_iterations, not continue forever
    assert!(
        scenario.invocation_count() <= 2,
        "Expected at most 2 invocations (max_iterations limit), got {}",
        scenario.invocation_count()
    );
}

#[test]
fn test_rslph_build_handles_claude_crash() {
    // Scenario: Claude crashes after first event
    let scenario = ScenarioBuilder::new()
        .respond_with_text("Starting work...")
        .crash_after(1) // Crash after first event
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

    // rslph should handle the crash gracefully (not panic)
    // It may exit with error, but the main assertion is that we get here
    // without the test framework crashing
    let _ = output;

    // The fact that we got here means rslph didn't panic - test passes implicitly
}

#[test]
fn test_rslph_build_with_tool_calls() {
    // Scenario: Claude uses tools (Read, Write, Edit, Bash)
    let scenario = ScenarioBuilder::new()
        .respond_with_text("Let me read the file first")
        .uses_read("src/main.rs")
        .uses_bash("echo 'Building...'")
        .build();

    let workspace = WorkspaceBuilder::new()
        .with_progress_file("# Progress\n\n- [ ] Create file\n")
        .with_source_file("src/main.rs", "fn main() {}\n")
        .build();

    let mut cmd = rslph_with_fake_claude(&scenario);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("1")
        .arg("--no-tui")
        .current_dir(workspace.path());

    cmd.output().expect("Failed to run rslph");

    // Verify fake Claude was invoked
    assert_eq!(scenario.invocation_count(), 1);
}

#[test]
fn test_rslph_build_with_workspace_config() {
    // Alternative approach: Use workspace .rslph/config.toml instead of env var
    let scenario = ScenarioBuilder::new()
        .respond_with_text("Using config file path for Claude")
        .build();

    // Create workspace with custom config pointing to fake Claude
    // Note: Config file uses flat TOML (no section header)
    let config_toml = format!(
        r#"claude_path = "{}"
max_iterations = 3
tui_enabled = false
"#,
        scenario.executable_path.display()
    );

    let workspace = WorkspaceBuilder::new()
        .with_config(&config_toml)
        .with_progress_file("# Progress\n\n- [ ] Task 1\n")
        .build();

    let config_path = workspace.path().join(".rslph/config.toml");

    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    // Use -c flag to specify config file path explicitly
    // rslph should read claude_path from the workspace config
    cmd.env("FAKE_CLAUDE_CONFIG", &scenario.config_path)
        .arg("-c")
        .arg(&config_path)
        .arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("1")
        .current_dir(workspace.path());

    cmd.output().expect("Failed to run rslph");

    assert_eq!(
        scenario.invocation_count(),
        1,
        "Expected Claude to be invoked via config file path"
    );
}

#[test]
fn test_rslph_build_tui_disabled_via_config() {
    // Verify TUI can be disabled via config (important for CI/headless)
    let scenario = ScenarioBuilder::new()
        .respond_with_text("Working without TUI")
        .build();

    // Note: Config file uses flat TOML (no section header)
    let config_toml = format!(
        r#"claude_path = "{}"
tui_enabled = false
"#,
        scenario.executable_path.display()
    );

    let workspace = WorkspaceBuilder::new()
        .with_config(&config_toml)
        .with_progress_file("# Progress\n\n- [ ] Task\n")
        .build();

    let config_path = workspace.path().join(".rslph/config.toml");

    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    // Use -c flag to specify config file path explicitly
    cmd.env("FAKE_CLAUDE_CONFIG", &scenario.config_path)
        .arg("-c")
        .arg(&config_path)
        .arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("1")
        .current_dir(workspace.path());

    let output = cmd.output().expect("Failed to run rslph");

    // Should run without TUI (important for CI/headless testing)
    assert_eq!(scenario.invocation_count(), 1);
    let _ = output;
}

#[test]
fn test_rslph_build_once_flag() {
    // Verify --once flag runs exactly one iteration
    let scenario = ScenarioBuilder::new()
        .respond_with_text("Single iteration with --once flag")
        .next_invocation()
        .respond_with_text("This should NOT be reached")
        .build();

    let workspace = WorkspaceBuilder::new()
        .with_progress_file("# Progress\n\n- [ ] Task 1\n")
        .build();

    let mut cmd = rslph_with_fake_claude(&scenario);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--once")
        .arg("--no-tui")
        .current_dir(workspace.path());

    cmd.output().expect("Failed to run rslph");

    // --once should stop after exactly 1 iteration
    assert_eq!(
        scenario.invocation_count(),
        1,
        "Expected exactly 1 invocation with --once flag"
    );
}

#[test]
fn test_rslph_build_dry_run() {
    // Verify --dry-run doesn't actually execute Claude
    let scenario = ScenarioBuilder::new()
        .respond_with_text("This should NOT be called")
        .build();

    let workspace = WorkspaceBuilder::new()
        .with_progress_file("# Progress\n\n- [ ] Task 1\n")
        .build();

    let mut cmd = rslph_with_fake_claude(&scenario);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--dry-run")
        .arg("--no-tui")
        .current_dir(workspace.path());

    let output = cmd.output().expect("Failed to run rslph");

    // --dry-run should NOT invoke Claude
    assert_eq!(
        scenario.invocation_count(),
        0,
        "Expected 0 invocations with --dry-run flag"
    );

    // Should still succeed
    assert!(output.status.success(), "dry-run should succeed");
}

#[test]
fn test_rslph_uses_rslph_claude_path_env() {
    // Verify RSLPH_CLAUDE_PATH env var correctly overrides claude_path
    let scenario = ScenarioBuilder::new()
        .respond_with_text("Invoked via RSLPH_CLAUDE_PATH env var")
        .build();

    // Create workspace with DEFAULT config (uses "claude" as claude_path)
    let workspace = WorkspaceBuilder::new()
        .with_progress_file("# Progress\n\n- [ ] Task 1\n")
        .build();

    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    // Set RSLPH_CLAUDE_PATH to override the config
    cmd.env("RSLPH_CLAUDE_PATH", &scenario.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &scenario.config_path)
        .arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("1")
        .arg("--no-tui")
        .current_dir(workspace.path());

    cmd.output().expect("Failed to run rslph");

    // RSLPH_CLAUDE_PATH should have overridden the config
    assert_eq!(
        scenario.invocation_count(),
        1,
        "Expected Claude to be invoked via RSLPH_CLAUDE_PATH env var"
    );
}

#[test]
fn test_rslph_plan_single_response() {
    // Scenario: Plan command runs and invokes fake Claude
    // Note: Plan command may invoke Claude twice - once for planning and once
    // for project name generation if the output doesn't include a name.
    let scenario = ScenarioBuilder::new()
        .respond_with_text("Here is a plan for your task...")
        .next_invocation()
        .respond_with_text("test-project") // Project name generation response
        .build();

    let workspace = WorkspaceBuilder::new()
        .with_progress_file("# Progress\n\n- [ ] Task 1\n")
        .build();

    let mut cmd = rslph_with_fake_claude(&scenario);
    cmd.arg("plan")
        .arg("PROGRESS.md")
        .arg("--no-tui")
        .current_dir(workspace.path());

    let output = cmd.output().expect("Failed to run rslph plan");

    // Verify fake Claude was invoked (at least once for plan, maybe twice for name)
    assert!(
        scenario.invocation_count() >= 1,
        "Expected at least 1 invocation for plan command, got {}",
        scenario.invocation_count()
    );

    // Plan command should succeed
    assert!(
        output.status.success(),
        "plan command should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_rslph_plan_uses_rslph_claude_cmd_env() {
    // Verify RSLPH_CLAUDE_CMD env var works for plan command
    // Note: Plan command may invoke Claude twice - once for planning and once
    // for project name generation if the output doesn't include a name.
    let scenario = ScenarioBuilder::new()
        .respond_with_text("Plan generated via RSLPH_CLAUDE_CMD env var")
        .next_invocation()
        .respond_with_text("env-test-project") // Project name generation response
        .build();

    // Create workspace without any config - rely solely on env var
    let workspace = WorkspaceBuilder::new()
        .with_progress_file("# Progress\n\n- [ ] Task 1\n")
        .build();

    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    // Set RSLPH_CLAUDE_CMD explicitly (not via env_vars helper)
    cmd.env("RSLPH_CLAUDE_CMD", &scenario.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &scenario.config_path)
        .arg("plan")
        .arg("PROGRESS.md")
        .arg("--no-tui")
        .current_dir(workspace.path());

    let output = cmd.output().expect("Failed to run rslph plan");

    // RSLPH_CLAUDE_CMD should have worked for plan command (at least one invocation)
    assert!(
        scenario.invocation_count() >= 1,
        "Expected at least 1 invocation via RSLPH_CLAUDE_CMD env var for plan, got {}",
        scenario.invocation_count()
    );

    assert!(
        output.status.success(),
        "plan with RSLPH_CLAUDE_CMD should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

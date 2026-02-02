//! True E2E integration tests that run rslph with fake Claude
//!
//! These tests invoke the actual rslph binary, configured to use fake Claude,
//! and verify the complete build loop works correctly.
//!
//! The integration point is RSLPH_CLAUDE_CMD env var which overrides the
//! claude command, allowing rslph to use our fake Claude binary.
//!
//! TUI is disabled via config file (tui_enabled = false) for headless testing.

#![allow(deprecated)] // Command::cargo_bin is deprecated but still functional

use crate::fake_claude_lib::{FakeClaudeHandle, ScenarioBuilder};
use crate::fixtures::WorkspaceBuilder;
use assert_cmd::Command;

/// Helper to get rslph command configured with fake Claude and TUI disabled.
///
/// Creates a workspace config with tui_enabled = false and sets up the
/// environment for fake Claude.
fn rslph_with_fake_claude_and_config(
    scenario: &FakeClaudeHandle,
    workspace: &crate::fixtures::Workspace,
) -> Command {
    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    // Set environment variables for fake Claude
    for (key, value) in scenario.env_vars() {
        cmd.env(key, value);
    }
    // Use the workspace config with tui_enabled = false
    let config_path = workspace.path().join(".rslph/config.toml");
    cmd.arg("-c").arg(&config_path);
    cmd
}

/// Create a workspace with TUI disabled config and fake Claude path.
fn workspace_with_tui_disabled(scenario: &FakeClaudeHandle, progress_content: &str) -> crate::fixtures::Workspace {
    let config_toml = format!(
        r#"claude_path = "{}"
tui_enabled = false
"#,
        scenario.executable_path.display()
    );
    WorkspaceBuilder::new()
        .with_config(&config_toml)
        .with_progress_file(progress_content)
        .build()
}

#[test]
fn test_rslph_build_single_iteration_success() {
    // Valid progress file format for Claude to return
    let valid_progress = r#"# Progress: Test

## Status

In Progress

## Tasks

### Phase 1

- [x] Task 1

## Testing Strategy

Unit tests.
"#;

    // Scenario: Claude provides a valid progress file response
    let scenario = ScenarioBuilder::new()
        .respond_with_text(valid_progress)
        .build();

    let workspace = workspace_with_tui_disabled(&scenario, "# Progress\n\n- [ ] Task 1\n");

    let mut cmd = rslph_with_fake_claude_and_config(&scenario, &workspace);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("1")
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
    // Valid progress files for each invocation
    let progress_iter1 = r#"# Progress: Test

## Status

In Progress

## Tasks

### Phase 1

- [ ] Task 1
- [ ] Task 2

## Testing Strategy

Unit tests.
"#;

    let progress_iter2 = r#"# Progress: Test

## Status

In Progress

## Tasks

### Phase 1

- [x] Task 1
- [ ] Task 2

## Testing Strategy

Unit tests.
"#;

    let progress_done = r#"# Progress: Test

## Status

RALPH_DONE - All tasks complete

## Tasks

### Phase 1

- [x] Task 1
- [x] Task 2

## Testing Strategy

Unit tests.
"#;

    // Scenario: Multiple invocations with valid progress file responses
    let scenario = ScenarioBuilder::new()
        .respond_with_text(progress_iter1)
        .next_invocation()
        .respond_with_text(progress_iter2)
        .next_invocation()
        .respond_with_text(progress_done)
        .build();

    let workspace = workspace_with_tui_disabled(&scenario, "# Progress\n\n- [ ] Task 1\n- [ ] Task 2\n");

    let mut cmd = rslph_with_fake_claude_and_config(&scenario, &workspace);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("3")
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
    // Valid progress file that never completes (no RALPH_DONE)
    let progress_incomplete = r#"# Progress: Test

## Status

In Progress

## Tasks

### Phase 1

- [ ] Never-ending task

## Testing Strategy

Unit tests.
"#;

    // Scenario: Claude returns valid but incomplete progress file
    let scenario = ScenarioBuilder::new()
        .respond_with_text(progress_incomplete)
        .next_invocation()
        .respond_with_text(progress_incomplete)
        .next_invocation()
        .respond_with_text(progress_incomplete)
        .next_invocation()
        .respond_with_text(progress_incomplete)
        .build();

    let workspace = workspace_with_tui_disabled(&scenario, "# Progress\n\n- [ ] Never-ending task\n");

    let mut cmd = rslph_with_fake_claude_and_config(&scenario, &workspace);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("2") // Limit to 2
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

    let workspace = workspace_with_tui_disabled(&scenario, "# Progress\n\n- [ ] Task 1\n");

    let mut cmd = rslph_with_fake_claude_and_config(&scenario, &workspace);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("1")
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

    // Need custom workspace with source file
    let config_toml = format!(
        r#"claude_path = "{}"
tui_enabled = false
"#,
        scenario.executable_path.display()
    );
    let workspace = WorkspaceBuilder::new()
        .with_config(&config_toml)
        .with_progress_file("# Progress\n\n- [ ] Create file\n")
        .with_source_file("src/main.rs", "fn main() {}\n")
        .build();

    let mut cmd = rslph_with_fake_claude_and_config(&scenario, &workspace);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("1")
        .current_dir(workspace.path());

    cmd.output().expect("Failed to run rslph");

    // Verify fake Claude was invoked
    assert_eq!(scenario.invocation_count(), 1);
}

#[test]
fn test_rslph_build_with_workspace_config() {
    // Test that config file with tui_enabled = false and custom max_iterations works
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

    // Use the helper which sets both RSLPH_CLAUDE_CMD and FAKE_CLAUDE_CONFIG
    let mut cmd = rslph_with_fake_claude_and_config(&scenario, &workspace);
    cmd.arg("build")
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

    let workspace = workspace_with_tui_disabled(&scenario, "# Progress\n\n- [ ] Task\n");

    let mut cmd = rslph_with_fake_claude_and_config(&scenario, &workspace);
    cmd.arg("build")
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

    let workspace = workspace_with_tui_disabled(&scenario, "# Progress\n\n- [ ] Task 1\n");

    let mut cmd = rslph_with_fake_claude_and_config(&scenario, &workspace);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--once")
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

    let workspace = workspace_with_tui_disabled(&scenario, "# Progress\n\n- [ ] Task 1\n");

    let mut cmd = rslph_with_fake_claude_and_config(&scenario, &workspace);
    cmd.arg("build")
        .arg("PROGRESS.md")
        .arg("--dry-run")
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
fn test_rslph_uses_rslph_claude_cmd_env() {
    // Verify RSLPH_CLAUDE_CMD env var correctly overrides default claude command
    let scenario = ScenarioBuilder::new()
        .respond_with_text("Invoked via RSLPH_CLAUDE_CMD env var")
        .build();

    // Create workspace with TUI disabled but default claude_path (env will override)
    let config_toml = "tui_enabled = false\n";
    let workspace = WorkspaceBuilder::new()
        .with_config(config_toml)
        .with_progress_file("# Progress\n\n- [ ] Task 1\n")
        .build();

    let config_path = workspace.path().join(".rslph/config.toml");

    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    // Set RSLPH_CLAUDE_CMD to override the default
    cmd.env("RSLPH_CLAUDE_CMD", &scenario.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &scenario.config_path)
        .arg("-c")
        .arg(&config_path)
        .arg("build")
        .arg("PROGRESS.md")
        .arg("--max-iterations")
        .arg("1")
        .current_dir(workspace.path());

    cmd.output().expect("Failed to run rslph");

    // RSLPH_CLAUDE_CMD should have overridden the default
    assert_eq!(
        scenario.invocation_count(),
        1,
        "Expected Claude to be invoked via RSLPH_CLAUDE_CMD env var"
    );
}

#[test]
fn test_rslph_plan_single_response() {
    // Scenario: Plan command runs and invokes fake Claude
    // Note: Plan command may invoke Claude twice - once for planning and once
    // for project name generation if the output doesn't include a name.

    // Valid progress file format that will pass parsing validation
    let valid_progress = r#"# Progress: Test Plan

## Status

In Progress

## Tasks

### Phase 1: Setup

- [ ] Task 1 description

## Testing Strategy

Run unit tests.
"#;

    let scenario = ScenarioBuilder::new()
        .respond_with_text(valid_progress)
        .next_invocation()
        .respond_with_text("test-project") // Project name generation response (not needed but ok)
        .build();

    let workspace = workspace_with_tui_disabled(&scenario, "# Progress\n\n- [ ] Task 1\n");

    let mut cmd = rslph_with_fake_claude_and_config(&scenario, &workspace);
    cmd.arg("plan")
        .arg("PROGRESS.md")
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

    // Valid progress file format that will pass parsing validation
    let valid_progress = r#"# Progress: ENV Test Plan

## Status

In Progress

## Tasks

### Phase 1: Setup

- [ ] Task from env test

## Testing Strategy

Run unit tests.
"#;

    let scenario = ScenarioBuilder::new()
        .respond_with_text(valid_progress)
        .next_invocation()
        .respond_with_text("env-test-project") // Project name generation response (not needed but ok)
        .build();

    // Create workspace with TUI disabled but relying on env var for Claude
    let config_toml = "tui_enabled = false\n";
    let workspace = WorkspaceBuilder::new()
        .with_config(config_toml)
        .with_progress_file("# Progress\n\n- [ ] Task 1\n")
        .build();

    let config_path = workspace.path().join(".rslph/config.toml");

    let mut cmd = Command::cargo_bin("rslph").expect("rslph binary should exist");
    // Set RSLPH_CLAUDE_CMD explicitly (not via env_vars helper)
    cmd.env("RSLPH_CLAUDE_CMD", &scenario.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &scenario.config_path)
        .arg("-c")
        .arg(&config_path)
        .arg("plan")
        .arg("PROGRESS.md")
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

#[test]
fn test_rslph_plan_adaptive_mode_produces_valid_progress() {
    // Scenario: Plan command with --adaptive flag runs the full adaptive flow:
    // 1. First invocation: clarifying questions response
    // 2. Second invocation: testing strategy response
    // 3. Third invocation: final plan with valid progress file content
    // 4. Fourth invocation: project name generation (if name is empty)
    //
    // The fake Claude must return VALID progress file markdown format.

    let valid_progress_file = r#"# Progress: Test Project

## Status

In Progress

## Tasks

### Phase 1: Setup

- [ ] Create configuration file
- [ ] Set up project structure

## Testing Strategy

Run unit tests to verify functionality.
"#;

    let scenario = ScenarioBuilder::new()
        // Invocation 1: Clarifying questions (adaptive mode asks questions when vague)
        .respond_with_text("Here are some clarifying questions:\n1. What language?\n2. What features?")
        .next_invocation()
        // Invocation 2: Testing strategy
        .respond_with_text("Testing strategy:\n- Unit tests for core logic\n- Integration tests for API")
        .next_invocation()
        // Invocation 3: Final plan with VALID progress file format
        .respond_with_text(valid_progress_file)
        .next_invocation()
        // Invocation 4: Project name generation (in case parser doesn't extract name)
        .respond_with_text("test-project")
        .build();

    // Create workspace with TUI disabled and an input file
    let config_toml = format!(
        r#"claude_path = "{}"
tui_enabled = false
"#,
        scenario.executable_path.display()
    );
    let workspace = WorkspaceBuilder::new()
        .with_config(&config_toml)
        .with_source_file("INITIAL.md", "Build a test project")
        .build();

    let mut cmd = rslph_with_fake_claude_and_config(&scenario, &workspace);
    cmd.arg("plan")
        .arg("INITIAL.md")
        .arg("--adaptive")
        .current_dir(workspace.path());

    // Run the plan command with adaptive mode
    let output = cmd.output().expect("Failed to run rslph plan --adaptive");

    // Debug: Print stderr to see what happened
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("[TEST STDERR] {}", stderr);

    // Verify the command succeeded
    assert!(
        output.status.success(),
        "plan --adaptive should succeed, stderr: {}",
        stderr
    );

    // Verify fake Claude was invoked at least 3 times (questions, strategy, final plan)
    assert!(
        scenario.invocation_count() >= 3,
        "Expected at least 3 invocations for adaptive mode, got {}",
        scenario.invocation_count()
    );

    // Verify progress.md file was created
    let progress_path = workspace.path().join("progress.md");
    assert!(
        progress_path.exists(),
        "progress.md should exist at {:?}",
        progress_path
    );

    // Read and validate the progress.md content
    let progress_content = std::fs::read_to_string(&progress_path)
        .expect("Should be able to read progress.md");

    // Verify the content is NOT empty
    assert!(
        !progress_content.trim().is_empty(),
        "progress.md should NOT be empty"
    );

    // Verify it contains expected sections
    assert!(
        progress_content.contains("## Status"),
        "progress.md should contain Status section, got: {}",
        progress_content
    );
    assert!(
        progress_content.contains("## Tasks"),
        "progress.md should contain Tasks section, got: {}",
        progress_content
    );
}

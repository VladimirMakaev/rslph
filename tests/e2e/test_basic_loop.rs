//! Infrastructure verification tests
//!
//! These tests verify that fake Claude and workspace fixtures work correctly
//! as standalone components. They do NOT invoke rslph.
//!
//! True E2E integration tests that run rslph with fake Claude are in
//! a future plan.

use crate::fake_claude_lib::ScenarioBuilder;
use crate::fixtures::WorkspaceBuilder;
use crate::helpers::assert_file_contains;

/// Verify fake Claude binary outputs text responses correctly.
#[test]
fn test_fake_claude_outputs_text() {
    let handle = ScenarioBuilder::new()
        .respond_with_text("Hello from fake Claude!")
        .build();

    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hello from fake Claude!"),
        "Expected output to contain greeting, got: {}",
        stdout
    );
    assert!(output.status.success());
}

/// Verify tool calls output correct stream-json format.
#[test]
fn test_fake_claude_tool_call_format() {
    let handle = ScenarioBuilder::new()
        .uses_read("/test/file.txt")
        .build();

    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("tool_use"),
        "Expected tool_use in output, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Read"),
        "Expected Read tool name, got: {}",
        stdout
    );
    assert!(
        stdout.contains("/test/file.txt"),
        "Expected file path, got: {}",
        stdout
    );
}

/// Verify multi-invocation counter works correctly.
#[test]
fn test_fake_claude_multi_invocation() {
    let handle = ScenarioBuilder::new()
        .respond_with_text("First invocation")
        .next_invocation()
        .respond_with_text("Second invocation")
        .build();

    // Initially counter should be 0
    assert_eq!(handle.invocation_count(), 0);

    // First invocation
    let output1 = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");
    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    assert!(
        stdout1.contains("First invocation"),
        "Expected first invocation text, got: {}",
        stdout1
    );
    assert_eq!(handle.invocation_count(), 1);

    // Second invocation
    let output2 = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(
        stdout2.contains("Second invocation"),
        "Expected second invocation text, got: {}",
        stdout2
    );
    assert_eq!(handle.invocation_count(), 2);
}

/// Verify workspace fixture creates valid directory structure.
#[test]
fn test_workspace_fixture_creates_valid_structure() {
    let workspace = WorkspaceBuilder::new()
        .with_progress_file("- [ ] Task 1\n- [ ] Task 2")
        .with_source_file("src/main.rs", "fn main() {}")
        .build();

    // Verify structure
    assert!(workspace.file_exists("PROGRESS.md"));
    assert!(workspace.file_exists("src/main.rs"));
    assert!(workspace.file_exists(".rslph/config.toml"));
    assert!(workspace.path().join(".git").exists());

    // Verify content
    assert_file_contains(&workspace, "PROGRESS.md", "Task 1");
    assert_file_contains(&workspace, "src/main.rs", "fn main()");
}

/// Verify workspace can configure custom claude_path in config.
/// This is the mechanism for rslph integration tests.
#[test]
fn test_workspace_with_custom_claude_path() {
    let handle = ScenarioBuilder::new()
        .respond_with_text("Configured via custom path")
        .build();

    let config_toml = format!(
        r#"[rslph]
claude_path = "{}"
"#,
        handle.executable_path.display()
    );

    let workspace = WorkspaceBuilder::new()
        .with_config(&config_toml)
        .with_progress_file("- [ ] Task 1")
        .build();

    // Verify config has the custom claude_path
    let config_content = workspace.read_file(".rslph/config.toml");
    assert!(
        config_content.contains(&handle.executable_path.display().to_string()),
        "Expected config to contain executable path, got: {}",
        config_content
    );
}

/// Verify result event with cost is generated.
#[test]
fn test_fake_claude_result_event() {
    // respond_with_text adds system_init, assistant_text, and result events
    let handle = ScenarioBuilder::new()
        .respond_with_text("Some work")
        .build();

    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Result event should include cost info (result event is included by respond_with_text)
    assert!(
        stdout.contains("result"),
        "Expected result event in output, got: {}",
        stdout
    );
}

/// Verify multiple tool calls in single invocation.
#[test]
fn test_fake_claude_multiple_tools() {
    let handle = ScenarioBuilder::new()
        .uses_read("/path/one")
        .uses_write("/path/two", "content")
        .uses_edit("/path/three", "old", "new")
        .uses_bash("echo hello")
        .build();

    let output = std::process::Command::new(&handle.executable_path)
        .env("FAKE_CLAUDE_CONFIG", &handle.config_path)
        .output()
        .expect("Failed to run fake claude");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify all tool uses are present
    assert!(stdout.contains("/path/one"), "Expected read path");
    assert!(stdout.contains("/path/two"), "Expected write path");
    assert!(stdout.contains("/path/three"), "Expected edit path");
    assert!(stdout.contains("echo hello"), "Expected bash command");
}

/// Verify workspace without git initialization.
#[test]
fn test_workspace_without_git() {
    let workspace = WorkspaceBuilder::new()
        .without_git()
        .with_progress_file("- [ ] Task")
        .build();

    assert!(!workspace.path().join(".git").exists());
    assert!(workspace.file_exists("PROGRESS.md"));
    assert!(workspace.file_exists(".rslph/config.toml"));
}

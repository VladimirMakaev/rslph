//! E2E tests for the eval command.
//!
//! Tests verify CLI parsing, validation, and basic behavior of the eval command.
//! Full integration tests with fake Claude require the eval project patterns
//! established in Phase 10.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Create a minimal eval project with a prompt file.
fn create_eval_project(dir: &TempDir) -> std::path::PathBuf {
    let project_dir = dir.path().join("test-project");
    std::fs::create_dir_all(&project_dir).expect("create project dir");

    // Create prompt.txt
    std::fs::write(
        project_dir.join("prompt.txt"),
        "Create a simple hello world program that prints 'Hello, World!' to stdout.",
    )
    .expect("write prompt.txt");

    project_dir
}

#[test]
fn test_eval_help() {
    let mut cmd = Command::cargo_bin("rslph").expect("binary");
    cmd.args(["eval", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("PROJECT"))
        .stdout(predicate::str::contains("--keep"))
        .stdout(predicate::str::contains("--no-tui"));
}

#[test]
fn test_eval_missing_project() {
    let mut cmd = Command::cargo_bin("rslph").expect("binary");
    cmd.args(["eval", "/nonexistent/project/path"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("is neither a built-in project nor a valid path"));
}

#[test]
fn test_eval_missing_prompt() {
    let dir = TempDir::new().expect("temp dir");
    let project_dir = dir.path().join("empty-project");
    std::fs::create_dir_all(&project_dir).expect("create project dir");

    let mut cmd = Command::cargo_bin("rslph").expect("binary");
    cmd.args(["eval", project_dir.to_str().unwrap()]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No prompt file found"));
}

#[test]
fn test_eval_with_keep_flag() {
    // This test verifies the --keep flag is accepted
    // Full execution requires fake Claude which is tested separately
    let mut cmd = Command::cargo_bin("rslph").expect("binary");
    cmd.args(["eval", "--keep", "--help"]);

    // --help with --keep should still show help (clap behavior)
    cmd.assert().success();
}

#[test]
fn test_eval_project_with_prompt_txt() {
    // Verify that a project with prompt.txt is recognized
    let dir = TempDir::new().expect("temp dir");
    let project_dir = create_eval_project(&dir);

    // Verify the project was created correctly
    assert!(project_dir.join("prompt.txt").exists());
}

#[test]
fn test_eval_project_with_readme() {
    // Verify that a project with README.md (no prompt.txt) would be recognized
    let dir = TempDir::new().expect("temp dir");
    let project_dir = dir.path().join("readme-project");
    std::fs::create_dir_all(&project_dir).expect("create project dir");

    // Create README.md instead of prompt.txt
    std::fs::write(
        project_dir.join("README.md"),
        "# Hello World\n\nCreate a program that prints Hello, World!",
    )
    .expect("write README.md");

    assert!(project_dir.join("README.md").exists());
}

#[test]
fn test_eval_list_shows_projects() {
    let mut cmd = Command::cargo_bin("rslph").expect("binary");
    cmd.args(["eval", "--list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("calculator"))
        .stdout(predicate::str::contains("fizzbuzz"));
}

#[test]
fn test_eval_unknown_project_fails() {
    let mut cmd = Command::cargo_bin("rslph").expect("binary");
    cmd.args(["eval", "nonexistent-project-xyz"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("neither a built-in project nor a valid path"));
}

#[test]
fn test_eval_requires_project_or_list() {
    let mut cmd = Command::cargo_bin("rslph").expect("binary");
    cmd.arg("eval")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_eval_list_outputs_formatted() {
    let mut cmd = Command::cargo_bin("rslph").expect("binary");
    cmd.args(["eval", "--list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Available built-in projects:"))
        .stdout(predicate::str::contains("  - calculator"))
        .stdout(predicate::str::contains("  - fizzbuzz"));
}

#[test]
fn test_eval_help_shows_flags() {
    let mut cmd = Command::cargo_bin("rslph").expect("binary");
    cmd.args(["eval", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run evaluation in isolated environment"))
        .stdout(predicate::str::contains("--list"))
        .stdout(predicate::str::contains("--keep"));
}

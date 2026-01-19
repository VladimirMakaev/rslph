use super::fixtures::Workspace;
use std::process::Command;

/// Assert that a task is marked complete in progress file.
///
/// Checks for `- [x] {task_pattern}` in PROGRESS.md.
///
/// # Panics
/// Panics with detailed message if task is not found as complete.
pub fn assert_task_complete(workspace: &Workspace, task_pattern: &str) {
    let content = workspace.read_file("PROGRESS.md");
    let pattern = format!("- [x] {}", task_pattern);
    assert!(
        content.contains(&pattern),
        "Expected task '{}' to be complete in PROGRESS.md.\nActual content:\n{}",
        task_pattern,
        content
    );
}

/// Assert that a task is NOT complete (still pending).
///
/// Checks for `- [ ] {task_pattern}` in PROGRESS.md.
///
/// # Panics
/// Panics with detailed message if task is not found as pending.
pub fn assert_task_pending(workspace: &Workspace, task_pattern: &str) {
    let content = workspace.read_file("PROGRESS.md");
    let pattern = format!("- [ ] {}", task_pattern);
    assert!(
        content.contains(&pattern),
        "Expected task '{}' to be pending in PROGRESS.md.\nActual content:\n{}",
        task_pattern,
        content
    );
}

/// Assert that RALPH_DONE marker exists in progress file.
///
/// # Panics
/// Panics if RALPH_DONE is not found in PROGRESS.md.
pub fn assert_ralph_done(workspace: &Workspace) {
    let content = workspace.read_file("PROGRESS.md");
    assert!(
        content.contains("RALPH_DONE"),
        "Expected RALPH_DONE marker in PROGRESS.md.\nActual content:\n{}",
        content
    );
}

/// Assert that RALPH_DONE marker does NOT exist.
///
/// # Panics
/// Panics if RALPH_DONE is found in PROGRESS.md.
pub fn assert_not_ralph_done(workspace: &Workspace) {
    if workspace.file_exists("PROGRESS.md") {
        let content = workspace.read_file("PROGRESS.md");
        assert!(
            !content.contains("RALPH_DONE"),
            "Expected NO RALPH_DONE marker in PROGRESS.md.\nActual content:\n{}",
            content
        );
    }
}

/// Assert file contains expected content.
///
/// # Panics
/// Panics if file does not contain the expected string.
pub fn assert_file_contains(workspace: &Workspace, path: &str, expected: &str) {
    let content = workspace.read_file(path);
    assert!(
        content.contains(expected),
        "Expected '{}' to contain '{}'.\nActual content:\n{}",
        path,
        expected,
        content
    );
}

/// Assert file does NOT contain content.
///
/// # Panics
/// Panics if file contains the unexpected string.
pub fn assert_file_not_contains(workspace: &Workspace, path: &str, unexpected: &str) {
    let content = workspace.read_file(path);
    assert!(
        !content.contains(unexpected),
        "Expected '{}' to NOT contain '{}'.\nActual content:\n{}",
        path,
        unexpected,
        content
    );
}

/// Assert git commit exists with message pattern.
///
/// Searches the last 10 commits for a matching message pattern.
///
/// # Panics
/// Panics if no commit with the pattern is found.
pub fn assert_git_commit_exists(workspace: &Workspace, message_pattern: &str) {
    let output = Command::new("git")
        .args(["log", "--oneline", "-10"])
        .current_dir(workspace.path())
        .output()
        .expect("Failed to run git log");

    let log = String::from_utf8_lossy(&output.stdout);
    assert!(
        log.contains(message_pattern),
        "Expected commit with '{}' in git log.\nActual log:\n{}",
        message_pattern,
        log
    );
}

/// Assert no uncommitted changes exist (working tree clean).
///
/// # Panics
/// Panics if there are uncommitted changes.
pub fn assert_git_clean(workspace: &Workspace) {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(workspace.path())
        .output()
        .expect("Failed to run git status");

    let status = String::from_utf8_lossy(&output.stdout);
    assert!(
        status.trim().is_empty(),
        "Expected clean git working tree.\nActual status:\n{}",
        status
    );
}

/// Get number of git commits in repository.
///
/// Returns 0 if there are no commits or git is not initialized.
pub fn git_commit_count(workspace: &Workspace) -> usize {
    let output = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .current_dir(workspace.path())
        .output()
        .ok()
        .filter(|o| o.status.success());

    output
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::e2e::fixtures::WorkspaceBuilder;

    #[test]
    fn test_assert_task_complete() {
        let workspace = WorkspaceBuilder::new()
            .with_progress_file("- [x] Task 1\n- [ ] Task 2")
            .build();
        assert_task_complete(&workspace, "Task 1");
    }

    #[test]
    fn test_assert_task_pending() {
        let workspace = WorkspaceBuilder::new()
            .with_progress_file("- [x] Task 1\n- [ ] Task 2")
            .build();
        assert_task_pending(&workspace, "Task 2");
    }

    #[test]
    fn test_assert_ralph_done() {
        let workspace = WorkspaceBuilder::new()
            .with_progress_file("- [x] Task 1\nRALPH_DONE")
            .build();
        assert_ralph_done(&workspace);
    }

    #[test]
    fn test_assert_file_contains() {
        let workspace = WorkspaceBuilder::new()
            .with_source_file("test.txt", "hello world")
            .build();
        assert_file_contains(&workspace, "test.txt", "hello");
    }

    #[test]
    fn test_assert_git_commit_exists() {
        let workspace = WorkspaceBuilder::new()
            .with_source_file("test.txt", "content")
            .build();

        // Create a commit
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(workspace.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(workspace.path())
            .output()
            .unwrap();

        assert_git_commit_exists(&workspace, "Initial commit");
    }
}

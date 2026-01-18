//! Git VCS implementation.

use std::path::PathBuf;
use std::process::{Command, Output};

use crate::error::VcsError;
use crate::vcs::{Vcs, VcsType};

/// Git VCS implementation.
pub struct GitVcs {
    root: PathBuf,
}

impl GitVcs {
    /// Create a new GitVcs instance with the given repository root.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Run a git command and return the output.
    fn run_git(&self, args: &[&str]) -> Result<Output, VcsError> {
        Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .output()
            .map_err(|e| VcsError::CommandFailed {
                command: format!("git {}", args.join(" ")),
                error: e.to_string(),
            })
    }
}

impl Vcs for GitVcs {
    fn vcs_type(&self) -> VcsType {
        VcsType::Git
    }

    fn has_changes(&self) -> Result<bool, VcsError> {
        let output = self.run_git(&["status", "--porcelain"])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(VcsError::CommandFailed {
                command: "git status --porcelain".to_string(),
                error: stderr.to_string(),
            });
        }
        // Non-empty output means there are changes
        Ok(!output.stdout.is_empty())
    }

    fn stage_all(&self) -> Result<(), VcsError> {
        let output = self.run_git(&["add", "-A"])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(VcsError::CommandFailed {
                command: "git add -A".to_string(),
                error: stderr.to_string(),
            });
        }
        Ok(())
    }

    fn commit(&self, message: &str) -> Result<String, VcsError> {
        let output = self.run_git(&["commit", "-m", message, "--no-verify"])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Check for "nothing to commit" case
            if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
                return Err(VcsError::NothingToCommit);
            }
            return Err(VcsError::CommitFailed(stderr.to_string()));
        }

        // Parse commit hash from output
        // Git outputs something like: [branch abc1234] Message
        let stdout = String::from_utf8_lossy(&output.stdout);
        let hash = stdout
            .lines()
            .next()
            .and_then(|line| {
                // Find text between [ and ]
                let start = line.find('[')? + 1;
                let end = line.find(']')?;
                let bracket_content = &line[start..end];
                // Split by space, take last part (the hash)
                bracket_content.split_whitespace().last()
            })
            .unwrap_or("unknown")
            .to_string();

        Ok(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vcs::Vcs;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_git_repo() -> (TempDir, GitVcs) {
        let dir = TempDir::new().expect("temp dir");
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .expect("git init");
        // Configure user for commits
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .expect("git config email");
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .expect("git config name");

        let vcs = GitVcs::new(dir.path().to_path_buf());
        (dir, vcs)
    }

    #[test]
    fn test_git_has_no_changes_in_clean_repo() {
        let (_dir, vcs) = setup_git_repo();
        // Fresh repo with no files has no changes
        let has_changes = vcs.has_changes().expect("has_changes");
        assert!(!has_changes);
    }

    #[test]
    fn test_git_has_changes_with_new_file() {
        let (dir, vcs) = setup_git_repo();
        fs::write(dir.path().join("test.txt"), "hello").expect("write");
        assert!(vcs.has_changes().expect("has_changes"));
    }

    #[test]
    fn test_git_commit_all() {
        let (dir, vcs) = setup_git_repo();
        fs::write(dir.path().join("test.txt"), "hello").expect("write");

        let result = vcs.commit_all("Test commit");
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert!(hash.is_some()); // Should have committed
    }

    #[test]
    fn test_git_commit_all_no_changes() {
        let (dir, vcs) = setup_git_repo();
        // Make an initial commit first
        fs::write(dir.path().join("test.txt"), "hello").expect("write");
        vcs.commit_all("Initial").expect("initial commit");

        // Now try to commit with no changes
        let result = vcs.commit_all("No changes");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // Nothing to commit
    }
}

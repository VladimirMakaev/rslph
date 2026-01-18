//! Sapling VCS implementation.

use std::path::PathBuf;
use std::process::{Command, Output};

use crate::error::VcsError;
use crate::vcs::{Vcs, VcsType};

/// Sapling VCS implementation.
pub struct SaplingVcs {
    root: PathBuf,
}

impl SaplingVcs {
    /// Create a new SaplingVcs instance with the given repository root.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Run a Sapling command and return the output.
    fn run_sl(&self, args: &[&str]) -> Result<Output, VcsError> {
        Command::new("sl")
            .args(args)
            .current_dir(&self.root)
            .output()
            .map_err(|e| VcsError::CommandFailed {
                command: format!("sl {}", args.join(" ")),
                error: e.to_string(),
            })
    }
}

impl Vcs for SaplingVcs {
    fn vcs_type(&self) -> VcsType {
        VcsType::Sapling
    }

    fn has_changes(&self) -> Result<bool, VcsError> {
        let output = self.run_sl(&["status"])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(VcsError::CommandFailed {
                command: "sl status".to_string(),
                error: stderr.to_string(),
            });
        }
        // Non-empty output means there are changes
        Ok(!output.stdout.is_empty())
    }

    fn stage_all(&self) -> Result<(), VcsError> {
        let output = self.run_sl(&["addremove"])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(VcsError::CommandFailed {
                command: "sl addremove".to_string(),
                error: stderr.to_string(),
            });
        }
        Ok(())
    }

    fn commit(&self, message: &str) -> Result<String, VcsError> {
        let output = self.run_sl(&["commit", "-m", message])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Check for "nothing changed" case
            if stdout.contains("nothing changed") || stderr.contains("nothing changed") {
                return Err(VcsError::NothingToCommit);
            }
            return Err(VcsError::CommitFailed(stderr.to_string()));
        }

        // Parse commit hash from output
        // Sapling outputs the commit hash on stdout
        let stdout = String::from_utf8_lossy(&output.stdout);
        let hash = stdout
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().last())
            .unwrap_or("unknown")
            .to_string();

        Ok(hash)
    }
}

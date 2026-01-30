//! VCS (Version Control System) integration for auto-commit after iterations.
//!
//! Provides trait abstraction for Git and Sapling, with auto-detection.

mod git;
mod sapling;

pub use git::GitVcs;
pub use sapling::SaplingVcs;

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::VcsError;

/// Supported VCS types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsType {
    Git,
    Sapling,
}

impl fmt::Display for VcsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VcsType::Git => write!(f, "Git"),
            VcsType::Sapling => write!(f, "Sapling"),
        }
    }
}

/// Detection result containing VCS type and repository root.
#[derive(Debug, Clone)]
pub struct VcsDetection {
    pub vcs_type: VcsType,
    pub root: PathBuf,
}

/// Trait for VCS operations.
pub trait Vcs: Send + Sync {
    /// Get the VCS type.
    fn vcs_type(&self) -> VcsType;

    /// Check if there are uncommitted changes.
    fn has_changes(&self) -> Result<bool, VcsError>;

    /// Stage all changes for commit.
    fn stage_all(&self) -> Result<(), VcsError>;

    /// Create a commit with the given message, returns commit hash.
    fn commit(&self, message: &str) -> Result<String, VcsError>;

    /// Stage all changes and commit if there are changes.
    /// Returns None if there was nothing to commit.
    fn commit_all(&self, message: &str) -> Result<Option<String>, VcsError> {
        if !self.has_changes()? {
            return Ok(None);
        }
        self.stage_all()?;
        let hash = self.commit(message)?;
        Ok(Some(hash))
    }
}

/// Detect VCS type for the given path.
///
/// Detection order:
/// 1. Try `sl root` command (Sapling)
/// 2. Walk up directories looking for `.git` directory (Git)
///
/// Returns None if no VCS is detected (not an error).
pub fn detect_vcs(start_path: &Path) -> Result<Option<VcsDetection>, VcsError> {
    // If start_path is a file, use parent directory
    let search_path = if start_path.is_file() {
        start_path.parent().unwrap_or(start_path)
    } else {
        start_path
    };

    // Canonicalize path for consistent detection
    let canonical = search_path
        .canonicalize()
        .map_err(|e| VcsError::Detection(format!("Failed to canonicalize path: {}", e)))?;

    // Try Sapling first via `sl root`
    if let Ok(output) = Command::new("sl")
        .args(["root"])
        .current_dir(&canonical)
        .output()
    {
        if output.status.success() {
            let root_str = String::from_utf8_lossy(&output.stdout);
            let root = PathBuf::from(root_str.trim());
            return Ok(Some(VcsDetection {
                vcs_type: VcsType::Sapling,
                root,
            }));
        }
    }

    // Fall back to Git via .git directory search
    let mut current = canonical.as_path();
    loop {
        let git_dir = current.join(".git");
        if git_dir.exists() {
            // Verify git is available
            if Command::new("git")
                .args(["--version"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Ok(Some(VcsDetection {
                    vcs_type: VcsType::Git,
                    root: current.to_path_buf(),
                }));
            }
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    // No VCS found
    Ok(None)
}

/// Create a VCS instance for the given working directory.
///
/// Returns None if no VCS is detected.
pub fn create_vcs(working_dir: &Path) -> Option<Box<dyn Vcs>> {
    match detect_vcs(working_dir) {
        Ok(Some(detection)) => match detection.vcs_type {
            VcsType::Git => Some(Box::new(GitVcs::new(detection.root))),
            VcsType::Sapling => Some(Box::new(SaplingVcs::new(detection.root))),
        },
        Ok(None) => None,
        Err(e) => {
            eprintln!("[VCS] Warning: Detection failed: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn test_vcs_type_display() {
        assert_eq!(VcsType::Git.to_string(), "Git");
        assert_eq!(VcsType::Sapling.to_string(), "Sapling");
    }

    #[test]
    fn test_detect_vcs_in_git_repo() {
        // Create temp dir with git init
        let dir = TempDir::new().expect("temp dir");
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .expect("git init");

        let detection = detect_vcs(dir.path()).expect("detect");
        assert!(detection.is_some());
        let det = detection.unwrap();
        // May be Git or Sapling depending on environment
        assert!(det.vcs_type == VcsType::Git || det.vcs_type == VcsType::Sapling);
    }

    #[test]
    fn test_detect_vcs_no_repo() {
        let dir = TempDir::new().expect("temp dir");
        // No git init, just empty dir - may still detect Sapling in sl-enabled environments
        let detection = detect_vcs(dir.path()).expect("detect");
        // Either None or Sapling depending on environment
        // This test validates detection doesn't error, not specific result
        let _ = detection;
    }

    #[test]
    fn test_create_vcs_returns_implementation() {
        let dir = TempDir::new().expect("temp dir");
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .expect("git init");

        let vcs = create_vcs(dir.path());
        assert!(vcs.is_some());
    }
}

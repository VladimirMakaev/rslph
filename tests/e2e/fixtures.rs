use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Builder for creating isolated test workspaces.
///
/// Provides a fluent API for configuring test workspaces with:
/// - Git initialization
/// - Config file setup
/// - Progress file content
/// - Source files
pub struct WorkspaceBuilder {
    temp_dir: TempDir,
    init_git: bool,
    config: Option<String>,
    progress_content: Option<String>,
    source_files: Vec<(PathBuf, String)>,
}

impl WorkspaceBuilder {
    /// Create a new workspace builder with default settings.
    ///
    /// By default:
    /// - Git is initialized
    /// - Default config is created
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("Failed to create temp dir"),
            init_git: true,
            config: None,
            progress_content: None,
            source_files: vec![],
        }
    }

    /// Set progress file content.
    ///
    /// Creates a PROGRESS.md file in the workspace root.
    pub fn with_progress_file(mut self, content: &str) -> Self {
        self.progress_content = Some(content.to_string());
        self
    }

    /// Add a source file to the workspace.
    ///
    /// Parent directories are created automatically.
    pub fn with_source_file(mut self, path: &str, content: &str) -> Self {
        self.source_files
            .push((PathBuf::from(path), content.to_string()));
        self
    }

    /// Set custom config TOML content.
    ///
    /// Overrides the default config file at .rslph/config.toml
    pub fn with_config(mut self, config_toml: &str) -> Self {
        self.config = Some(config_toml.to_string());
        self
    }

    /// Disable git initialization.
    ///
    /// Useful for testing non-git scenarios.
    pub fn without_git(mut self) -> Self {
        self.init_git = false;
        self
    }

    /// Build the workspace with all configured settings.
    ///
    /// Returns a Workspace that is automatically cleaned up when dropped.
    pub fn build(self) -> Workspace {
        let path = self.temp_dir.path().to_path_buf();

        // Initialize git if requested
        if self.init_git {
            Command::new("git")
                .args(["init"])
                .current_dir(&path)
                .output()
                .expect("Failed to init git");

            Command::new("git")
                .args(["config", "user.email", "test@test.com"])
                .current_dir(&path)
                .output()
                .expect("Failed to set git email");

            Command::new("git")
                .args(["config", "user.name", "Test"])
                .current_dir(&path)
                .output()
                .expect("Failed to set git name");
        }

        // Write config
        let config_dir = path.join(".rslph");
        std::fs::create_dir_all(&config_dir).expect("Failed to create config dir");
        let config_content = self
            .config
            .unwrap_or_else(|| "[rslph]\nclaude_path = \"claude\"\n".to_string());
        std::fs::write(config_dir.join("config.toml"), &config_content)
            .expect("Failed to write config");

        // Write progress file
        if let Some(content) = self.progress_content {
            std::fs::write(path.join("PROGRESS.md"), &content)
                .expect("Failed to write progress file");
        }

        // Write source files
        for (rel_path, content) in self.source_files {
            let full_path = path.join(&rel_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).expect("Failed to create parent dirs");
            }
            std::fs::write(full_path, content).expect("Failed to write source file");
        }

        Workspace {
            _temp_dir: self.temp_dir,
            path,
        }
    }
}

impl Default for WorkspaceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// An isolated test workspace with automatic cleanup.
///
/// The workspace is backed by a temporary directory that is
/// automatically deleted when the Workspace is dropped.
pub struct Workspace {
    _temp_dir: TempDir, // Keep alive for RAII cleanup
    path: PathBuf,
}

impl Workspace {
    /// Get the workspace root path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read a file from the workspace.
    ///
    /// # Panics
    /// Panics if the file does not exist or cannot be read.
    pub fn read_file(&self, rel_path: &str) -> String {
        std::fs::read_to_string(self.path.join(rel_path))
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", rel_path, e))
    }

    /// Check if a file exists in the workspace.
    pub fn file_exists(&self, rel_path: &str) -> bool {
        self.path.join(rel_path).exists()
    }

    /// Write a file to the workspace.
    ///
    /// Parent directories are created automatically.
    ///
    /// # Panics
    /// Panics if the file cannot be written.
    pub fn write_file(&self, rel_path: &str, content: &str) {
        let full_path = self.path.join(rel_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(full_path, content)
            .unwrap_or_else(|e| panic!("Failed to write {}: {}", rel_path, e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creates_temp_directory() {
        let workspace = WorkspaceBuilder::new().build();
        assert!(workspace.path().exists());
    }

    #[test]
    fn test_workspace_initializes_git() {
        let workspace = WorkspaceBuilder::new().build();
        assert!(workspace.path().join(".git").exists());
    }

    #[test]
    fn test_workspace_creates_config() {
        let workspace = WorkspaceBuilder::new().build();
        assert!(workspace.file_exists(".rslph/config.toml"));
    }

    #[test]
    fn test_workspace_with_progress_file() {
        let workspace = WorkspaceBuilder::new()
            .with_progress_file("- [ ] Task 1\n- [ ] Task 2")
            .build();
        let content = workspace.read_file("PROGRESS.md");
        assert!(content.contains("Task 1"));
        assert!(content.contains("Task 2"));
    }

    #[test]
    fn test_workspace_with_source_file() {
        let workspace = WorkspaceBuilder::new()
            .with_source_file("src/main.rs", "fn main() {}")
            .build();
        let content = workspace.read_file("src/main.rs");
        assert_eq!(content, "fn main() {}");
    }

    #[test]
    fn test_workspace_without_git() {
        let workspace = WorkspaceBuilder::new().without_git().build();
        assert!(!workspace.path().join(".git").exists());
    }

    #[test]
    fn test_workspace_custom_config() {
        let workspace = WorkspaceBuilder::new()
            .with_config("[custom]\nkey = \"value\"")
            .build();
        let content = workspace.read_file(".rslph/config.toml");
        assert!(content.contains("key = \"value\""));
    }

    #[test]
    fn test_workspace_write_file() {
        let workspace = WorkspaceBuilder::new().build();
        workspace.write_file("test.txt", "hello");
        assert_eq!(workspace.read_file("test.txt"), "hello");
    }
}

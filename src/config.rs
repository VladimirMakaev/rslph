use crate::prompts::PromptMode;
use directories::{BaseDirs, ProjectDirs};
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve a command name to absolute path using `which`.
/// Returns original path if resolution fails (graceful fallback).
fn resolve_command_path(command: &str) -> String {
    // Already absolute - no resolution needed
    if Path::new(command).is_absolute() {
        return command.to_string();
    }

    // Try which to find absolute path
    Command::new("which")
        .arg(command)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| command.to_string())
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Path to claude CLI executable (CFG-03)
    pub claude_path: String,

    /// Maximum iterations before stopping (CFG-06)
    pub max_iterations: u32,

    /// Number of recent threads to display (CFG-07)
    pub recent_threads: u32,

    /// Notification interval - every N iterations (CFG-08)
    pub notify_interval: u32,

    /// Path to plan prompt file override (CFG-04)
    pub plan_prompt: Option<PathBuf>,

    /// Path to build prompt file override (CFG-04)
    pub build_prompt: Option<PathBuf>,

    /// Shell for notify script execution (CFG-05)
    pub notify_shell: String,

    /// Enable TUI mode for build command (TUI-09)
    pub tui_enabled: bool,

    /// Number of recent messages to display in TUI (TUI-09)
    pub tui_recent_messages: usize,

    /// Directory for persisting eval workspaces and results (EVAL-06)
    /// Defaults to ~/.rslph/evals
    pub eval_dir: PathBuf,

    /// Timeout in seconds for each build iteration (default 600)
    pub iteration_timeout: u64,

    /// Maximum retries for timed-out iterations before failing (default 3)
    pub timeout_retries: u32,

    /// Prompt mode selection (basic, gsd, gsd_tdd)
    pub prompt_mode: PromptMode,
}

impl Default for Config {
    fn default() -> Self {
        // Default eval_dir to ~/.rslph/evals
        let eval_dir = BaseDirs::new()
            .map(|d| d.home_dir().join(".rslph").join("evals"))
            .unwrap_or_else(|| PathBuf::from(".rslph/evals"));

        Self {
            claude_path: "claude".to_string(),
            max_iterations: 20,
            recent_threads: 5,
            notify_interval: 10,
            plan_prompt: None,
            build_prompt: None,
            notify_shell: "/bin/sh".to_string(),
            tui_enabled: true,
            tui_recent_messages: 10,
            eval_dir,
            iteration_timeout: 600,
            timeout_retries: 3,
            prompt_mode: PromptMode::default(),
        }
    }
}

impl Config {
    /// Get the default config file path (XDG-compliant)
    pub fn default_path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "rslph").map(|dirs| dirs.config_dir().join("config.toml"))
    }

    /// Load config from file and environment (CFG-01)
    /// Precedence: defaults < config file < environment
    /// CLI args are merged by the caller (Plan 01-02)
    pub fn load(config_path: Option<&Path>) -> color_eyre::Result<Self> {
        let path = config_path.map(PathBuf::from).or_else(Self::default_path);

        let mut figment = Figment::new().merge(Serialized::defaults(Config::default()));

        // Only merge file if it exists (handle first-run gracefully)
        if let Some(ref p) = path {
            if p.exists() {
                figment = figment.merge(Toml::file(p));
            }
        }

        // Environment variables with RSLPH_ prefix (lowercase, no split for flat config)
        figment = figment.merge(Env::prefixed("RSLPH_").lowercase(true));

        let mut config: Config = figment.extract()?;
        config.claude_path = resolve_command_path(&config.claude_path);
        Ok(config)
    }

    /// Load config with explicit CLI overrides merged last
    /// This is the main entry point used by the CLI
    pub fn load_with_overrides(
        config_path: Option<&Path>,
        overrides: PartialConfig,
    ) -> color_eyre::Result<Self> {
        let path = config_path.map(PathBuf::from).or_else(Self::default_path);

        let mut figment = Figment::new().merge(Serialized::defaults(Config::default()));

        if let Some(ref p) = path {
            if p.exists() {
                figment = figment.merge(Toml::file(p));
            }
        }

        // Environment variables with RSLPH_ prefix (lowercase, no split for flat config)
        figment = figment.merge(Env::prefixed("RSLPH_").lowercase(true));

        // CLI overrides are highest precedence
        figment = figment.merge(Serialized::defaults(overrides));

        let mut config: Config = figment.extract()?;
        config.claude_path = resolve_command_path(&config.claude_path);
        Ok(config)
    }
}

/// Partial config for CLI overrides (only set fields are merged)
#[derive(Debug, Default, Serialize)]
pub struct PartialConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_threads: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_interval: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_prompt: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_prompt: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_shell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tui_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tui_recent_messages: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iteration_timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_mode: Option<PromptMode>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize tests that use environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.claude_path, "claude");
        assert_eq!(config.max_iterations, 20);
        assert_eq!(config.recent_threads, 5);
        assert_eq!(config.notify_interval, 10);
        assert!(config.plan_prompt.is_none());
        assert!(config.build_prompt.is_none());
        assert_eq!(config.notify_shell, "/bin/sh");
        assert!(config.tui_enabled);
        assert_eq!(config.tui_recent_messages, 10);
        assert_eq!(config.iteration_timeout, 600);
        assert_eq!(config.timeout_retries, 3);
        assert_eq!(config.prompt_mode, PromptMode::Basic);
        // eval_dir should end with .rslph/evals
        assert!(
            config.eval_dir.ends_with(".rslph/evals"),
            "eval_dir should end with .rslph/evals, got: {:?}",
            config.eval_dir
        );
    }

    #[test]
    fn test_load_missing_file_uses_defaults() {
        let _guard = ENV_MUTEX.lock().unwrap();
        // Ensure env var is not set
        std::env::remove_var("RSLPH_MAX_ITERATIONS");
        let config = Config::load(Some(Path::new("/nonexistent/config.toml")))
            .expect("Should use defaults when file missing");
        assert_eq!(config.max_iterations, 20);
    }

    #[test]
    fn test_env_override() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("RSLPH_MAX_ITERATIONS", "50");
        let config = Config::load(None).expect("Should load");
        assert_eq!(config.max_iterations, 50);
        std::env::remove_var("RSLPH_MAX_ITERATIONS");
    }

    #[test]
    fn test_default_path_is_xdg_compliant() {
        let path = Config::default_path();
        assert!(path.is_some());
        let path = path.unwrap();
        // Should end with rslph/config.toml
        assert!(path.ends_with("rslph/config.toml"));
    }

    #[test]
    fn test_cli_overrides_highest() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("RSLPH_MAX_ITERATIONS", "50");
        let overrides = PartialConfig {
            max_iterations: Some(100),
            ..Default::default()
        };
        let config = Config::load_with_overrides(None, overrides).expect("Should load");
        assert_eq!(config.max_iterations, 100); // CLI wins over env
        std::env::remove_var("RSLPH_MAX_ITERATIONS");
    }

    #[test]
    fn test_resolve_command_path_absolute_unchanged() {
        // Absolute paths should be returned unchanged
        let result = resolve_command_path("/bin/echo");
        assert_eq!(result, "/bin/echo");
    }

    #[test]
    fn test_resolve_command_path_relative_resolved() {
        // Relative command names should be resolved to absolute path
        let result = resolve_command_path("echo");
        // Should resolve to an absolute path containing "echo"
        assert!(
            result.starts_with('/'),
            "Expected absolute path, got: {}",
            result
        );
        assert!(
            result.ends_with("echo"),
            "Expected path ending in echo, got: {}",
            result
        );
    }

    #[test]
    fn test_resolve_command_path_nonexistent_fallback() {
        // Non-existent commands should fall back to original value
        let result = resolve_command_path("nonexistent_command_xyz_12345");
        assert_eq!(result, "nonexistent_command_xyz_12345");
    }

    #[test]
    fn test_default_prompt_mode() {
        let config = Config::default();
        assert_eq!(config.prompt_mode, PromptMode::Basic);
    }
}

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            claude_path: "claude".to_string(),
            max_iterations: 20,
            recent_threads: 5,
            notify_interval: 10,
            plan_prompt: None,
            build_prompt: None,
            notify_shell: "/bin/sh".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}

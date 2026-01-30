//! Prompt loading with config override support.

use super::PromptMode;
use crate::config::Config;

/// Get the planning prompt, using config override if specified.
///
/// Precedence: file override > mode selection
pub fn get_plan_prompt(config: &Config) -> color_eyre::Result<String> {
    // File override takes precedence (power users)
    if let Some(path) = &config.plan_prompt {
        return std::fs::read_to_string(path).map_err(|e| {
            color_eyre::eyre::eyre!(
                "Failed to read plan prompt from '{}': {}",
                path.display(),
                e
            )
        });
    }

    // Mode-based selection
    Ok(config.prompt_mode.plan_prompt().to_string())
}

/// Get the planning prompt for a specific mode.
///
/// This function bypasses config file overrides and returns the prompt
/// for the specified mode directly. Used by eval to ensure different
/// modes use their correct prompts.
pub fn get_plan_prompt_for_mode(mode: PromptMode) -> String {
    mode.plan_prompt().to_string()
}

/// Get the build prompt, using config override if specified.
///
/// Precedence: file override > mode selection
pub fn get_build_prompt(config: &Config) -> color_eyre::Result<String> {
    // File override takes precedence (power users)
    if let Some(path) = &config.build_prompt {
        return std::fs::read_to_string(path).map_err(|e| {
            color_eyre::eyre::eyre!(
                "Failed to read build prompt from '{}': {}",
                path.display(),
                e
            )
        });
    }

    // Mode-based selection
    Ok(config.prompt_mode.build_prompt().to_string())
}

/// Get the build prompt for a specific mode.
///
/// This function bypasses config file overrides and returns the prompt
/// for the specified mode directly. Used by eval to ensure different
/// modes use their correct prompts.
pub fn get_build_prompt_for_mode(mode: PromptMode) -> String {
    mode.build_prompt().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::PromptMode;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_get_plan_prompt_uses_mode_default() {
        let config = Config::default();
        let prompt = get_plan_prompt(&config).expect("Should get default prompt");
        // Default is basic mode
        assert!(prompt.contains("Planning"));
    }

    #[test]
    fn test_get_plan_prompt_respects_mode() {
        let config = Config {
            prompt_mode: PromptMode::Gsd,
            ..Default::default()
        };
        let prompt = get_plan_prompt(&config).expect("Should get GSD prompt");
        // GSD prompt should have different content
        assert!(!prompt.is_empty());
    }

    #[test]
    fn test_get_plan_prompt_file_override_wins() {
        let mut temp = NamedTempFile::new().expect("Should create temp file");
        writeln!(temp, "Custom planning prompt").expect("Should write");

        let config = Config {
            plan_prompt: Some(temp.path().to_path_buf()),
            prompt_mode: PromptMode::Gsd, // Mode should be ignored
            ..Default::default()
        };

        let prompt = get_plan_prompt(&config).expect("Should read override");
        assert!(prompt.contains("Custom planning prompt"));
    }

    #[test]
    fn test_get_build_prompt_uses_mode_default() {
        let config = Config::default();
        let prompt = get_build_prompt(&config).expect("Should get default prompt");
        assert!(prompt.contains("RALPH_DONE"));
    }

    #[test]
    fn test_get_build_prompt_respects_mode() {
        let config = Config {
            prompt_mode: PromptMode::GsdTdd,
            ..Default::default()
        };
        let prompt = get_build_prompt(&config).expect("Should get TDD prompt");
        assert!(prompt.contains("TDD") || prompt.contains("tdd"));
    }

    #[test]
    fn test_get_build_prompt_file_override_wins() {
        let mut temp = NamedTempFile::new().expect("Should create temp file");
        writeln!(temp, "Custom build prompt").expect("Should write");

        let config = Config {
            build_prompt: Some(temp.path().to_path_buf()),
            prompt_mode: PromptMode::GsdTdd, // Mode should be ignored
            ..Default::default()
        };

        let prompt = get_build_prompt(&config).expect("Should read override");
        assert!(prompt.contains("Custom build prompt"));
    }

    #[test]
    fn test_get_plan_prompt_error_on_missing_override() {
        let config = Config {
            plan_prompt: Some("/nonexistent/path/prompt.md".into()),
            ..Default::default()
        };

        let result = get_plan_prompt(&config);
        assert!(result.is_err());
    }
}

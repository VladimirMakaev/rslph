//! Prompt loading with config override support.

use crate::config::Config;

use super::defaults;

/// Get the planning prompt, using config override if specified.
///
/// If `config.plan_prompt` is set, reads the prompt from that file path.
/// Otherwise, returns the baked-in default prompt.
pub fn get_plan_prompt(config: &Config) -> color_eyre::Result<String> {
    match &config.plan_prompt {
        Some(path) => {
            std::fs::read_to_string(path).map_err(|e| {
                color_eyre::eyre::eyre!(
                    "Failed to read plan prompt from '{}': {}",
                    path.display(),
                    e
                )
            })
        }
        None => Ok(defaults::default_plan_prompt().to_string()),
    }
}

/// Get the build prompt, using config override if specified.
///
/// If `config.build_prompt` is set, reads the prompt from that file path.
/// Otherwise, returns the baked-in default prompt.
pub fn get_build_prompt(config: &Config) -> color_eyre::Result<String> {
    match &config.build_prompt {
        Some(path) => {
            std::fs::read_to_string(path).map_err(|e| {
                color_eyre::eyre::eyre!(
                    "Failed to read build prompt from '{}': {}",
                    path.display(),
                    e
                )
            })
        }
        None => Ok(defaults::default_build_prompt().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_get_plan_prompt_uses_default() {
        let config = Config::default();
        let prompt = get_plan_prompt(&config).expect("Should get default prompt");
        assert!(prompt.contains("Planning Assistant"));
    }

    #[test]
    fn test_get_plan_prompt_uses_override() {
        let mut temp = NamedTempFile::new().expect("Should create temp file");
        writeln!(temp, "Custom planning prompt").expect("Should write");

        let config = Config {
            plan_prompt: Some(temp.path().to_path_buf()),
            ..Default::default()
        };

        let prompt = get_plan_prompt(&config).expect("Should read override");
        assert!(prompt.contains("Custom planning prompt"));
    }

    #[test]
    fn test_get_plan_prompt_error_on_missing_override() {
        let config = Config {
            plan_prompt: Some("/nonexistent/path/prompt.md".into()),
            ..Default::default()
        };

        let result = get_plan_prompt(&config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to read plan prompt"));
    }

    #[test]
    fn test_get_build_prompt_uses_default() {
        let config = Config::default();
        let prompt = get_build_prompt(&config).expect("Should get default prompt");
        assert!(prompt.contains("Build Agent"));
        assert!(prompt.contains("RALPH_DONE"));
    }

    #[test]
    fn test_get_build_prompt_uses_override() {
        let mut temp = NamedTempFile::new().expect("Should create temp file");
        writeln!(temp, "Custom build prompt").expect("Should write");

        let config = Config {
            build_prompt: Some(temp.path().to_path_buf()),
            ..Default::default()
        };

        let prompt = get_build_prompt(&config).expect("Should read override");
        assert!(prompt.contains("Custom build prompt"));
    }

    #[test]
    fn test_get_build_prompt_error_on_missing_override() {
        let config = Config {
            build_prompt: Some("/nonexistent/path/prompt.md".into()),
            ..Default::default()
        };

        let result = get_build_prompt(&config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to read build prompt"));
    }
}

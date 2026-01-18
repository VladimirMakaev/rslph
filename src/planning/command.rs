//! Planning command handler.
//!
//! Executes Claude in headless mode to transform user ideas into structured progress files.

use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::error::RslphError;
use crate::planning::detect_stack;
use crate::progress::ProgressFile;
use crate::prompts::get_plan_prompt;
use crate::subprocess::{ClaudeRunner, OutputLine};

/// Run the planning command.
///
/// Executes Claude CLI in headless mode with the planning system prompt,
/// parses the output into a ProgressFile, and writes it to disk.
///
/// # Arguments
///
/// * `input` - User's idea/plan description
/// * `config` - Application configuration
/// * `working_dir` - Directory to use as working directory and output location
/// * `cancel_token` - Token for graceful cancellation
/// * `timeout` - Maximum duration to wait for Claude
///
/// # Returns
///
/// Path to the generated progress file.
pub async fn run_plan_command(
    input: &str,
    config: &Config,
    working_dir: &Path,
    cancel_token: CancellationToken,
    timeout: Duration,
) -> color_eyre::Result<PathBuf> {
    // Step 1: Detect project stack for testing strategy
    let stack = detect_stack(working_dir);

    // Step 2: Get the planning prompt (default or override)
    let system_prompt = get_plan_prompt(config)?;

    // Step 3: Build user input with stack context
    let full_input = format!(
        "## Detected Stack\n{}\n\n## User Request\n{}",
        stack.to_summary(),
        input
    );

    // Step 4: Build Claude CLI args for headless mode
    let args = vec![
        "-p".to_string(),                    // Print mode (headless)
        "--output-format".to_string(),       // Output format
        "text".to_string(),                  // Plain text
        "--system-prompt".to_string(),       // Custom system prompt
        system_prompt,
        full_input,                          // User input as positional arg
    ];

    // Step 5: Spawn Claude
    let mut runner = ClaudeRunner::spawn(&config.claude_path, &args, working_dir)
        .await
        .map_err(|e| RslphError::Subprocess(format!("Failed to spawn claude: {}", e)))?;

    // Step 6: Run with timeout and collect output
    let output = runner.run_with_timeout(timeout, cancel_token).await?;

    // Step 7: Collect stdout lines into response text
    let response_text: String = output
        .iter()
        .filter_map(|line| match line {
            OutputLine::Stdout(s) => Some(s.as_str()),
            OutputLine::Stderr(_) => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Step 8: Parse response into ProgressFile
    let progress_file = ProgressFile::parse(&response_text)?;

    // Step 9: Write to file
    let output_path = working_dir.join("progress.md");
    progress_file.write(&output_path)?;

    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_run_plan_command_spawns_and_writes_file() {
        // This test verifies the full command flow using echo as a mock.
        // Echo outputs garbage but ProgressFile::parse is lenient so it succeeds.
        let dir = TempDir::new().expect("temp dir");

        // Create a config pointing to echo instead of claude
        let config = Config {
            claude_path: "/bin/echo".to_string(),
            ..Default::default()
        };

        let token = CancellationToken::new();

        let result = run_plan_command(
            "build something",
            &config,
            dir.path(),
            token,
            Duration::from_secs(5),
        )
        .await;

        // Echo outputs something, parse is lenient, so this actually succeeds
        assert!(result.is_ok(), "Command should complete: {:?}", result);

        // Verify progress.md was created
        let output_path = result.unwrap();
        assert!(output_path.exists(), "Progress file should exist");
        assert!(output_path.ends_with("progress.md"));
    }

    #[tokio::test]
    async fn test_run_plan_command_timeout() {
        let dir = TempDir::new().expect("temp dir");

        // Use a script that ignores arguments and sleeps
        let script = "#!/bin/sh\nsleep 60\n";
        let script_path = dir.path().join("slow_script.sh");
        std::fs::write(&script_path, script).expect("write script");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
                .expect("set permissions");
        }

        let config = Config {
            claude_path: script_path.to_string_lossy().to_string(),
            ..Default::default()
        };

        let token = CancellationToken::new();
        let result = run_plan_command(
            "anything",
            &config,
            dir.path(),
            token,
            Duration::from_millis(100),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("timeout"), "Should timeout: {}", err);
    }

    #[tokio::test]
    async fn test_run_plan_command_cancellation() {
        let dir = TempDir::new().expect("temp dir");

        // Use a script that ignores arguments and sleeps
        let script = "#!/bin/sh\nsleep 60\n";
        let script_path = dir.path().join("slow_script.sh");
        std::fs::write(&script_path, script).expect("write script");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
                .expect("set permissions");
        }

        let config = Config {
            claude_path: script_path.to_string_lossy().to_string(),
            ..Default::default()
        };

        let token = CancellationToken::new();
        let token_clone = token.clone();

        // Cancel after 50ms
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            token_clone.cancel();
        });

        let result = run_plan_command(
            "anything",
            &config,
            dir.path(),
            token,
            Duration::from_secs(10),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cancelled"), "Should be cancelled: {}", err);
    }

    #[tokio::test]
    async fn test_run_plan_command_nonexistent_command() {
        let dir = TempDir::new().expect("temp dir");

        let config = Config {
            claude_path: "/nonexistent/command".to_string(),
            ..Default::default()
        };

        let token = CancellationToken::new();
        let result = run_plan_command(
            "anything",
            &config,
            dir.path(),
            token,
            Duration::from_secs(5),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("spawn") || err.contains("Subprocess"),
            "Should fail to spawn: {}",
            err
        );
    }
}

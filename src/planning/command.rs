//! Planning command handler.
//!
//! Executes Claude in headless mode to transform user ideas into structured progress files.

use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::error::RslphError;
use crate::planning::{
    assess_vagueness, detect_stack, REQUIREMENTS_CLARIFIER_PERSONA, TESTING_STRATEGIST_PERSONA,
};
use crate::progress::ProgressFile;
use crate::prompts::get_plan_prompt;
use crate::subprocess::{ClaudeRunner, OutputLine, StreamResponse};

/// Run the planning command.
///
/// Executes Claude CLI in headless mode with the planning system prompt,
/// parses the output into a ProgressFile, and writes it to disk.
///
/// # Arguments
///
/// * `input` - User's idea/plan description
/// * `adaptive` - Whether to use adaptive mode with clarifying questions
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
    adaptive: bool,
    config: &Config,
    working_dir: &Path,
    cancel_token: CancellationToken,
    timeout: Duration,
) -> color_eyre::Result<PathBuf> {
    // If adaptive mode, run the adaptive planning flow
    if adaptive {
        return run_adaptive_planning(input, config, working_dir, cancel_token, timeout).await;
    }

    // Basic mode: direct planning without clarification
    run_basic_planning(input, config, working_dir, cancel_token, timeout).await
}

/// Run basic (non-adaptive) planning mode.
async fn run_basic_planning(
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
    // TODO: Remove --internet flag once we fix the underlying issue with Claude CLI hanging without it
    let args = vec![
        "--internet".to_string(),      // WORKAROUND: Required to prevent Claude CLI from hanging
        "-p".to_string(),              // Print mode (headless)
        "--verbose".to_string(),       // Required for stream-json with -p
        "--output-format".to_string(), // Output format
        "stream-json".to_string(),     // JSONL for structured parsing
        "--system-prompt".to_string(), // Custom system prompt
        system_prompt,
        full_input, // User input as positional arg
    ];

    eprintln!("[TRACE] Spawning: {} {:?}", config.claude_path, args.iter().take(4).collect::<Vec<_>>());

    // Step 5: Spawn Claude
    let mut runner = ClaudeRunner::spawn(&config.claude_path, &args, working_dir)
        .await
        .map_err(|e| RslphError::Subprocess(format!("Failed to spawn claude: {}", e)))?;

    eprintln!("[TRACE] Spawned subprocess with PID: {:?}", runner.id());

    // Step 6: Run with timeout and collect output, tracing each line
    let output = run_with_tracing(&mut runner, timeout, cancel_token).await?;

    // Step 7: Parse JSONL output using StreamResponse
    let mut stream_response = StreamResponse::new();
    for line in &output {
        if let OutputLine::Stdout(s) = line {
            stream_response.process_line(s);
        }
    }
    let response_text = stream_response.text;

    eprintln!("[TRACE] Claude output length: {} chars", response_text.len());
    if let Some(model) = &stream_response.model {
        eprintln!("[TRACE] Model: {}", model);
    }
    eprintln!(
        "[TRACE] Tokens: {} in / {} out",
        stream_response.input_tokens, stream_response.output_tokens
    );

    // Step 8: Parse response into ProgressFile
    let progress_file = ProgressFile::parse(&response_text)?;

    // Step 9: Write to file
    let output_path = working_dir.join("progress.md");
    progress_file.write(&output_path)?;

    eprintln!("[TRACE] Wrote progress file to: {}", output_path.display());

    Ok(output_path)
}

/// Run subprocess with tracing to stderr for each line of output.
async fn run_with_tracing(
    runner: &mut ClaudeRunner,
    max_duration: Duration,
    cancel_token: CancellationToken,
) -> Result<Vec<OutputLine>, RslphError> {
    use tokio::time::timeout;

    let collect_with_trace = async {
        let mut output = Vec::new();
        loop {
            tokio::select! {
                biased;

                _ = cancel_token.cancelled() => {
                    eprintln!("[TRACE] Cancellation requested");
                    runner.terminate_gracefully(Duration::from_secs(5)).await
                        .map_err(|e| RslphError::Subprocess(e.to_string()))?;
                    return Err(RslphError::Cancelled);
                }

                line = runner.next_output() => {
                    match line {
                        Some(OutputLine::Stdout(s)) => {
                            eprintln!("[STDOUT] {}", s);
                            output.push(OutputLine::Stdout(s));
                        }
                        Some(OutputLine::Stderr(s)) => {
                            eprintln!("[STDERR] {}", s);
                            output.push(OutputLine::Stderr(s));
                        }
                        None => {
                            eprintln!("[TRACE] Subprocess streams closed");
                            break;
                        }
                    }
                }
            }
        }
        Ok(output)
    };

    match timeout(max_duration, collect_with_trace).await {
        Ok(result) => result,
        Err(_elapsed) => {
            eprintln!("[TRACE] Timeout after {:?}", max_duration);
            runner.terminate_gracefully(Duration::from_secs(5))
                .await
                .map_err(|e| RslphError::Subprocess(e.to_string()))?;
            Err(RslphError::Timeout(max_duration.as_secs()))
        }
    }
}

/// Run adaptive planning mode with clarifying questions.
///
/// Adaptive mode:
/// 1. Detects project stack
/// 2. Assesses vagueness of input
/// 3. If vague, asks clarifying questions via requirements clarifier persona
/// 4. Generates testing strategy via testing strategist persona
/// 5. Runs final planning with all gathered context
pub async fn run_adaptive_planning(
    input: &str,
    config: &Config,
    working_dir: &Path,
    cancel_token: CancellationToken,
    timeout: Duration,
) -> color_eyre::Result<PathBuf> {
    // Step 1: Detect project stack
    let stack = detect_stack(working_dir);
    println!("Detected stack:\n{}", stack.to_summary());

    // Step 2: Assess vagueness
    let vagueness = assess_vagueness(input);
    println!(
        "\nVagueness score: {:.2} (threshold: 0.5)",
        vagueness.score
    );
    if !vagueness.reasons.is_empty() {
        println!("Reasons: {}", vagueness.reasons.join(", "));
    }

    // Step 3: Initialize clarifications
    let mut clarifications = String::new();

    // Step 4: If vague, run requirements clarifier
    if vagueness.is_vague() {
        println!("\nInput appears vague, gathering requirements...\n");

        let clarifier_input = format!(
            "## Project Stack\n{}\n\n## User Idea\n{}",
            stack.to_summary(),
            input
        );

        let questions = run_claude_headless(
            &config.claude_path,
            REQUIREMENTS_CLARIFIER_PERSONA,
            &clarifier_input,
            working_dir,
            cancel_token.clone(),
            timeout,
        )
        .await?;

        if !questions.contains("REQUIREMENTS_CLEAR") {
            // Print questions and get user input
            println!("Clarifying Questions:\n");
            println!("{}", questions);
            println!("\nPlease answer the questions above (type your answers, then Enter twice to submit):\n");

            // Read multi-line input from stdin
            clarifications = read_multiline_input()?;
            println!("\nGathered clarifications. Continuing...\n");
        } else {
            println!("Requirements are clear enough, skipping clarification.\n");
        }
    } else {
        println!("\nInput is specific enough, skipping clarification.\n");
    }

    // Step 5: Run testing strategist
    println!("Generating testing strategy...\n");

    let testing_input = format!(
        "## Project Stack\n{}\n\n## Requirements\n{}\n\n## Clarifications\n{}",
        stack.to_summary(),
        input,
        if clarifications.is_empty() {
            "None"
        } else {
            &clarifications
        }
    );

    let testing_strategy = run_claude_headless(
        &config.claude_path,
        TESTING_STRATEGIST_PERSONA,
        &testing_input,
        working_dir,
        cancel_token.clone(),
        timeout,
    )
    .await?;

    println!("Testing strategy generated.\n");

    // Step 6: Run final planning with all context
    println!("Generating final plan...\n");

    let plan_prompt = get_plan_prompt(config)?;
    let final_input = format!(
        "## Detected Stack\n{}\n\n## Requirements\n{}\n\n## Clarifications\n{}\n\n## Testing Strategy\n{}",
        stack.to_summary(),
        input,
        if clarifications.is_empty() { "None" } else { &clarifications },
        testing_strategy
    );

    // Build Claude CLI args for headless mode
    // TODO: Remove --internet flag once we fix the underlying issue with Claude CLI hanging without it
    let args = vec![
        "--internet".to_string(),      // WORKAROUND: Required to prevent Claude CLI from hanging
        "-p".to_string(),
        "--verbose".to_string(),       // Required for stream-json with -p
        "--output-format".to_string(),
        "stream-json".to_string(),     // JSONL for structured parsing
        "--system-prompt".to_string(),
        plan_prompt,
        final_input,
    ];

    // Spawn Claude
    let mut runner = ClaudeRunner::spawn(&config.claude_path, &args, working_dir)
        .await
        .map_err(|e| RslphError::Subprocess(format!("Failed to spawn claude: {}", e)))?;

    // Run with timeout and collect output
    let output = runner.run_with_timeout(timeout, cancel_token).await?;

    // Parse JSONL output using StreamResponse
    let mut stream_response = StreamResponse::new();
    for line in &output {
        if let OutputLine::Stdout(s) = line {
            stream_response.process_line(s);
        }
    }
    let response_text = stream_response.text;

    // Parse response into ProgressFile
    let progress_file = ProgressFile::parse(&response_text)?;

    // Write to file
    let output_path = working_dir.join("progress.md");
    progress_file.write(&output_path)?;

    Ok(output_path)
}

/// Run Claude CLI in headless mode with a system prompt and return the response.
async fn run_claude_headless(
    claude_path: &str,
    system_prompt: &str,
    user_input: &str,
    working_dir: &Path,
    cancel_token: CancellationToken,
    timeout: Duration,
) -> color_eyre::Result<String> {
    // TODO: Remove --internet flag once we fix the underlying issue with Claude CLI hanging without it
    let args = vec![
        "--internet".to_string(),      // WORKAROUND: Required to prevent Claude CLI from hanging
        "-p".to_string(),
        "--verbose".to_string(),       // Required for stream-json with -p
        "--output-format".to_string(),
        "stream-json".to_string(),     // JSONL for structured parsing
        "--system-prompt".to_string(),
        system_prompt.to_string(),
        user_input.to_string(),
    ];

    let mut runner = ClaudeRunner::spawn(claude_path, &args, working_dir)
        .await
        .map_err(|e| RslphError::Subprocess(format!("Failed to spawn claude: {}", e)))?;

    let output = runner.run_with_timeout(timeout, cancel_token).await?;

    // Parse JSONL output using StreamResponse
    let mut stream_response = StreamResponse::new();
    for line in &output {
        if let OutputLine::Stdout(s) = line {
            stream_response.process_line(s);
        }
    }

    Ok(stream_response.text)
}

/// Read multi-line input from stdin.
///
/// Reading continues until two consecutive empty lines are entered.
fn read_multiline_input() -> color_eyre::Result<String> {
    let stdin = io::stdin();
    let mut lines = Vec::new();
    let mut empty_count = 0;

    for line in stdin.lock().lines() {
        let line = line?;
        if line.is_empty() {
            empty_count += 1;
            if empty_count >= 2 {
                break;
            }
            lines.push(line);
        } else {
            empty_count = 0;
            lines.push(line);
        }
    }

    // Remove trailing empty lines
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    Ok(lines.join("\n"))
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
            false, // basic mode
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
            false, // basic mode
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
            false, // basic mode
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
            false, // basic mode
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

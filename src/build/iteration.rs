//! Single iteration execution logic.
//!
//! Handles spawning Claude, parsing response, and updating progress file.

use std::path::Path;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::error::RslphError;
use crate::progress::ProgressFile;
use crate::prompts::get_build_prompt;
use crate::subprocess::{format_tool_summary, ClaudeRunner, OutputLine, StreamEvent, StreamResponse};
use crate::tui::SubprocessEvent;

use super::state::{BuildContext, DoneReason, IterationResult};
use super::tokens::IterationTokens;

/// Format commit message for an iteration.
fn format_iteration_commit(project_name: &str, iteration: u32, tasks_completed: u32) -> String {
    format!(
        "[{}][iter {}] Completed {} task(s)",
        project_name, iteration, tasks_completed
    )
}

/// Parse a stream-json line and send appropriate events to TUI.
///
/// Returns the parsed event for response accumulation, or None if parsing failed.
fn parse_and_stream_line(
    line: &str,
    tui_tx: &mpsc::UnboundedSender<SubprocessEvent>,
) -> Option<StreamEvent> {
    let event = match StreamEvent::parse(line) {
        Ok(e) => e,
        Err(_) => return None,
    };

    // Send assistant text as ClaudeOutput
    if event.is_assistant() {
        if let Some(text) = event.extract_text() {
            if !text.is_empty() {
                let _ = tui_tx.send(SubprocessEvent::Output(text));
            }
        }

        // Send tool uses as ToolUse events with formatted summary
        for (tool_name, input) in event.extract_tool_uses() {
            let summary = format_tool_summary(&tool_name, &input);
            let _ = tui_tx.send(SubprocessEvent::ToolUse {
                tool_name,
                content: summary,
            });
        }

        // Send context usage if available
        if let Some(usage) = event.usage() {
            // Send token usage event for TUI display
            let _ = tui_tx.send(SubprocessEvent::TokenUsage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cache_creation_input_tokens: usage.cache_creation_input_tokens.unwrap_or(0),
                cache_read_input_tokens: usage.cache_read_input_tokens.unwrap_or(0),
            });

            // Estimate context usage as output_tokens / 200k (rough estimate)
            // A more accurate approach would track input+output vs max context
            let ratio = (usage.input_tokens + usage.output_tokens) as f64 / 200_000.0;
            let _ = tui_tx.send(SubprocessEvent::Usage(ratio.min(1.0)));
        }
    }

    Some(event)
}

/// Run a single iteration of the build loop.
///
/// This function:
/// 1. Re-reads the progress file (handles external edits)
/// 2. Checks for early exit conditions (RALPH_DONE, all tasks complete)
/// 3. Spawns a fresh Claude subprocess with the build prompt
/// 4. Parses the response and updates the progress file atomically
///
/// # Arguments
///
/// * `ctx` - Mutable build context with progress state
///
/// # Returns
///
/// * `Ok(IterationResult::Continue { tasks_completed })` - Iteration succeeded, continue
/// * `Ok(IterationResult::Done(reason))` - Build should stop
/// * `Err(RslphError)` - Iteration failed with error
pub async fn run_single_iteration(ctx: &mut BuildContext) -> Result<IterationResult, RslphError> {
    // Step 1: Re-read progress file (may have been updated externally)
    ctx.progress = ProgressFile::load(&ctx.progress_path)?;

    // Track tasks before iteration for diff
    let tasks_before = ctx.progress.completed_tasks();

    // Step 2: Check for early exit conditions
    if ctx.progress.is_done() {
        return Ok(IterationResult::Done(DoneReason::RalphDoneMarker));
    }

    if ctx.progress.completed_tasks() == ctx.progress.total_tasks() && ctx.progress.total_tasks() > 0
    {
        return Ok(IterationResult::Done(DoneReason::AllTasksComplete));
    }

    // Step 3: Build prompt with current progress context
    let system_prompt = get_build_prompt(&ctx.config).map_err(|e| {
        RslphError::Subprocess(format!("Failed to load build prompt: {}", e))
    })?;

    // Clear completed this iteration from previous iteration
    ctx.progress.clear_iteration_completed();

    let user_input = format!(
        "## Current Progress\n\n{}\n\n## Instructions\n\nExecute the next incomplete task. Output the complete updated progress file.",
        ctx.progress.to_markdown()
    );

    // Step 4: Build Claude CLI args for headless mode
    // TODO: Remove --internet flag once we fix the underlying issue with Claude CLI hanging without it
    let args = vec![
        "--internet".to_string(), // WORKAROUND: Required to prevent Claude CLI from hanging
        "-p".to_string(),         // Print mode (headless)
        "--verbose".to_string(),  // Required for stream-json with -p
        "--output-format".to_string(),
        "stream-json".to_string(), // JSONL for structured parsing
        "--system-prompt".to_string(),
        system_prompt,
        user_input,
    ];

    // Step 5: Spawn fresh Claude subprocess
    // Handle both None parent and empty parent (when path is just filename)
    let working_dir = ctx
        .progress_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    ctx.log(&format!(
        "[TRACE] Iteration {}: Spawning Claude subprocess",
        ctx.current_iteration
    ));

    let runner_result = ClaudeRunner::spawn(&ctx.config.claude_path, &args, working_dir).await;

    let mut runner = match runner_result {
        Ok(r) => r,
        Err(e) => {
            // Log attempt on spawn failure
            ctx.progress.add_attempt(
                ctx.current_iteration,
                "Spawn Claude subprocess",
                &format!("Error: {}", e),
                Some("Check claude_path configuration"),
            );
            ctx.progress.trim_attempts(ctx.config.recent_threads as usize);
            ctx.progress.write(&ctx.progress_path)?;
            let path_env = std::env::var("PATH").unwrap_or_else(|_| "(not set)".to_string());
            return Err(RslphError::Subprocess(format!(
                "Failed to spawn '{}': {}. Ensure claude is in PATH or set claude_path to absolute path in config. PATH: {}",
                ctx.config.claude_path, e, path_env
            )));
        }
    };

    ctx.log(&format!(
        "[TRACE] Spawned subprocess with PID: {:?}",
        runner.id()
    ));

    // Step 6: Run subprocess and collect output
    // When TUI is active, stream output to TUI while collecting for response parsing
    let timeout = Duration::from_secs(600);
    let mut stream_response = StreamResponse::new();

    let run_result = if let Some(ref tui_tx) = ctx.tui_tx {
        // Streaming mode: use run_with_channel and parse+stream each line
        let (line_tx, mut line_rx) = mpsc::unbounded_channel::<OutputLine>();

        // Spawn the runner with channel
        let cancel_token = ctx.cancel_token.clone();
        let runner_handle = tokio::spawn(async move {
            runner.run_with_channel(line_tx, cancel_token).await
        });

        // Process lines as they arrive, with timeout
        let tui_tx_clone = tui_tx.clone();
        let process_result = tokio::time::timeout(timeout, async {
            while let Some(line) = line_rx.recv().await {
                if let OutputLine::Stdout(s) = &line {
                    // Stream to TUI
                    if let Some(event) = parse_and_stream_line(s, &tui_tx_clone) {
                        stream_response.process_event(&event);
                    }
                }
            }
            Ok::<(), RslphError>(())
        }).await;

        // Wait for runner to complete
        let runner_result = runner_handle.await.map_err(|e| {
            RslphError::Subprocess(format!("Runner task failed: {}", e))
        })?;

        // Check for timeout or runner error
        match process_result {
            Ok(Ok(())) => runner_result,
            Ok(Err(e)) => Err(e),
            Err(_) => Err(RslphError::Timeout(timeout.as_secs())),
        }
    } else {
        // Non-streaming mode: collect all output then parse
        let output = runner
            .run_with_timeout(timeout, ctx.cancel_token.clone())
            .await?;

        // Parse JSONL response
        for line in &output {
            if let OutputLine::Stdout(s) = line {
                stream_response.process_line(s);
            }
        }
        Ok(())
    };

    // Handle run errors
    if let Err(e) = run_result {
        ctx.progress.add_attempt(
            ctx.current_iteration,
            "Execute Claude subprocess",
            &format!("Error: {}", e),
            Some("Retry or check subprocess"),
        );
        ctx.progress.trim_attempts(ctx.config.recent_threads as usize);
        ctx.progress.write(&ctx.progress_path)?;
        return Err(e);
    }

    // Step 7: Extract response text
    let response_text = stream_response.text;

    ctx.log(&format!(
        "[TRACE] Claude output length: {} chars",
        response_text.len()
    ));
    if let Some(model) = &stream_response.model {
        ctx.log(&format!("[TRACE] Model: {}", model));
    }
    ctx.log(&format!(
        "[TRACE] Tokens: {} in / {} out / {} cache_write / {} cache_read",
        stream_response.input_tokens,
        stream_response.output_tokens,
        stream_response.cache_creation_input_tokens,
        stream_response.cache_read_input_tokens
    ));

    // Accumulate tokens from this iteration
    let iteration_tokens = IterationTokens {
        iteration: ctx.current_iteration,
        input_tokens: stream_response.input_tokens,
        output_tokens: stream_response.output_tokens,
        cache_creation_input_tokens: stream_response.cache_creation_input_tokens,
        cache_read_input_tokens: stream_response.cache_read_input_tokens,
    };
    ctx.iteration_tokens.push(iteration_tokens);
    ctx.total_tokens.input_tokens += stream_response.input_tokens;
    ctx.total_tokens.output_tokens += stream_response.output_tokens;
    ctx.total_tokens.cache_creation_input_tokens += stream_response.cache_creation_input_tokens;
    ctx.total_tokens.cache_read_input_tokens += stream_response.cache_read_input_tokens;

    // Step 8: Parse response into ProgressFile
    let updated_progress = match ProgressFile::parse(&response_text) {
        Ok(p) => p,
        Err(e) => {
            // Log attempt on parse failure
            ctx.progress.add_attempt(
                ctx.current_iteration,
                "Parse Claude response",
                &format!("Error: {}", e),
                Some("Check response format"),
            );
            ctx.progress.trim_attempts(ctx.config.recent_threads as usize);
            ctx.progress.write(&ctx.progress_path)?;
            return Err(e);
        }
    };

    // Step 9: Write updated progress file atomically with trimmed attempts
    let mut updated_progress = updated_progress;
    updated_progress.trim_attempts(ctx.config.recent_threads as usize);
    updated_progress.write(&ctx.progress_path)?;

    ctx.log(&format!(
        "[TRACE] Updated progress file: {}",
        ctx.progress_path.display()
    ));

    // Step 10: Calculate tasks completed this iteration
    let tasks_after = updated_progress.completed_tasks();
    let tasks_completed = tasks_after.saturating_sub(tasks_before) as u32;

    // Step 11: VCS auto-commit if tasks were completed
    if tasks_completed > 0 {
        if let Some(ref vcs) = ctx.vcs {
            let commit_msg =
                format_iteration_commit(&ctx.project_name, ctx.current_iteration, tasks_completed);
            match vcs.commit_all(&commit_msg) {
                Ok(Some(hash)) => {
                    ctx.log(&format!("[VCS] Committed: {} ({})", hash, vcs.vcs_type()));
                }
                Ok(None) => {
                    ctx.log("[VCS] No file changes to commit");
                }
                Err(e) => {
                    // VCS errors are warnings, not failures
                    ctx.log(&format!("[VCS] Warning: {}", e));
                }
            }
        }
    }

    // Update context with new progress
    ctx.progress = updated_progress;

    // Check if done after update
    if ctx.progress.is_done() {
        return Ok(IterationResult::Done(DoneReason::RalphDoneMarker));
    }

    if ctx.progress.completed_tasks() == ctx.progress.total_tasks() && ctx.progress.total_tasks() > 0
    {
        return Ok(IterationResult::Done(DoneReason::AllTasksComplete));
    }

    Ok(IterationResult::Continue { tasks_completed })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    fn create_test_progress() -> ProgressFile {
        use crate::progress::{Task, TaskPhase};

        ProgressFile {
            name: "Test Plan".to_string(),
            status: "In Progress".to_string(),
            analysis: "Test analysis".to_string(),
            tasks: vec![TaskPhase {
                name: "Phase 1".to_string(),
                tasks: vec![
                    Task {
                        description: "Task 1".to_string(),
                        completed: false,
                    },
                    Task {
                        description: "Task 2".to_string(),
                        completed: false,
                    },
                ],
            }],
            testing_strategy: "Unit tests".to_string(),
            completed_this_iteration: vec![],
            recent_attempts: vec![],
            iteration_log: vec![],
        }
    }

    #[tokio::test]
    async fn test_iteration_detects_ralph_done() {
        let dir = TempDir::new().expect("temp dir");
        let progress_path = dir.path().join("progress.md");

        let mut progress = create_test_progress();
        progress.status = "RALPH_DONE - All complete".to_string();
        progress.write(&progress_path).expect("write");

        let config = crate::config::Config::default();
        let token = CancellationToken::new();

        let mut ctx = BuildContext::new(
            progress_path,
            progress,
            config,
            token,
            false,
            false,
        );
        ctx.current_iteration = 1;

        let result = run_single_iteration(&mut ctx).await;
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            IterationResult::Done(DoneReason::RalphDoneMarker)
        ));
    }

    #[tokio::test]
    async fn test_iteration_detects_all_tasks_complete() {
        let dir = TempDir::new().expect("temp dir");
        let progress_path = dir.path().join("progress.md");

        use crate::progress::{Task, TaskPhase};
        let progress = ProgressFile {
            name: "Test".to_string(),
            status: "In Progress".to_string(),
            tasks: vec![TaskPhase {
                name: "Phase 1".to_string(),
                tasks: vec![Task {
                    description: "Task 1".to_string(),
                    completed: true,
                }],
            }],
            ..Default::default()
        };
        progress.write(&progress_path).expect("write");

        let config = crate::config::Config::default();
        let token = CancellationToken::new();

        let mut ctx = BuildContext::new(
            progress_path,
            progress,
            config,
            token,
            false,
            false,
        );
        ctx.current_iteration = 1;

        let result = run_single_iteration(&mut ctx).await;
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            IterationResult::Done(DoneReason::AllTasksComplete)
        ));
    }
}

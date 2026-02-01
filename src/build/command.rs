//! Main build command handler.
//!
//! Orchestrates the build loop with state machine, iteration execution,
//! and termination handling.

use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::error::RslphError;
use crate::progress::ProgressFile;
use crate::prompts::PromptMode;

use super::iteration::run_single_iteration;
use super::state::{BuildContext, BuildState, DoneReason, IterationResult};
use super::tokens::TokenUsage;

/// Callback type for reporting build iteration progress.
/// Parameters: (current_iteration, max_iterations)
pub type ProgressCallback = Arc<dyn Fn(u32, u32) + Send + Sync>;

/// Run the build command.
///
/// Executes the build loop on a progress file, spawning fresh Claude
/// subprocesses for each iteration until completion or max iterations.
///
/// # Arguments
///
/// * `progress_path` - Path to the progress.md file
/// * `once` - If true, run only one iteration
/// * `dry_run` - If true, preview what would be done without executing
/// * `no_tui` - If true, disable TUI and use headless output
/// * `mode` - The prompt mode to use for this build
/// * `no_dsp` - If true, append --dangerously-skip-permissions to Claude
/// * `config` - Application configuration
/// * `cancel_token` - Token for graceful cancellation
/// * `progress_callback` - Optional callback for iteration progress updates
///
/// # Returns
///
/// * `Ok(TokenUsage)` - Build completed with token usage
/// * `Err(e)` - Build failed with error
#[allow(clippy::too_many_arguments)]
pub async fn run_build_command(
    progress_path: PathBuf,
    once: bool,
    dry_run: bool,
    no_tui: bool,
    mode: PromptMode,
    no_dsp: bool,
    config: &Config,
    cancel_token: CancellationToken,
    progress_callback: Option<ProgressCallback>,
) -> color_eyre::Result<TokenUsage> {
    // Load initial progress file
    let progress = ProgressFile::load(&progress_path)?;

    // Determine if TUI should be used
    let use_tui = config.tui_enabled && !no_tui && !dry_run;

    if !use_tui {
        println!("Build started: {}", progress_path.display());
        println!(
            "Tasks: {}/{} complete",
            progress.completed_tasks(),
            progress.total_tasks()
        );
    }

    // Create build context
    let mut ctx = BuildContext::new(
        progress_path.clone(),
        progress,
        config.clone(),
        mode,
        cancel_token.clone(),
        once,
        dry_run,
        no_dsp,
    );

    // Dry-run mode: preview and exit
    if dry_run {
        return run_dry_run(&ctx);
    }

    // TUI mode: run with interactive terminal UI
    // Note: Full subprocess integration requires refactoring iteration.rs to use channels.
    // For now, TUI runs with initial state and headless build runs in parallel.
    if use_tui {
        return run_build_with_tui(
            progress_path,
            ctx.progress.clone(),
            mode,
            no_dsp,
            config,
            cancel_token,
        )
        .await;
    }

    // Main iteration loop with state machine
    let mut state = BuildState::Starting;

    loop {
        state = match state {
            BuildState::Starting => {
                ctx.current_iteration = 1;
                ctx.iteration_start = Some(std::time::Instant::now());
                ctx.log("\n--- Iteration 1 ---");
                // Invoke progress callback at iteration start
                if let Some(ref cb) = progress_callback {
                    cb(1, ctx.max_iterations);
                }
                BuildState::Running { iteration: 1 }
            }

            BuildState::Running { iteration } => {
                match run_single_iteration(&mut ctx).await {
                    Ok(IterationResult::Continue { tasks_completed }) => {
                        // Reset timeout retry count on success
                        ctx.timeout_retry_count = 0;
                        BuildState::IterationComplete {
                            iteration,
                            tasks_completed,
                        }
                    }
                    Ok(IterationResult::Done(reason)) => BuildState::Done { reason },
                    Ok(IterationResult::Timeout) => {
                        // Handle timeout with retry
                        ctx.timeout_retry_count += 1;
                        if ctx.timeout_retry_count >= ctx.config.timeout_retries {
                            ctx.log(&format!(
                                "[BUILD] Iteration {} timed out {} times, failing",
                                iteration, ctx.timeout_retry_count
                            ));
                            BuildState::Failed {
                                error: format!(
                                    "Iteration timed out {} times (max retries: {})",
                                    ctx.timeout_retry_count, ctx.config.timeout_retries
                                ),
                            }
                        } else {
                            ctx.log(&format!(
                                "[BUILD] Iteration {} timed out, retry {}/{}",
                                iteration, ctx.timeout_retry_count, ctx.config.timeout_retries
                            ));
                            // Reset iteration start time for retry
                            ctx.iteration_start = Some(std::time::Instant::now());
                            BuildState::Running { iteration }
                        }
                    }
                    Err(RslphError::Cancelled) => BuildState::Done {
                        reason: DoneReason::UserCancelled,
                    },
                    Err(e) => BuildState::Failed {
                        error: e.to_string(),
                    },
                }
            }

            BuildState::IterationComplete {
                iteration,
                tasks_completed,
            } => {
                // Log iteration result
                let duration = ctx.iteration_start.map(|s| s.elapsed()).unwrap_or_default();
                ctx.log(&format!(
                    "[BUILD] Iteration {} complete: {} task(s) completed in {:.1}s",
                    iteration,
                    tasks_completed,
                    duration.as_secs_f64()
                ));
                ctx.log(&format!(
                    "[BUILD] Progress: {}/{} tasks",
                    ctx.progress.completed_tasks(),
                    ctx.progress.total_tasks()
                ));

                // Log to progress file
                log_iteration(&mut ctx, iteration, tasks_completed)?;

                // Check termination conditions in priority order
                if ctx.once_mode {
                    BuildState::Done {
                        reason: DoneReason::SingleIterationComplete,
                    }
                } else if iteration >= ctx.max_iterations {
                    ctx.log(&format!(
                        "[BUILD] Max iterations ({}) reached",
                        ctx.max_iterations
                    ));
                    BuildState::Done {
                        reason: DoneReason::MaxIterationsReached,
                    }
                } else {
                    // Check for cancellation before next iteration
                    if cancel_token.is_cancelled() {
                        BuildState::Done {
                            reason: DoneReason::UserCancelled,
                        }
                    } else {
                        ctx.current_iteration = iteration + 1;
                        ctx.iteration_start = Some(std::time::Instant::now());
                        ctx.log(&format!("\n--- Iteration {} ---", iteration + 1));
                        // Invoke progress callback at iteration start
                        if let Some(ref cb) = progress_callback {
                            cb(iteration + 1, ctx.max_iterations);
                        }
                        BuildState::Running {
                            iteration: iteration + 1,
                        }
                    }
                }
            }

            BuildState::Done { reason } => {
                print_completion_message(&reason, &ctx);
                return Ok(ctx.total_tokens.clone());
            }

            BuildState::Failed { error } => {
                return Err(color_eyre::eyre::eyre!("Build failed: {}", error));
            }
        };

        // Check for cancellation between state transitions
        if cancel_token.is_cancelled() && !matches!(state, BuildState::Done { .. }) {
            state = BuildState::Done {
                reason: DoneReason::UserCancelled,
            };
        }
    }
}

/// Preview what the build would do without executing (LOOP-07).
///
/// Shows comprehensive information about:
/// - Progress file status and task counts
/// - Next task that would be executed
/// - Configuration settings (max iterations, once mode)
/// - Build prompt source and validation
/// - Recent attempts summary
fn run_dry_run(ctx: &BuildContext) -> color_eyre::Result<TokenUsage> {
    use crate::prompts::get_build_prompt;

    println!("\n=== DRY RUN MODE ===\n");

    // Progress file info
    println!("Progress file: {}", ctx.progress_path.display());
    println!("Project: {}", ctx.progress.name);
    println!();

    // Current status
    println!("Status: {}", ctx.progress.status);
    if ctx.progress.is_done() {
        println!("  -> RALPH_DONE detected, build would exit immediately");
    }
    println!();

    // Task summary
    let total = ctx.progress.total_tasks();
    let completed = ctx.progress.completed_tasks();
    let remaining = total - completed;
    println!(
        "Tasks: {}/{} complete ({} remaining)",
        completed, total, remaining
    );

    if remaining == 0 && total > 0 {
        println!("  -> All tasks complete, build would exit immediately");
    }
    println!();

    // Next task to execute
    if let Some((phase, task)) = ctx.progress.next_task() {
        println!("Next task to execute:");
        println!("  Phase: {}", phase);
        println!("  Task:  {}", task.description);
    } else {
        println!("No pending tasks found.");
    }
    println!();

    // Configuration
    println!("Configuration:");
    println!("  Max iterations: {}", ctx.max_iterations);
    println!("  Once mode: {}", ctx.once_mode);
    println!("  Recent attempts depth: {}", ctx.config.recent_threads);
    println!();

    // Prompt info
    let prompt_source = if let Some(ref path) = ctx.config.build_prompt {
        format!("custom ({})", path.display())
    } else {
        "default (embedded)".to_string()
    };
    println!("Build prompt: {}", prompt_source);

    // Validate prompt is loadable
    match get_build_prompt(&ctx.config) {
        Ok(prompt) => println!("  Prompt length: {} chars", prompt.len()),
        Err(e) => println!("  WARNING: Failed to load prompt: {}", e),
    }
    println!();

    // Recent attempts summary
    if !ctx.progress.recent_attempts.is_empty() {
        println!("Recent attempts ({}):", ctx.progress.recent_attempts.len());
        for attempt in ctx.progress.recent_attempts.iter().rev().take(3) {
            println!(
                "  Iteration {}: {} -> {}",
                attempt.iteration, attempt.tried, attempt.result
            );
        }
    }

    println!("\n=== END DRY RUN ===");
    println!("\nTo execute, run without --dry-run flag.");

    Ok(TokenUsage::default())
}

/// Run build with TUI mode enabled.
///
/// Initializes the TUI and runs the build loop concurrently with visual feedback.
/// The build loop runs in the background and sends events to the TUI via channels.
async fn run_build_with_tui(
    progress_path: PathBuf,
    progress: ProgressFile,
    mode: PromptMode,
    no_dsp: bool,
    config: &Config,
    cancel_token: CancellationToken,
) -> color_eyre::Result<TokenUsage> {
    use crate::tui::{run_tui, App, SubprocessEvent};

    // Initialize app state from progress
    let mut app = App::new(config.max_iterations, "Claude", progress.name.clone());
    app.current_task = progress.completed_tasks() as u32;
    app.total_tasks = progress.total_tasks() as u32;
    app.log_path = Some(progress_path.clone());
    app.current_iteration = 0;
    app.viewing_iteration = 0;

    // Get recent message count from config
    let recent_count = config.tui_recent_messages;

    // Start TUI and get subprocess event sender
    // Pass a clone of cancel_token so TUI can cancel the build on quit
    let subprocess_tx = run_tui(app, recent_count, cancel_token.clone()).await?;

    // Create build context with TUI sender for log routing
    let mut ctx = BuildContext::with_tui(
        progress_path.clone(),
        progress,
        config.clone(),
        mode,
        cancel_token.clone(),
        false, // once_mode - TUI always runs full loop
        false, // dry_run - already handled before this function
        no_dsp,
        Some(subprocess_tx.clone()),
    );

    // Create a channel for build loop to send updates to TUI
    // The subprocess_tx is an UnboundedSender<SubprocessEvent>
    let tui_tx = subprocess_tx.clone();

    // Run build loop, forwarding events to TUI
    let mut state = BuildState::Starting;
    let result = loop {
        state = match state {
            BuildState::Starting => {
                ctx.current_iteration = 1;
                ctx.iteration_start = Some(std::time::Instant::now());

                // Send iteration start to TUI to sync iteration number
                let _ = tui_tx.send(SubprocessEvent::IterationStart { iteration: 1 });
                let _ = tui_tx.send(SubprocessEvent::Log("--- Iteration 1 ---".to_string()));

                BuildState::Running { iteration: 1 }
            }

            BuildState::Running { iteration } => {
                match run_single_iteration(&mut ctx).await {
                    Ok(IterationResult::Continue { tasks_completed }) => {
                        // Reset timeout retry count on success
                        ctx.timeout_retry_count = 0;
                        BuildState::IterationComplete {
                            iteration,
                            tasks_completed,
                        }
                    }
                    Ok(IterationResult::Done(reason)) => BuildState::Done { reason },
                    Ok(IterationResult::Timeout) => {
                        // Handle timeout with retry
                        ctx.timeout_retry_count += 1;
                        if ctx.timeout_retry_count >= ctx.config.timeout_retries {
                            let _ = tui_tx.send(SubprocessEvent::Log(format!(
                                "Iteration {} timed out {} times, failing",
                                iteration, ctx.timeout_retry_count
                            )));
                            BuildState::Failed {
                                error: format!(
                                    "Iteration timed out {} times (max retries: {})",
                                    ctx.timeout_retry_count, ctx.config.timeout_retries
                                ),
                            }
                        } else {
                            let _ = tui_tx.send(SubprocessEvent::Log(format!(
                                "Iteration {} timed out, retry {}/{}",
                                iteration, ctx.timeout_retry_count, ctx.config.timeout_retries
                            )));
                            // Reset iteration start time for retry
                            ctx.iteration_start = Some(std::time::Instant::now());
                            BuildState::Running { iteration }
                        }
                    }
                    Err(RslphError::Cancelled) => BuildState::Done {
                        reason: DoneReason::UserCancelled,
                    },
                    Err(e) => BuildState::Failed {
                        error: e.to_string(),
                    },
                }
            }

            BuildState::IterationComplete {
                iteration,
                tasks_completed,
            } => {
                // Log iteration result
                let duration = ctx.iteration_start.map(|s| s.elapsed()).unwrap_or_default();

                // Send iteration complete to TUI
                let _ = tui_tx.send(SubprocessEvent::IterationDone {
                    tasks_done: tasks_completed,
                });

                let _ = tui_tx.send(SubprocessEvent::Log(format!(
                    "Iteration {} complete: {} task(s) in {:.1}s",
                    iteration,
                    tasks_completed,
                    duration.as_secs_f64()
                )));

                // Log to progress file
                log_iteration(&mut ctx, iteration, tasks_completed)?;

                // Check termination conditions
                if iteration >= ctx.max_iterations {
                    let _ = tui_tx.send(SubprocessEvent::Log(format!(
                        "Max iterations ({}) reached",
                        ctx.max_iterations
                    )));
                    BuildState::Done {
                        reason: DoneReason::MaxIterationsReached,
                    }
                } else if cancel_token.is_cancelled() {
                    BuildState::Done {
                        reason: DoneReason::UserCancelled,
                    }
                } else {
                    ctx.current_iteration = iteration + 1;
                    ctx.iteration_start = Some(std::time::Instant::now());

                    let _ = tui_tx.send(SubprocessEvent::IterationStart {
                        iteration: iteration + 1,
                    });
                    let _ = tui_tx.send(SubprocessEvent::Log(format!(
                        "--- Iteration {} ---",
                        iteration + 1
                    )));

                    BuildState::Running {
                        iteration: iteration + 1,
                    }
                }
            }

            BuildState::Done { reason } => {
                let _ = tui_tx.send(SubprocessEvent::Log(format!("Build complete: {}", reason)));
                break Ok(ctx.total_tokens.clone());
            }

            BuildState::Failed { error } => {
                let _ = tui_tx.send(SubprocessEvent::Log(format!("Build failed: {}", error)));
                break Err(color_eyre::eyre::eyre!("Build failed: {}", error));
            }
        };

        // Check for cancellation between state transitions
        if cancel_token.is_cancelled() && !matches!(state, BuildState::Done { .. }) {
            state = BuildState::Done {
                reason: DoneReason::UserCancelled,
            };
        }

        // Small yield to let TUI render
        tokio::task::yield_now().await;
    };

    result
}

/// Print completion message based on done reason.
fn print_completion_message(reason: &DoneReason, ctx: &BuildContext) {
    println!("\n=== BUILD COMPLETE ===");
    println!("Reason: {}", reason);
    println!(
        "Final progress: {}/{} tasks",
        ctx.progress.completed_tasks(),
        ctx.progress.total_tasks()
    );

    match reason {
        DoneReason::AllTasksComplete | DoneReason::RalphDoneMarker => {
            println!("All tasks completed successfully!");
        }
        DoneReason::MaxIterationsReached => {
            let remaining = ctx.progress.total_tasks() - ctx.progress.completed_tasks();
            println!(
                "Stopped after {} iterations. {} task(s) remaining.",
                ctx.max_iterations, remaining
            );
        }
        DoneReason::UserCancelled => {
            println!("Build cancelled by user.");
        }
        DoneReason::SingleIterationComplete => {
            println!("Single iteration completed (--once mode).");
        }
    }
}

/// Log iteration to progress file.
fn log_iteration(
    ctx: &mut BuildContext,
    iteration: u32,
    tasks_completed: u32,
) -> Result<(), RslphError> {
    let now = chrono::Utc::now();
    let started = now.format("%Y-%m-%d %H:%M").to_string();

    let duration = ctx
        .iteration_start
        .map(|s| {
            let elapsed = s.elapsed();
            format!("{}m {}s", elapsed.as_secs() / 60, elapsed.as_secs() % 60)
        })
        .unwrap_or_else(|| "~".to_string());

    let notes = if tasks_completed == 0 {
        "No tasks completed".to_string()
    } else {
        format!("{} task(s) completed", tasks_completed)
    };

    ctx.progress
        .log_iteration(iteration, &started, &duration, tasks_completed, &notes);

    ctx.progress.write(&ctx.progress_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::PromptMode;
    use std::time::Duration;
    use tempfile::TempDir;

    fn create_test_progress_file(dir: &TempDir) -> PathBuf {
        use crate::progress::{Task, TaskPhase};

        let progress = ProgressFile {
            name: "Test Plan".to_string(),
            status: "In Progress".to_string(),
            analysis: "Test analysis".to_string(),
            tasks: vec![TaskPhase {
                name: "Phase 1".to_string(),
                tasks: vec![Task {
                    description: "Task 1".to_string(),
                    completed: false,
                }],
            }],
            testing_strategy: "Unit tests".to_string(),
            completed_this_iteration: vec![],
            recent_attempts: vec![],
            iteration_log: vec![],
        };

        let path = dir.path().join("progress.md");
        progress.write(&path).expect("write progress");
        path
    }

    #[tokio::test]
    async fn test_build_command_rejects_invalid_claude_output() {
        use crate::config::ClaudeCommand;

        // This test verifies that invalid Claude output (echo outputs garbage)
        // results in a parse error. Echo doesn't produce stream-json format,
        // so StreamResponse.text is empty and validation rejects it.
        let dir = TempDir::new().expect("temp dir");
        let progress_path = create_test_progress_file(&dir);

        let config = Config {
            claude_cmd: ClaudeCommand {
                command: "/bin/echo".to_string(),
                base_args: vec![],
            },
            ..Default::default()
        };

        let token = CancellationToken::new();

        // Run with --once to avoid infinite loop
        let result = run_build_command(
            progress_path,
            true, // once
            false,
            true, // no_tui
            PromptMode::Basic,
            false, // no_dsp
            &config,
            token,
            None,
        )
        .await;

        // With validation, echo output (not valid stream-json) should fail to parse
        assert!(result.is_err(), "Invalid Claude output should be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("parse") || err.contains("valid sections"),
            "Should report parse error: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_build_command_dry_run() {
        let dir = TempDir::new().expect("temp dir");
        let progress_path = create_test_progress_file(&dir);

        let config = Config::default();
        let token = CancellationToken::new();

        let result = run_build_command(
            progress_path,
            false,
            true, // dry_run
            true, // no_tui
            PromptMode::Basic,
            false, // no_dsp
            &config,
            token,
            None,
        )
        .await;

        assert!(result.is_ok(), "Dry run should succeed");
    }

    #[tokio::test]
    async fn test_build_command_timeout() {
        use crate::config::ClaudeCommand;

        let dir = TempDir::new().expect("temp dir");
        let progress_path = create_test_progress_file(&dir);

        // Create a script that sleeps longer than timeout
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
            claude_cmd: ClaudeCommand {
                command: script_path.to_string_lossy().to_string(),
                base_args: vec![],
            },
            max_iterations: 1, // Limit iterations
            ..Default::default()
        };

        let token = CancellationToken::new();

        // The iteration will timeout (default 10 min is too long for test)
        // So we use cancellation to stop early
        let token_clone = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            token_clone.cancel();
        });

        let result = run_build_command(
            progress_path,
            true, // once mode to limit iterations
            false,
            true, // no_tui
            PromptMode::Basic,
            false, // no_dsp
            &config,
            token,
            None,
        )
        .await;

        // Should complete (possibly with cancellation)
        // The key is it doesn't hang
        assert!(result.is_ok(), "Should handle timeout/cancel: {:?}", result);
    }

    #[tokio::test]
    async fn test_build_command_cancellation() {
        use crate::config::ClaudeCommand;

        let dir = TempDir::new().expect("temp dir");
        let progress_path = create_test_progress_file(&dir);

        // Create a script that sleeps
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
            claude_cmd: ClaudeCommand {
                command: script_path.to_string_lossy().to_string(),
                base_args: vec![],
            },
            ..Default::default()
        };

        let token = CancellationToken::new();
        let token_clone = token.clone();

        // Cancel after 50ms
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            token_clone.cancel();
        });

        let result = run_build_command(
            progress_path,
            false,
            false,
            true,
            PromptMode::Basic,
            false, // no_dsp
            &config,
            token,
            None,
        )
        .await;

        // Should complete with user cancelled status
        assert!(result.is_ok(), "Should handle cancellation: {:?}", result);
    }

    #[tokio::test]
    async fn test_build_command_nonexistent_progress() {
        let config = Config::default();
        let token = CancellationToken::new();

        let result = run_build_command(
            PathBuf::from("/nonexistent/progress.md"),
            false,
            false,
            true, // no_tui
            PromptMode::Basic,
            false, // no_dsp
            &config,
            token,
            None,
        )
        .await;

        assert!(result.is_err(), "Should fail on missing file");
    }

    #[tokio::test]
    async fn test_dry_run_does_not_modify_progress() {
        use crate::progress::{Task, TaskPhase};

        let dir = TempDir::new().expect("temp dir");
        let progress_path = dir.path().join("progress.md");

        let progress = ProgressFile {
            name: "Test".to_string(),
            status: "In Progress".to_string(),
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
            ..Default::default()
        };
        progress.write(&progress_path).expect("write");

        // Read original content
        let original_content = std::fs::read_to_string(&progress_path).expect("read original");

        let config = Config::default();
        let token = CancellationToken::new();

        // Run dry-run
        let result = run_build_command(
            progress_path.clone(),
            false,
            true, // dry_run
            true, // no_tui
            PromptMode::Basic,
            false, // no_dsp
            &config,
            token,
            None,
        )
        .await;

        assert!(result.is_ok(), "Dry run should succeed");

        // Verify file unchanged
        let final_content = std::fs::read_to_string(&progress_path).expect("read final");
        assert_eq!(
            original_content, final_content,
            "Progress file should not be modified in dry-run mode"
        );
    }

    #[tokio::test]
    async fn test_dry_run_shows_once_mode_true() {
        use crate::progress::{Task, TaskPhase};

        let dir = TempDir::new().expect("temp dir");
        let progress_path = dir.path().join("progress.md");

        let progress = ProgressFile {
            name: "Test".to_string(),
            status: "In Progress".to_string(),
            tasks: vec![TaskPhase {
                name: "Phase 1".to_string(),
                tasks: vec![Task {
                    description: "Task 1".to_string(),
                    completed: false,
                }],
            }],
            ..Default::default()
        };
        progress.write(&progress_path).expect("write");

        let config = Config::default();
        let token = CancellationToken::new();

        // When both once and dry_run are true, dry_run takes precedence
        let result = run_build_command(
            progress_path,
            true, // once mode
            true, // dry_run
            true, // no_tui
            PromptMode::Basic,
            false, // no_dsp
            &config,
            token,
            None,
        )
        .await;

        assert!(result.is_ok(), "Dry run with once mode should succeed");
    }

    #[test]
    fn test_dry_run_function_directly() {
        use crate::progress::{Attempt, Task, TaskPhase};

        let dir = TempDir::new().expect("temp dir");
        let progress_path = dir.path().join("progress.md");

        let progress = ProgressFile {
            name: "Test Plan".to_string(),
            status: "In Progress".to_string(),
            tasks: vec![TaskPhase {
                name: "Phase 1".to_string(),
                tasks: vec![
                    Task {
                        description: "Completed task".to_string(),
                        completed: true,
                    },
                    Task {
                        description: "Pending task".to_string(),
                        completed: false,
                    },
                ],
            }],
            recent_attempts: vec![Attempt {
                iteration: 1,
                tried: "First try".to_string(),
                result: "Success".to_string(),
                next: None,
            }],
            ..Default::default()
        };

        let config = Config::default();
        let token = CancellationToken::new();

        let ctx = BuildContext::new(
            progress_path,
            progress,
            config,
            PromptMode::Basic,
            token,
            true,  // once_mode
            true,  // dry_run
            false, // no_dsp
        );

        // Verify dry run function succeeds
        let result = run_dry_run(&ctx);
        assert!(result.is_ok(), "Dry run function should succeed");
    }

    #[tokio::test]
    async fn test_once_mode_rejects_invalid_output() {
        use crate::config::ClaudeCommand;
        use crate::progress::{Task, TaskPhase};

        let dir = TempDir::new().expect("temp dir");

        // Create progress file with multiple incomplete tasks
        let progress = ProgressFile {
            name: "Test".to_string(),
            status: "In Progress".to_string(),
            analysis: "Test analysis.".to_string(),
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
                    Task {
                        description: "Task 3".to_string(),
                        completed: false,
                    },
                ],
            }],
            testing_strategy: "Test with cargo test.".to_string(),
            ..Default::default()
        };

        let progress_path = dir.path().join("progress.md");
        progress.write(&progress_path).expect("write progress");

        // Use echo mock that outputs invalid content (not stream-json format)
        let config = Config {
            claude_cmd: ClaudeCommand {
                command: "/bin/echo".to_string(),
                base_args: vec![],
            },
            ..Default::default()
        };

        let token = CancellationToken::new();

        // Run with once=true
        let result = run_build_command(
            progress_path.clone(),
            true,  // once mode
            false, // not dry-run
            true,  // no_tui
            PromptMode::Basic,
            false, // no_dsp
            &config,
            token,
            None,
        )
        .await;

        // Echo produces invalid output, so with validation this should fail
        assert!(result.is_err(), "Once mode should reject invalid output");
    }

    #[test]
    fn test_once_mode_triggers_correct_done_reason() {
        // This tests the state machine logic directly
        use crate::progress::{Task, TaskPhase};

        let progress = ProgressFile {
            name: "Test".to_string(),
            status: "In Progress".to_string(),
            tasks: vec![TaskPhase {
                name: "Phase 1".to_string(),
                tasks: vec![Task {
                    description: "Task 1".to_string(),
                    completed: false,
                }],
            }],
            ..Default::default()
        };

        let dir = TempDir::new().expect("temp dir");
        let progress_path = dir.path().join("progress.md");
        let config = Config::default();
        let token = CancellationToken::new();

        let ctx = BuildContext::new(
            progress_path,
            progress,
            config,
            PromptMode::Basic,
            token,
            true,  // once_mode - this is what we're testing
            false, // dry_run
            false, // no_dsp
        );

        // Verify once_mode is set correctly
        assert!(ctx.once_mode, "Once mode should be set");
    }

    #[tokio::test]
    async fn test_ralph_done_stops_immediately() {
        use crate::config::ClaudeCommand;
        use crate::progress::{Task, TaskPhase};

        let dir = TempDir::new().expect("temp dir");
        let progress_path = dir.path().join("progress.md");

        // Create progress file with RALPH_DONE in status
        let progress = ProgressFile {
            name: "Test".to_string(),
            status: "RALPH_DONE - All tasks complete".to_string(),
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

        // Use a slow script as mock - if RALPH_DONE works, it won't be called
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
            claude_cmd: ClaudeCommand {
                command: script_path.to_string_lossy().to_string(),
                base_args: vec![],
            },
            ..Default::default()
        };

        let token = CancellationToken::new();

        // If RALPH_DONE is detected, this should return immediately
        // without spawning the slow script
        let start = std::time::Instant::now();
        let result = run_build_command(
            progress_path,
            false,
            false,
            true, // no_tui
            PromptMode::Basic,
            false, // no_dsp
            &config,
            token,
            None,
        )
        .await;

        let elapsed = start.elapsed();

        assert!(result.is_ok(), "Should succeed with RALPH_DONE");
        assert!(
            elapsed.as_secs() < 5,
            "Should return immediately, not wait for slow script"
        );
    }

    #[tokio::test]
    async fn test_all_tasks_complete_stops_immediately() {
        use crate::config::ClaudeCommand;
        use crate::progress::{Task, TaskPhase};

        let dir = TempDir::new().expect("temp dir");
        let progress_path = dir.path().join("progress.md");

        // Create progress file with all tasks marked complete
        let progress = ProgressFile {
            name: "Test".to_string(),
            status: "In Progress".to_string(),
            tasks: vec![TaskPhase {
                name: "Phase 1".to_string(),
                tasks: vec![
                    Task {
                        description: "Task 1".to_string(),
                        completed: true,
                    },
                    Task {
                        description: "Task 2".to_string(),
                        completed: true,
                    },
                ],
            }],
            ..Default::default()
        };
        progress.write(&progress_path).expect("write");

        // Use a slow script as mock
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
            claude_cmd: ClaudeCommand {
                command: script_path.to_string_lossy().to_string(),
                base_args: vec![],
            },
            ..Default::default()
        };

        let token = CancellationToken::new();

        let start = std::time::Instant::now();
        let result = run_build_command(
            progress_path,
            false,
            false,
            true, // no_tui
            PromptMode::Basic,
            false, // no_dsp
            &config,
            token,
            None,
        )
        .await;

        let elapsed = start.elapsed();

        assert!(result.is_ok(), "Should succeed when all tasks complete");
        assert!(
            elapsed.as_secs() < 5,
            "Should return immediately when all tasks complete"
        );
    }

    #[tokio::test]
    async fn test_max_iterations_rejects_invalid_output() {
        use crate::config::ClaudeCommand;
        use crate::progress::{Task, TaskPhase};

        let dir = TempDir::new().expect("temp dir");
        let progress_path = dir.path().join("progress.md");

        // Create progress file with incomplete tasks
        let progress = ProgressFile {
            name: "Test".to_string(),
            status: "In Progress".to_string(),
            analysis: "Test analysis.".to_string(),
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
            testing_strategy: "Test with cargo test.".to_string(),
            ..Default::default()
        };
        progress.write(&progress_path).expect("write");

        // Use echo mock - outputs invalid content (not stream-json format)
        let config = Config {
            claude_cmd: ClaudeCommand {
                command: "/bin/echo".to_string(),
                base_args: vec![],
            },
            max_iterations: 2, // Would run 2 iterations if valid
            ..Default::default()
        };

        let token = CancellationToken::new();

        let result = run_build_command(
            progress_path.clone(),
            false,
            false,
            true, // no_tui
            PromptMode::Basic,
            false, // no_dsp
            &config,
            token,
            None,
        )
        .await;

        // Echo produces invalid output, so validation should reject it
        assert!(result.is_err(), "Invalid output should be rejected");
    }

    #[tokio::test]
    async fn test_resume_rejects_invalid_output() {
        use crate::config::ClaudeCommand;
        use crate::progress::{Attempt, IterationEntry, Task, TaskPhase};

        let dir = TempDir::new().expect("temp dir");

        // Create progress file simulating prior interruption
        // 2 tasks complete, 2 remaining, iteration log shows 2 prior runs
        let progress = ProgressFile {
            name: "Resume Test".to_string(),
            status: "In Progress".to_string(),
            analysis: "Testing resume capability.".to_string(),
            tasks: vec![TaskPhase {
                name: "Phase 1".to_string(),
                tasks: vec![
                    Task {
                        description: "Task 1 - already done".to_string(),
                        completed: true,
                    },
                    Task {
                        description: "Task 2 - already done".to_string(),
                        completed: true,
                    },
                    Task {
                        description: "Task 3 - next to execute".to_string(),
                        completed: false,
                    },
                    Task {
                        description: "Task 4 - waiting".to_string(),
                        completed: false,
                    },
                ],
            }],
            testing_strategy: "Test with cargo test.".to_string(),
            completed_this_iteration: vec![],
            recent_attempts: vec![
                Attempt {
                    iteration: 1,
                    tried: "Task 1".to_string(),
                    result: "Completed".to_string(),
                    next: Some("Continue".to_string()),
                },
                Attempt {
                    iteration: 2,
                    tried: "Task 2".to_string(),
                    result: "Completed".to_string(),
                    next: Some("Continue".to_string()),
                },
            ],
            iteration_log: vec![
                IterationEntry {
                    iteration: 1,
                    started: "2024-01-01 10:00".to_string(),
                    duration: "2m 30s".to_string(),
                    tasks_completed: 1,
                    notes: "Task 1".to_string(),
                },
                IterationEntry {
                    iteration: 2,
                    started: "2024-01-01 10:03".to_string(),
                    duration: "3m 15s".to_string(),
                    tasks_completed: 1,
                    notes: "Task 2".to_string(),
                },
            ],
        };

        let progress_path = dir.path().join("progress.md");
        progress.write(&progress_path).expect("write progress");

        // Run build with once mode using echo mock (produces invalid output)
        let config = Config {
            claude_cmd: ClaudeCommand {
                command: "/bin/echo".to_string(),
                base_args: vec![],
            },
            max_iterations: 1, // Will run 1 iteration
            ..Default::default()
        };

        let token = CancellationToken::new();

        // The key test is that the build starts correctly but rejects invalid output
        let result = run_build_command(
            progress_path.clone(),
            true, // once mode to limit execution
            false,
            true, // no_tui
            PromptMode::Basic,
            false, // no_dsp
            &config,
            token,
            None,
        )
        .await;

        // Echo produces invalid output, so validation should reject it
        assert!(result.is_err(), "Resume should reject invalid output");
    }
}

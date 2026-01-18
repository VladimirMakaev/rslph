//! Main build command handler.
//!
//! Orchestrates the build loop with state machine, iteration execution,
//! and termination handling.

use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::error::RslphError;
use crate::progress::ProgressFile;

use super::iteration::run_single_iteration;
use super::state::{BuildContext, BuildState, DoneReason, IterationResult};

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
/// * `config` - Application configuration
/// * `cancel_token` - Token for graceful cancellation
///
/// # Returns
///
/// * `Ok(())` - Build completed successfully
/// * `Err(e)` - Build failed with error
pub async fn run_build_command(
    progress_path: PathBuf,
    once: bool,
    dry_run: bool,
    config: &Config,
    cancel_token: CancellationToken,
) -> color_eyre::Result<()> {
    // Load initial progress file
    let progress = ProgressFile::load(&progress_path)?;

    println!(
        "Build started: {}",
        progress_path.display()
    );
    println!(
        "Tasks: {}/{} complete",
        progress.completed_tasks(),
        progress.total_tasks()
    );

    // Create build context
    let mut ctx = BuildContext::new(
        progress_path.clone(),
        progress,
        config.clone(),
        cancel_token.clone(),
        once,
        dry_run,
    );

    // Dry-run mode: preview and exit
    if dry_run {
        return run_dry_run(&ctx);
    }

    // Main iteration loop with state machine
    let mut state = BuildState::Starting;

    loop {
        state = match state {
            BuildState::Starting => {
                ctx.current_iteration = 1;
                ctx.iteration_start = Some(std::time::Instant::now());
                eprintln!("\n--- Iteration 1 ---");
                BuildState::Running { iteration: 1 }
            }

            BuildState::Running { iteration } => {
                match run_single_iteration(&mut ctx).await {
                    Ok(IterationResult::Continue { tasks_completed }) => {
                        BuildState::IterationComplete {
                            iteration,
                            tasks_completed,
                        }
                    }
                    Ok(IterationResult::Done(reason)) => BuildState::Done { reason },
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
                let duration = ctx
                    .iteration_start
                    .map(|s| s.elapsed())
                    .unwrap_or_default();
                eprintln!(
                    "[BUILD] Iteration {} complete: {} task(s) completed in {:.1}s",
                    iteration,
                    tasks_completed,
                    duration.as_secs_f64()
                );
                eprintln!(
                    "[BUILD] Progress: {}/{} tasks",
                    ctx.progress.completed_tasks(),
                    ctx.progress.total_tasks()
                );

                // Log to progress file
                log_iteration(&mut ctx, iteration, tasks_completed)?;

                // Check termination conditions in priority order
                if ctx.once_mode {
                    BuildState::Done {
                        reason: DoneReason::SingleIterationComplete,
                    }
                } else if iteration >= ctx.max_iterations {
                    eprintln!("[BUILD] Max iterations ({}) reached", ctx.max_iterations);
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
                        eprintln!("\n--- Iteration {} ---", iteration + 1);
                        BuildState::Running {
                            iteration: iteration + 1,
                        }
                    }
                }
            }

            BuildState::Done { reason } => {
                print_completion_message(&reason, &ctx);
                return Ok(());
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
fn run_dry_run(ctx: &BuildContext) -> color_eyre::Result<()> {
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

    Ok(())
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

    ctx.progress.log_iteration(
        iteration,
        &started,
        &duration,
        tasks_completed,
        &notes,
    );

    ctx.progress.write(&ctx.progress_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn test_build_command_with_echo_mock() {
        // This test uses echo as a mock for Claude.
        // Echo outputs garbage but ProgressFile::parse is lenient.
        let dir = TempDir::new().expect("temp dir");
        let progress_path = create_test_progress_file(&dir);

        let config = Config {
            claude_path: "/bin/echo".to_string(),
            ..Default::default()
        };

        let token = CancellationToken::new();

        // Run with --once to avoid infinite loop
        let result = run_build_command(
            progress_path,
            true, // once
            false,
            &config,
            token,
        )
        .await;

        // Echo outputs garbage, but we complete one iteration
        assert!(result.is_ok(), "Should complete: {:?}", result);
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
            &config,
            token,
        )
        .await;

        assert!(result.is_ok(), "Dry run should succeed");
    }

    #[tokio::test]
    async fn test_build_command_timeout() {
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
            claude_path: script_path.to_string_lossy().to_string(),
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
            &config,
            token,
        )
        .await;

        // Should complete (possibly with cancellation)
        // The key is it doesn't hang
        assert!(result.is_ok(), "Should handle timeout/cancel: {:?}", result);
    }

    #[tokio::test]
    async fn test_build_command_cancellation() {
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

        let result = run_build_command(progress_path, false, false, &config, token).await;

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
            &config,
            token,
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
            &config,
            token,
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
            true,  // once mode
            true,  // dry_run
            &config,
            token,
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
            token,
            true,  // once_mode
            true,  // dry_run
        );

        // Verify dry run function succeeds
        let result = run_dry_run(&ctx);
        assert!(result.is_ok(), "Dry run function should succeed");
    }
}

//! Build loop state machine types.
//!
//! Provides state enum, done reason, iteration result, and build context.

use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::progress::ProgressFile;
use crate::prompts::PromptMode;
use crate::tui::SubprocessEvent;
use crate::vcs::{create_vcs, Vcs};

use super::tokens::{IterationTokens, TokenUsage};

/// Build loop states for the state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum BuildState {
    /// Initial state, about to start first iteration.
    Starting,

    /// Running an iteration (subprocess active).
    Running { iteration: u32 },

    /// Iteration complete, deciding next action.
    IterationComplete {
        iteration: u32,
        tasks_completed: u32,
    },

    /// All tasks done or termination condition met.
    Done { reason: DoneReason },

    /// Error occurred during execution.
    Failed { error: String },
}

/// Reason for build loop termination.
#[derive(Debug, Clone, PartialEq)]
pub enum DoneReason {
    /// All tasks in progress file are marked complete.
    AllTasksComplete,
    /// RALPH_DONE marker detected in status.
    RalphDoneMarker,
    /// Maximum iterations reached.
    MaxIterationsReached,
    /// User cancelled via Ctrl+C.
    UserCancelled,
    /// Single iteration mode (--once flag).
    SingleIterationComplete,
}

impl std::fmt::Display for DoneReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DoneReason::AllTasksComplete => write!(f, "All tasks complete"),
            DoneReason::RalphDoneMarker => write!(f, "RALPH_DONE marker detected"),
            DoneReason::MaxIterationsReached => write!(f, "Maximum iterations reached"),
            DoneReason::UserCancelled => write!(f, "Cancelled by user"),
            DoneReason::SingleIterationComplete => write!(f, "Single iteration complete (--once)"),
        }
    }
}

/// Result of a single iteration.
#[derive(Debug)]
pub enum IterationResult {
    /// Iteration completed successfully, continue to next.
    Continue {
        /// Number of tasks marked complete this iteration.
        tasks_completed: u32,
    },
    /// Build should stop.
    Done(DoneReason),
    /// Iteration timed out, can be retried.
    Timeout,
}

impl IterationResult {
    /// Check if this result indicates completion.
    pub fn is_done(&self) -> bool {
        matches!(self, IterationResult::Done(_))
    }
}

/// Context for build execution.
///
/// Contains all state needed for the build loop.
pub struct BuildContext {
    /// Path to the progress file.
    pub progress_path: PathBuf,
    /// Parsed progress file (re-read each iteration).
    pub progress: ProgressFile,
    /// Application configuration.
    pub config: Config,
    /// Prompt mode for this build (Basic, Gsd, GsdTdd).
    pub mode: PromptMode,
    /// Cancellation token for graceful shutdown.
    pub cancel_token: CancellationToken,
    /// Current iteration number (1-indexed).
    pub current_iteration: u32,
    /// Maximum iterations before stopping.
    pub max_iterations: u32,
    /// Single iteration mode (--once flag).
    pub once_mode: bool,
    /// Dry run mode (--dry-run flag).
    pub dry_run: bool,
    /// Iteration start time for duration tracking.
    pub iteration_start: Option<std::time::Instant>,
    /// VCS for auto-commit after iterations (None if not in a repository).
    pub vcs: Option<Box<dyn Vcs>>,
    /// Project name captured at construction for commit messages.
    pub project_name: String,
    /// TUI event sender for routing logs when TUI is active.
    /// If None, logs go to stderr.
    pub tui_tx: Option<mpsc::UnboundedSender<SubprocessEvent>>,
    /// Per-iteration token usage history.
    pub iteration_tokens: Vec<IterationTokens>,
    /// Cumulative token usage across all iterations.
    pub total_tokens: TokenUsage,
    /// Current iteration's token usage (reset each iteration).
    pub current_iteration_tokens: TokenUsage,
    /// Number of timeout retries for the current iteration.
    pub timeout_retry_count: u32,
}

impl BuildContext {
    /// Create a new build context.
    pub fn new(
        progress_path: PathBuf,
        progress: ProgressFile,
        config: Config,
        mode: PromptMode,
        cancel_token: CancellationToken,
        once_mode: bool,
        dry_run: bool,
    ) -> Self {
        Self::with_tui(
            progress_path,
            progress,
            config,
            mode,
            cancel_token,
            once_mode,
            dry_run,
            None,
        )
    }

    /// Create a new build context with optional TUI sender.
    #[allow(clippy::too_many_arguments)]
    pub fn with_tui(
        progress_path: PathBuf,
        progress: ProgressFile,
        config: Config,
        mode: PromptMode,
        cancel_token: CancellationToken,
        once_mode: bool,
        dry_run: bool,
        tui_tx: Option<mpsc::UnboundedSender<SubprocessEvent>>,
    ) -> Self {
        let max_iterations = config.max_iterations;

        // Detect and create VCS for auto-commit
        // Handle both None parent and empty parent (when path is just filename)
        let working_dir = progress_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let vcs = create_vcs(working_dir);

        // Capture project name at construction for commit messages
        // Fall back to "Unnamed" if progress file has no project name
        let project_name = if progress.name.is_empty() {
            "Unnamed".to_string()
        } else {
            progress.name.clone()
        };

        let ctx = Self {
            progress_path,
            progress,
            config,
            mode,
            cancel_token,
            current_iteration: 0,
            max_iterations,
            once_mode,
            dry_run,
            iteration_start: None,
            vcs,
            project_name: project_name.clone(),
            tui_tx,
            iteration_tokens: Vec::new(),
            total_tokens: TokenUsage::default(),
            current_iteration_tokens: TokenUsage::default(),
            timeout_retry_count: 0,
        };

        // Log initialization info
        if let Some(ref v) = ctx.vcs {
            ctx.log(&format!("[VCS] Detected {} repository", v.vcs_type()));
        }
        if ctx.project_name == "Unnamed" {
            ctx.log("[BUILD] Warning: Progress file has no project name, using 'Unnamed'");
        } else {
            ctx.log(&format!("[BUILD] Project: {}", project_name));
        }

        ctx
    }

    /// Log a message to TUI or stderr depending on mode.
    pub fn log(&self, msg: &str) {
        if let Some(ref tx) = self.tui_tx {
            let _ = tx.send(SubprocessEvent::Log(msg.to_string()));
        } else {
            eprintln!("{}", msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_state_transitions() {
        let state = BuildState::Starting;
        assert_eq!(state, BuildState::Starting);

        let state = BuildState::Running { iteration: 1 };
        assert!(matches!(state, BuildState::Running { iteration: 1 }));

        let state = BuildState::IterationComplete {
            iteration: 1,
            tasks_completed: 2,
        };
        assert!(matches!(
            state,
            BuildState::IterationComplete {
                iteration: 1,
                tasks_completed: 2
            }
        ));

        let state = BuildState::Done {
            reason: DoneReason::AllTasksComplete,
        };
        assert!(matches!(
            state,
            BuildState::Done {
                reason: DoneReason::AllTasksComplete
            }
        ));

        let state = BuildState::Failed {
            error: "test error".to_string(),
        };
        assert!(matches!(state, BuildState::Failed { error: _ }));
    }

    #[test]
    fn test_done_reason_display() {
        assert_eq!(
            DoneReason::AllTasksComplete.to_string(),
            "All tasks complete"
        );
        assert_eq!(
            DoneReason::RalphDoneMarker.to_string(),
            "RALPH_DONE marker detected"
        );
        assert_eq!(
            DoneReason::MaxIterationsReached.to_string(),
            "Maximum iterations reached"
        );
        assert_eq!(DoneReason::UserCancelled.to_string(), "Cancelled by user");
        assert_eq!(
            DoneReason::SingleIterationComplete.to_string(),
            "Single iteration complete (--once)"
        );
    }

    #[test]
    fn test_iteration_result_is_done() {
        let result = IterationResult::Continue { tasks_completed: 1 };
        assert!(!result.is_done());

        let result = IterationResult::Done(DoneReason::AllTasksComplete);
        assert!(result.is_done());
    }
}

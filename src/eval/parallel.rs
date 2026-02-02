//! Parallel eval execution infrastructure.
//!
//! Provides types and functions for running multiple eval trials
//! in parallel across different prompt modes using tokio::JoinSet.

use std::sync::Arc;

use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::prompts::PromptMode;

use super::EvalResult;

/// Maximum number of parallel trials to run.
/// Prevents API rate limiting from too many concurrent Claude CLI processes.
const DEFAULT_PARALLEL_LIMIT: usize = 3;

/// Event from a trial for progress tracking.
#[derive(Debug, Clone)]
pub struct TrialEvent {
    /// The prompt mode being used
    pub mode: PromptMode,
    /// Trial number within this mode (1-indexed)
    pub trial_num: u32,
    /// The specific event
    pub event: TrialEventKind,
}

/// Types of events that can occur during a trial.
#[derive(Debug, Clone)]
pub enum TrialEventKind {
    /// Trial has started
    Started,
    /// Planning phase is in progress
    Planning,
    /// Building phase with iteration progress
    Building { iteration: u32, max_iterations: u32 },
    /// Testing phase is running
    Testing,
    /// Trial completed successfully
    Complete { result: Box<TrialResult> },
    /// Trial failed with error
    Failed { error: String },
}

/// Result of a single trial run.
#[derive(Debug, Clone)]
pub struct TrialResult {
    /// The prompt mode that was used
    pub mode: PromptMode,
    /// Trial number (1-indexed)
    pub trial_num: u32,
    /// The eval result from this trial
    pub eval_result: EvalResult,
}

/// Run multiple eval trials in parallel across modes.
///
/// Uses tokio::JoinSet for task management and a Semaphore to limit
/// concurrent trials to avoid API rate limiting.
///
/// # Arguments
///
/// * `modes` - List of prompt modes to evaluate
/// * `trials_per_mode` - Number of trials to run per mode
/// * `project_name` - Name of the project to evaluate
/// * `keep` - Whether to keep workspace after completion
/// * `config` - Application configuration
/// * `event_tx` - Channel to send progress events
/// * `cancel_token` - Token for graceful cancellation
///
/// # Returns
///
/// Vector of TrialResult for all completed trials
#[allow(clippy::too_many_arguments)]
pub async fn run_parallel_evals(
    modes: Vec<PromptMode>,
    trials_per_mode: u32,
    project_name: String,
    keep: bool,
    config: Config,
    event_tx: mpsc::UnboundedSender<TrialEvent>,
    cancel_token: CancellationToken,
) -> Vec<TrialResult> {
    // Limit parallelism to avoid API rate limits
    let semaphore = Arc::new(Semaphore::new(DEFAULT_PARALLEL_LIMIT));
    let mut set = JoinSet::new();

    // Spawn a task for each mode/trial combination
    for mode in &modes {
        for trial_num in 1..=trials_per_mode {
            let permit = semaphore.clone();
            let tx = event_tx.clone();
            let mode = *mode;
            let config = config.clone();
            let project_name = project_name.clone();
            let cancel = cancel_token.clone();

            set.spawn(async move {
                // Acquire permit to limit parallelism
                let _permit = permit.acquire().await.expect("semaphore closed");

                // Send started event
                let _ = tx.send(TrialEvent {
                    mode,
                    trial_num,
                    event: TrialEventKind::Started,
                });

                // Run the trial
                run_single_trial_parallel(
                    mode,
                    trial_num,
                    &project_name,
                    keep,
                    &config,
                    tx.clone(),
                    cancel,
                )
                .await
            });
        }
    }

    // Collect results from all tasks
    let mut results = Vec::new();
    while let Some(result) = set.join_next().await {
        match result {
            Ok(Ok(trial_result)) => results.push(trial_result),
            Ok(Err(e)) => eprintln!("Trial failed: {}", e),
            Err(e) => eprintln!("Task panicked: {}", e),
        }
    }

    results
}

/// Run a single trial with event reporting.
///
/// Wraps the existing run_single_trial function and sends TrialEvents
/// for progress tracking.
#[allow(clippy::too_many_arguments)]
async fn run_single_trial_parallel(
    mode: PromptMode,
    trial_num: u32,
    project_name: &str,
    _keep: bool,
    config: &Config,
    event_tx: mpsc::UnboundedSender<TrialEvent>,
    cancel_token: CancellationToken,
) -> color_eyre::Result<TrialResult> {
    use super::command::run_single_trial_with_mode;

    // Send planning event
    let _ = event_tx.send(TrialEvent {
        mode,
        trial_num,
        event: TrialEventKind::Planning,
    });

    // Create progress callback that sends Building events
    let tx_for_callback = event_tx.clone();
    let max_iterations = config.max_iterations;
    let progress_callback: super::command::ProgressCallback = Arc::new(move |iteration, _total| {
        let _ = tx_for_callback.send(TrialEvent {
            mode,
            trial_num,
            event: TrialEventKind::Building {
                iteration,
                max_iterations,
            },
        });
    });

    // Run the trial with the specified mode
    let result = run_single_trial_with_mode(
        project_name,
        trial_num,
        mode,
        config,
        cancel_token,
        Some(progress_callback),
    )
    .await;

    // Send completion or failure event
    match &result {
        Ok(eval_result) => {
            let trial_result = TrialResult {
                mode,
                trial_num,
                eval_result: eval_result.clone(),
            };
            let _ = event_tx.send(TrialEvent {
                mode,
                trial_num,
                event: TrialEventKind::Complete {
                    result: Box::new(trial_result.clone()),
                },
            });
            Ok(trial_result)
        }
        Err(e) => {
            let _ = event_tx.send(TrialEvent {
                mode,
                trial_num,
                event: TrialEventKind::Failed {
                    error: e.to_string(),
                },
            });
            Err(color_eyre::eyre::eyre!("Trial failed: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trial_event_creation() {
        let event = TrialEvent {
            mode: PromptMode::Basic,
            trial_num: 1,
            event: TrialEventKind::Started,
        };

        assert_eq!(event.mode, PromptMode::Basic);
        assert_eq!(event.trial_num, 1);
        matches!(event.event, TrialEventKind::Started);
    }

    #[test]
    fn test_trial_event_building() {
        let event = TrialEvent {
            mode: PromptMode::Gsd,
            trial_num: 2,
            event: TrialEventKind::Building {
                iteration: 3,
                max_iterations: 10,
            },
        };

        assert_eq!(event.mode, PromptMode::Gsd);
        assert_eq!(event.trial_num, 2);
        if let TrialEventKind::Building {
            iteration,
            max_iterations,
        } = event.event
        {
            assert_eq!(iteration, 3);
            assert_eq!(max_iterations, 10);
        } else {
            panic!("Expected Building event");
        }
    }

    #[test]
    fn test_trial_event_failed() {
        let event = TrialEvent {
            mode: PromptMode::Gsd,
            trial_num: 5,
            event: TrialEventKind::Failed {
                error: "Test error".to_string(),
            },
        };

        if let TrialEventKind::Failed { error } = event.event {
            assert_eq!(error, "Test error");
        } else {
            panic!("Expected Failed event");
        }
    }

    #[test]
    fn test_default_parallel_limit() {
        // Verify the constant is a reasonable value
        const { assert!(DEFAULT_PARALLEL_LIMIT >= 1) };
        const { assert!(DEFAULT_PARALLEL_LIMIT <= 10) };
    }
}

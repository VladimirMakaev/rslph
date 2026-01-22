//! Eval command for controlled benchmarking in isolated environments.
//!
//! Provides the `eval` command handler that runs plan+build in temp directories
//! and aggregates metrics (tokens, timing).

mod command;
mod parallel;
mod projects;
mod statistics;
mod test_runner;

pub use command::{run_compare_command, run_eval_command, run_retest_command};
pub use parallel::{run_parallel_evals, TrialEvent, TrialEventKind, TrialResult};
pub use projects::{extract_project_files, get_project, get_test_data, is_builtin, list_projects};
pub use statistics::{StatSummary, TrialStatistics};
pub use test_runner::{load_test_cases, TestCase, TestResult, TestResults, TestRunner};

use std::path::PathBuf;
use crate::build::tokens::TokenUsage;
use crate::prompts::PromptMode;

/// Result of an eval run (EVAL-04, EVAL-05).
#[derive(Debug, Clone)]
pub struct EvalResult {
    /// Project that was evaluated
    pub project: String,
    /// Prompt mode used for this trial
    pub mode: PromptMode,
    /// Trial number (1-indexed)
    pub trial_num: u32,
    /// Total execution time in seconds
    pub elapsed_secs: f64,
    /// Total tokens consumed across plan and build
    pub total_tokens: TokenUsage,
    /// Number of build iterations
    pub iterations: u32,
    /// Path to preserved workspace (if --keep was used)
    pub workspace_path: Option<PathBuf>,
    /// Test results for built-in projects (EVAL-02, EVAL-03)
    pub test_results: Option<TestResults>,
}

/// Aggregated result of multiple trials (EVAL-06, EVAL-07).
#[derive(Debug, Clone)]
pub struct MultiTrialResult {
    /// Project that was evaluated
    pub project: String,
    /// Number of trials executed
    pub trial_count: u32,
    /// Results from each trial
    pub trials: Vec<EvalResult>,
    /// Aggregated statistics across trials
    pub statistics: TrialStatistics,
}

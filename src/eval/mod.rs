//! Eval command for controlled benchmarking in isolated environments.
//!
//! Provides the `eval` command handler that runs plan+build in temp directories
//! and aggregates metrics (tokens, timing).

mod command;
mod projects;
mod test_runner;

pub use command::run_eval_command;
pub use projects::{extract_project_files, get_project, get_test_data, is_builtin, list_projects};
pub use test_runner::{load_test_cases, TestCase, TestResult, TestResults, TestRunner};

use std::path::PathBuf;
use crate::build::tokens::TokenUsage;

/// Result of an eval run (EVAL-04, EVAL-05).
#[derive(Debug, Clone)]
pub struct EvalResult {
    /// Project that was evaluated
    pub project: String,
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

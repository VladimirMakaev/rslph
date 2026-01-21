//! Eval command for controlled benchmarking in isolated environments.
//!
//! Provides the `eval` command handler that runs plan+build in temp directories
//! and aggregates metrics (tokens, timing).

mod command;

pub use command::run_eval_command;

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
}

//! Eval command handler.

use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::build::tokens::TokenUsage;
use super::EvalResult;

/// Run the eval command (stub - to be implemented in plan 02).
pub async fn run_eval_command(
    project: String,
    _keep: bool, // Deprecated: always persist
    _no_tui: bool,
    _config: &Config,
    _cancel_token: CancellationToken,
) -> color_eyre::Result<EvalResult> {
    // Stub implementation - returns placeholder result
    Ok(EvalResult {
        project,
        elapsed_secs: 0.0,
        total_tokens: TokenUsage::default(),
        iterations: 0,
        workspace_path: if keep { Some(PathBuf::new()) } else { None },
    })
}

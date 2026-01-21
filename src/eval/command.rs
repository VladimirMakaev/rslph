//! Eval command handler.
//!
//! Orchestrates plan+build execution in persistent eval directories
//! for controlled benchmarking.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::build::run_build_command;
use crate::build::tokens::{format_tokens, TokenUsage};
use crate::config::Config;
use crate::planning::run_plan_command;
use crate::progress::ProgressFile;
use crate::prompts::test_discovery_prompt;
use crate::subprocess::{ClaudeRunner, OutputLine, StreamResponse};

use super::EvalResult;

/// Run the eval command (EVAL-01, EVAL-05, EVAL-06).
///
/// Executes plan and build in a persistent eval directory,
/// collecting metrics for tokens and timing. Results are saved
/// to `result.json` in the workspace.
///
/// # Arguments
///
/// * `project` - Path to project directory to evaluate
/// * `_keep` - Deprecated: workspaces are always persisted now
/// * `no_tui` - If true, disable TUI output
/// * `config` - Application configuration
/// * `cancel_token` - Token for graceful cancellation
///
/// # Returns
///
/// * `Ok(EvalResult)` - Eval completed with metrics
/// * `Err(e)` - Eval failed
pub async fn run_eval_command(
    project: String,
    _keep: bool, // Deprecated: always persist
    no_tui: bool,
    config: &Config,
    cancel_token: CancellationToken,
) -> color_eyre::Result<EvalResult> {
    let start = Instant::now();

    // Step 1: Resolve project path
    let project_path = PathBuf::from(&project);
    if !project_path.exists() {
        return Err(color_eyre::eyre::eyre!(
            "Project path does not exist: {}",
            project_path.display()
        ));
    }

    // Step 2: Create persistent eval workspace in config.eval_dir
    let workspace = TempDir::with_prefix(&format!(
        "rslph-eval-{}-",
        project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
    ))?;
    let working_dir = workspace.path().to_path_buf();

    println!("Eval workspace: {}", working_dir.display());

    // Step 3: Copy project files to temp directory
    copy_dir_recursive(&project_path, &working_dir)?;
    println!("Copied project files to workspace");

    // Step 4: Initialize git in workspace (required for VCS tracking)
    init_git_repo(&working_dir)?;

    // Step 5: Detect starting prompt
    let prompt = detect_eval_prompt(&working_dir)?;
    println!("Detected prompt: {} chars", prompt.len());

    // Step 6: Run plan command and capture tokens
    println!("\n=== PLANNING PHASE ===\n");
    let timeout = Duration::from_secs(config.max_iterations as u64 * 600);
    let (progress_path, plan_tokens) = run_plan_command(
        &prompt,
        false, // not adaptive
        config,
        &working_dir,
        cancel_token.clone(),
        timeout,
    )
    .await?;

    println!(
        "Planning tokens: In: {} | Out: {} | CacheW: {} | CacheR: {}",
        format_tokens(plan_tokens.input_tokens),
        format_tokens(plan_tokens.output_tokens),
        format_tokens(plan_tokens.cache_creation_input_tokens),
        format_tokens(plan_tokens.cache_read_input_tokens),
    );

    // Step 7: Run build command and capture tokens
    println!("\n=== BUILD PHASE ===\n");
    let build_tokens = run_build_command(
        progress_path.clone(),
        false,          // not once
        false,          // not dry-run
        no_tui || true, // force no-tui for eval to get clean output
        config,
        cancel_token.clone(),
    )
    .await?;

    println!(
        "Build tokens: In: {} | Out: {} | CacheW: {} | CacheR: {}",
        format_tokens(build_tokens.input_tokens),
        format_tokens(build_tokens.output_tokens),
        format_tokens(build_tokens.cache_creation_input_tokens),
        format_tokens(build_tokens.cache_read_input_tokens),
    );

    // Step 8: Aggregate tokens from plan + build
    let total_tokens = TokenUsage {
        input_tokens: plan_tokens.input_tokens + build_tokens.input_tokens,
        output_tokens: plan_tokens.output_tokens + build_tokens.output_tokens,
        cache_creation_input_tokens: plan_tokens.cache_creation_input_tokens
            + build_tokens.cache_creation_input_tokens,
        cache_read_input_tokens: plan_tokens.cache_read_input_tokens
            + build_tokens.cache_read_input_tokens,
    };

    // Step 9: Collect metrics from progress file
    let progress = ProgressFile::load(&progress_path)?;
    let iterations = progress.iteration_log.len() as u32;

    let elapsed_secs = start.elapsed().as_secs_f64();

    // Step 10: Handle workspace cleanup
    let workspace_path = if keep {
        let preserved = workspace.keep();
        println!("\nWorkspace preserved at: {}", preserved.display());
        Some(preserved)
    } else {
        // TempDir will be dropped and cleaned up automatically
        drop(workspace);
        None
    };

    Ok(EvalResult {
        project,
        elapsed_secs,
        total_tokens: total_tokens.clone(),
        iterations,
        workspace_path,
    })
}

/// Copy directory contents recursively.
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            // Skip .git directories
            if entry.file_name() == ".git" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Initialize a git repository in the workspace.
fn init_git_repo(working_dir: &PathBuf) -> std::io::Result<()> {
    use std::process::Command;

    Command::new("git")
        .args(["init"])
        .current_dir(working_dir)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "eval@rslph.local"])
        .current_dir(working_dir)
        .output()?;

    Command::new("git")
        .args(["config", "user.name", "Eval"])
        .current_dir(working_dir)
        .output()?;

    // Initial commit so we have a clean baseline
    Command::new("git")
        .args(["add", "."])
        .current_dir(working_dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "Initial eval state", "--allow-empty"])
        .current_dir(working_dir)
        .output()?;

    Ok(())
}

/// Detect the eval prompt from the project directory.
///
/// Looks for prompt.txt or README.md in the project root.
fn detect_eval_prompt(working_dir: &PathBuf) -> color_eyre::Result<String> {
    // Priority 1: prompt.txt
    let prompt_file = working_dir.join("prompt.txt");
    if prompt_file.exists() {
        return Ok(std::fs::read_to_string(prompt_file)?);
    }

    // Priority 2: README.md
    let readme_file = working_dir.join("README.md");
    if readme_file.exists() {
        return Ok(std::fs::read_to_string(readme_file)?);
    }

    // Priority 3: PROMPT.md
    let prompt_md = working_dir.join("PROMPT.md");
    if prompt_md.exists() {
        return Ok(std::fs::read_to_string(prompt_md)?);
    }

    Err(color_eyre::eyre::eyre!(
        "No prompt file found. Expected prompt.txt, README.md, or PROMPT.md in project root"
    ))
}

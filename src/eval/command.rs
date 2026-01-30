//! Eval command handler.
//!
//! Orchestrates plan+build execution in persistent eval directories
//! for controlled benchmarking.

use chrono::Utc;
use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// Callback type for reporting build iteration progress.
/// Parameters: (current_iteration, max_iterations)
pub type ProgressCallback = Arc<dyn Fn(u32, u32) + Send + Sync>;

use crate::build::run_build_command;
use crate::build::tokens::{format_tokens, TokenUsage};
use crate::config::Config;
use crate::eval::{load_test_cases, TestResults, TestRunner};
use crate::planning::run_plan_command;
use crate::progress::ProgressFile;
use crate::prompts::{test_discovery_prompt, PromptMode};
use crate::subprocess::{ClaudeRunner, OutputLine, StreamResponse};
use crate::tui::run_dashboard_tui;

use super::parallel::{run_parallel_evals, TrialEvent, TrialResult as ParallelTrialResult};
use super::{EvalResult, StatSummary, TrialStatistics};

/// Run the eval command (EVAL-01, EVAL-05, EVAL-06).
///
/// Executes plan and build in a persistent eval directory,
/// collecting metrics for tokens and timing. Results are saved
/// to `result.json` in the workspace.
///
/// # Arguments
///
/// * `project` - Path to project directory to evaluate
/// * `trials` - Number of independent trials to run
/// * `modes` - Optional list of prompt modes to evaluate (parallel when > 1)
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
    trials: u32,
    modes: Option<Vec<PromptMode>>,
    _keep: bool, // Deprecated: always persist
    no_tui: bool,
    config: &Config,
    cancel_token: CancellationToken,
) -> color_eyre::Result<EvalResult> {
    // Resolve modes: use provided list, or fall back to config default
    let resolved_modes = modes.unwrap_or_else(|| vec![config.prompt_mode]);

    // If multiple modes or multiple trials, use parallel execution
    if resolved_modes.len() > 1 {
        return run_parallel_eval_mode(
            &project,
            trials,
            &resolved_modes,
            no_tui,
            config,
            cancel_token,
        )
        .await;
    }

    // Single mode - use existing sequential behavior
    // Get the single mode (we know there's exactly one)
    let mode = resolved_modes[0];

    // Execute trials
    let mut trial_results = Vec::with_capacity(trials as usize);

    for trial_num in 1..=trials {
        if trials > 1 {
            println!("\n=== TRIAL {}/{} ===\n", trial_num, trials);
        }
        let result = run_single_trial(
            &project,
            trial_num,
            mode,
            no_tui,
            config,
            cancel_token.clone(),
            None,
        )
        .await?;
        trial_results.push(result);
    }

    // For multi-trial runs, compute and print statistics, and save results
    if trials > 1 {
        let statistics = compute_statistics(&trial_results);
        print_statistics(&statistics, trials);

        // Save multi-trial results to JSON file (EVAL-08)
        let result_path =
            save_multi_trial_result(&config.eval_dir, &project, &trial_results, &statistics)?;
        println!("\nResults saved to: {}", result_path.display());
    }

    // Return the last trial result (for backward compatibility with single-trial case)
    // The caller can access all results through the statistics if needed
    trial_results
        .pop()
        .ok_or_else(|| eyre!("No trials completed"))
}

/// Run evals across multiple modes in parallel.
///
/// This function handles parallel execution when multiple modes are specified.
/// It uses tokio::JoinSet and channels to coordinate parallel trials.
///
/// # Arguments
///
/// * `project` - Path to project directory to evaluate
/// * `trials_per_mode` - Number of trials per mode
/// * `modes` - List of prompt modes to evaluate
/// * `no_tui` - If true, disable TUI output
/// * `config` - Application configuration
/// * `cancel_token` - Token for graceful cancellation
///
/// # Returns
///
/// * `Ok(EvalResult)` - Last trial result for backward compatibility
/// * `Err(e)` - Parallel eval failed
async fn run_parallel_eval_mode(
    project: &str,
    trials_per_mode: u32,
    modes: &[PromptMode],
    no_tui: bool,
    config: &Config,
    cancel_token: CancellationToken,
) -> color_eyre::Result<EvalResult> {
    use std::collections::HashMap;
    use tokio::sync::mpsc;

    if no_tui {
        println!(
            "\n=== PARALLEL EVAL: {} modes x {} trials = {} total trials ===\n",
            modes.len(),
            trials_per_mode,
            modes.len() as u32 * trials_per_mode
        );
    }

    // Create channel for trial events
    let (event_tx, event_rx) = mpsc::unbounded_channel::<TrialEvent>();

    // Spawn TUI or print-based event handler based on no_tui flag
    let tui_handle = if !no_tui {
        // Spawn dashboard TUI
        let modes_clone = modes.to_vec();
        let cancel_clone = cancel_token.clone();
        Some(tokio::spawn(async move {
            if let Err(e) =
                run_dashboard_tui(modes_clone, trials_per_mode, event_rx, cancel_clone).await
            {
                eprintln!("Dashboard error: {}", e);
            }
        }))
    } else {
        // Spawn print-based event handler for no-TUI mode
        let mut event_rx = event_rx;
        Some(tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match &event.event {
                    super::parallel::TrialEventKind::Started => {
                        println!(
                            "[{}/{}] {} - Started",
                            event.mode, event.trial_num, event.mode
                        );
                    }
                    super::parallel::TrialEventKind::Planning => {
                        println!("[{}/{}] Planning...", event.mode, event.trial_num);
                    }
                    super::parallel::TrialEventKind::Building {
                        iteration,
                        max_iterations,
                    } => {
                        println!(
                            "[{}/{}] Building iteration {}/{}",
                            event.mode, event.trial_num, iteration, max_iterations
                        );
                    }
                    super::parallel::TrialEventKind::Testing => {
                        println!("[{}/{}] Testing...", event.mode, event.trial_num);
                    }
                    super::parallel::TrialEventKind::Complete { result } => {
                        let pass_rate = result
                            .eval_result
                            .test_results
                            .as_ref()
                            .map(|tr| tr.pass_rate())
                            .unwrap_or(0.0);
                        println!(
                            "[{}/{}] Complete - {:.1}% pass rate",
                            event.mode, event.trial_num, pass_rate
                        );
                    }
                    super::parallel::TrialEventKind::Failed { error } => {
                        println!("[{}/{}] FAILED: {}", event.mode, event.trial_num, error);
                    }
                }
            }
        }))
    };

    // Run parallel evals
    let results = run_parallel_evals(
        modes.to_vec(),
        trials_per_mode,
        project.to_string(),
        false, // keep
        no_tui,
        config.clone(),
        event_tx,
        cancel_token,
    )
    .await;

    // Wait for TUI/event handler to finish
    if let Some(handle) = tui_handle {
        // Don't wait forever - the handle will finish when the channel closes
        let _ = handle.await;
    }

    if results.is_empty() {
        return Err(eyre!("No trials completed successfully"));
    }

    // Group results by mode
    let mut by_mode: HashMap<PromptMode, Vec<&ParallelTrialResult>> = HashMap::new();
    for result in &results {
        by_mode.entry(result.mode).or_default().push(result);
    }

    // Print statistics for each mode
    println!("\n=== RESULTS BY MODE ===\n");
    for mode in modes {
        if let Some(mode_results) = by_mode.get(mode) {
            let eval_results: Vec<EvalResult> =
                mode_results.iter().map(|r| r.eval_result.clone()).collect();
            let statistics = compute_statistics(&eval_results);

            println!("--- {} ---", mode);
            print_statistics(&statistics, mode_results.len() as u32);
            println!();
        }
    }

    // Save multi-mode results to JSON
    let all_eval_results: Vec<EvalResult> = results.iter().map(|r| r.eval_result.clone()).collect();
    let _combined_stats = compute_statistics(&all_eval_results);
    let result_path = save_multi_mode_result(&config.eval_dir, project, modes, &results, &by_mode)?;
    println!("Results saved to: {}", result_path.display());

    // Return last result for backward compatibility
    results
        .into_iter()
        .last()
        .map(|r| r.eval_result)
        .ok_or_else(|| eyre!("No trials completed"))
}

/// Save multi-mode results to JSON file.
fn save_multi_mode_result(
    eval_dir: &Path,
    project: &str,
    modes: &[PromptMode],
    results: &[ParallelTrialResult],
    by_mode: &std::collections::HashMap<PromptMode, Vec<&ParallelTrialResult>>,
) -> color_eyre::Result<PathBuf> {
    // Generate filename: eval-results-{project}-multimode-{YYYY-MM-DD}.json
    let filename = format!(
        "eval-results-{}-multimode-{}.json",
        project,
        Utc::now().format("%Y-%m-%d-%H%M%S")
    );
    let path = eval_dir.join(&filename);

    // Build mode results
    let mode_results: Vec<SerializableModeResult> = modes
        .iter()
        .filter_map(|mode| {
            by_mode.get(mode).map(|mode_trials| {
                let eval_results: Vec<EvalResult> =
                    mode_trials.iter().map(|r| r.eval_result.clone()).collect();
                let statistics = compute_statistics(&eval_results);

                SerializableModeResult {
                    mode: mode.to_string(),
                    trial_count: mode_trials.len() as u32,
                    trials: mode_trials
                        .iter()
                        .map(|r| convert_trial_to_serializable(&r.eval_result))
                        .collect(),
                    statistics: convert_statistics_to_serializable(&statistics),
                }
            })
        })
        .collect();

    let result = SerializableMultiModeResult {
        project: project.to_string(),
        timestamp: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        modes: modes.iter().map(|m| m.to_string()).collect(),
        total_trials: results.len() as u32,
        results_by_mode: mode_results,
    };

    // Write pretty-printed JSON
    let json = serde_json::to_string_pretty(&result)?;
    std::fs::write(&path, json)?;

    Ok(path)
}

/// Serializable multi-mode result for JSON output.
#[derive(Debug, Serialize, Deserialize)]
struct SerializableMultiModeResult {
    project: String,
    timestamp: String,
    modes: Vec<String>,
    total_trials: u32,
    results_by_mode: Vec<SerializableModeResult>,
}

/// Serializable mode result for JSON output.
#[derive(Debug, Serialize, Deserialize)]
struct SerializableModeResult {
    mode: String,
    trial_count: u32,
    trials: Vec<SerializableTrialSummary>,
    statistics: SerializableStatistics,
}

/// Run a single trial of the eval command.
///
/// This helper function contains the core eval logic:
/// 1. Resolve project (built-in or external path)
/// 2. Create persistent eval workspace
/// 3. Extract/copy project files
/// 4. Initialize git
/// 5. Detect prompt
/// 6. Run plan command
/// 7. Run build command
/// 8. Aggregate tokens
/// 9. Run hidden tests (for built-in projects)
/// 10. Save result.json
///
/// # Arguments
///
/// * `project` - Path to project directory to evaluate
/// * `trial_num` - Trial number (1-indexed, used in workspace naming)
/// * `no_tui` - If true, disable TUI output
/// * `config` - Application configuration
/// * `cancel_token` - Token for graceful cancellation
///
/// # Returns
///
/// * `Ok(EvalResult)` - Trial completed with metrics
/// * `Err(e)` - Trial failed
async fn run_single_trial(
    project: &str,
    trial_num: u32,
    mode: PromptMode,
    _no_tui: bool,
    config: &Config,
    cancel_token: CancellationToken,
    progress_callback: Option<ProgressCallback>,
) -> color_eyre::Result<EvalResult> {
    let start = Instant::now();

    // Step 1: Resolve project - check if built-in or external path
    let (is_builtin_project, project_source) = if crate::eval::is_builtin(project) {
        (true, None)
    } else {
        let path = PathBuf::from(&project);
        if !path.exists() {
            return Err(eyre!(
                "Project '{}' is neither a built-in project nor a valid path",
                project
            ));
        }
        (false, Some(path))
    };

    // Step 2: Create persistent eval workspace in config.eval_dir
    let project_name = if is_builtin_project {
        project.to_string()
    } else {
        project_source
            .as_ref()
            .unwrap()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string()
    };

    // Create timestamped directory name with trial suffix
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let workspace_name = format!("{}-{}-trial{}", project_name, timestamp, trial_num);
    let working_dir = config.eval_dir.join(&workspace_name);

    // Ensure eval_dir exists and create workspace
    std::fs::create_dir_all(&working_dir)?;

    println!("Eval workspace: {}", working_dir.display());

    // Step 3: Copy/extract project files to temp directory
    if is_builtin_project {
        let proj = crate::eval::get_project(project)
            .ok_or_else(|| eyre!("Built-in project not found: {}", project))?;
        crate::eval::extract_project_files(proj, &working_dir)?;
        println!("Extracted built-in project: {}", project);
    } else {
        copy_dir_recursive(project_source.as_ref().unwrap(), &working_dir)?;
        println!("Copied project files to workspace");
    }

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
        false, // not tui
        mode,
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
        false, // not once
        false, // not dry-run
        true,  // force no-tui for eval to get clean output
        mode,
        config,
        cancel_token.clone(),
        progress_callback,
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

    // Step 10: Execute hidden tests for built-in projects (EVAL-02, EVAL-03)
    let test_results = if is_builtin_project {
        run_project_tests(project, &working_dir, config, cancel_token).await
    } else {
        None // External projects don't have hidden tests
    };

    // Step 11: Save result.json to workspace (EVAL-06)
    let result = EvalResult {
        project: project.to_string(),
        mode,
        trial_num,
        elapsed_secs,
        total_tokens: total_tokens.clone(),
        iterations,
        workspace_path: Some(working_dir.clone()),
        test_results: test_results.clone(),
    };
    save_result_json(&working_dir, &result)?;
    println!(
        "\nResults saved to: {}",
        working_dir.join("result.json").display()
    );

    Ok(result)
}

/// Run a single trial with a specific prompt mode.
///
/// This is the mode-aware version of run_single_trial for parallel evaluation.
/// It allows running trials with different prompt modes (basic, gsd, gsd_tdd).
///
/// # Arguments
///
/// * `project` - Path to project directory to evaluate
/// * `trial_num` - Trial number (1-indexed, used in workspace naming)
/// * `mode` - The prompt mode to use for this trial
/// * `no_tui` - If true, disable TUI output
/// * `config` - Application configuration
/// * `cancel_token` - Token for graceful cancellation
///
/// # Returns
///
/// * `Ok(EvalResult)` - Trial completed with metrics
/// * `Err(e)` - Trial failed
pub async fn run_single_trial_with_mode(
    project: &str,
    trial_num: u32,
    mode: PromptMode,
    no_tui: bool,
    config: &Config,
    cancel_token: CancellationToken,
    progress_callback: Option<ProgressCallback>,
) -> color_eyre::Result<EvalResult> {
    // Forward to run_single_trial with the mode parameter
    run_single_trial(
        project,
        trial_num,
        mode,
        no_tui,
        config,
        cancel_token,
        progress_callback,
    )
    .await
}

/// Re-run only the test phase on an existing eval workspace.
///
/// This is useful when:
/// - The build completed successfully
/// - But the test run script had a bug
/// - User fixed the script manually
/// - Now they want to re-run just the tests
///
/// # Arguments
///
/// * `workspace` - Path to existing eval workspace directory
/// * `config` - Application configuration
/// * `cancel_token` - Token for graceful cancellation
///
/// # Returns
///
/// * `Ok(EvalResult)` - Retest completed with updated metrics
/// * `Err(e)` - Retest failed
pub async fn run_retest_command(
    workspace: PathBuf,
    config: &Config,
    cancel_token: CancellationToken,
) -> color_eyre::Result<EvalResult> {
    let start = Instant::now();

    // Verify workspace exists
    if !workspace.exists() {
        return Err(eyre!(
            "Workspace directory does not exist: {}",
            workspace.display()
        ));
    }

    // Load existing result.json to get project name and previous metrics
    let result_path = workspace.join("result.json");
    if !result_path.exists() {
        return Err(eyre!(
            "No result.json found in workspace. Is this a valid eval workspace?\n\
             Expected: {}",
            result_path.display()
        ));
    }

    let existing_result = load_result_json(&result_path)?;
    let project = existing_result.project.clone();

    println!("Retest workspace: {}", workspace.display());
    println!("Project: {}", project);

    // Check if this is a built-in project
    if !crate::eval::is_builtin(&project) {
        return Err(eyre!(
            "Retest is only supported for built-in projects.\n\
             Project '{}' is not a built-in project.",
            project
        ));
    }

    // Run tests
    let test_results = run_project_tests(&project, &workspace, config, cancel_token).await;

    let elapsed_secs = start.elapsed().as_secs_f64();

    // Build result with original metrics but updated test results
    let result = EvalResult {
        project: existing_result.project,
        mode: existing_result.mode,
        trial_num: 1, // Retest is always a single-trial operation
        elapsed_secs: existing_result.elapsed_secs, // Keep original timing
        total_tokens: TokenUsage {
            input_tokens: existing_result.tokens.input,
            output_tokens: existing_result.tokens.output,
            cache_creation_input_tokens: existing_result.tokens.cache_creation,
            cache_read_input_tokens: existing_result.tokens.cache_read,
        },
        iterations: existing_result.iterations,
        workspace_path: Some(workspace.clone()),
        test_results: test_results.clone(),
    };

    // Save updated result.json
    save_result_json(&workspace, &result)?;

    println!("\nRetest completed in {:.1}s", elapsed_secs);
    println!("Results saved to: {}", result_path.display());

    Ok(result)
}

/// Load result.json from workspace directory.
fn load_result_json(path: &PathBuf) -> color_eyre::Result<StoredResult> {
    let content = std::fs::read_to_string(path)?;
    let result: StoredResult = serde_json::from_str(&content)?;
    Ok(result)
}

/// Stored result format (matches what we write to result.json).
#[derive(Debug, Deserialize)]
struct StoredResult {
    project: String,
    #[serde(default)]
    mode: PromptMode,
    elapsed_secs: f64,
    iterations: u32,
    tokens: StoredTokens,
    #[allow(dead_code)]
    test_results: Option<StoredTestResults>,
}

#[derive(Debug, Deserialize)]
struct StoredTokens {
    input: u64,
    output: u64,
    cache_creation: u64,
    cache_read: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StoredTestResults {
    passed: u32,
    total: u32,
    pass_rate: f64,
}

/// Serializable result for JSON output.
#[derive(Debug, Serialize)]
struct SerializableResult {
    project: String,
    mode: PromptMode,
    elapsed_secs: f64,
    iterations: u32,
    tokens: SerializableTokens,
    test_results: Option<SerializableTestResults>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializableTokens {
    input: u64,
    output: u64,
    cache_creation: u64,
    cache_read: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializableTestResults {
    passed: u32,
    total: u32,
    pass_rate: f64,
}

/// Serializable multi-trial result for JSON output (EVAL-08).
#[derive(Debug, Serialize, Deserialize)]
struct SerializableMultiTrialResult {
    project: String,
    timestamp: String,
    trial_count: u32,
    trials: Vec<SerializableTrialSummary>,
    statistics: SerializableStatistics,
}

/// Serializable trial summary for JSON output.
#[derive(Debug, Serialize, Deserialize)]
struct SerializableTrialSummary {
    trial_num: u32,
    elapsed_secs: f64,
    iterations: u32,
    tokens: SerializableTokens,
    test_results: Option<SerializableTestResults>,
    workspace_path: String,
}

/// Serializable statistics for JSON output.
#[derive(Debug, Serialize, Deserialize)]
struct SerializableStatistics {
    pass_rate: SerializableStatSummary,
    elapsed_secs: SerializableStatSummary,
    total_input_tokens: SerializableStatSummary,
    total_output_tokens: SerializableStatSummary,
    iterations: SerializableStatSummary,
}

/// Serializable stat summary for JSON output.
#[derive(Debug, Serialize, Deserialize)]
struct SerializableStatSummary {
    mean: f64,
    variance: f64,
    std_dev: f64,
    min: f64,
    max: f64,
    count: usize,
}

/// Save result.json to workspace directory.
fn save_result_json(working_dir: &Path, result: &EvalResult) -> color_eyre::Result<()> {
    let serializable = SerializableResult {
        project: result.project.clone(),
        mode: result.mode,
        elapsed_secs: result.elapsed_secs,
        iterations: result.iterations,
        tokens: SerializableTokens {
            input: result.total_tokens.input_tokens,
            output: result.total_tokens.output_tokens,
            cache_creation: result.total_tokens.cache_creation_input_tokens,
            cache_read: result.total_tokens.cache_read_input_tokens,
        },
        test_results: result
            .test_results
            .as_ref()
            .map(|tr| SerializableTestResults {
                passed: tr.passed,
                total: tr.total,
                pass_rate: tr.pass_rate(),
            }),
    };

    let json = serde_json::to_string_pretty(&serializable)?;
    std::fs::write(working_dir.join("result.json"), json)?;
    Ok(())
}

/// Compute statistics from a slice of trial results.
///
/// Extracts metrics from each trial and computes summary statistics:
/// - Pass rate (0.0 to 1.0)
/// - Elapsed time in seconds
/// - Total input tokens
/// - Total output tokens
/// - Number of build iterations
fn compute_statistics(trials: &[EvalResult]) -> TrialStatistics {
    // Extract pass rates (only from trials with test results)
    let pass_rates: Vec<f64> = trials
        .iter()
        .filter_map(|t| t.test_results.as_ref())
        .map(|tr| tr.pass_rate() / 100.0) // Convert from percentage to 0.0-1.0
        .collect();

    // Extract elapsed time
    let elapsed_secs: Vec<f64> = trials.iter().map(|t| t.elapsed_secs).collect();

    // Extract token counts
    let input_tokens: Vec<f64> = trials
        .iter()
        .map(|t| t.total_tokens.input_tokens as f64)
        .collect();

    let output_tokens: Vec<f64> = trials
        .iter()
        .map(|t| t.total_tokens.output_tokens as f64)
        .collect();

    // Extract iteration counts
    let iterations: Vec<f64> = trials.iter().map(|t| t.iterations as f64).collect();

    TrialStatistics {
        pass_rate: StatSummary::from_values(&pass_rates),
        elapsed_secs: StatSummary::from_values(&elapsed_secs),
        total_input_tokens: StatSummary::from_values(&input_tokens),
        total_output_tokens: StatSummary::from_values(&output_tokens),
        iterations: StatSummary::from_values(&iterations),
    }
}

/// Print statistics summary to stdout.
fn print_statistics(stats: &TrialStatistics, trial_count: u32) {
    println!("\n=== STATISTICAL SUMMARY ({} trials) ===\n", trial_count);

    // Pass Rate (convert back to percentage for display)
    if stats.pass_rate.count > 0 {
        println!(
            "Pass Rate:      Mean: {:.1}%  Std Dev: {:.1}%  Min: {:.1}%  Max: {:.1}%",
            stats.pass_rate.mean * 100.0,
            stats.pass_rate.std_dev() * 100.0,
            stats.pass_rate.min * 100.0,
            stats.pass_rate.max * 100.0,
        );
    } else {
        println!("Pass Rate:      N/A (no test results)");
    }

    // Execution Time
    println!(
        "Execution Time: Mean: {:.1}s  Std Dev: {:.1}s  Min: {:.1}s  Max: {:.1}s",
        stats.elapsed_secs.mean,
        stats.elapsed_secs.std_dev(),
        stats.elapsed_secs.min,
        stats.elapsed_secs.max,
    );

    // Token Usage
    println!(
        "Input Tokens:   Mean: {}  Std Dev: {}  Min: {}  Max: {}",
        format_tokens(stats.total_input_tokens.mean as u64),
        format_tokens(stats.total_input_tokens.std_dev() as u64),
        format_tokens(stats.total_input_tokens.min as u64),
        format_tokens(stats.total_input_tokens.max as u64),
    );
    println!(
        "Output Tokens:  Mean: {}  Std Dev: {}  Min: {}  Max: {}",
        format_tokens(stats.total_output_tokens.mean as u64),
        format_tokens(stats.total_output_tokens.std_dev() as u64),
        format_tokens(stats.total_output_tokens.min as u64),
        format_tokens(stats.total_output_tokens.max as u64),
    );

    // Iterations
    println!(
        "Iterations:     Mean: {:.1}  Std Dev: {:.1}  Min: {}  Max: {}",
        stats.iterations.mean,
        stats.iterations.std_dev(),
        stats.iterations.min as u32,
        stats.iterations.max as u32,
    );
}

/// Save multi-trial results to a JSON file in eval_dir (EVAL-08).
///
/// Creates a timestamped JSON file containing all trial summaries
/// and aggregated statistics for later comparison and analysis.
///
/// # Arguments
///
/// * `eval_dir` - Directory to save the results file
/// * `project` - Project name
/// * `trials` - Trial results to save
/// * `statistics` - Computed statistics across trials
///
/// # Returns
///
/// * `Ok(PathBuf)` - Path to the saved JSON file
/// * `Err(e)` - File write failed
fn save_multi_trial_result(
    eval_dir: &Path,
    project: &str,
    trials: &[EvalResult],
    statistics: &TrialStatistics,
) -> color_eyre::Result<PathBuf> {
    // Generate filename: eval-results-{project}-{YYYY-MM-DD}.json
    let filename = format!(
        "eval-results-{}-{}.json",
        project,
        Utc::now().format("%Y-%m-%d")
    );
    let path = eval_dir.join(&filename);

    // Convert to serializable format
    let serializable = convert_to_serializable(project, trials, statistics);

    // Write pretty-printed JSON
    let json = serde_json::to_string_pretty(&serializable)?;
    std::fs::write(&path, json)?;

    Ok(path)
}

/// Convert multi-trial results to serializable format.
fn convert_to_serializable(
    project: &str,
    trials: &[EvalResult],
    statistics: &TrialStatistics,
) -> SerializableMultiTrialResult {
    SerializableMultiTrialResult {
        project: project.to_string(),
        timestamp: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        trial_count: trials.len() as u32,
        trials: trials.iter().map(convert_trial_to_serializable).collect(),
        statistics: convert_statistics_to_serializable(statistics),
    }
}

/// Convert a single trial result to serializable format.
fn convert_trial_to_serializable(trial: &EvalResult) -> SerializableTrialSummary {
    SerializableTrialSummary {
        trial_num: trial.trial_num,
        elapsed_secs: trial.elapsed_secs,
        iterations: trial.iterations,
        tokens: SerializableTokens {
            input: trial.total_tokens.input_tokens,
            output: trial.total_tokens.output_tokens,
            cache_creation: trial.total_tokens.cache_creation_input_tokens,
            cache_read: trial.total_tokens.cache_read_input_tokens,
        },
        test_results: trial
            .test_results
            .as_ref()
            .map(|tr| SerializableTestResults {
                passed: tr.passed,
                total: tr.total,
                pass_rate: tr.pass_rate(),
            }),
        workspace_path: trial
            .workspace_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    }
}

/// Convert statistics to serializable format.
fn convert_statistics_to_serializable(stats: &TrialStatistics) -> SerializableStatistics {
    SerializableStatistics {
        pass_rate: convert_stat_summary(&stats.pass_rate),
        elapsed_secs: convert_stat_summary(&stats.elapsed_secs),
        total_input_tokens: convert_stat_summary(&stats.total_input_tokens),
        total_output_tokens: convert_stat_summary(&stats.total_output_tokens),
        iterations: convert_stat_summary(&stats.iterations),
    }
}

/// Convert a StatSummary to serializable format, including std_dev.
fn convert_stat_summary(stat: &StatSummary) -> SerializableStatSummary {
    SerializableStatSummary {
        mean: stat.mean,
        variance: stat.variance,
        std_dev: stat.std_dev(),
        min: stat.min,
        max: stat.max,
        count: stat.count,
    }
}

/// Load a multi-trial result from JSON file.
///
/// # Arguments
///
/// * `path` - Path to the JSON file
///
/// # Returns
///
/// * `Ok(SerializableMultiTrialResult)` - Loaded result
/// * `Err(e)` - File not found or invalid JSON
fn load_multi_trial_result(path: &Path) -> color_eyre::Result<SerializableMultiTrialResult> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| eyre!("Failed to read {}: {}", path.display(), e))?;

    let result: SerializableMultiTrialResult = serde_json::from_str(&content)
        .map_err(|e| eyre!("Invalid JSON in {}: {}", path.display(), e))?;

    Ok(result)
}

/// Run the compare command to compare two eval result files (EVAL-09).
///
/// Loads two multi-trial result JSON files and displays deltas for key metrics:
/// - Pass rate (higher is better)
/// - Execution time (lower is better)
/// - Input tokens (lower is better)
/// - Output tokens (lower is better)
///
/// # Arguments
///
/// * `file1` - Path to baseline result file
/// * `file2` - Path to comparison result file
///
/// # Returns
///
/// * `Ok(())` - Comparison completed successfully
/// * `Err(e)` - File loading or parsing failed
pub fn run_compare_command(file1: PathBuf, file2: PathBuf) -> color_eyre::Result<()> {
    let result1 = load_multi_trial_result(&file1)?;
    let result2 = load_multi_trial_result(&file2)?;

    println!("Comparing results:");
    println!(
        "  Baseline:   {} ({} trials)",
        file1.display(),
        result1.trial_count
    );
    println!(
        "  Comparison: {} ({} trials)",
        file2.display(),
        result2.trial_count
    );
    println!();

    // Print deltas for each metric
    // Pass rate: higher is better
    print_delta(
        "Pass Rate",
        result1.statistics.pass_rate.mean * 100.0,
        result2.statistics.pass_rate.mean * 100.0,
        "%",
        true, // higher is better
    );

    // Execution time: lower is better
    print_delta(
        "Execution Time",
        result1.statistics.elapsed_secs.mean,
        result2.statistics.elapsed_secs.mean,
        "s",
        false, // lower is better
    );

    // Input tokens: lower is better
    print_delta(
        "Input Tokens",
        result1.statistics.total_input_tokens.mean,
        result2.statistics.total_input_tokens.mean,
        "",
        false, // lower is better
    );

    // Output tokens: lower is better
    print_delta(
        "Output Tokens",
        result1.statistics.total_output_tokens.mean,
        result2.statistics.total_output_tokens.mean,
        "",
        false, // lower is better
    );

    Ok(())
}

/// Print a delta comparison between two values.
///
/// Shows: `{name}: {baseline}{unit} -> {comparison}{unit} ({arrow}{delta}{unit}, {percent}%)`
///
/// # Arguments
///
/// * `name` - Metric name
/// * `baseline` - Baseline value
/// * `comparison` - Comparison value
/// * `unit` - Unit suffix (e.g., "%", "s", "")
/// * `higher_is_better` - If true, positive delta shows ^, else shows v
fn print_delta(name: &str, baseline: f64, comparison: f64, unit: &str, higher_is_better: bool) {
    let delta = comparison - baseline;
    let percent = if baseline.abs() > 0.0001 {
        (delta / baseline) * 100.0
    } else {
        0.0
    };

    // Determine arrow based on whether this is an improvement
    // For "higher is better" metrics: positive delta = improvement (^)
    // For "lower is better" metrics: negative delta = improvement (^)
    let is_improvement = if higher_is_better {
        delta > 0.0
    } else {
        delta < 0.0
    };
    let arrow = if is_improvement { "^" } else { "v" };

    // Format the sign for display
    let sign = if delta >= 0.0 { "+" } else { "" };

    println!(
        "{}: {:.1}{} -> {:.1}{} ({}{:.1}{}, {}{}%)",
        name,
        baseline,
        unit,
        comparison,
        unit,
        arrow,
        delta.abs(),
        unit,
        sign,
        percent as i64
    );
}

/// Copy directory contents recursively.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
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
fn detect_eval_prompt(working_dir: &Path) -> color_eyre::Result<String> {
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

/// Run hidden tests for a built-in project.
///
/// Loads test cases from the embedded project and runs them against
/// the built program, displaying results. Uses Claude to discover
/// how to run the program, falling back to hardcoded detection.
async fn run_project_tests(
    project: &str,
    working_dir: &Path,
    config: &Config,
    cancel_token: CancellationToken,
) -> Option<TestResults> {
    println!("\n=== TEST PHASE ===\n");

    // Get test data from embedded project
    let proj = crate::eval::get_project(project)?;
    let test_content = crate::eval::get_test_data(proj)?;
    let test_cases = load_test_cases(test_content);

    if test_cases.is_empty() {
        println!("Warning: No test cases found in project");
        return None;
    }

    // Try to discover run script using Claude
    let run_script = match discover_run_script(&config.claude_path, working_dir, cancel_token).await
    {
        Ok(script_path) => Some(script_path),
        Err(e) => {
            println!("Discovery failed ({}), trying fallback detection...", e);
            None
        }
    };

    // If discovery succeeded, use script-based runner
    if let Some(script_path) = run_script {
        println!("Testing with script: {}", script_path.display());
        let runner = TestRunner::from_script(script_path, working_dir.to_path_buf());
        let results = runner.run_tests(&test_cases);

        print_test_results(&results);
        return Some(results);
    }

    // Fallback: Find the built program using hardcoded patterns
    let program_path = match find_built_program(working_dir) {
        Some(path) => path,
        None => {
            println!("Warning: Could not find built program to test");
            return None;
        }
    };

    println!("Testing program: {}", program_path.display());

    // Run tests with direct program execution
    let runner = TestRunner::new(program_path);
    let results = runner.run_tests(&test_cases);

    print_test_results(&results);
    Some(results)
}

/// Print test results summary.
fn print_test_results(results: &TestResults) {
    println!(
        "Tests: {}/{} passed ({:.1}%)",
        results.passed,
        results.total,
        results.pass_rate()
    );

    // Print failed tests for debugging
    for case in &results.cases {
        if !case.passed {
            println!(
                "  FAIL: input='{}' expected='{}' got='{}'",
                case.input, case.expected, case.actual
            );
        }
    }
}

/// Attempt to find a runnable program in the workspace.
///
/// Looks for common patterns: Rust target, Python script, shell script.
fn find_built_program(working_dir: &Path) -> Option<PathBuf> {
    // Check for Rust binary in target/debug or target/release
    let cargo_toml = working_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        // Parse Cargo.toml to find package name
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            for line in content.lines() {
                if line.trim().starts_with("name = ") {
                    let name = line.split('"').nth(1)?;
                    let debug_path = working_dir.join("target/debug").join(name);
                    let release_path = working_dir.join("target/release").join(name);
                    if debug_path.exists() {
                        return Some(debug_path);
                    }
                    if release_path.exists() {
                        return Some(release_path);
                    }
                }
            }
        }
    }

    // Check for executable scripts
    for script_name in &["main.py", "main.sh", "calculator", "calc"] {
        let script_path = working_dir.join(script_name);
        if script_path.exists() {
            return Some(script_path);
        }
    }

    None
}

/// Discover how to run the program using Claude.
///
/// Uses Claude to analyze the workspace and generate a shell script
/// that can run the built program. This is language-agnostic and works
/// for any project structure.
///
/// # Arguments
///
/// * `claude_path` - Path to Claude CLI
/// * `working_dir` - Workspace directory containing the built project
/// * `cancel_token` - Token for graceful cancellation
///
/// # Returns
///
/// * `Ok(PathBuf)` - Path to the generated run script
/// * `Err(e)` - Discovery failed
async fn discover_run_script(
    claude_path: &str,
    working_dir: &Path,
    cancel_token: CancellationToken,
) -> color_eyre::Result<PathBuf> {
    println!("Discovering how to run the program...");

    // Build workspace context for Claude
    let context = build_workspace_context(working_dir)?;

    // Prepare Claude args
    let system_prompt = test_discovery_prompt();
    let args = vec![
        "-p".to_string(),
        "--verbose".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--system-prompt".to_string(),
        system_prompt.to_string(),
        context,
    ];

    // Spawn Claude
    let mut runner = ClaudeRunner::spawn(claude_path, &args, working_dir)
        .await
        .map_err(|e| eyre!("Failed to spawn claude for test discovery: {}", e))?;

    // Run with 60 second timeout (discovery should be quick)
    let timeout = Duration::from_secs(60);
    let output = runner
        .run_with_timeout(timeout, cancel_token)
        .await
        .map_err(|e| eyre!("Claude test discovery failed: {}", e))?;

    // Parse response
    let mut stream_response = StreamResponse::new();
    for line in &output {
        if let OutputLine::Stdout(s) = line {
            stream_response.process_line(s);
        }
    }

    let script_content = extract_script(&stream_response.text)?;

    // Write script to workspace
    let script_path = working_dir.join("_run_tests.sh");
    std::fs::write(&script_path, &script_content)?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&script_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms)?;
    }

    println!("Generated run script: {}", script_path.display());
    Ok(script_path)
}

/// Build workspace context string for Claude to analyze.
fn build_workspace_context(working_dir: &Path) -> color_eyre::Result<String> {
    let mut context = String::new();

    // Add file listing
    context.push_str("## Project Files\n\n```\n");
    if let Ok(entries) = std::fs::read_dir(working_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            // Skip hidden files except config files
            if name.starts_with('.') && !name.starts_with(".python") {
                continue;
            }
            if path.is_dir() {
                context.push_str(&format!("{}/\n", name));
                // List first-level contents of directories
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    for sub in sub_entries.flatten().take(10) {
                        let sub_name = sub.file_name().to_string_lossy().to_string();
                        if !sub_name.starts_with('.') {
                            context.push_str(&format!("  {}\n", sub_name));
                        }
                    }
                }
            } else {
                context.push_str(&format!("{}\n", name));
            }
        }
    }
    context.push_str("```\n\n");

    // Add key configuration files
    let config_files = [
        "Cargo.toml",
        "pyproject.toml",
        "setup.py",
        "package.json",
        "go.mod",
        "Makefile",
        "build.zig",
        "CMakeLists.txt",
    ];

    for config_file in config_files {
        let path = working_dir.join(config_file);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                context.push_str(&format!("## {}\n\n```\n{}\n```\n\n", config_file, content));
            }
        }
    }

    // Look for main entry point files
    let entry_files = [
        "main.py", "main.rs", "main.go", "index.js", "index.ts", "main.sh",
    ];

    for entry_file in entry_files {
        let path = working_dir.join(entry_file);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                // Only include first 50 lines
                let truncated: String = content.lines().take(50).collect::<Vec<_>>().join("\n");
                context.push_str(&format!(
                    "## {} (first 50 lines)\n\n```\n{}\n```\n\n",
                    entry_file, truncated
                ));
            }
        }
    }

    // Look for Python files with __main__ in subdirectories
    if let Ok(entries) = std::fs::read_dir(working_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name.starts_with('.') || name == "__pycache__" || name == "tests" {
                    continue;
                }
                // Check for Python files in subdirectory
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    for sub in sub_entries.flatten() {
                        let sub_path = sub.path();
                        if sub_path.extension().is_some_and(|e| e == "py") {
                            if let Ok(content) = std::fs::read_to_string(&sub_path) {
                                if content.contains("if __name__") || content.contains("def main") {
                                    let truncated: String =
                                        content.lines().take(50).collect::<Vec<_>>().join("\n");
                                    context.push_str(&format!(
                                        "## {}/{} (first 50 lines - has main)\n\n```python\n{}\n```\n\n",
                                        name,
                                        sub_path.file_name().unwrap_or_default().to_string_lossy(),
                                        truncated
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(context)
}

/// Extract shell script from Claude's response.
fn extract_script(response: &str) -> color_eyre::Result<String> {
    let text = response.trim();

    // If response starts with shebang, use it directly
    if text.starts_with("#!/") {
        return Ok(text.to_string());
    }

    // Try to extract from code fence
    if let Some(start) = text.find("```") {
        let after_fence = &text[start + 3..];
        // Skip language identifier (bash, sh, etc.)
        let content_start = after_fence.find('\n').unwrap_or(0) + 1;
        let content = &after_fence[content_start..];
        if let Some(end) = content.find("```") {
            let script = content[..end].trim();
            if script.starts_with("#!/") {
                return Ok(script.to_string());
            }
            // Add shebang if missing
            return Ok(format!("#!/bin/sh\n{}", script));
        }
    }

    // Fallback: assume the whole response is the script
    if !text.is_empty() {
        if text.starts_with("#!/") {
            return Ok(text.to_string());
        }
        return Ok(format!("#!/bin/sh\n{}", text));
    }

    Err(eyre!("Could not extract script from Claude's response"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_copy_dir_recursive() {
        let src_dir = TempDir::new().expect("src temp dir");
        let dst_dir = TempDir::new().expect("dst temp dir");

        // Create source structure
        std::fs::write(src_dir.path().join("file.txt"), "content").expect("write file");
        std::fs::create_dir(src_dir.path().join("subdir")).expect("create subdir");
        std::fs::write(src_dir.path().join("subdir/nested.txt"), "nested").expect("write nested");

        // Create .git directory that should be skipped
        std::fs::create_dir(src_dir.path().join(".git")).expect("create .git");
        std::fs::write(src_dir.path().join(".git/config"), "git stuff").expect("write git config");

        // Copy
        copy_dir_recursive(src_dir.path(), dst_dir.path()).expect("copy");

        // Verify
        assert!(dst_dir.path().join("file.txt").exists());
        assert!(dst_dir.path().join("subdir/nested.txt").exists());
        assert!(
            !dst_dir.path().join(".git").exists(),
            ".git should be skipped"
        );
    }

    #[test]
    fn test_detect_eval_prompt_priority() {
        let dir = TempDir::new().expect("temp dir");

        // No prompt file
        let result = detect_eval_prompt(dir.path());
        assert!(result.is_err());

        // Add README.md
        std::fs::write(dir.path().join("README.md"), "readme content").expect("write readme");
        let result = detect_eval_prompt(dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "readme content");

        // Add prompt.txt (should take priority)
        std::fs::write(dir.path().join("prompt.txt"), "prompt content").expect("write prompt");
        let result = detect_eval_prompt(dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "prompt content");
    }

    #[test]
    fn test_init_git_repo() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().to_path_buf();

        init_git_repo(&path).expect("init git");

        assert!(path.join(".git").exists(), ".git directory should exist");
    }

    #[test]
    fn test_detect_eval_prompt_with_prompt_md() {
        let dir = TempDir::new().expect("temp dir");

        // Add PROMPT.md (priority 3)
        std::fs::write(dir.path().join("PROMPT.md"), "prompt md content").expect("write prompt md");
        let result = detect_eval_prompt(dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "prompt md content");

        // Add README.md (should take priority over PROMPT.md)
        std::fs::write(dir.path().join("README.md"), "readme content").expect("write readme");
        let result = detect_eval_prompt(dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "readme content");
    }

    #[test]
    fn test_copy_dir_recursive_empty_src() {
        let src_dir = TempDir::new().expect("src temp dir");
        let dst_dir = TempDir::new().expect("dst temp dir");

        // Copy empty directory
        copy_dir_recursive(src_dir.path(), dst_dir.path()).expect("copy");

        // Verify destination exists and is empty
        assert!(dst_dir.path().exists());
    }

    #[test]
    fn test_find_built_program_cargo_project() {
        let dir = TempDir::new().expect("temp dir");

        // Create Cargo.toml
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "myapp"
version = "0.1.0"
"#,
        )
        .expect("write Cargo.toml");

        // Create fake binary
        std::fs::create_dir_all(dir.path().join("target/debug")).expect("create target/debug");
        std::fs::write(dir.path().join("target/debug/myapp"), "binary").expect("write binary");

        let result = find_built_program(dir.path());
        assert!(result.is_some(), "Should find Cargo binary");
        assert!(
            result.unwrap().ends_with("myapp"),
            "Path should end with binary name"
        );
    }

    #[test]
    fn test_find_built_program_release_build() {
        let dir = TempDir::new().expect("temp dir");

        // Create Cargo.toml
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "myrelease"
version = "0.1.0"
"#,
        )
        .expect("write Cargo.toml");

        // Create only release binary (no debug)
        std::fs::create_dir_all(dir.path().join("target/release")).expect("create target/release");
        std::fs::write(dir.path().join("target/release/myrelease"), "binary")
            .expect("write binary");

        let result = find_built_program(dir.path());
        assert!(result.is_some(), "Should find release binary");
        assert!(
            result.unwrap().to_str().unwrap().contains("release"),
            "Path should contain 'release'"
        );
    }

    #[test]
    fn test_find_built_program_script() {
        let dir = TempDir::new().expect("temp dir");

        // Create main.py
        std::fs::write(dir.path().join("main.py"), "print('hello')").expect("write main.py");

        let result = find_built_program(dir.path());
        assert!(result.is_some(), "Should find Python script");
        assert!(
            result.unwrap().ends_with("main.py"),
            "Path should end with main.py"
        );
    }

    #[test]
    fn test_find_built_program_shell_script() {
        let dir = TempDir::new().expect("temp dir");

        // Create main.sh
        std::fs::write(dir.path().join("main.sh"), "#!/bin/bash\necho hello")
            .expect("write main.sh");

        let result = find_built_program(dir.path());
        assert!(result.is_some(), "Should find shell script");
        assert!(
            result.unwrap().ends_with("main.sh"),
            "Path should end with main.sh"
        );
    }

    #[test]
    fn test_find_built_program_calculator_name() {
        let dir = TempDir::new().expect("temp dir");

        // Create "calculator" executable
        std::fs::write(dir.path().join("calculator"), "#!/bin/bash").expect("write calculator");

        let result = find_built_program(dir.path());
        assert!(result.is_some(), "Should find calculator");
        assert!(
            result.unwrap().ends_with("calculator"),
            "Path should end with calculator"
        );
    }

    #[test]
    fn test_find_built_program_no_match() {
        let dir = TempDir::new().expect("temp dir");

        // Create a random file that doesn't match any pattern
        std::fs::write(dir.path().join("random.txt"), "content").expect("write");

        let result = find_built_program(dir.path());
        assert!(result.is_none(), "Should not find any program");
    }

    #[test]
    fn test_find_built_program_cargo_debug_over_release() {
        let dir = TempDir::new().expect("temp dir");

        // Create Cargo.toml
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "myapp"
version = "0.1.0"
"#,
        )
        .expect("write Cargo.toml");

        // Create both debug and release binaries
        std::fs::create_dir_all(dir.path().join("target/debug")).expect("create target/debug");
        std::fs::create_dir_all(dir.path().join("target/release")).expect("create target/release");
        std::fs::write(dir.path().join("target/debug/myapp"), "debug").expect("write debug");
        std::fs::write(dir.path().join("target/release/myapp"), "release").expect("write release");

        let result = find_built_program(dir.path());
        assert!(result.is_some(), "Should find binary");
        // Debug should be preferred over release
        assert!(
            result.unwrap().to_str().unwrap().contains("debug"),
            "Debug build should be preferred"
        );
    }

    #[test]
    fn test_builtin_project_detection() {
        // Test that calculator is detected as built-in
        assert!(crate::eval::is_builtin("calculator"));
        assert!(!crate::eval::is_builtin("nonexistent"));
        assert!(!crate::eval::is_builtin("/some/path"));
    }

    #[test]
    fn test_save_result_json() {
        use crate::build::tokens::TokenUsage;
        use crate::eval::TestResults;

        let dir = TempDir::new().expect("temp dir");
        let result = EvalResult {
            project: "test-project".to_string(),
            mode: PromptMode::Basic,
            trial_num: 1,
            elapsed_secs: 123.45,
            total_tokens: TokenUsage {
                input_tokens: 1000,
                output_tokens: 500,
                cache_creation_input_tokens: 100,
                cache_read_input_tokens: 50,
            },
            iterations: 5,
            workspace_path: Some(dir.path().to_path_buf()),
            test_results: Some(TestResults {
                passed: 3,
                total: 5,
                cases: vec![],
            }),
        };

        save_result_json(dir.path(), &result).expect("save result");

        // Verify file was created
        let result_path = dir.path().join("result.json");
        assert!(result_path.exists(), "result.json should exist");

        // Verify JSON content
        let content = std::fs::read_to_string(&result_path).expect("read result.json");
        let json: serde_json::Value = serde_json::from_str(&content).expect("parse json");

        assert_eq!(json["project"], "test-project");
        assert_eq!(json["elapsed_secs"], 123.45);
        assert_eq!(json["iterations"], 5);
        assert_eq!(json["tokens"]["input"], 1000);
        assert_eq!(json["tokens"]["output"], 500);
        assert_eq!(json["test_results"]["passed"], 3);
        assert_eq!(json["test_results"]["total"], 5);
        assert_eq!(json["test_results"]["pass_rate"], 60.0);
    }

    #[test]
    fn test_save_result_json_without_tests() {
        use crate::build::tokens::TokenUsage;

        let dir = TempDir::new().expect("temp dir");
        let result = EvalResult {
            project: "external-project".to_string(),
            mode: PromptMode::Basic,
            trial_num: 1,
            elapsed_secs: 50.0,
            total_tokens: TokenUsage {
                input_tokens: 200,
                output_tokens: 100,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            },
            iterations: 3,
            workspace_path: Some(dir.path().to_path_buf()),
            test_results: None,
        };

        save_result_json(dir.path(), &result).expect("save result");

        let content = std::fs::read_to_string(dir.path().join("result.json")).expect("read");
        let json: serde_json::Value = serde_json::from_str(&content).expect("parse");

        assert_eq!(json["project"], "external-project");
        assert!(json["test_results"].is_null());
    }

    #[test]
    fn test_load_result_json() {
        let dir = TempDir::new().expect("temp dir");
        let result_path = dir.path().join("result.json");

        // Write a sample result.json
        let json = r#"{
            "project": "calculator",
            "elapsed_secs": 123.45,
            "iterations": 5,
            "tokens": {
                "input": 1000,
                "output": 500,
                "cache_creation": 100,
                "cache_read": 50
            },
            "test_results": {
                "passed": 8,
                "total": 10,
                "pass_rate": 80.0
            }
        }"#;
        std::fs::write(&result_path, json).expect("write");

        let loaded = load_result_json(&result_path).expect("load");
        assert_eq!(loaded.project, "calculator");
        assert_eq!(loaded.elapsed_secs, 123.45);
        assert_eq!(loaded.iterations, 5);
        assert_eq!(loaded.tokens.input, 1000);
        assert_eq!(loaded.tokens.output, 500);
    }

    #[test]
    fn test_load_result_json_missing_file() {
        let dir = TempDir::new().expect("temp dir");
        let result_path = dir.path().join("nonexistent.json");

        let result = load_result_json(&result_path);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retest_missing_workspace() {
        let config = crate::config::Config::default();
        let cancel_token = tokio_util::sync::CancellationToken::new();

        let result = run_retest_command(
            std::path::PathBuf::from("/nonexistent/workspace"),
            &config,
            cancel_token,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("does not exist"), "Error: {}", err);
    }

    #[tokio::test]
    async fn test_retest_missing_result_json() {
        let dir = TempDir::new().expect("temp dir");
        let config = crate::config::Config::default();
        let cancel_token = tokio_util::sync::CancellationToken::new();

        let result = run_retest_command(dir.path().to_path_buf(), &config, cancel_token).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("result.json"), "Error: {}", err);
    }

    #[tokio::test]
    async fn test_retest_non_builtin_project() {
        let dir = TempDir::new().expect("temp dir");

        // Write result.json with a non-builtin project
        let json = r#"{
            "project": "my-custom-project",
            "elapsed_secs": 10.0,
            "iterations": 1,
            "tokens": {
                "input": 100,
                "output": 50,
                "cache_creation": 0,
                "cache_read": 0
            },
            "test_results": null
        }"#;
        std::fs::write(dir.path().join("result.json"), json).expect("write");

        let config = crate::config::Config::default();
        let cancel_token = tokio_util::sync::CancellationToken::new();

        let result = run_retest_command(dir.path().to_path_buf(), &config, cancel_token).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not a built-in project"),
            "Error should mention not built-in: {}",
            err
        );
    }

    #[test]
    fn test_compute_statistics_with_multiple_trials() {
        use crate::build::tokens::TokenUsage;
        use crate::eval::TestResults;

        let trials = vec![
            EvalResult {
                project: "test".to_string(),
                mode: PromptMode::Basic,
                trial_num: 1,
                elapsed_secs: 10.0,
                total_tokens: TokenUsage {
                    input_tokens: 1000,
                    output_tokens: 500,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                iterations: 3,
                workspace_path: None,
                test_results: Some(TestResults {
                    passed: 8,
                    total: 10,
                    cases: vec![],
                }),
            },
            EvalResult {
                project: "test".to_string(),
                mode: PromptMode::Basic,
                trial_num: 2,
                elapsed_secs: 15.0,
                total_tokens: TokenUsage {
                    input_tokens: 1200,
                    output_tokens: 600,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                iterations: 4,
                workspace_path: None,
                test_results: Some(TestResults {
                    passed: 10,
                    total: 10,
                    cases: vec![],
                }),
            },
            EvalResult {
                project: "test".to_string(),
                mode: PromptMode::Basic,
                trial_num: 3,
                elapsed_secs: 12.5,
                total_tokens: TokenUsage {
                    input_tokens: 800,
                    output_tokens: 400,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                iterations: 2,
                workspace_path: None,
                test_results: Some(TestResults {
                    passed: 9,
                    total: 10,
                    cases: vec![],
                }),
            },
        ];

        let stats = compute_statistics(&trials);

        // Verify pass rate (80%, 100%, 90%) -> mean = 90% = 0.9
        assert_eq!(stats.pass_rate.count, 3);
        assert!((stats.pass_rate.mean - 0.9).abs() < 0.001);
        assert!((stats.pass_rate.min - 0.8).abs() < 0.001);
        assert!((stats.pass_rate.max - 1.0).abs() < 0.001);

        // Verify elapsed time (10, 15, 12.5) -> mean = 12.5
        assert_eq!(stats.elapsed_secs.count, 3);
        assert!((stats.elapsed_secs.mean - 12.5).abs() < 0.001);
        assert!((stats.elapsed_secs.min - 10.0).abs() < 0.001);
        assert!((stats.elapsed_secs.max - 15.0).abs() < 0.001);

        // Verify iterations (3, 4, 2) -> mean = 3.0
        assert_eq!(stats.iterations.count, 3);
        assert!((stats.iterations.mean - 3.0).abs() < 0.001);
        assert!((stats.iterations.min - 2.0).abs() < 0.001);
        assert!((stats.iterations.max - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_statistics_empty_trials() {
        let trials: Vec<EvalResult> = vec![];
        let stats = compute_statistics(&trials);

        assert_eq!(stats.pass_rate.count, 0);
        assert_eq!(stats.elapsed_secs.count, 0);
        assert_eq!(stats.iterations.count, 0);
    }

    #[test]
    fn test_compute_statistics_no_test_results() {
        use crate::build::tokens::TokenUsage;

        // Trials without test results (external projects)
        let trials = vec![EvalResult {
            project: "external".to_string(),
            mode: PromptMode::Basic,
            trial_num: 1,
            elapsed_secs: 10.0,
            total_tokens: TokenUsage {
                input_tokens: 1000,
                output_tokens: 500,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            },
            iterations: 3,
            workspace_path: None,
            test_results: None, // No test results
        }];

        let stats = compute_statistics(&trials);

        // Pass rate should have count 0 since no test results
        assert_eq!(stats.pass_rate.count, 0);
        // Other stats should still work
        assert_eq!(stats.elapsed_secs.count, 1);
        assert!((stats.elapsed_secs.mean - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_save_multi_trial_result() {
        use crate::build::tokens::TokenUsage;
        use crate::eval::TestResults;

        let dir = TempDir::new().expect("temp dir");

        // Create mock trials
        let trials = vec![
            EvalResult {
                project: "calculator".to_string(),
                mode: PromptMode::Basic,
                trial_num: 1,
                elapsed_secs: 10.0,
                total_tokens: TokenUsage {
                    input_tokens: 1000,
                    output_tokens: 500,
                    cache_creation_input_tokens: 100,
                    cache_read_input_tokens: 50,
                },
                iterations: 3,
                workspace_path: Some(PathBuf::from("/tmp/workspace1")),
                test_results: Some(TestResults {
                    passed: 8,
                    total: 10,
                    cases: vec![],
                }),
            },
            EvalResult {
                project: "calculator".to_string(),
                mode: PromptMode::Basic,
                trial_num: 2,
                elapsed_secs: 12.0,
                total_tokens: TokenUsage {
                    input_tokens: 1200,
                    output_tokens: 600,
                    cache_creation_input_tokens: 120,
                    cache_read_input_tokens: 60,
                },
                iterations: 4,
                workspace_path: Some(PathBuf::from("/tmp/workspace2")),
                test_results: Some(TestResults {
                    passed: 10,
                    total: 10,
                    cases: vec![],
                }),
            },
        ];

        // Compute statistics
        let statistics = compute_statistics(&trials);

        // Save to temp directory
        let result_path = save_multi_trial_result(dir.path(), "calculator", &trials, &statistics)
            .expect("save multi-trial result");

        // Verify file exists
        assert!(result_path.exists(), "JSON file should exist");
        assert!(
            result_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("eval-results-calculator-"),
            "Filename should match pattern"
        );
        assert!(
            result_path.extension().unwrap() == "json",
            "File should have .json extension"
        );

        // Verify JSON content
        let content = std::fs::read_to_string(&result_path).expect("read json");
        let json: serde_json::Value = serde_json::from_str(&content).expect("parse json");

        // Check top-level fields
        assert_eq!(json["project"], "calculator");
        assert_eq!(json["trial_count"], 2);
        assert!(json["timestamp"].as_str().is_some());

        // Check trials array
        let trials_arr = json["trials"].as_array().expect("trials array");
        assert_eq!(trials_arr.len(), 2);
        assert_eq!(trials_arr[0]["trial_num"], 1);
        assert_eq!(trials_arr[0]["elapsed_secs"], 10.0);
        assert_eq!(trials_arr[0]["iterations"], 3);
        assert_eq!(trials_arr[0]["tokens"]["input"], 1000);
        assert_eq!(trials_arr[0]["test_results"]["passed"], 8);
        assert_eq!(trials_arr[1]["trial_num"], 2);

        // Check statistics
        let stats = &json["statistics"];
        assert!(stats["pass_rate"]["mean"].as_f64().is_some());
        assert!(stats["pass_rate"]["std_dev"].as_f64().is_some());
        assert!(stats["elapsed_secs"]["mean"].as_f64().is_some());
        assert!(stats["total_input_tokens"]["mean"].as_f64().is_some());
        assert!(stats["total_output_tokens"]["mean"].as_f64().is_some());
        assert!(stats["iterations"]["mean"].as_f64().is_some());
    }

    #[test]
    fn test_load_multi_trial_result() {
        let dir = TempDir::new().expect("temp dir");
        let result_path = dir.path().join("eval-results.json");

        // Write valid multi-trial JSON
        let json = r#"{
            "project": "calculator",
            "timestamp": "2026-01-22T01:00:00Z",
            "trial_count": 2,
            "trials": [
                {
                    "trial_num": 1,
                    "elapsed_secs": 10.0,
                    "iterations": 3,
                    "tokens": { "input": 1000, "output": 500, "cache_creation": 100, "cache_read": 50 },
                    "test_results": { "passed": 8, "total": 10, "pass_rate": 80.0 },
                    "workspace_path": "/tmp/workspace1"
                },
                {
                    "trial_num": 2,
                    "elapsed_secs": 12.0,
                    "iterations": 4,
                    "tokens": { "input": 1200, "output": 600, "cache_creation": 120, "cache_read": 60 },
                    "test_results": { "passed": 10, "total": 10, "pass_rate": 100.0 },
                    "workspace_path": "/tmp/workspace2"
                }
            ],
            "statistics": {
                "pass_rate": { "mean": 0.9, "variance": 0.01, "std_dev": 0.1, "min": 0.8, "max": 1.0, "count": 2 },
                "elapsed_secs": { "mean": 11.0, "variance": 2.0, "std_dev": 1.414, "min": 10.0, "max": 12.0, "count": 2 },
                "total_input_tokens": { "mean": 1100.0, "variance": 20000.0, "std_dev": 141.4, "min": 1000.0, "max": 1200.0, "count": 2 },
                "total_output_tokens": { "mean": 550.0, "variance": 5000.0, "std_dev": 70.7, "min": 500.0, "max": 600.0, "count": 2 },
                "iterations": { "mean": 3.5, "variance": 0.5, "std_dev": 0.707, "min": 3.0, "max": 4.0, "count": 2 }
            }
        }"#;
        std::fs::write(&result_path, json).expect("write");

        let loaded = load_multi_trial_result(&result_path).expect("load");
        assert_eq!(loaded.project, "calculator");
        assert_eq!(loaded.trial_count, 2);
        assert_eq!(loaded.trials.len(), 2);
        assert_eq!(loaded.trials[0].trial_num, 1);
        assert_eq!(loaded.trials[1].trial_num, 2);
        assert!((loaded.statistics.pass_rate.mean - 0.9).abs() < 0.001);
        assert!((loaded.statistics.elapsed_secs.mean - 11.0).abs() < 0.001);
    }

    #[test]
    fn test_load_multi_trial_result_missing_file() {
        let dir = TempDir::new().expect("temp dir");
        let result_path = dir.path().join("nonexistent.json");

        let result = load_multi_trial_result(&result_path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to read"), "Error: {}", err);
        assert!(err.contains("nonexistent.json"), "Error: {}", err);
    }

    #[test]
    fn test_load_multi_trial_result_invalid_json() {
        let dir = TempDir::new().expect("temp dir");
        let result_path = dir.path().join("invalid.json");

        // Write invalid JSON
        std::fs::write(&result_path, "{ not valid json }").expect("write");

        let result = load_multi_trial_result(&result_path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid JSON"), "Error: {}", err);
        assert!(err.contains("invalid.json"), "Error: {}", err);
    }
}

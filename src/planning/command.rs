//! Planning command handler.
//!
//! Executes Claude in headless mode to transform user ideas into structured progress files.

use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::build::tokens::{format_tokens, TokenUsage};
use crate::config::Config;
use crate::error::RslphError;
use crate::planning::{
    assess_vagueness, detect_stack, REQUIREMENTS_CLARIFIER_PERSONA, TESTING_STRATEGIST_PERSONA,
};
use crate::progress::ProgressFile;
use crate::prompts::{get_plan_prompt_for_mode, PromptMode};
use crate::subprocess::{build_claude_args, ClaudeRunner, OutputLine, StreamEvent, StreamResponse};
use crate::tui::plan_tui::{run_plan_tui, PlanTuiEvent};

/// Run the planning command.
///
/// Executes Claude CLI in headless mode with the planning system prompt,
/// parses the output into a ProgressFile, and writes it to disk.
///
/// # Arguments
///
/// * `input` - User's idea/plan description
/// * `adaptive` - Whether to use adaptive mode with clarifying questions
/// * `tui` - Whether to use TUI mode with streaming output
/// * `mode` - The prompt mode to use (Basic, Gsd, GsdTdd)
/// * `no_dsp` - If true, append --dangerously-skip-permissions to Claude
/// * `config` - Application configuration
/// * `working_dir` - Directory to use as working directory and output location
/// * `cancel_token` - Token for graceful cancellation
/// * `timeout` - Maximum duration to wait for Claude
///
/// # Returns
///
/// Tuple of (path to generated progress file, token usage).
#[allow(clippy::too_many_arguments)]
pub async fn run_plan_command(
    input: &str,
    adaptive: bool,
    tui: bool,
    mode: PromptMode,
    no_dsp: bool,
    config: &Config,
    working_dir: &Path,
    cancel_token: CancellationToken,
    timeout: Duration,
) -> color_eyre::Result<(PathBuf, TokenUsage)> {
    // If TUI mode, run the TUI planning flow
    if tui {
        return run_tui_planning(
            input,
            adaptive,
            mode,
            no_dsp,
            config,
            working_dir,
            cancel_token,
            timeout,
        )
        .await;
    }

    // If adaptive mode, run the adaptive planning flow
    if adaptive {
        return run_adaptive_planning(
            input,
            mode,
            no_dsp,
            config,
            working_dir,
            cancel_token,
            timeout,
        )
        .await;
    }

    // Basic mode: direct planning without clarification
    run_basic_planning(
        input,
        mode,
        no_dsp,
        config,
        working_dir,
        cancel_token,
        timeout,
    )
    .await
}

/// Run basic (non-adaptive) planning mode.
async fn run_basic_planning(
    input: &str,
    mode: PromptMode,
    no_dsp: bool,
    config: &Config,
    working_dir: &Path,
    cancel_token: CancellationToken,
    timeout: Duration,
) -> color_eyre::Result<(PathBuf, TokenUsage)> {
    // Step 1: Detect project stack for testing strategy
    let stack = detect_stack(working_dir);

    // Step 2: Get the planning prompt for the specified mode
    let system_prompt = get_plan_prompt_for_mode(mode);

    // Step 3: Build user input with stack context
    let full_input = format!(
        "## Detected Stack\n{}\n\n## User Request\n{}",
        stack.to_summary(),
        input
    );

    // Step 4: Build Claude CLI args for headless mode
    let args = vec![
        "-p".to_string(),              // Print mode (headless)
        "--verbose".to_string(),       // Required for stream-json with -p
        "--output-format".to_string(), // Output format
        "stream-json".to_string(),     // JSONL for structured parsing
        "--system-prompt".to_string(), // Custom system prompt
        system_prompt,
        full_input, // User input as positional arg
    ];

    // Step 5: Spawn Claude
    let combined_args = build_claude_args(&config.claude_cmd.base_args, &args, no_dsp);

    eprintln!(
        "[TRACE] Spawning: {} {:?}",
        config.claude_cmd.command,
        combined_args.iter().take(6).collect::<Vec<_>>()
    );

    let mut runner = ClaudeRunner::spawn(&config.claude_cmd.command, &combined_args, working_dir)
        .await
        .map_err(|e| RslphError::Subprocess(format!("Failed to spawn claude: {}", e)))?;

    eprintln!("[TRACE] Spawned subprocess with PID: {:?}", runner.id());

    // Step 6: Run with timeout and collect output, tracing each line
    let output = run_with_tracing(&mut runner, timeout, cancel_token.clone()).await?;

    // Step 7: Parse JSONL output using StreamResponse
    let mut stream_response = StreamResponse::new();
    for line in &output {
        if let OutputLine::Stdout(s) = line {
            stream_response.process_line(s);
        }
    }
    let response_text = stream_response.text.clone();

    eprintln!(
        "[TRACE] Claude output length: {} chars",
        response_text.len()
    );
    if let Some(model) = &stream_response.model {
        eprintln!("[TRACE] Model: {}", model);
    }
    eprintln!(
        "[TRACE] Tokens: {} in / {} out / {} cache_write / {} cache_read",
        stream_response.input_tokens,
        stream_response.output_tokens,
        stream_response.cache_creation_input_tokens,
        stream_response.cache_read_input_tokens
    );

    // Display token summary for user
    println!(
        "Tokens used: In: {} | Out: {} | CacheW: {} | CacheR: {}",
        format_tokens(stream_response.input_tokens),
        format_tokens(stream_response.output_tokens),
        format_tokens(stream_response.cache_creation_input_tokens),
        format_tokens(stream_response.cache_read_input_tokens),
    );

    // Step 7.5: Check if Claude asked questions (basic mode doesn't support interactive)
    if stream_response.has_questions() {
        let questions = stream_response.get_all_questions();
        eprintln!(
            "[TRACE] Claude asked {} question(s), but basic mode doesn't support interactive input",
            questions.len()
        );
        display_questions(&questions);
        println!("\nNote: Basic planning mode does not support interactive question answering.");
        println!("Please run with --adaptive flag for full interactive planning flow:");
        println!("  rslph plan --adaptive \"{}\"", input);
        println!("\nContinuing with available context...\n");
    }

    // Step 8: Parse response into ProgressFile
    let mut progress_file = ProgressFile::parse(&response_text)?;

    // Step 8.5: Generate project name if empty
    if progress_file.name.is_empty() {
        eprintln!("[TRACE] Progress file has no name, generating one...");
        let generated_name = generate_project_name(
            input,
            no_dsp,
            config,
            working_dir,
            cancel_token.clone(),
            timeout,
        )
        .await?;
        eprintln!("[TRACE] Generated project name: {}", generated_name);
        progress_file.name = generated_name;
    }

    // Step 9: Write to file
    let output_path = working_dir.join("progress.md");
    progress_file.write(&output_path)?;

    eprintln!("[TRACE] Wrote progress file to: {}", output_path.display());

    // Step 10: Create TokenUsage from stream_response
    let tokens = TokenUsage {
        input_tokens: stream_response.input_tokens,
        output_tokens: stream_response.output_tokens,
        cache_creation_input_tokens: stream_response.cache_creation_input_tokens,
        cache_read_input_tokens: stream_response.cache_read_input_tokens,
    };

    Ok((output_path, tokens))
}

/// Run TUI planning mode with streaming output display.
///
/// TUI mode:
/// 1. Detects project stack
/// 2. Spawns Claude with stream-json output
/// 3. Streams events to TUI for real-time display
/// 4. If Claude asks questions, enters Q&A mode in TUI
/// 5. Parses output and writes progress file
async fn run_tui_planning(
    input: &str,
    adaptive: bool,
    mode: PromptMode,
    no_dsp: bool,
    config: &Config,
    working_dir: &Path,
    cancel_token: CancellationToken,
    timeout: Duration,
) -> color_eyre::Result<(PathBuf, TokenUsage)> {
    use tokio::time::timeout as tokio_timeout;

    // Step 1: Detect project stack for testing strategy
    let stack = detect_stack(working_dir);

    // Step 2: Get the planning prompt for the specified mode
    let base_prompt = get_plan_prompt_for_mode(mode);

    // Step 2.5: Append mode indicator for adaptive mode
    let system_prompt = if adaptive {
        format!(
            "{}\n\n---\n\n**ACTIVE MODE: ADAPTIVE**\n\nYou are running in adaptive mode. You SHOULD use the `AskUserQuestion` tool to ask 2-5 clarifying questions before generating the plan. Focus on ambiguous technology choices, critical scope decisions, and project-specific context.",
            base_prompt
        )
    } else {
        base_prompt
    };

    // Step 3: Build user input with stack context
    let full_input = format!(
        "## Detected Stack\n{}\n\n## User Request\n{}",
        stack.to_summary(),
        input
    );

    // Step 4: Build Claude CLI args for streaming mode
    let args = vec![
        "-p".to_string(),
        "--verbose".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--system-prompt".to_string(),
        system_prompt,
        full_input.clone(),
    ];

    // Step 5: Spawn Claude with interactive stdin for potential questions
    let combined_args = build_claude_args(&config.claude_cmd.base_args, &args, no_dsp);
    let mut runner =
        ClaudeRunner::spawn_interactive(&config.claude_cmd.command, &combined_args, working_dir)
            .await
            .map_err(|e| RslphError::Subprocess(format!("Failed to spawn claude: {}", e)))?;

    // NOTE: Claude CLI in -p mode may wait for stdin EOF before proceeding.
    // For now, close stdin immediately. Session resume uses a new process.
    runner.close_stdin();

    // Step 6: Create channel for plan TUI events
    let (event_tx, event_rx) = mpsc::unbounded_channel::<PlanTuiEvent>();

    // Step 7: Spawn TUI task
    let tui_cancel = cancel_token.clone();
    let tui_handle = tokio::spawn(async move { run_plan_tui(event_rx, tui_cancel).await });

    // Step 8: Stream events to TUI with timeout
    let mut stream_response = StreamResponse::new();
    let stream_cancel = cancel_token.clone();

    let stream_result = tokio_timeout(timeout, async {
        loop {
            tokio::select! {
                biased;

                _ = stream_cancel.cancelled() => {
                    runner.terminate_gracefully(Duration::from_secs(5)).await
                        .map_err(|e| RslphError::Subprocess(e.to_string()))?;
                    return Err::<(), RslphError>(RslphError::Cancelled);
                }

                line = runner.next_output() => {
                    match line {
                        Some(OutputLine::Stdout(s)) => {
                            // Try to parse as JSON, forward either way
                            match StreamEvent::parse(&s) {
                                Ok(event) => {
                                    stream_response.process_event(&event);
                                    let _ = event_tx.send(PlanTuiEvent::Stream(Box::new(event)));
                                }
                                Err(_) => {
                                    // Forward raw line if not empty
                                    if !s.trim().is_empty() {
                                        let _ = event_tx.send(PlanTuiEvent::RawStdout(s));
                                    }
                                }
                            }
                        }
                        Some(OutputLine::Stderr(s)) => {
                            // Forward stderr to TUI
                            if !s.trim().is_empty() {
                                let _ = event_tx.send(PlanTuiEvent::Stderr(s));
                            }
                        }
                        None => {
                            // Stream complete
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    })
    .await;

    // Check for timeout
    match &stream_result {
        Err(_) => {
            drop(event_tx);
            let _ = tui_handle.await;
            return Err(RslphError::Timeout(timeout.as_secs()).into());
        }
        Ok(Err(e)) => {
            drop(event_tx);
            let _ = tui_handle.await;
            return Err(RslphError::Subprocess(e.to_string()).into());
        }
        Ok(Ok(())) => {}
    }

    // Step 9: Check if Claude asked questions
    if stream_response.has_questions() {
        if let Some(ref session_id) = stream_response.session_id {
            let questions = stream_response.get_all_questions();

            // Send questions to TUI for user input
            let _ = event_tx.send(PlanTuiEvent::QuestionsAsked {
                questions: questions.clone(),
                session_id: session_id.clone(),
            });

            // Drop sender to let TUI know stream is done (but keep TUI open for input)
            drop(event_tx);

            // Wait for TUI to return with answers
            let tui_state = tui_handle
                .await
                .map_err(|e| RslphError::Subprocess(format!("TUI task failed: {}", e)))?
                .map_err(|e| RslphError::Subprocess(format!("TUI error: {}", e)))?;

            // Check if user quit
            if tui_state.should_quit {
                return Err(RslphError::Cancelled.into());
            }

            // If answers were submitted, resume the session with user's answers
            if tui_state.answers_submitted {
                let answers = &tui_state.input_buffer;
                let formatted_answers = format_answers_for_resume(&questions, answers);

                eprintln!("[TRACE] TUI Session ID: {}", session_id);
                eprintln!("[TRACE] TUI answers collected ({} chars)", answers.len());

                // Resume session with answers
                match resume_session(
                    session_id,
                    &formatted_answers,
                    no_dsp,
                    config,
                    working_dir,
                    cancel_token.clone(),
                    timeout,
                )
                .await
                {
                    Ok(new_response) => {
                        eprintln!(
                            "[TRACE] TUI resume complete, output length: {} chars",
                            new_response.text.len()
                        );
                        // Update stream_response with resumed output
                        stream_response = new_response;
                    }
                    Err(e) => {
                        eprintln!("[TRACE] TUI resume failed: {}", e);
                        // Continue with what we have
                    }
                }
            }

            // Parse response into ProgressFile
            let mut progress_file = match ProgressFile::parse(&stream_response.text) {
                Ok(pf) => pf,
                Err(e) => {
                    eprintln!(
                        "[TRACE] Parse failed: {}",
                        e
                    );
                    return Err(e.into());
                }
            };

            // Generate project name if empty
            if progress_file.name.is_empty() {
                let generated_name = generate_project_name(
                    input,
                    no_dsp,
                    config,
                    working_dir,
                    cancel_token.clone(),
                    timeout,
                )
                .await?;
                progress_file.name = generated_name;
            }

            // Write to file
            let output_path = working_dir.join("progress.md");
            progress_file.write(&output_path)?;

            // Create TokenUsage from stream_response
            let tokens = TokenUsage {
                input_tokens: stream_response.input_tokens,
                output_tokens: stream_response.output_tokens,
                cache_creation_input_tokens: stream_response.cache_creation_input_tokens,
                cache_read_input_tokens: stream_response.cache_read_input_tokens,
            };

            return Ok((output_path, tokens));
        }
    }

    // No questions - normal flow
    // Step 9b: Drop sender to signal completion to TUI
    drop(event_tx);

    // Step 10: Wait for TUI to finish
    let tui_state = tui_handle
        .await
        .map_err(|e| RslphError::Subprocess(format!("TUI task failed: {}", e)))?
        .map_err(|e| RslphError::Subprocess(format!("TUI error: {}", e)))?;

    // Check if user quit
    if tui_state.should_quit {
        return Err(RslphError::Cancelled.into());
    }

    // Step 11: Parse response into ProgressFile
    let mut progress_file = ProgressFile::parse(&stream_response.text)?;

    // Step 12: Generate project name if empty (non-TUI for simplicity)
    if progress_file.name.is_empty() {
        let generated_name = generate_project_name(
            input,
            no_dsp,
            config,
            working_dir,
            cancel_token.clone(),
            timeout,
        )
        .await?;
        progress_file.name = generated_name;
    }

    // Step 13: Write to file
    let output_path = working_dir.join("progress.md");
    progress_file.write(&output_path)?;

    // Step 14: Create TokenUsage from stream_response
    let tokens = TokenUsage {
        input_tokens: stream_response.input_tokens,
        output_tokens: stream_response.output_tokens,
        cache_creation_input_tokens: stream_response.cache_creation_input_tokens,
        cache_read_input_tokens: stream_response.cache_read_input_tokens,
    };

    Ok((output_path, tokens))
}

/// Run subprocess with tracing to stderr for each line of output.
async fn run_with_tracing(
    runner: &mut ClaudeRunner,
    max_duration: Duration,
    cancel_token: CancellationToken,
) -> Result<Vec<OutputLine>, RslphError> {
    use tokio::time::timeout;

    let collect_with_trace = async {
        let mut output = Vec::new();
        loop {
            tokio::select! {
                biased;

                _ = cancel_token.cancelled() => {
                    eprintln!("[TRACE] Cancellation requested");
                    runner.terminate_gracefully(Duration::from_secs(5)).await
                        .map_err(|e| RslphError::Subprocess(e.to_string()))?;
                    return Err(RslphError::Cancelled);
                }

                line = runner.next_output() => {
                    match line {
                        Some(OutputLine::Stdout(s)) => {
                            eprintln!("[STDOUT] {}", s);
                            output.push(OutputLine::Stdout(s));
                        }
                        Some(OutputLine::Stderr(s)) => {
                            eprintln!("[STDERR] {}", s);
                            output.push(OutputLine::Stderr(s));
                        }
                        None => {
                            eprintln!("[TRACE] Subprocess streams closed");
                            break;
                        }
                    }
                }
            }
        }
        Ok(output)
    };

    match timeout(max_duration, collect_with_trace).await {
        Ok(result) => result,
        Err(_elapsed) => {
            eprintln!("[TRACE] Timeout after {:?}", max_duration);
            runner
                .terminate_gracefully(Duration::from_secs(5))
                .await
                .map_err(|e| RslphError::Subprocess(e.to_string()))?;
            Err(RslphError::Timeout(max_duration.as_secs()))
        }
    }
}

/// Run adaptive planning mode with clarifying questions.
///
/// Adaptive mode:
/// 1. Detects project stack
/// 2. Assesses vagueness of input
/// 3. If vague, asks clarifying questions via requirements clarifier persona
/// 4. Generates testing strategy via testing strategist persona
/// 5. Runs final planning with all gathered context
pub async fn run_adaptive_planning(
    input: &str,
    mode: PromptMode,
    no_dsp: bool,
    config: &Config,
    working_dir: &Path,
    cancel_token: CancellationToken,
    timeout: Duration,
) -> color_eyre::Result<(PathBuf, TokenUsage)> {
    // Step 1: Detect project stack
    let stack = detect_stack(working_dir);
    println!("Detected stack:\n{}", stack.to_summary());

    // Step 2: Assess vagueness
    let vagueness = assess_vagueness(input);
    println!("\nVagueness score: {:.2} (threshold: 0.5)", vagueness.score);
    if !vagueness.reasons.is_empty() {
        println!("Reasons: {}", vagueness.reasons.join(", "));
    }

    // Step 3: Initialize clarifications
    let mut clarifications = String::new();

    // Step 4: If vague, run requirements clarifier
    if vagueness.is_vague() {
        println!("\nInput appears vague, gathering requirements...\n");

        let clarifier_input = format!(
            "## Project Stack\n{}\n\n## User Idea\n{}",
            stack.to_summary(),
            input
        );

        let questions = run_claude_headless(
            REQUIREMENTS_CLARIFIER_PERSONA,
            &clarifier_input,
            no_dsp,
            config,
            working_dir,
            cancel_token.clone(),
            timeout,
        )
        .await?;

        // Print questions and get user input
        println!("Clarifying Questions:\n");
        println!("{}", questions);
        println!("\nPlease answer the questions above (type your answers, then Enter twice to submit):\n");

        // Read multi-line input from stdin
        clarifications = read_multiline_input()?;
        println!("\nGathered clarifications. Continuing...\n");
    } else {
        println!("\nInput is specific enough, skipping clarification.\n");
    }

    // Step 5: Run testing strategist
    println!("Generating testing strategy...\n");

    let testing_input = format!(
        "## Project Stack\n{}\n\n## Requirements\n{}\n\n## Clarifications\n{}",
        stack.to_summary(),
        input,
        if clarifications.is_empty() {
            "None"
        } else {
            &clarifications
        }
    );

    let testing_strategy = run_claude_headless(
        TESTING_STRATEGIST_PERSONA,
        &testing_input,
        no_dsp,
        config,
        working_dir,
        cancel_token.clone(),
        timeout,
    )
    .await?;

    println!("Testing strategy generated.\n");

    // Step 6: Run final planning with all context
    println!("Generating final plan...\n");

    let base_prompt = get_plan_prompt_for_mode(mode);
    // Append adaptive mode indicator
    let plan_prompt = format!(
        "{}\n\n---\n\n**ACTIVE MODE: ADAPTIVE**\n\nYou are running in adaptive mode. You SHOULD use the `AskUserQuestion` tool to ask 2-5 clarifying questions before generating the plan. Focus on ambiguous technology choices, critical scope decisions, and project-specific context.",
        base_prompt
    );
    let final_input = format!(
        "## Detected Stack\n{}\n\n## Requirements\n{}\n\n## Clarifications\n{}\n\n## Testing Strategy\n{}",
        stack.to_summary(),
        input,
        if clarifications.is_empty() { "None" } else { &clarifications },
        testing_strategy
    );

    // Build Claude CLI args for headless mode
    let args = vec![
        "-p".to_string(),
        "--verbose".to_string(), // Required for stream-json with -p
        "--output-format".to_string(),
        "stream-json".to_string(), // JSONL for structured parsing
        "--system-prompt".to_string(),
        plan_prompt,
        final_input,
    ];

    // Spawn Claude
    let combined_args = build_claude_args(&config.claude_cmd.base_args, &args, no_dsp);
    let mut runner = ClaudeRunner::spawn(&config.claude_cmd.command, &combined_args, working_dir)
        .await
        .map_err(|e| RslphError::Subprocess(format!("Failed to spawn claude: {}", e)))?;

    // Run with timeout and collect output
    let output = runner
        .run_with_timeout(timeout, cancel_token.clone())
        .await?;

    // Parse JSONL output using StreamResponse
    let mut stream_response = StreamResponse::new();
    for line in &output {
        if let OutputLine::Stdout(s) = line {
            stream_response.process_line(s);
        }
    }

    // Debug tracing for session_id and questions
    if let Some(ref session_id) = stream_response.session_id {
        eprintln!("[TRACE] Session ID captured: {}", session_id);
    } else {
        eprintln!("[TRACE] No session_id in response");
    }
    let initial_question_count = stream_response.get_all_questions().len();
    eprintln!("[TRACE] Questions detected: {}", initial_question_count);

    // Track accumulated tokens across all resume calls
    let mut total_input_tokens = stream_response.input_tokens;
    let mut total_output_tokens = stream_response.output_tokens;
    let mut total_cache_creation = stream_response.cache_creation_input_tokens;
    let mut total_cache_read = stream_response.cache_read_input_tokens;

    // Interactive loop: handle questions with session resume
    const MAX_QUESTION_ROUNDS: u32 = 5;
    let mut round = 0;

    while stream_response.has_questions() {
        round += 1;
        if round > MAX_QUESTION_ROUNDS {
            eprintln!(
                "[TRACE] Maximum question rounds ({}) reached, proceeding with current response",
                MAX_QUESTION_ROUNDS
            );
            println!(
                "\nMaximum clarification rounds ({}) reached. Proceeding with available context.\n",
                MAX_QUESTION_ROUNDS
            );
            break;
        }

        let questions = stream_response.get_all_questions();
        eprintln!(
            "[TRACE] Round {}: Claude asked {} question(s)",
            round,
            questions.len()
        );

        // Check if we have a session_id for resumption
        let session_id = match &stream_response.session_id {
            Some(id) => id.clone(),
            None => {
                eprintln!("[TRACE] No session_id available, cannot resume session");
                println!("\nNote: Session ID not available. Cannot continue with follow-up questions.");
                println!("Proceeding with current response...\n");
                break;
            }
        };

        // Display questions and collect user answers
        println!("\n--- Round {} of clarifications ---", round);
        display_questions(&questions);
        let answers = read_multiline_input()?;
        let formatted_answers = format_answers_for_resume(&questions, &answers);

        // Resume session with answers
        println!("Resuming session with your answers...\n");
        eprintln!(
            "[TRACE] Resuming session {} with answers (length: {} chars)",
            session_id,
            formatted_answers.len()
        );

        match resume_session(
            &session_id,
            &formatted_answers,
            no_dsp,
            config,
            working_dir,
            cancel_token.clone(),
            timeout,
        )
        .await
        {
            Ok(new_response) => {
                // Accumulate tokens
                total_input_tokens += new_response.input_tokens;
                total_output_tokens += new_response.output_tokens;
                total_cache_creation += new_response.cache_creation_input_tokens;
                total_cache_read += new_response.cache_read_input_tokens;

                // Update stream_response for next iteration or final output
                stream_response = new_response;

                eprintln!(
                    "[TRACE] Resume complete, output length: {} chars, has_questions: {}",
                    stream_response.text.len(),
                    stream_response.has_questions()
                );
            }
            Err(e) => {
                eprintln!("[TRACE] Resume failed: {}", e);
                println!("\nFailed to resume session: {}. Proceeding with previous response.\n", e);
                break;
            }
        }
    }

    // Final response is in stream_response
    let response_text = stream_response.text.clone();

    // Display total token summary for user
    println!(
        "Total tokens used: In: {} | Out: {} | CacheW: {} | CacheR: {}",
        format_tokens(total_input_tokens),
        format_tokens(total_output_tokens),
        format_tokens(total_cache_creation),
        format_tokens(total_cache_read),
    );
    if round > 0 {
        println!("(Accumulated across {} round(s) of Q&A)", round);
    }

    // Parse response into ProgressFile
    let mut progress_file = ProgressFile::parse(&response_text)?;

    // Generate project name if empty
    if progress_file.name.is_empty() {
        eprintln!("[TRACE] Progress file has no name, generating one...");
        let generated_name = generate_project_name(
            input,
            no_dsp,
            config,
            working_dir,
            cancel_token.clone(),
            timeout,
        )
        .await?;
        eprintln!("[TRACE] Generated project name: {}", generated_name);
        progress_file.name = generated_name;
    }

    // Write to file
    let output_path = working_dir.join("progress.md");
    progress_file.write(&output_path)?;

    // Create TokenUsage from accumulated totals
    let tokens = TokenUsage {
        input_tokens: total_input_tokens,
        output_tokens: total_output_tokens,
        cache_creation_input_tokens: total_cache_creation,
        cache_read_input_tokens: total_cache_read,
    };

    Ok((output_path, tokens))
}

/// Run Claude CLI in headless mode with a system prompt and return the response.
async fn run_claude_headless(
    system_prompt: &str,
    user_input: &str,
    no_dsp: bool,
    config: &Config,
    working_dir: &Path,
    cancel_token: CancellationToken,
    timeout: Duration,
) -> color_eyre::Result<String> {
    let args = vec![
        "-p".to_string(),
        "--verbose".to_string(), // Required for stream-json with -p
        "--output-format".to_string(),
        "stream-json".to_string(), // JSONL for structured parsing
        "--system-prompt".to_string(),
        system_prompt.to_string(),
        user_input.to_string(),
    ];

    let combined_args = build_claude_args(&config.claude_cmd.base_args, &args, no_dsp);
    let mut runner = ClaudeRunner::spawn(&config.claude_cmd.command, &combined_args, working_dir)
        .await
        .map_err(|e| RslphError::Subprocess(format!("Failed to spawn claude: {}", e)))?;

    let output = runner.run_with_timeout(timeout, cancel_token).await?;

    // Parse JSONL output using StreamResponse
    let mut stream_response = StreamResponse::new();
    for line in &output {
        if let OutputLine::Stdout(s) = line {
            stream_response.process_line(s);
        }
    }

    Ok(stream_response.text)
}

/// Resume a Claude session with user-provided answers.
///
/// Uses the Claude CLI `--resume` flag to continue an existing session.
/// This allows the conversation to continue with the user's answers to questions.
///
/// # Arguments
///
/// * `session_id` - The session ID from the previous run's init event
/// * `message` - The user's message (typically formatted answers)
/// * `no_dsp` - If true, append --dangerously-skip-permissions to Claude
/// * `config` - Application configuration
/// * `working_dir` - Directory to use as working directory
/// * `cancel_token` - Token for graceful cancellation
/// * `timeout` - Maximum duration to wait for Claude
///
/// # Returns
///
/// StreamResponse containing the resumed session output and token usage.
async fn resume_session(
    session_id: &str,
    message: &str,
    no_dsp: bool,
    config: &Config,
    working_dir: &Path,
    cancel_token: CancellationToken,
    timeout: Duration,
) -> color_eyre::Result<StreamResponse> {
    let args = vec![
        "-p".to_string(),              // Print mode (headless)
        "--verbose".to_string(),       // Required for stream-json with -p
        "--output-format".to_string(), // Output format
        "stream-json".to_string(),     // JSONL for structured parsing
        "--resume".to_string(),        // Resume the session
        session_id.to_string(),        // Session ID to resume
        message.to_string(),           // User's message (answers)
    ];

    let combined_args = build_claude_args(&config.claude_cmd.base_args, &args, no_dsp);

    eprintln!(
        "[TRACE] Resuming session {} with message length {}",
        session_id,
        message.len()
    );

    let mut runner = ClaudeRunner::spawn(&config.claude_cmd.command, &combined_args, working_dir)
        .await
        .map_err(|e| RslphError::Subprocess(format!("Failed to spawn claude for resume: {}", e)))?;

    // Run with timeout and collect output
    let output = runner.run_with_timeout(timeout, cancel_token).await?;

    // Parse JSONL output using StreamResponse
    let mut stream_response = StreamResponse::new();
    for line in &output {
        if let OutputLine::Stdout(s) = line {
            stream_response.process_line(s);
        }
    }

    eprintln!(
        "[TRACE] Resume output length: {} chars",
        stream_response.text.len()
    );
    eprintln!(
        "[TRACE] Resume tokens: {} in / {} out",
        stream_response.input_tokens, stream_response.output_tokens
    );

    Ok(stream_response)
}

/// Generate a short kebab-case project name from the user's input.
///
/// Asks Claude to summarize the project in 2 words max, formatted as kebab-case.
async fn generate_project_name(
    user_input: &str,
    no_dsp: bool,
    config: &Config,
    working_dir: &Path,
    cancel_token: CancellationToken,
    timeout: Duration,
) -> color_eyre::Result<String> {
    const NAME_GENERATOR_PROMPT: &str = r#"Generate a short project name for the following idea.

Rules:
- Exactly 2 words maximum
- Use kebab-case (lowercase with hyphens, e.g., "quadratic-solver", "todo-app", "file-sync")
- Be descriptive but concise
- Output ONLY the project name, nothing else

Example outputs:
- task-manager
- code-formatter
- weather-api
- chat-bot"#;

    let response = run_claude_headless(
        NAME_GENERATOR_PROMPT,
        user_input,
        no_dsp,
        config,
        working_dir,
        cancel_token,
        timeout,
    )
    .await?;

    // Clean up the response - take first line, trim, convert to kebab-case
    let name = response
        .lines()
        .next()
        .unwrap_or("unnamed-project")
        .trim()
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    // Ensure it's not empty
    if name.is_empty() {
        Ok("unnamed-project".to_string())
    } else {
        Ok(name)
    }
}

/// Display questions to the user in a formatted way.
///
/// Prints a header, numbered questions, and footer with instructions.
fn display_questions(questions: &[String]) {
    println!("\n========================================");
    println!("Claude is asking clarifying questions:");
    println!("========================================\n");

    for (i, question) in questions.iter().enumerate() {
        println!("{}. {}", i + 1, question);
    }

    println!("\n----------------------------------------");
    println!("Please answer the questions above.");
    println!("(Type your answers, then press Enter twice to submit)");
    println!("----------------------------------------\n");
}

/// Format user answers for session resume.
///
/// Creates a formatted message that pairs questions with the user's free-form answers.
/// Claude will interpret which parts of the answer address each question.
fn format_answers_for_resume(questions: &[String], answers: &str) -> String {
    let mut formatted = String::from("Here are my answers to your questions:\n\n");

    for (i, question) in questions.iter().enumerate() {
        formatted.push_str(&format!("Q{}: {}\n", i + 1, question));
    }

    formatted.push_str(&format!("\nMy answers:\n{}\n", answers));

    formatted
}

/// Read multi-line input from stdin.
///
/// Reading continues until two consecutive empty lines are entered.
fn read_multiline_input() -> color_eyre::Result<String> {
    let stdin = io::stdin();
    let mut lines = Vec::new();
    let mut empty_count = 0;

    for line in stdin.lock().lines() {
        let line = line?;
        if line.is_empty() {
            empty_count += 1;
            if empty_count >= 2 {
                break;
            }
            lines.push(line);
        } else {
            empty_count = 0;
            lines.push(line);
        }
    }

    // Remove trailing empty lines
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_run_plan_command_rejects_invalid_response() {
        use crate::config::ClaudeCommand;

        // This test verifies that invalid/empty Claude output is rejected.
        // Echo outputs garbage (not stream-json format) so StreamResponse.text is empty,
        // and the new validation in ProgressFile::parse correctly rejects empty content.
        let dir = TempDir::new().expect("temp dir");

        // Create a config pointing to echo instead of claude
        let config = Config {
            claude_cmd: ClaudeCommand {
                command: "/bin/echo".to_string(),
                base_args: vec![],
            },
            ..Default::default()
        };

        let token = CancellationToken::new();

        let result = run_plan_command(
            "build something",
            false, // basic mode
            false, // no TUI
            PromptMode::Basic,
            false, // no_dsp
            &config,
            dir.path(),
            token,
            Duration::from_secs(5),
        )
        .await;

        // With validation, echo output (not valid stream-json) should fail to parse
        assert!(result.is_err(), "Invalid output should be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("parse") || err.contains("valid sections"),
            "Should report parse error: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_run_plan_command_timeout() {
        use crate::config::ClaudeCommand;

        let dir = TempDir::new().expect("temp dir");

        // Use a script that ignores arguments and sleeps
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
        let result = run_plan_command(
            "anything",
            false, // basic mode
            false, // no TUI
            PromptMode::Basic,
            false, // no_dsp
            &config,
            dir.path(),
            token,
            Duration::from_millis(100),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("timeout"), "Should timeout: {}", err);
    }

    #[tokio::test]
    async fn test_run_plan_command_cancellation() {
        use crate::config::ClaudeCommand;

        let dir = TempDir::new().expect("temp dir");

        // Use a script that ignores arguments and sleeps
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

        let result = run_plan_command(
            "anything",
            false, // basic mode
            false, // no TUI
            PromptMode::Basic,
            false, // no_dsp
            &config,
            dir.path(),
            token,
            Duration::from_secs(10),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cancelled"), "Should be cancelled: {}", err);
    }

    #[tokio::test]
    async fn test_run_plan_command_nonexistent_command() {
        use crate::config::ClaudeCommand;

        let dir = TempDir::new().expect("temp dir");

        let config = Config {
            claude_cmd: ClaudeCommand {
                command: "/nonexistent/command".to_string(),
                base_args: vec![],
            },
            ..Default::default()
        };

        let token = CancellationToken::new();
        let result = run_plan_command(
            "anything",
            false, // basic mode
            false, // no TUI
            PromptMode::Basic,
            false, // no_dsp
            &config,
            dir.path(),
            token,
            Duration::from_secs(5),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("spawn") || err.contains("Subprocess"),
            "Should fail to spawn: {}",
            err
        );
    }

    #[test]
    fn test_display_questions_formats_correctly() {
        // Test that display_questions doesn't panic with various inputs
        let questions = vec![
            "What programming language?".to_string(),
            "What database?".to_string(),
        ];
        display_questions(&questions);

        // Empty questions should also not panic
        display_questions(&[]);
    }

    #[test]
    fn test_format_answers_for_resume_pairs_correctly() {
        let questions = vec![
            "What language?".to_string(),
            "What database?".to_string(),
        ];
        let answers = "Rust\nPostgreSQL";

        let formatted = format_answers_for_resume(&questions, answers);

        assert!(formatted.contains("Q1: What language?"));
        assert!(formatted.contains("Q2: What database?"));
        assert!(formatted.contains("Rust\nPostgreSQL"));
        assert!(formatted.starts_with("Here are my answers to your questions:"));
    }

    #[test]
    fn test_format_answers_for_resume_handles_empty_questions() {
        let questions: Vec<String> = vec![];
        let answers = "Some answers anyway";

        let formatted = format_answers_for_resume(&questions, answers);

        assert!(formatted.contains("Some answers anyway"));
        assert!(!formatted.contains("Q1:"));
    }

    #[test]
    fn test_format_answers_for_resume_handles_empty_answers() {
        let questions = vec!["Question 1?".to_string()];
        let answers = "";

        let formatted = format_answers_for_resume(&questions, answers);

        assert!(formatted.contains("Q1: Question 1?"));
        assert!(formatted.contains("My answers:"));
    }
}

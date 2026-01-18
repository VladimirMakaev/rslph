# Phase 4: Core Build Loop - Research

**Researched:** 2026-01-18
**Domain:** Iteration loop control, completion detection, state machine, Claude CLI execution
**Confidence:** HIGH (Existing codebase verified, Ralph pattern documented, Rust patterns established)

## Summary

Phase 4 implements the core build loop that autonomously executes tasks from a progress file until completion or max iterations. The existing infrastructure from Phases 1-3 provides all necessary components:

1. **ClaudeRunner** (Phase 2): Subprocess execution with streaming, cancellation, and timeout
2. **ProgressFile** (Phase 1): Parsing, writing, task tracking, completion detection
3. **Prompt System** (Phase 3): `include_str!` embedding with config override
4. **CLI Skeleton** (Phase 1): `rslph build` command with `--once` and `--dry-run` flags already defined

The core challenge is orchestrating these components into an iteration loop with:
- State machine for loop control (`Starting`, `Running`, `IterationComplete`, `Done`)
- Fresh Claude context per iteration (new subprocess spawn each time)
- RALPH_DONE marker detection for early termination
- Recent attempts accumulation for failure memory across iterations

**Primary recommendation:** Use a simple Rust enum state machine pattern with async loop, spawn fresh ClaudeRunner per iteration, and update ProgressFile atomically after each iteration. The existing infrastructure handles 90% of the work.

## Standard Stack

The established libraries/tools for this phase:

### Core (Already in Codebase)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.x | Async runtime, subprocess, timeout | Already implemented via ClaudeRunner |
| tokio-util | 0.7.x | CancellationToken for graceful shutdown | Already implemented for Ctrl+C |
| clap | 4.x | CLI argument parsing (`--once`, `--dry-run`) | Already defined in cli.rs |
| pulldown-cmark | 0.x | Progress file markdown parsing | Already in progress.rs |
| atomicwrites | 3.x | Atomic progress file updates | Already in progress.rs |
| thiserror | 1.x | Error types | Already in error.rs |
| color-eyre | 0.6.x | User-facing error display | Already in main.rs |

### New (Minimal Additions)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| chrono | 0.4.x | Iteration timestamps | Already a dependency for timestamping iteration log |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Simple enum state machine | rust-fsm, statig crates | Overkill for 4 states; simple match is clearer |
| Manual loop | tokio-cron-scheduler | Not cron-based; simple loop is appropriate |
| Per-iteration subprocess | Persistent Claude session | Violates fresh context requirement |

**Installation:**
No new dependencies required. All libraries already in Cargo.toml.

## Architecture Patterns

### Recommended Project Structure (Extensions)

```
src/
├── build/
│   ├── mod.rs           # Module root, re-exports
│   ├── command.rs       # Main build command handler (run_build_command)
│   ├── state.rs         # BuildState enum, BuildContext struct
│   ├── iteration.rs     # Single iteration execution logic
│   └── prompt.rs        # PROMPT_build loading (mirror of prompts/loader.rs pattern)
├── prompts/
│   ├── mod.rs           # Add get_build_prompt export
│   ├── defaults.rs      # Add BUILD_PROMPT include_str!
│   └── loader.rs        # Add get_build_prompt function
└── main.rs              # Add build command handling

prompts/
├── PROMPT_plan.md       # Existing
└── PROMPT_build.md      # NEW: Build phase instructions
```

### Pattern 1: Enum State Machine for Build Loop

**What:** Model build loop states as Rust enum, transitions via match
**When to use:** Main loop control in `run_build_command`

```rust
// Source: Standard Rust enum pattern, verified with existing codebase style

/// Build loop states
#[derive(Debug, Clone, PartialEq)]
pub enum BuildState {
    /// Initial state, about to start first iteration
    Starting,

    /// Running an iteration (subprocess active)
    Running { iteration: u32 },

    /// Iteration complete, deciding next action
    IterationComplete {
        iteration: u32,
        tasks_completed: u32,
    },

    /// All tasks done or RALPH_DONE detected
    Done { reason: DoneReason },

    /// Error occurred
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DoneReason {
    AllTasksComplete,
    RalphDoneMarker,
    MaxIterationsReached,
    UserCancelled,
}

/// Context for build execution
pub struct BuildContext {
    pub progress_path: PathBuf,
    pub progress: ProgressFile,
    pub config: Config,
    pub cancel_token: CancellationToken,
    pub current_iteration: u32,
    pub max_iterations: u32,
    pub once_mode: bool,
    pub dry_run: bool,
}
```

### Pattern 2: Fresh Context Per Iteration

**What:** Spawn new Claude subprocess for each iteration
**When to use:** Every iteration in the build loop

```rust
// Source: Existing planning/command.rs pattern

async fn run_single_iteration(
    ctx: &mut BuildContext,
) -> Result<IterationResult, RslphError> {
    // Step 1: Re-read progress file (may have been updated externally or by previous iteration)
    ctx.progress = ProgressFile::load(&ctx.progress_path)?;

    // Step 2: Check for early exit conditions
    if ctx.progress.is_done() {
        return Ok(IterationResult::Done(DoneReason::RalphDoneMarker));
    }

    if ctx.progress.completed_tasks() == ctx.progress.total_tasks() {
        return Ok(IterationResult::Done(DoneReason::AllTasksComplete));
    }

    // Step 3: Build prompt with current progress context
    let system_prompt = get_build_prompt(&ctx.config)?;
    let user_input = format!(
        "## Current Progress\n{}\n\n## Instructions\nExecute the next incomplete task.",
        ctx.progress.to_markdown()
    );

    // Step 4: Spawn FRESH Claude subprocess (no context pollution)
    let args = vec![
        "--internet".to_string(),      // WORKAROUND for CLI hang
        "-p".to_string(),              // Headless mode
        "--verbose".to_string(),       // Required for stream-json
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--system-prompt".to_string(),
        system_prompt,
        user_input,
    ];

    let working_dir = ctx.progress_path.parent().unwrap_or(Path::new("."));
    let mut runner = ClaudeRunner::spawn(&ctx.config.claude_path, &args, working_dir)
        .await
        .map_err(|e| RslphError::Subprocess(format!("Failed to spawn claude: {}", e)))?;

    // Step 5: Run with timeout and cancellation
    let timeout = Duration::from_secs(600); // 10 minutes per iteration
    let output = runner.run_with_timeout(timeout, ctx.cancel_token.clone()).await?;

    // Step 6: Parse response, update progress file
    // ...

    Ok(IterationResult::Continue { tasks_completed: N })
}
```

### Pattern 3: Main Loop Structure

**What:** Async loop with state machine transitions
**When to use:** Entry point for build command

```rust
// Source: Standard async loop pattern

pub async fn run_build_command(
    progress_path: PathBuf,
    once: bool,
    dry_run: bool,
    config: &Config,
    cancel_token: CancellationToken,
) -> color_eyre::Result<()> {
    // Load initial progress
    let progress = ProgressFile::load(&progress_path)?;

    let mut ctx = BuildContext {
        progress_path,
        progress,
        config: config.clone(),
        cancel_token: cancel_token.clone(),
        current_iteration: 0,
        max_iterations: config.max_iterations,
        once_mode: once,
        dry_run,
    };

    // Dry-run mode: preview and exit
    if dry_run {
        return run_dry_run(&ctx);
    }

    // Main iteration loop
    let mut state = BuildState::Starting;

    loop {
        state = match state {
            BuildState::Starting => {
                ctx.current_iteration = 1;
                BuildState::Running { iteration: 1 }
            }

            BuildState::Running { iteration } => {
                match run_single_iteration(&mut ctx).await {
                    Ok(IterationResult::Continue { tasks_completed }) => {
                        BuildState::IterationComplete { iteration, tasks_completed }
                    }
                    Ok(IterationResult::Done(reason)) => {
                        BuildState::Done { reason }
                    }
                    Err(RslphError::Cancelled) => {
                        BuildState::Done { reason: DoneReason::UserCancelled }
                    }
                    Err(e) => {
                        BuildState::Failed { error: e.to_string() }
                    }
                }
            }

            BuildState::IterationComplete { iteration, tasks_completed } => {
                // Log iteration
                log_iteration(&mut ctx, iteration, tasks_completed)?;

                // Check termination conditions
                if ctx.once_mode {
                    BuildState::Done { reason: DoneReason::MaxIterationsReached }
                } else if iteration >= ctx.max_iterations {
                    BuildState::Done { reason: DoneReason::MaxIterationsReached }
                } else {
                    ctx.current_iteration = iteration + 1;
                    BuildState::Running { iteration: iteration + 1 }
                }
            }

            BuildState::Done { reason } => {
                print_completion_message(&reason);
                return Ok(());
            }

            BuildState::Failed { error } => {
                return Err(color_eyre::eyre::eyre!("Build failed: {}", error));
            }
        };

        // Check for cancellation between iterations
        if cancel_token.is_cancelled() {
            state = BuildState::Done { reason: DoneReason::UserCancelled };
        }
    }
}
```

### Pattern 4: RALPH_DONE Detection

**What:** Check progress file status section for completion marker
**When to use:** Before and after each iteration

```rust
// Source: Existing progress.rs pattern

impl ProgressFile {
    /// Check if progress file indicates completion (PROG-01)
    /// Already implemented in progress.rs
    pub fn is_done(&self) -> bool {
        self.status.contains("RALPH_DONE")
    }
}

// In iteration loop:
if ctx.progress.is_done() {
    return Ok(IterationResult::Done(DoneReason::RalphDoneMarker));
}
```

### Pattern 5: Recent Attempts Accumulation

**What:** Track recent iteration attempts for failure memory
**When to use:** After each iteration, configurable depth

```rust
// Source: Existing progress.rs Attempt struct

/// Add attempt record after iteration (PROG-06)
fn record_iteration_attempt(
    progress: &mut ProgressFile,
    iteration: u32,
    tried: &str,
    result: &str,
    next: Option<&str>,
    max_attempts: usize,
) {
    progress.add_attempt(iteration, tried, result, next);

    // Trim to configurable depth (CFG-07: recent_threads)
    while progress.recent_attempts.len() > max_attempts {
        progress.recent_attempts.remove(0);
    }
}
```

### Pattern 6: Dry-Run Mode

**What:** Preview what would be executed without running Claude
**When to use:** When `--dry-run` flag is set

```rust
// Source: Standard dry-run pattern

fn run_dry_run(ctx: &BuildContext) -> color_eyre::Result<()> {
    println!("=== DRY RUN MODE ===\n");
    println!("Progress file: {}", ctx.progress_path.display());
    println!("Max iterations: {}", ctx.max_iterations);
    println!("Once mode: {}", ctx.once_mode);
    println!();

    println!("Current status: {}", ctx.progress.status);
    println!("Tasks: {}/{} complete",
             ctx.progress.completed_tasks(),
             ctx.progress.total_tasks());
    println!();

    if let Some((phase, task)) = ctx.progress.next_task() {
        println!("Next task to execute:");
        println!("  Phase: {}", phase);
        println!("  Task: {}", task.description);
    } else {
        println!("No pending tasks found.");
    }

    println!();
    println!("Would use prompt: {}",
             if ctx.config.build_prompt.is_some() {
                 "custom (from config)"
             } else {
                 "default (embedded)"
             });

    println!("\n=== END DRY RUN ===");
    Ok(())
}
```

### Anti-Patterns to Avoid

- **Persistent Claude session across iterations:** Each iteration MUST spawn fresh subprocess for clean context
- **Modifying progress file during Claude execution:** Only update after subprocess completes
- **Unbounded iterations:** ALWAYS respect max_iterations, never remove this guard
- **Swallowing cancellation:** Check CancellationToken between iterations, propagate Cancelled error
- **Blocking on subprocess:** Use async runner with timeout, never wait indefinitely

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Progress file parsing | Custom markdown regex | `ProgressFile::parse()` (exists) | Already handles all sections, checkboxes |
| Progress file writing | Direct file I/O | `ProgressFile::write()` (exists) | Atomic writes via atomicwrites crate |
| Subprocess streaming | Channel-based manual | `ClaudeRunner` (exists) | Already handles stdout/stderr, timeout, cancel |
| Completion detection | Custom status parsing | `ProgressFile::is_done()` (exists) | Already checks RALPH_DONE |
| Task finding | Manual iteration | `ProgressFile::next_task()` (exists) | Returns first incomplete task |
| Prompt loading | Custom file loader | Extend prompts/loader.rs pattern | Already has config override logic |
| Graceful shutdown | Manual signal handling | `setup_ctrl_c_handler()` (exists) | Returns CancellationToken |

**Key insight:** Phase 1-3 built all the infrastructure. Phase 4 is primarily orchestration of existing components.

## Common Pitfalls

### Pitfall 1: Context Pollution Across Iterations

**What goes wrong:** Reusing Claude session/context between iterations
**Why it happens:** Optimization attempt, or forgetting to spawn fresh
**How to avoid:** ALWAYS spawn new ClaudeRunner for each iteration
**Warning signs:** Claude referencing tasks from previous iterations, confusion

### Pitfall 2: Lost Progress on Interrupt

**What goes wrong:** Ctrl+C during iteration loses work
**Why it happens:** Not saving partial progress before shutdown
**How to avoid:**
- ClaudeRunner handles graceful termination (5s grace period)
- Save progress file atomically before iteration starts
- Consider saving partial results if Claude was running long
**Warning signs:** Users reporting lost work on interrupt

### Pitfall 3: RALPH_DONE Parsed as Task Text

**What goes wrong:** Claude outputs "RALPH_DONE" in response but not in status section
**Why it happens:** Prompt doesn't constrain where marker should appear
**How to avoid:** PROMPT_build must instruct: "Write RALPH_DONE on its own line in the Status section"
**Warning signs:** Loop continues after Claude claims completion

### Pitfall 4: Max Iterations Silent Failure

**What goes wrong:** Loop stops at max iterations without clear message
**Why it happens:** Not communicating reason for stop
**How to avoid:** Clear message: "Stopped: max iterations (20) reached. Tasks remaining: N"
**Warning signs:** Users confused why build stopped

### Pitfall 5: Iteration Log Growing Unbounded

**What goes wrong:** Thousands of iteration log entries
**Why it happens:** Never trimming old entries
**How to avoid:**
- Recent attempts: trim to config.recent_threads (default 5)
- Iteration log: could grow indefinitely, but individual entries are small
**Warning signs:** Progress file becomes huge

### Pitfall 6: Timeout Too Short or Too Long

**What goes wrong:** Complex tasks killed early, or stuck tasks hang forever
**Why it happens:** Fixed timeout doesn't fit all tasks
**How to avoid:**
- Default 10 minutes per iteration (reasonable for most tasks)
- Consider making configurable in future
**Warning signs:** Users reporting timeouts on legitimate work, or hangs

## Code Examples

Verified patterns from official sources and existing codebase:

### Example 1: PROMPT_build.md Structure

```markdown
# Build Agent

You are an autonomous coding agent executing tasks from a progress file.

## Your Role

Each iteration, you will:
1. Read the progress file provided in your context
2. Find the FIRST incomplete task (marked `[ ]`)
3. Implement ONLY that task - do not attempt multiple tasks
4. Mark the task complete by changing `[ ]` to `[x]`
5. Update the "Completed This Iteration" section
6. If ALL tasks are now complete, write `RALPH_DONE` on its own line in the Status section

## Critical Rules

1. **ONE TASK PER ITERATION** - Do not try to complete multiple tasks
2. **VERIFY BEFORE MARKING** - Only mark `[x]` after actually completing the work
3. **RALPH_DONE PLACEMENT** - Write RALPH_DONE as the first line of the Status section, alone
4. **FAILURE HANDLING** - If blocked, document in Recent Attempts and move to next iteration

## Output Format

After completing your work, output the COMPLETE updated progress file in markdown format.
Start with `# Progress:` and include all sections.

## Failure Memory

If you encounter issues, check the "Recent Attempts" section for what was tried previously.
Learn from past failures - do not repeat the same approaches that failed.

## When to Write RALPH_DONE

ONLY write RALPH_DONE when:
- Every task in EVERY phase is marked `[x]`
- You have verified the implementation works
- There are NO incomplete tasks remaining

When in doubt, do NOT write RALPH_DONE - continue with Status: In Progress
```

### Example 2: Adding get_build_prompt

```rust
// src/prompts/defaults.rs - ADD:
pub const BUILD_PROMPT: &str = include_str!("../../prompts/PROMPT_build.md");

pub fn default_build_prompt() -> &'static str {
    BUILD_PROMPT
}

// src/prompts/loader.rs - ADD:
use super::defaults;

pub fn get_build_prompt(config: &Config) -> color_eyre::Result<String> {
    match &config.build_prompt {
        Some(path) => {
            std::fs::read_to_string(path).map_err(|e| {
                color_eyre::eyre::eyre!(
                    "Failed to read build prompt from '{}': {}",
                    path.display(),
                    e
                )
            })
        }
        None => Ok(defaults::default_build_prompt().to_string()),
    }
}

// src/prompts/mod.rs - ADD:
pub use loader::get_build_prompt;
```

### Example 3: Iteration Result Type

```rust
// src/build/iteration.rs

/// Result of a single iteration
pub enum IterationResult {
    /// Iteration completed, continue to next
    Continue {
        /// Number of tasks marked complete this iteration
        tasks_completed: u32,
    },
    /// Build should stop
    Done(DoneReason),
}

impl IterationResult {
    pub fn is_done(&self) -> bool {
        matches!(self, IterationResult::Done(_))
    }
}
```

### Example 4: Logging Iteration

```rust
// src/build/command.rs

fn log_iteration(
    ctx: &mut BuildContext,
    iteration: u32,
    tasks_completed: u32,
) -> Result<(), RslphError> {
    let now = chrono::Utc::now();
    let started = now.format("%Y-%m-%d %H:%M").to_string();

    // Calculate duration (would need to track start time)
    let duration = "~5m"; // Placeholder

    let notes = if tasks_completed == 0 {
        "No tasks completed".to_string()
    } else {
        format!("{} task(s) completed", tasks_completed)
    };

    ctx.progress.log_iteration(
        iteration,
        &started,
        duration,
        tasks_completed,
        &notes,
    );

    ctx.progress.write(&ctx.progress_path)?;

    Ok(())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Bash while-loop with sleep | Rust async state machine | 2025+ | Proper cancellation, timeout, error handling |
| Simple completion string | RALPH_DONE in Status section | Ralph standard | Clear section-specific detection |
| Unbounded iterations | Configurable max (default 20) | Safety standard | Prevents runaway cost |
| Single progress.txt | Structured markdown sections | portableralph | Rich metadata, iteration history |

**Deprecated/outdated:**
- Using `<promise>COMPLETE</promise>` tags: Original Ralph pattern, now standardized as RALPH_DONE
- Storing progress in prd.json: Markdown progress file is current standard

## Open Questions

Things that couldn't be fully resolved:

1. **Claude Response Parsing for Progress Updates**
   - What we know: Claude outputs updated progress file as markdown
   - What's unclear: Exact format for extracting progress file from response vs. other output
   - Recommendation: Expect Claude to output ONLY the progress file markdown, parse entire response

2. **Partial Task Completion**
   - What we know: One task per iteration is the pattern
   - What's unclear: How to handle tasks too large for single iteration
   - Recommendation: Trust Claude to break work into sub-steps, document in Recent Attempts

3. **Concurrent File Modifications**
   - What we know: User might edit progress file during build
   - What's unclear: How to handle conflicts
   - Recommendation: Re-read progress file at start of each iteration, atomic writes handle crash safety

4. **Recent Attempts Format**
   - What we know: Need iteration number, what was tried, result
   - What's unclear: How Claude should structure this in its output
   - Recommendation: Document format in PROMPT_build, parse with existing Attempt struct

## Sources

### Primary (HIGH confidence)
- Existing codebase: `progress.rs`, `subprocess/runner.rs`, `planning/command.rs` - Verified implementation patterns
- [kylemclaren/ralph](https://github.com/kylemclaren/ralph) - Configuration patterns, `--once`, `--dry-run`, max iterations
- [portableralph](https://github.com/aaron777collins/portableralph) - RALPH_DONE marker, progress file format, one task per iteration

### Secondary (MEDIUM confidence)
- [Rust Enum State Machine Patterns](https://dev.to/digclo/state-pattern-with-rust-enums-61g) - State modeling and transition patterns
- [awesomeclaude.ai Ralph Wiggum](https://awesomeclaude.ai/ralph-wiggum) - Completion promise pattern, max iterations
- [PJFP Ralph Guide](https://pjfp.com/what-is-the-ralph-wiggum-loop-in-programming-ultimate-guide-to-ai-powered-iterative-coding/) - Fresh context rationale

### Tertiary (LOW confidence)
- [Rust Async State Machines](https://medium.com/@theopinionatedev/inside-rusts-async-state-machines-what-the-compiler-actually-generates-dd6d0b0723cc) - Background on async patterns

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All libraries already in Cargo.toml, verified working
- Architecture: HIGH - Follows established patterns from Phase 1-3
- State machine: HIGH - Standard Rust enum pattern, well documented
- RALPH_DONE detection: HIGH - Already implemented in ProgressFile::is_done()
- Recent attempts: MEDIUM - Struct exists, accumulation logic needs implementation
- PROMPT_build: MEDIUM - Pattern established, specific content based on research

**Research date:** 2026-01-18
**Valid until:** 60 days (Core patterns stable, prompt content may evolve)

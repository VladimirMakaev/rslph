# Architecture: Eval System and Context Engineering

**Domain:** Eval command, token tracking, and test-driven flow for rslph
**Researched:** 2026-01-20
**Confidence:** HIGH (based on existing codebase analysis)

## Executive Summary

This document defines how the eval command, token tracking, and test-driven flow should integrate with rslph's existing architecture. The design follows existing patterns (commands in `src/commands/` equivalent modules, Claude interaction through `src/subprocess/`, TUI reuse) while introducing minimal new components.

**Key architectural decisions:**
1. Token tracking hooks into existing `StreamEvent` parsing in iteration loop
2. Eval command orchestrates existing plan + build commands, not duplicates them
3. Built-in eval projects embedded via `include_str!` at compile time
4. Test-driven prompts layered on existing prompt system

---

## Current Architecture Reference

### Existing Component Structure

```
src/
+-- main.rs           # CLI entry, dispatches Commands enum
+-- cli.rs            # clap CLI definition, Commands enum
+-- lib.rs            # Module exports
+-- config.rs         # figment-based config
+-- error.rs          # RslphError enum
+-- progress.rs       # ProgressFile markdown parser/writer
|
+-- planning/         # Plan command
|   +-- command.rs    # run_plan_command()
|
+-- build/            # Build command
|   +-- command.rs    # run_build_command()
|   +-- iteration.rs  # run_single_iteration()
|   +-- state.rs      # BuildContext, BuildState, IterationResult
|
+-- subprocess/       # Claude subprocess management
|   +-- runner.rs     # ClaudeRunner (spawn, output, terminate)
|   +-- stream_json.rs # StreamEvent, Usage, StreamResponse
|   +-- output.rs     # OutputLine enum
|   +-- signals.rs    # Ctrl+C handling
|
+-- prompts/          # System prompt management
|   +-- loader.rs     # get_build_prompt(), get_plan_prompt()
|   +-- defaults.rs   # Embedded default prompts
|
+-- tui/              # Terminal UI
|   +-- app.rs        # App state, AppEvent, Message
|   +-- run.rs        # run_tui()
|   +-- event.rs      # EventHandler, SubprocessEvent
|
+-- vcs/              # Git/Sapling integration
```

### Key Integration Points

| Component | Relevance to Eval | Integration Strategy |
|-----------|-------------------|---------------------|
| `build/iteration.rs` | Token tracking point | Hook into `StreamResponse.process_event()` |
| `subprocess/stream_json.rs` | Already parses `Usage` | Extend with accumulator |
| `cli.rs` | Add `Commands::Eval` | Same pattern as Plan/Build |
| `prompts/loader.rs` | Test-driven prompts | Add `get_eval_prompt()` |
| `tui/app.rs` | Eval display | Reuse with `EvalMode` variant |
| `config.rs` | Eval-specific config | Add `eval_*` fields |

---

## Token Tracking Architecture

### Current Token Flow

Currently, tokens are parsed but only displayed transiently in TUI:

```
Claude CLI (stream-json output)
          |
          v
+-----------------------+
| run_single_iteration()|  build/iteration.rs
+-----------------------+
          |
  (line by line JSONL)
          v
+-----------------------+
| StreamEvent::parse()  |  stream_json.rs
+-----------------------+
          |
          v
+-----------------------+
| StreamResponse        |
| .process_event()      |
+-----------------------+
          |
  (Usage extracted once)
          v
+-----------------------+
| TUI display only      |
| (not persisted)       |
+-----------------------+
```

### Proposed Token Tracking Architecture

```
Claude CLI (stream-json output)
          |
          v
+-----------------------+
| run_single_iteration()|
+-----------------------+
          |
          v
+-----------------------+
| StreamEvent::parse()  |
+-----------------------+
          |
          v
    +-----+-----+
    |           |
    v           v
+--------+  +-----------------+
|Response|  |TokenAccumulator |  <-- NEW
+--------+  +-----------------+
                    |
            +-----------------+
            | IterationTokens |  <-- NEW
            | { input, output,|
            |   cache_create, |
            |   cache_read }  |
            +-----------------+
                    |
        +-----------+-----------+
        |           |           |
        v           v           v
+----------+  +---------+  +-----------+
|BuildCtx  |  |EvalResult|  |ProgressFile|
|.tokens   |  |.tokens   |  |.iteration_log|
+----------+  +---------+  |(extended)   |
                          +-----------+
```

### New Types for Token Tracking

**File:** `src/tokens.rs` (NEW)

```rust
/// Token usage for a single API call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

/// Accumulated tokens across an iteration (multiple API calls).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IterationTokens {
    /// Total input tokens for this iteration
    pub total_input: u64,
    /// Total output tokens for this iteration
    pub total_output: u64,
    /// Total cache creation tokens
    pub total_cache_creation: u64,
    /// Total cache read tokens (hits)
    pub total_cache_read: u64,
    /// Number of API calls in this iteration
    pub api_call_count: u32,
}

impl IterationTokens {
    /// Add a single usage event to the accumulator.
    pub fn add_usage(&mut self, usage: &Usage) {
        self.total_input += usage.input_tokens;
        self.total_output += usage.output_tokens;
        self.total_cache_creation += usage.cache_creation_input_tokens.unwrap_or(0);
        self.total_cache_read += usage.cache_read_input_tokens.unwrap_or(0);
        self.api_call_count += 1;
    }

    /// Calculate total tokens (input + output).
    pub fn total(&self) -> u64 {
        self.total_input + self.total_output
    }

    /// Calculate effective input (cache hits reduce effective input).
    pub fn effective_input(&self) -> u64 {
        self.total_input.saturating_sub(self.total_cache_read)
    }
}

/// Complete token statistics for an eval run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvalTokenStats {
    /// Tokens per iteration
    pub by_iteration: Vec<IterationTokens>,
    /// Grand total across all iterations
    pub total: IterationTokens,
    /// Planning phase tokens (if applicable)
    pub planning: Option<IterationTokens>,
}
```

### Integration with Existing Iteration Loop

**Modifications to:** `build/iteration.rs`

```rust
pub async fn run_single_iteration(ctx: &mut BuildContext) -> Result<IterationResult, RslphError> {
    // ... existing code ...

    // NEW: Create iteration token accumulator
    let mut iteration_tokens = IterationTokens::default();

    // In the TUI streaming path:
    if let Some(event) = parse_and_stream_line(s, &tui_tx) {
        stream_response.process_event(&event);

        // NEW: Accumulate token usage
        if let Some(usage) = event.usage() {
            iteration_tokens.add_usage(usage);
        }
    }

    // In the non-streaming path:
    for line in &output {
        if let OutputLine::Stdout(s) = line {
            if let Ok(event) = StreamEvent::parse(s) {
                stream_response.process_event(&event);

                // NEW: Accumulate token usage
                if let Some(usage) = event.usage() {
                    iteration_tokens.add_usage(usage);
                }
            }
        }
    }

    // ... existing code ...

    // NEW: Store iteration tokens in context
    ctx.iteration_tokens.push(iteration_tokens);

    Ok(IterationResult::Continue { tasks_completed })
}
```

### Extended BuildContext

**Modifications to:** `build/state.rs`

```rust
pub struct BuildContext {
    // ... existing fields ...

    /// Token usage per iteration (NEW)
    pub iteration_tokens: Vec<IterationTokens>,
}
```

---

## Eval Command Architecture

### Command Structure

```
rslph eval [OPTIONS] <eval-name>

Arguments:
  <eval-name>       Name of built-in eval OR path to custom eval project

Options:
  --once            Run single iteration only
  --visible-tests   Show hidden tests to Claude (for debugging)
  --no-cleanup      Keep temp directory after completion
  --no-tui          Disable TUI mode
  --json            Output results as JSON
```

### Eval Command Data Flow

```
+------------------+
| rslph eval <name>|
+------------------+
        |
        v
+------------------+
| EvalRunner       |  (NEW: src/eval/runner.rs)
| - create temp dir|
| - extract project|
| - configure tests|
+------------------+
        |
        v
+------------------+
| ProgressFile     |  (generated for eval)
| - hidden tests   |
| - eval tasks     |
+------------------+
        |
        v
+------------------+     +------------------+
| plan command     | --> | build command    |
| (optional)       |     | (with eval mode) |
+------------------+     +------------------+
        |                        |
        +------------------------+
                    |
                    v
          +------------------+
          | EvalResult       |  (NEW: src/eval/result.rs)
          | - tokens used    |
          | - iterations     |
          | - tests passed   |
          | - tests failed   |
          | - time elapsed   |
          +------------------+
                    |
          +---------+---------+
          |                   |
          v                   v
    +----------+       +----------+
    | JSON out |       | TUI out  |
    +----------+       +----------+
```

### New Module Structure

```
src/
+-- eval/                    # NEW MODULE
|   +-- mod.rs               # Module exports
|   +-- command.rs           # run_eval_command()
|   +-- runner.rs            # EvalRunner orchestration
|   +-- result.rs            # EvalResult, EvalStats
|   +-- project.rs           # EvalProject extraction
|   +-- builtin.rs           # Built-in eval registry
|   +-- prompts.rs           # Eval-specific prompt modifications
|
+-- tokens.rs                # NEW: TokenUsage, IterationTokens
```

### CLI Integration

**Modifications to:** `cli.rs`

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    Plan { ... },
    Build { ... },

    /// Run an evaluation project (NEW)
    Eval {
        /// Name of built-in eval or path to custom eval directory
        eval_name: String,

        /// Run single iteration only
        #[arg(long)]
        once: bool,

        /// Expose hidden tests to Claude (debugging)
        #[arg(long)]
        visible_tests: bool,

        /// Keep temp directory after completion
        #[arg(long)]
        no_cleanup: bool,

        /// Disable TUI and use simple output
        #[arg(long)]
        no_tui: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
}
```

### EvalRunner Implementation Pattern

**File:** `src/eval/runner.rs`

```rust
pub struct EvalRunner {
    /// Name of the eval being run
    pub name: String,
    /// Temporary directory for eval execution
    pub temp_dir: TempDir,
    /// Path to progress file within temp dir
    pub progress_path: PathBuf,
    /// Whether tests are visible to Claude
    pub visible_tests: bool,
    /// Application config
    pub config: Config,
    /// Cancellation token
    pub cancel_token: CancellationToken,
}

impl EvalRunner {
    /// Create a new eval runner for a built-in or custom eval.
    pub fn new(
        eval_name: &str,
        visible_tests: bool,
        config: Config,
        cancel_token: CancellationToken,
    ) -> Result<Self, RslphError> {
        // 1. Resolve eval project (built-in or custom path)
        // 2. Create temp directory
        // 3. Extract project files
        // 4. Configure test visibility
        // 5. Generate progress file
    }

    /// Run the eval and collect results.
    pub async fn run(&mut self, once: bool, no_tui: bool) -> Result<EvalResult, RslphError> {
        // 1. Optional: Run plan command for complex evals
        // 2. Run build command with eval-specific prompts
        // 3. Collect token statistics from BuildContext
        // 4. Run final test suite
        // 5. Aggregate results
    }
}
```

---

## Built-in Eval Projects Architecture

### Embedding Strategy

Eval projects are embedded at compile time using `include_str!`. This approach:
- Ensures evals are always available (no external dependencies)
- Simplifies distribution (single binary)
- Allows version control of evals with codebase

**File:** `src/eval/builtin.rs`

```rust
/// Built-in eval project definition.
pub struct BuiltinEval {
    /// Eval identifier (e.g., "fizzbuzz", "cli-todo")
    pub name: &'static str,
    /// Short description
    pub description: &'static str,
    /// Embedded project files
    pub files: &'static [EmbeddedFile],
    /// Hidden test file (run at end, not visible to Claude)
    pub hidden_test: &'static str,
    /// Initial progress file content
    pub progress_template: &'static str,
}

/// An embedded file within an eval project.
pub struct EmbeddedFile {
    pub path: &'static str,
    pub content: &'static str,
}

/// Registry of all built-in evals.
pub static BUILTIN_EVALS: &[BuiltinEval] = &[
    BuiltinEval {
        name: "fizzbuzz",
        description: "Classic FizzBuzz implementation",
        files: &[
            EmbeddedFile {
                path: "Cargo.toml",
                content: include_str!("../../evals/fizzbuzz/Cargo.toml"),
            },
            EmbeddedFile {
                path: "src/lib.rs",
                content: include_str!("../../evals/fizzbuzz/src/lib.rs"),
            },
        ],
        hidden_test: include_str!("../../evals/fizzbuzz/tests/hidden.rs"),
        progress_template: include_str!("../../evals/fizzbuzz/PROGRESS.md"),
    },
    // ... more evals
];

/// Find a built-in eval by name.
pub fn find_builtin(name: &str) -> Option<&'static BuiltinEval> {
    BUILTIN_EVALS.iter().find(|e| e.name == name)
}

/// List all available built-in evals.
pub fn list_builtins() -> impl Iterator<Item = &'static BuiltinEval> {
    BUILTIN_EVALS.iter()
}
```

### Eval Project Directory Structure

```
evals/                       # Source evals (compiled into binary)
+-- fizzbuzz/
|   +-- Cargo.toml           # Project manifest
|   +-- src/
|   |   +-- lib.rs           # Skeleton with TODOs
|   +-- tests/
|   |   +-- public.rs        # Tests visible to Claude
|   |   +-- hidden.rs        # Hidden tests (run at end)
|   +-- PROGRESS.md          # Progress file template
|
+-- cli-todo/
|   +-- ...
```

### Temp Directory Layout (Runtime)

```
/tmp/rslph-eval-XXXXXX/      # Created by EvalRunner
+-- project/                  # Extracted eval project
|   +-- Cargo.toml
|   +-- src/
|   |   +-- lib.rs
|   +-- tests/
|       +-- public.rs         # Always visible
|       +-- hidden.rs         # Only if --visible-tests
|
+-- PROGRESS.md               # Generated from template
+-- .rslph/
    +-- config.toml           # Eval-specific config
```

---

## Test-Driven Flow Architecture

### Prompt Modification Strategy

Test-driven flow modifies the build prompt to emphasize test execution. This is done by layering on the existing prompt system.

**File:** `src/eval/prompts.rs`

```rust
/// Get the eval-specific build prompt.
///
/// This wraps the standard build prompt with additional test-driven instructions.
pub fn get_eval_build_prompt(config: &Config, eval: &EvalRunner) -> color_eyre::Result<String> {
    let base_prompt = get_build_prompt(config)?;

    let test_instructions = r#"
## Test-Driven Development Mode

This is an evaluation run. You MUST follow test-driven development:

1. **Run tests first**: Before any implementation, run `cargo test` to see failing tests
2. **Fix one test at a time**: Focus on making one test pass before moving to the next
3. **Verify after changes**: Run tests after each implementation change
4. **Report test output**: Include test results in your progress updates

### Test Command
```bash
cargo test --test public
```

### Success Criteria
All tests in `tests/public.rs` must pass.
"#;

    Ok(format!("{}\n\n{}", base_prompt, test_instructions))
}
```

### Progress File Modifications for Eval

Eval progress files include test-specific sections:

```markdown
# Progress: FizzBuzz Implementation

## Status

In Progress

## Analysis

Implementing FizzBuzz according to test specifications.

## Tasks

### Phase 1: Core Implementation

- [ ] Implement fizzbuzz(n) function
- [ ] Handle divisibility by 3 (Fizz)
- [ ] Handle divisibility by 5 (Buzz)
- [ ] Handle divisibility by both (FizzBuzz)

## Testing Strategy

Run `cargo test --test public` to validate implementation.

## Test Results

### Latest Run

- Command: `cargo test --test public`
- Result: PENDING
- Tests passed: 0/5

## Completed This Iteration

## Recent Attempts

## Iteration Log

| Iteration | Started | Duration | Tasks Completed | Notes |
|-----------|---------|----------|-----------------|-------|
```

---

## EvalResult Structure

**File:** `src/eval/result.rs`

```rust
use crate::tokens::EvalTokenStats;

/// Complete result of an eval run.
#[derive(Debug, Clone, Serialize)]
pub struct EvalResult {
    /// Name of the eval that was run
    pub eval_name: String,

    /// Whether the eval passed (all hidden tests pass)
    pub passed: bool,

    /// Number of iterations used
    pub iterations: u32,

    /// Token usage statistics
    pub tokens: EvalTokenStats,

    /// Time elapsed
    pub elapsed_secs: f64,

    /// Test results
    pub test_results: TestResults,

    /// Final progress file state
    pub final_progress: String,
}

/// Test execution results.
#[derive(Debug, Clone, Serialize)]
pub struct TestResults {
    /// Public test results
    pub public: TestSuite,

    /// Hidden test results (final validation)
    pub hidden: TestSuite,
}

/// Results for a single test suite.
#[derive(Debug, Clone, Serialize)]
pub struct TestSuite {
    pub passed: u32,
    pub failed: u32,
    pub total: u32,
    /// Individual test case results
    pub cases: Vec<TestCase>,
}

/// A single test case result.
#[derive(Debug, Clone, Serialize)]
pub struct TestCase {
    pub name: String,
    pub passed: bool,
    pub output: Option<String>,
}
```

---

## Component Dependencies and Build Order

### Dependency Graph

```
tokens.rs
    ^
    |
    +-- build/iteration.rs (uses IterationTokens)
    |
    +-- build/state.rs (stores iteration_tokens)
    |
    +-- eval/result.rs (uses EvalTokenStats)

eval/builtin.rs
    ^
    |
    +-- eval/project.rs (extracts BuiltinEval)

eval/project.rs
    ^
    |
    +-- eval/runner.rs (uses EvalProject)

eval/prompts.rs
    ^
    |
    +-- eval/runner.rs (uses get_eval_build_prompt)

eval/runner.rs
    ^
    |
    +-- eval/command.rs (calls EvalRunner::run)

eval/command.rs
    ^
    |
    +-- cli.rs (dispatches Commands::Eval)
    |
    +-- main.rs (matches Commands::Eval)
```

### Suggested Build Order (Phases)

**Phase 1: Token Tracking Foundation**
1. Create `src/tokens.rs` with `TokenUsage`, `IterationTokens`
2. Modify `build/iteration.rs` to accumulate tokens
3. Modify `build/state.rs` to store `iteration_tokens`
4. Add token display to TUI (optional, can defer)

**Phase 2: Eval Infrastructure**
1. Create `src/eval/mod.rs` module structure
2. Create `src/eval/result.rs` with `EvalResult`
3. Create `src/eval/project.rs` for project extraction
4. Create `evals/` directory with first eval (fizzbuzz)

**Phase 3: Eval Command Core**
1. Create `src/eval/builtin.rs` with embedding
2. Create `src/eval/runner.rs` orchestration
3. Create `src/eval/command.rs` entry point
4. Add `Commands::Eval` to `cli.rs`
5. Update `main.rs` dispatch

**Phase 4: Test-Driven Flow**
1. Create `src/eval/prompts.rs`
2. Modify progress file format for test results
3. Add test execution to iteration loop
4. Add hidden test final validation

**Phase 5: TUI Integration**
1. Add eval mode to TUI
2. Display token statistics
3. Display test results in real-time

---

## Integration Patterns to Follow

### Pattern 1: Command Entry Point

Follow the `run_build_command` pattern:

```rust
// src/eval/command.rs

pub async fn run_eval_command(
    eval_name: String,
    once: bool,
    visible_tests: bool,
    no_cleanup: bool,
    no_tui: bool,
    json_output: bool,
    config: &Config,
    cancel_token: CancellationToken,
) -> color_eyre::Result<()> {
    // Create runner
    let mut runner = EvalRunner::new(
        &eval_name,
        visible_tests,
        config.clone(),
        cancel_token,
    )?;

    // Run eval
    let result = runner.run(once, no_tui).await?;

    // Output results
    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print_eval_summary(&result);
    }

    // Cleanup (unless --no-cleanup)
    if !no_cleanup {
        // TempDir drops automatically
    } else {
        std::mem::forget(runner.temp_dir); // Keep temp dir
        println!("Eval directory preserved at: {}", runner.working_dir().display());
    }

    Ok(())
}
```

### Pattern 2: Reuse SubprocessEvent for TUI

The eval can use the same TUI infrastructure:

```rust
use crate::tui::SubprocessEvent;

// The EvalRunner can use the same TUI infrastructure
// by creating a build command with the eval's progress file
```

### Pattern 3: Config Extension

Follow existing config pattern:

```rust
// Additions to config.rs

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    // ... existing fields ...

    /// Default eval to run when none specified
    #[serde(default)]
    pub default_eval: Option<String>,

    /// Eval-specific prompt override
    #[serde(default)]
    pub eval_prompt: Option<PathBuf>,
}
```

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Duplicating Build Loop

**Wrong approach:**
```rust
// DON'T create a separate eval loop
impl EvalRunner {
    async fn run_eval_loop(&mut self) {
        // Duplicates build/command.rs logic
    }
}
```

**Correct approach:**
```rust
// DO reuse existing build command
impl EvalRunner {
    async fn run(&mut self) -> Result<EvalResult> {
        // Configure eval-specific settings
        let config = self.eval_config();

        // Reuse existing build command
        run_build_command(
            self.progress_path.clone(),
            once,
            false, // not dry_run
            no_tui,
            &config,
            self.cancel_token.clone(),
        ).await?;

        // Post-process: run hidden tests, collect results
        self.finalize()
    }
}
```

### Anti-Pattern 2: Scattered Token Logic

**Wrong approach:**
```rust
// DON'T put token tracking in multiple places
// stream_json.rs
impl StreamResponse {
    fn process_event(&mut self, event: &StreamEvent) {
        self.total_tokens += ...; // Here
    }
}

// iteration.rs
fn run_single_iteration() {
    // And also here
    ctx.tokens += stream_response.total_tokens;
}
```

**Correct approach:**
```rust
// DO centralize token tracking
// Single accumulator in iteration loop
let mut iteration_tokens = IterationTokens::default();

for event in events {
    stream_response.process_event(&event);
    if let Some(usage) = event.usage() {
        iteration_tokens.add_usage(usage);
    }
}

ctx.iteration_tokens.push(iteration_tokens);
```

### Anti-Pattern 3: Hardcoded Eval Paths

**Wrong approach:**
```rust
// DON'T hardcode paths
let eval_path = "/usr/share/rslph/evals/fizzbuzz";
```

**Correct approach:**
```rust
// DO embed evals at compile time
static FIZZBUZZ: BuiltinEval = BuiltinEval {
    files: &[
        EmbeddedFile {
            path: "src/lib.rs",
            content: include_str!("../../evals/fizzbuzz/src/lib.rs"),
        },
    ],
    // ...
};
```

---

## Scalability Considerations

| Concern | At 5 Evals | At 20 Evals | At 100+ Evals |
|---------|------------|-------------|---------------|
| Binary size | +500KB | +2MB | Consider external repo |
| Compile time | +5s | +15s | Conditional compilation |
| Eval discovery | Static list | Static list | Plugin system |

For MVP (5-10 evals), embedding is appropriate. At scale, consider:
- External eval repository with download on first use
- Plugin system for community evals
- Lazy loading from filesystem

---

## Summary: Files to Create/Modify

### New Files

| File | Purpose |
|------|---------|
| `src/tokens.rs` | Token tracking types |
| `src/eval/mod.rs` | Eval module root |
| `src/eval/command.rs` | `run_eval_command()` |
| `src/eval/runner.rs` | `EvalRunner` orchestration |
| `src/eval/result.rs` | `EvalResult`, `TestResults` |
| `src/eval/project.rs` | Project extraction |
| `src/eval/builtin.rs` | Built-in eval registry |
| `src/eval/prompts.rs` | Eval-specific prompts |
| `evals/fizzbuzz/*` | First built-in eval |

### Files to Modify

| File | Changes |
|------|---------|
| `src/lib.rs` | Add `pub mod eval;` and `pub mod tokens;` |
| `src/cli.rs` | Add `Commands::Eval` |
| `src/main.rs` | Match `Commands::Eval` |
| `src/config.rs` | Add eval config fields |
| `src/build/iteration.rs` | Token accumulation |
| `src/build/state.rs` | Add `iteration_tokens` field |

---

## Sources

- Codebase analysis: `/Users/vmakaev/NonWork/rslph/src/`
- Existing patterns: `build/command.rs`, `subprocess/stream_json.rs`
- TUI integration: `tui/app.rs`, `tui/event.rs`
- Test infrastructure: `tests/fake_claude.rs`, `tests/e2e/`

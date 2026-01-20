# Phase 9: Eval Command Foundation - Research

**Researched:** 2026-01-20
**Domain:** CLI command orchestration, temp directory isolation, metrics aggregation
**Confidence:** HIGH

## Summary

Phase 9 implements the foundation for a controlled evaluation command that orchestrates existing `plan` and `build` commands within isolated temporary directories. The research confirms that rslph already has 90% of the required infrastructure:

1. **Existing token tracking:** Phase 8 implemented `TokenUsage` and `IterationTokens` in `build/tokens.rs`, with accumulation in `build/iteration.rs` lines 263-274
2. **Existing temp directory patterns:** The `tempfile` crate is already a dev-dependency with extensive usage patterns in `tests/e2e/fixtures.rs`
3. **Existing command orchestration:** Both `run_plan_command` and `run_build_command` are public async functions that accept paths and configs

**Primary recommendation:** Create an `eval` subcommand that wraps existing commands, promoting `tempfile` from dev-dependency to regular dependency. The eval command creates a `TempDir`, copies/initializes project files, runs plan+build, and aggregates the token/timing metrics already tracked by `BuildContext`.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tempfile | 3.x | Temp directory creation and cleanup | Already in dev-deps, RAII pattern, cross-platform, used in 30+ locations |
| std::time::Instant | stdlib | Execution timing | Already used in `build/state.rs` for `iteration_start` |
| clap | 4.5 | Subcommand definition | Already in deps, consistent with existing commands |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio-util | 0.7 | CancellationToken | Already used for graceful shutdown in plan/build |
| human_format | 1.2 | Token display formatting | Already integrated in Phase 8 for SI suffixes |
| serde_json | 1.0 | Results output (future --json flag) | Already in deps, deferred to later phase |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tempfile::TempDir | std::env::temp_dir + manual cleanup | tempfile handles RAII, unique names, and cross-platform; manual is error-prone |
| Embedding project via include_str! | Runtime file loading | Embedding deferred to Phase 10+; Phase 9 uses path-based projects |

**Installation:**
```toml
# Move from [dev-dependencies] to [dependencies]
[dependencies]
tempfile = "3"

# Remove from dev-dependencies (will still work for tests)
[dev-dependencies]
# tempfile = "3"  # REMOVED - now in regular deps
```

## Architecture Patterns

### Recommended Project Structure
```
src/
+-- eval/
|   +-- mod.rs           # Module exports, EvalResult struct
|   +-- command.rs       # run_eval_command() entry point
+-- build/
|   +-- tokens.rs        # EXISTING: TokenUsage, IterationTokens, format_tokens
|   +-- state.rs         # EXISTING: BuildContext with total_tokens field
+-- cli.rs               # ADD: Commands::Eval variant
+-- main.rs              # ADD: match Commands::Eval
```

### Pattern 1: Command Orchestration via Composition
**What:** The eval command composes existing plan and build commands rather than duplicating logic
**When to use:** When building a higher-level command that coordinates existing commands
**Example:**
```rust
// Source: Existing pattern from run_build_command signature
pub async fn run_eval_command(
    project: String,
    config: &Config,
    cancel_token: CancellationToken,
) -> color_eyre::Result<EvalResult> {
    // 1. Create isolated workspace
    let workspace = TempDir::with_prefix(&format!("rslph-eval-{}-", project))?;
    let working_dir = workspace.path();

    // 2. Copy project files to workspace
    copy_project_files(&project, working_dir)?;

    // 3. Run plan command (produces progress.md)
    let progress_path = run_plan_command(
        "Build the project per the existing files",
        false,  // not adaptive
        config,
        working_dir,
        cancel_token.clone(),
        Duration::from_secs(600),
    ).await?;

    // 4. Run build command (executes tasks)
    let build_result = run_build_with_metrics(
        progress_path,
        false,  // not once mode
        false,  // not dry run
        true,   // no_tui for eval
        config,
        cancel_token,
    ).await?;

    // 5. Aggregate and return results
    Ok(EvalResult {
        project,
        elapsed_secs: start.elapsed().as_secs_f64(),
        total_tokens: build_result.total_tokens,
        iterations: build_result.iterations,
    })
}
```

### Pattern 2: TempDir with Prefix for Debugging
**What:** Use `TempDir::with_prefix()` to create identifiable temp directories
**When to use:** When temp directories might need manual inspection
**Example:**
```rust
// Source: tempfile crate documentation pattern
use tempfile::TempDir;

// Creates: /tmp/rslph-eval-calculator-XXXXXX/
let workspace = TempDir::with_prefix(&format!("rslph-eval-{}-", project_name))?;
let path = workspace.path();  // Use during eval

// For --keep-workspace flag:
if keep_workspace {
    let preserved = workspace.into_path();  // Disables RAII cleanup
    println!("Workspace preserved at: {}", preserved.display());
}
// Without into_path(), directory is deleted on drop
```

### Pattern 3: Token Aggregation from Existing BuildContext
**What:** Extract token totals from BuildContext after build completes
**When to use:** When eval needs access to metrics collected during build
**Example:**
```rust
// Source: Existing BuildContext fields from build/state.rs lines 114-118
// These fields are already populated during build execution

/// After run_build_command completes, access metrics from BuildContext:
pub struct EvalMetrics {
    pub total_tokens: TokenUsage,      // From ctx.total_tokens
    pub iteration_tokens: Vec<IterationTokens>,  // From ctx.iteration_tokens
    pub elapsed_secs: f64,             // From Instant::now() at start
    pub iterations: u32,               // Length of iteration_tokens
}
```

### Anti-Patterns to Avoid
- **Duplicating build loop:** Do NOT create a separate iteration loop in eval; reuse `run_build_command`
- **Manual temp cleanup:** Do NOT use `std::fs::remove_dir_all`; rely on TempDir RAII
- **Scattered timing:** Do NOT track time in multiple places; single `Instant::now()` at eval start

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Temp directory creation | `std::fs::create_dir` + random names | `TempDir::with_prefix()` | Handles uniqueness, cleanup, cross-platform |
| Temp cleanup | Manual `remove_dir_all` in finally block | TempDir drop | RAII is exception-safe, handles panics |
| Token accumulation | Custom counter in eval | `BuildContext.total_tokens` | Already implemented in Phase 8, tested |
| Time measurement | chrono or custom Duration | `std::time::Instant::elapsed()` | Already pattern in codebase, monotonic |
| Command parsing | Manual arg parsing | clap derive macros | Consistent with Plan/Build commands |

**Key insight:** Phase 9 should wire together existing components, not build new low-level infrastructure. The token tracking, timing, and subprocess management are all already implemented.

## Common Pitfalls

### Pitfall 1: Token Accumulation Bug (Phase 8 Issue)
**What goes wrong:** Token counts reset instead of accumulate across iterations
**Why it happens:** Using `=` instead of `+=` for token assignment
**How to avoid:** The fix is tracked in Phase 8 UAT; ensure `BuildContext.total_tokens` uses `+=` in iteration.rs (verified: lines 271-274 already use `+=`)
**Warning signs:** Token counts "jumping" or showing only last iteration's values

### Pitfall 2: Orphaned Temp Directories on Panic
**What goes wrong:** Temp directories left behind after errors
**Why it happens:** Early return or panic before TempDir goes out of scope
**How to avoid:** Let TempDir RAII handle cleanup; use `?` operator consistently
**Warning signs:** `/tmp/rslph-eval-*` directories accumulating

### Pitfall 3: Working Directory Confusion
**What goes wrong:** Commands run in wrong directory, files not found
**Why it happens:** `run_plan_command` and `run_build_command` both take `working_dir` parameter
**How to avoid:** Always pass `workspace.path()` as working directory, use absolute paths
**Warning signs:** "file not found" errors, git init in wrong location

### Pitfall 4: Cancellation Not Propagating
**What goes wrong:** Ctrl+C doesn't stop nested plan/build commands
**Why it happens:** CancellationToken not passed through command chain
**How to avoid:** Clone and pass `cancel_token` to each nested command call
**Warning signs:** Subprocess continues after Ctrl+C, orphan processes

### Pitfall 5: Progress File Path Mismatch
**What goes wrong:** Build command can't find progress file created by plan
**Why it happens:** Plan returns relative path, build expects absolute
**How to avoid:** Use `working_dir.join("progress.md")` or return absolute path from plan
**Warning signs:** "Failed to load progress file" immediately after successful plan

## Code Examples

Verified patterns from the existing codebase:

### CLI Subcommand Definition
```rust
// Source: src/cli.rs existing pattern
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing Plan and Build ...

    /// Run evaluation in isolated environment (EVAL-01)
    Eval {
        /// Project directory or name to evaluate
        project: String,

        /// Keep temp directory after completion
        #[arg(long)]
        keep: bool,

        /// Disable TUI output
        #[arg(long)]
        no_tui: bool,
    },
}
```

### Main Dispatch
```rust
// Source: src/main.rs existing pattern
Commands::Eval { project, keep, no_tui } => {
    let cancel_token = setup_ctrl_c_handler();

    match run_eval_command(project, keep, no_tui, &config, cancel_token).await {
        Ok(result) => {
            println!("Eval complete!");
            println!("Time: {:.1}s", result.elapsed_secs);
            println!("Iterations: {}", result.iterations);
            println!(
                "Tokens: In: {} | Out: {} | CacheW: {} | CacheR: {}",
                format_tokens(result.total_tokens.input_tokens),
                format_tokens(result.total_tokens.output_tokens),
                format_tokens(result.total_tokens.cache_creation_input_tokens),
                format_tokens(result.total_tokens.cache_read_input_tokens),
            );
        }
        Err(e) => {
            eprintln!("Eval failed: {}", e);
            std::process::exit(1);
        }
    }
}
```

### Eval Result Struct
```rust
// Source: New, based on existing TokenUsage from build/tokens.rs
use crate::build::tokens::TokenUsage;

/// Result of an eval run (EVAL-04, EVAL-05)
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
```

### TempDir Setup with Prefix
```rust
// Source: Pattern from existing tests and tempfile docs
use tempfile::TempDir;
use std::path::Path;

fn setup_eval_workspace(project: &str) -> std::io::Result<TempDir> {
    // Creates e.g., /tmp/rslph-eval-calculator-a1b2c3/
    TempDir::with_prefix(&format!("rslph-eval-{}-", project))
}

fn preserve_workspace(workspace: TempDir) -> PathBuf {
    // Converts TempDir to PathBuf, disabling automatic cleanup
    workspace.into_path()
}
```

### Timing Pattern
```rust
// Source: Existing pattern from build/command.rs
use std::time::Instant;

let start = Instant::now();

// ... run plan and build ...

let elapsed_secs = start.elapsed().as_secs_f64();
println!("Total time: {:.1}s", elapsed_secs);
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual temp dirs | tempfile crate RAII | Standard practice | Automatic cleanup, no leaks |
| Separate eval loop | Compose existing commands | Phase 9 design | Code reuse, single source of truth |
| Per-command metrics | Centralized in BuildContext | Phase 8 | Token tracking already wired |

**Deprecated/outdated:**
- **Manual temp cleanup:** Use RAII; `remove_dir_all` in finally blocks is fragile
- **Building parallel eval infrastructure:** Reuse plan+build; don't duplicate

## Open Questions

Things that couldn't be fully resolved:

1. **Token aggregation across plan+build**
   - What we know: BuildContext tracks tokens for build iterations (Phase 8)
   - What's unclear: Plan command's token usage is printed but not returned programmatically
   - Recommendation: For Phase 9, modify `run_plan_command` to return `(PathBuf, TokenUsage)` tuple, or track separately and sum

2. **TempDir::keep() vs into_path()**
   - What we know: Prior research mentions `.keep()`, current tempfile docs show `into_path()`
   - What's unclear: API may have changed; need to verify at implementation time
   - Recommendation: Use `into_path()` which is the current documented method

3. **Built-in project embedding (Phase 10+)**
   - What we know: Phase 9 uses path-based projects; embedding deferred
   - What's unclear: Exact include_str! structure for embedded projects
   - Recommendation: Phase 9 focuses on path-based projects; embedding is Phase 10 scope

## Sources

### Primary (HIGH confidence)
- `/Users/vmakaev/NonWork/rslph/src/build/iteration.rs` - Token accumulation implementation (lines 263-274)
- `/Users/vmakaev/NonWork/rslph/src/build/state.rs` - BuildContext with token fields (lines 114-118)
- `/Users/vmakaev/NonWork/rslph/src/build/tokens.rs` - TokenUsage and IterationTokens types
- `/Users/vmakaev/NonWork/rslph/src/cli.rs` - Existing command pattern
- `/Users/vmakaev/NonWork/rslph/tests/e2e/fixtures.rs` - WorkspaceBuilder pattern with TempDir
- `/Users/vmakaev/NonWork/rslph/.planning/research/STACK-v1.2.md` - Prior stack research
- `/Users/vmakaev/NonWork/rslph/.planning/research/ARCHITECTURE-EVAL.md` - Eval architecture design

### Secondary (MEDIUM confidence)
- [tempfile crate docs](https://docs.rs/tempfile/3.24.0/tempfile/) - TempDir API
- [std::time::Instant](https://doc.rust-lang.org/std/time/struct.Instant.html) - Timing primitives
- [SWE-bench Agent Harness](https://github.com/augmentcode/augment-swebench-agent) - Industry patterns for isolated eval

### Tertiary (LOW confidence)
- WebSearch results on benchmark architectures - General patterns, not Rust-specific

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All libraries already in use in codebase
- Architecture: HIGH - Pattern validated by existing plan/build composition
- Pitfalls: HIGH - Based on actual Phase 8 UAT issues and existing code patterns
- Token aggregation: MEDIUM - Plan command return value may need modification

**Research date:** 2026-01-20
**Valid until:** 2026-02-20 (stable crates, well-established patterns)

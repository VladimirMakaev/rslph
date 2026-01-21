# Phase 12: Multi-Trial Results - Research

**Researched:** 2026-01-21
**Domain:** CLI extension, statistics computation, JSON storage/comparison
**Confidence:** HIGH

## Summary

This phase adds multi-trial support to the eval command, enabling users to run multiple independent trials and see statistical summaries. The requirements are straightforward:

1. **EVAL-06:** Add `--trials N` flag to `rslph eval` command
2. **EVAL-07:** Compute and display mean, variance, min, max across trials
3. **EVAL-08:** Store aggregated results in timestamped JSON file
4. **EVAL-09:** Add `rslph eval compare` subcommand to diff two result files

The existing codebase has all the foundations in place:
- `EvalResult` struct captures single-trial data (tokens, time, test results)
- JSON serialization already works (`save_result_json`, `load_result_json`)
- clap CLI parsing with derive macros for command/subcommand structure
- Chrono for timestamps

**Primary recommendation:** Add `--trials` flag to existing `Eval` command and create a separate `Compare` top-level command (simpler than nested subcommands). Implement simple statistics manually (avoid adding dependencies for 10 lines of code).

## Standard Stack

The established libraries/tools for this domain:

### Core (Already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.5 | CLI argument parsing | Already used, has derive macros |
| serde/serde_json | 1.0 | JSON serialization | Already used for result.json |
| chrono | 0.4.43 | Timestamps for filenames | Already used for workspace naming |

### Supporting (No additions needed)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std | - | Statistics (mean/variance) | Simple formulas, no crate needed |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Manual stats | `iterstats` crate | Crate is overkill for mean/variance/min/max |
| Manual stats | `statrs` crate | Much larger dependency, statistical functions we don't need |
| Top-level Compare | Nested eval subcommand | Nested subcommands require restructuring Commands enum |

**Installation:**
No new dependencies required.

## Architecture Patterns

### Recommended Project Structure
```
src/eval/
├── mod.rs           # Add MultiTrialResult export
├── command.rs       # Extend run_eval_command, add run_compare_command
├── test_runner.rs   # No changes needed
├── projects.rs      # No changes needed
└── statistics.rs    # NEW: TrialStatistics, compute_stats()
```

### Pattern 1: Multi-Trial Wrapper
**What:** Loop over single-trial execution, collect results, aggregate
**When to use:** Adding trials to existing eval command
**Example:**
```rust
// Source: Derived from existing run_eval_command pattern
pub async fn run_eval_command(
    project: String,
    trials: u32,  // NEW parameter
    // ... other params
) -> color_eyre::Result<MultiTrialResult> {
    let mut trial_results = Vec::with_capacity(trials as usize);

    for trial_num in 1..=trials {
        println!("\n=== TRIAL {}/{} ===\n", trial_num, trials);

        // Execute single trial (existing logic, modified workspace naming)
        let result = run_single_trial(&project, trial_num, /* ... */).await?;
        trial_results.push(result);
    }

    // Aggregate statistics
    let stats = compute_statistics(&trial_results);

    // Save multi-trial result
    let multi_result = MultiTrialResult {
        project: project.clone(),
        trials: trial_results,
        statistics: stats,
        timestamp: Utc::now(),
    };

    save_multi_trial_result(&multi_result)?;

    Ok(multi_result)
}
```

### Pattern 2: Statistics Computation
**What:** Simple formulas for mean, variance, min, max
**When to use:** Computing trial statistics
**Example:**
```rust
// Source: Standard statistical formulas
pub struct TrialStatistics {
    pub pass_rate: StatSummary,
    pub elapsed_secs: StatSummary,
    pub total_input_tokens: StatSummary,
    pub total_output_tokens: StatSummary,
    pub iterations: StatSummary,
}

pub struct StatSummary {
    pub mean: f64,
    pub variance: f64,
    pub min: f64,
    pub max: f64,
    pub count: usize,
}

impl StatSummary {
    pub fn from_values(values: &[f64]) -> Self {
        let count = values.len();
        if count == 0 {
            return Self { mean: 0.0, variance: 0.0, min: 0.0, max: 0.0, count: 0 };
        }

        let sum: f64 = values.iter().sum();
        let mean = sum / count as f64;

        let variance = if count > 1 {
            let sq_diff_sum: f64 = values.iter()
                .map(|v| (v - mean).powi(2))
                .sum();
            sq_diff_sum / (count - 1) as f64  // Sample variance (Bessel's correction)
        } else {
            0.0
        };

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        Self { mean, variance, min, max, count }
    }

    pub fn std_dev(&self) -> f64 {
        self.variance.sqrt()
    }
}
```

### Pattern 3: Result Comparison
**What:** Load two JSON files, compute deltas, display diff
**When to use:** `rslph compare` command
**Example:**
```rust
// Source: Derived from existing load_result_json pattern
pub fn run_compare_command(
    file1: PathBuf,
    file2: PathBuf,
) -> color_eyre::Result<()> {
    let result1 = load_multi_trial_result(&file1)?;
    let result2 = load_multi_trial_result(&file2)?;

    println!("Comparing results:");
    println!("  File 1: {} ({} trials)", file1.display(), result1.trials.len());
    println!("  File 2: {} ({} trials)", file2.display(), result2.trials.len());
    println!();

    // Pass rate delta
    let pr1 = result1.statistics.pass_rate.mean;
    let pr2 = result2.statistics.pass_rate.mean;
    let delta = pr2 - pr1;
    let arrow = if delta > 0.0 { "^" } else if delta < 0.0 { "v" } else { "=" };
    println!("Pass Rate: {:.1}% -> {:.1}% ({}{:.1}%)",
        pr1, pr2, arrow, delta.abs());

    // Token consumption delta
    // ... similar pattern

    // Execution time delta
    // ... similar pattern

    Ok(())
}
```

### Pattern 4: CLI Subcommand for Compare
**What:** Add Compare as top-level command (simpler than nested)
**When to use:** Adding the compare functionality
**Example:**
```rust
// Source: Existing cli.rs patterns
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands ...

    /// Compare two eval result files (EVAL-09)
    Compare {
        /// First result file to compare
        file1: PathBuf,

        /// Second result file to compare
        file2: PathBuf,
    },
}
```

Alternative (nested under Eval - more complex):
```rust
/// Run evaluation in isolated environment (EVAL-01)
Eval {
    #[command(subcommand)]
    command: EvalSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum EvalSubcommand {
    /// Run evaluation project
    Run {
        #[arg(required_unless_present = "list")]
        project: Option<String>,
        #[arg(long)]
        keep: bool,
        #[arg(long)]
        no_tui: bool,
        #[arg(long)]
        list: bool,
        #[arg(long, default_value = "1")]
        trials: u32,
    },
    /// Compare two result files
    Compare {
        file1: PathBuf,
        file2: PathBuf,
    },
}
```

**Recommendation:** Use top-level `Compare` command for simplicity. The existing `Eval` command has arguments that would require refactoring into a `Run` subcommand. This is lower-risk and achieves the same user outcome.

### Anti-Patterns to Avoid
- **Over-engineering statistics:** Don't use statistical libraries for 4 basic formulas
- **Nested workspace naming conflicts:** Each trial needs unique workspace - use trial number suffix
- **Blocking on all trials:** Show progress after each trial, not just at the end
- **Ignoring failed trials:** A trial that errors should still be recorded (with error state)

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Timestamp formatting | String manipulation | `chrono::Utc::now().format()` | Already used, handles edge cases |
| JSON pretty-printing | Manual indentation | `serde_json::to_string_pretty` | Already used, consistent output |
| CLI argument validation | Manual if/else | clap's `#[arg(value_parser)]` | Built-in validation, better errors |

**Key insight:** All needed utilities already exist in the codebase. This phase is primarily about orchestration and display.

## Common Pitfalls

### Pitfall 1: Trial Workspace Collisions
**What goes wrong:** Multiple trials create workspaces with same timestamp (within same second)
**Why it happens:** Chrono timestamp has second resolution, trials run faster than 1 second gap
**How to avoid:** Include trial number in workspace name: `{project}-{timestamp}-trial{N}`
**Warning signs:** "Directory already exists" errors, overwritten results

### Pitfall 2: Variance of Single Trial
**What goes wrong:** Division by zero or misleading variance when trials=1
**Why it happens:** Sample variance formula divides by (n-1)
**How to avoid:** Return 0 variance for single trial, or skip variance display
**Warning signs:** NaN or Infinity in output

### Pitfall 3: Inconsistent JSON Schema
**What goes wrong:** Old single-trial result.json incompatible with new multi-trial format
**Why it happens:** Changed schema breaks existing tooling
**How to avoid:**
1. Keep existing per-workspace `result.json` unchanged
2. Create new `multi-trial-results-{project}-{date}.json` at eval_dir level
**Warning signs:** JSON parse errors on old files

### Pitfall 4: Token Statistics Aggregation
**What goes wrong:** Aggregating cache tokens across trials is misleading
**Why it happens:** Cache behavior varies per-trial (first trial cold, subsequent may hit cache)
**How to avoid:**
1. Report total tokens (input + output) as primary metric
2. Show cache stats as supplementary info with caveat
3. Consider excluding cache tokens from comparison
**Warning signs:** Wildly different cache rates across "identical" trials

### Pitfall 5: Compare Command File Validation
**What goes wrong:** Cryptic errors when comparing incompatible files
**Why it happens:** Users might compare single-trial vs multi-trial, or wrong JSON format
**How to avoid:**
1. Check file exists before loading
2. Validate JSON schema with informative errors
3. Handle version mismatch gracefully
**Warning signs:** "Cannot deserialize" errors without context

## Code Examples

Verified patterns from existing codebase:

### Adding CLI Flag with Default Value
```rust
// Source: src/cli.rs existing patterns
/// Run evaluation in isolated environment (EVAL-01)
Eval {
    /// Project directory or name to evaluate
    #[arg(required_unless_present = "list")]
    project: Option<String>,

    /// Number of trials to run
    #[arg(long, default_value = "1")]
    trials: u32,

    // ... other fields
}
```

### Timestamp-based Filename
```rust
// Source: src/eval/command.rs:79-80
let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
let result_filename = format!("eval-results-{}-{}.json", project_name, timestamp);
```

### JSON Serialization with Serde
```rust
// Source: src/eval/command.rs:329-350
#[derive(Debug, Serialize)]
struct MultiTrialResult {
    project: String,
    timestamp: String,
    trial_count: u32,
    trials: Vec<TrialSummary>,
    statistics: TrialStatistics,
}

#[derive(Debug, Serialize)]
struct TrialSummary {
    trial_num: u32,
    elapsed_secs: f64,
    iterations: u32,
    tokens: SerializableTokens,
    test_results: Option<SerializableTestResults>,
    workspace_path: String,
}

fn save_multi_trial_result(
    eval_dir: &Path,
    result: &MultiTrialResult,
) -> color_eyre::Result<PathBuf> {
    let filename = format!(
        "eval-results-{}-{}.json",
        result.project,
        Utc::now().format("%Y-%m-%d")
    );
    let path = eval_dir.join(&filename);
    let json = serde_json::to_string_pretty(&result)?;
    std::fs::write(&path, json)?;
    Ok(path)
}
```

### Loading JSON with Error Context
```rust
// Source: src/eval/command.rs:270-274
fn load_multi_trial_result(path: &Path) -> color_eyre::Result<MultiTrialResult> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| eyre!("Failed to read {}: {}", path.display(), e))?;
    let result: MultiTrialResult = serde_json::from_str(&content)
        .map_err(|e| eyre!("Invalid JSON in {}: {}", path.display(), e))?;
    Ok(result)
}
```

### Formatted Output for Statistics
```rust
// Source: Derived from src/main.rs:104-125 output patterns
fn print_statistics(stats: &TrialStatistics, trial_count: u32) {
    println!("\n=== STATISTICAL SUMMARY ({} trials) ===\n", trial_count);

    println!("Pass Rate:");
    println!("  Mean:     {:.1}%", stats.pass_rate.mean);
    println!("  Std Dev:  {:.1}%", stats.pass_rate.std_dev());
    println!("  Min:      {:.1}%", stats.pass_rate.min);
    println!("  Max:      {:.1}%", stats.pass_rate.max);

    println!("\nExecution Time:");
    println!("  Mean:     {:.1}s", stats.elapsed_secs.mean);
    println!("  Std Dev:  {:.1}s", stats.elapsed_secs.std_dev());
    println!("  Min:      {:.1}s", stats.elapsed_secs.min);
    println!("  Max:      {:.1}s", stats.elapsed_secs.max);

    println!("\nToken Usage (total):");
    println!("  Mean:     {} in / {} out",
        format_tokens(stats.total_input_tokens.mean as u64),
        format_tokens(stats.total_output_tokens.mean as u64));
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single trial eval | Multi-trial with stats | This phase | More reliable benchmarking |
| Manual result comparison | CLI comparison command | This phase | Easier A/B testing |

**Deprecated/outdated:**
- None for this phase - building on existing foundations

## Open Questions

Things that couldn't be fully resolved:

1. **Should failed trials be included in statistics?**
   - What we know: A trial might error (timeout, Claude error, etc.)
   - What's unclear: Count as 0% pass rate or exclude from stats?
   - Recommendation: Record failed trials separately, exclude from pass rate stats but include in execution time stats

2. **Multi-trial result file location**
   - What we know: Individual `result.json` goes in workspace dir
   - What's unclear: Should aggregated multi-trial result go in workspace or eval_dir?
   - Recommendation: Store in `config.eval_dir` (parent of all workspaces) with timestamped name

3. **Compare command file format tolerance**
   - What we know: Need to compare results from different runs
   - What's unclear: Support comparing single-trial vs multi-trial?
   - Recommendation: Only support multi-trial format for compare; user can convert old results or re-run with `--trials 1`

## Sources

### Primary (HIGH confidence)
- Existing codebase: `src/eval/command.rs`, `src/eval/mod.rs` - Current eval implementation
- Existing codebase: `src/cli.rs` - CLI patterns with clap derive
- Existing codebase: `src/build/tokens.rs` - TokenUsage struct patterns
- [Rust doc: test::stats](https://doc.rust-lang.org/test/stats/trait.Stats.html) - Reference for statistics formulas (unstable, not used directly)

### Secondary (MEDIUM confidence)
- [iterstats crate](https://lib.rs/crates/iterstats) - Alternative for statistics (not recommended)
- [clap subcommands](https://rust.code-maven.com/clap-subcommand) - Nested subcommand patterns

### Tertiary (LOW confidence)
- None - all patterns verified in codebase

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - No new dependencies, all libraries already in use
- Architecture: HIGH - Direct extension of existing patterns
- Pitfalls: HIGH - Based on codebase analysis and prior phase research
- Statistics implementation: HIGH - Standard mathematical formulas

**Research date:** 2026-01-21
**Valid until:** 90 days (stable domain, no external dependencies)

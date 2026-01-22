---
phase: 12-multi-trial-results
plan: 04
subsystem: eval
tags: [compare, cli, delta-display, json-loading]

# Dependency graph
requires:
  - phase: 12-03
    provides: SerializableMultiTrialResult with Deserialize derive
provides:
  - "rslph compare command for comparing two result files"
  - "load_multi_trial_result function for JSON loading"
  - "Delta display with directional arrows (^ improvement, v regression)"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "higher_is_better flag for correct arrow direction"
    - "Percent change calculation with zero-guard"
    - "Graceful error handling with path in error messages"

key-files:
  created: []
  modified:
    - src/cli.rs
    - src/eval/command.rs
    - src/eval/mod.rs
    - src/main.rs

key-decisions:
  - "Pass rate: higher is better (^ for increase)"
  - "Time and tokens: lower is better (^ for decrease)"
  - "Display format: {name}: {baseline}{unit} -> {comparison}{unit} ({arrow}{delta}{unit}, {sign}%)"
  - "Error messages include file path for debugging"

patterns-established:
  - "print_delta helper for consistent delta formatting"
  - "load_multi_trial_result for JSON file loading"

# Metrics
duration: 3min
completed: 2026-01-22
---

# Phase 12 Plan 04: Compare Command Summary

**Added rslph compare command to load two eval result JSON files and display deltas with directional arrows showing improvement or regression**

## Performance

- **Duration:** 3 min
- **Started:** 2026-01-22T01:16:08Z
- **Completed:** 2026-01-22T01:18:44Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments
- Added Compare command to CLI with file1 and file2 PathBuf arguments
- Added test_parse_compare_command for CLI parsing
- Implemented load_multi_trial_result for JSON file loading
- Added 3 unit tests for load function (valid, missing file, invalid JSON)
- Implemented run_compare_command with delta display
- Added print_delta helper with higher_is_better logic
- Exported run_compare_command from eval module
- Added Compare command handling to main.rs
- All 237 tests passing

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Compare command to CLI** - `2f377c3` (feat)
2. **Task 2: Implement load_multi_trial_result function** - `3324ae3` (feat)
3. **Task 3: Implement run_compare_command with delta display** - `a127304` (feat)

## Files Created/Modified
- `src/cli.rs` - Added Compare command variant, test
- `src/eval/command.rs` - Added load_multi_trial_result, run_compare_command, print_delta, tests
- `src/eval/mod.rs` - Exported run_compare_command
- `src/main.rs` - Added Compare command match arm

## Key Code Changes

### Compare Command CLI
```rust
/// Compare two eval result files
Compare {
    /// First result file (baseline)
    file1: PathBuf,

    /// Second result file (comparison)
    file2: PathBuf,
},
```

### run_compare_command
```rust
pub fn run_compare_command(file1: PathBuf, file2: PathBuf) -> color_eyre::Result<()> {
    let result1 = load_multi_trial_result(&file1)?;
    let result2 = load_multi_trial_result(&file2)?;

    println!("Comparing results:");
    println!("  Baseline:   {} ({} trials)", file1.display(), result1.trial_count);
    println!("  Comparison: {} ({} trials)", file2.display(), result2.trial_count);
    println!();

    // Print deltas with correct arrow direction
    print_delta("Pass Rate", result1.statistics.pass_rate.mean * 100.0,
                result2.statistics.pass_rate.mean * 100.0, "%", true);
    print_delta("Execution Time", result1.statistics.elapsed_secs.mean,
                result2.statistics.elapsed_secs.mean, "s", false);
    // ... input tokens, output tokens
    Ok(())
}
```

### print_delta Helper
```rust
fn print_delta(name: &str, baseline: f64, comparison: f64, unit: &str, higher_is_better: bool) {
    let delta = comparison - baseline;
    let is_improvement = if higher_is_better { delta > 0.0 } else { delta < 0.0 };
    let arrow = if is_improvement { "^" } else { "v" };
    // Format and print
}
```

## Decisions Made
- Pass rate displays as percentage (mean * 100)
- Arrow logic: ^ for improvement, v for regression
- For pass rate, higher is better so positive delta = ^
- For time/tokens, lower is better so negative delta = ^
- Error messages include full file path for debugging

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 12 (Multi-Trial Results) complete
- All 4 plans executed successfully
- EVAL-09 (compare command) fully implemented

---
*Phase: 12-multi-trial-results*
*Completed: 2026-01-22*

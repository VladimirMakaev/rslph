---
phase: 04-core-build-loop
plan: 01
subsystem: build-loop
tags: [state-machine, subprocess, iteration, prompt, cli]

dependency-graph:
  requires:
    - phase-01 (config, cli, progress)
    - phase-02 (subprocess/ClaudeRunner)
    - phase-03 (prompts/loader pattern)
  provides:
    - build module (run_build_command)
    - BuildState state machine
    - PROMPT_build.md embedded prompt
    - get_build_prompt function
  affects:
    - phase-04-02 (termination logic)
    - phase-04-03 (dry-run enhancement)

tech-stack:
  added:
    - chrono 0.4 (iteration timestamps)
  patterns:
    - enum state machine (BuildState)
    - async iteration loop
    - fresh subprocess per iteration

key-files:
  created:
    - prompts/PROMPT_build.md
    - src/build/mod.rs
    - src/build/state.rs
    - src/build/iteration.rs
    - src/build/command.rs
  modified:
    - src/prompts/defaults.rs
    - src/prompts/loader.rs
    - src/prompts/mod.rs
    - src/lib.rs
    - src/main.rs
    - Cargo.toml

decisions:
  - id: CHRONO-TIMESTAMP
    description: Use chrono for iteration timestamps in log
    rationale: Research document listed chrono as standard for iteration timestamps

metrics:
  duration: 5m 26s
  completed: 2026-01-18
---

# Phase 4 Plan 1: Build Loop Infrastructure Summary

**One-liner:** Rust enum state machine with fresh Claude subprocess per iteration, PROMPT_build.md embedded in binary

## What Was Built

### 1. PROMPT_build.md and Prompt System Extension

Created `prompts/PROMPT_build.md` with comprehensive build agent instructions:
- One task per iteration rule
- RALPH_DONE placement in Status section
- Output format (complete progress file markdown)
- Failure memory via Recent Attempts section

Extended prompt system with `get_build_prompt()` following existing pattern from `get_plan_prompt()`.

### 2. Build Module State Machine

Created `src/build/state.rs` with:
- `BuildState` enum: Starting, Running, IterationComplete, Done, Failed
- `DoneReason` enum: AllTasksComplete, RalphDoneMarker, MaxIterationsReached, UserCancelled, SingleIterationComplete
- `IterationResult` enum: Continue, Done
- `BuildContext` struct with all execution state

### 3. Iteration Execution Logic

Created `src/build/iteration.rs` with `run_single_iteration()`:
- Re-reads progress file each iteration (handles external edits)
- Checks early exit conditions (RALPH_DONE, all tasks complete)
- Spawns fresh Claude subprocess with build prompt
- Parses JSONL response via StreamResponse
- Updates progress file atomically

### 4. Main Loop Command Handler

Created `src/build/command.rs` with `run_build_command()`:
- State machine-based loop orchestration
- Dry-run mode preview
- Iteration logging with timestamps
- Cancellation handling between iterations
- Completion messages based on done reason

### 5. CLI Integration

Updated `src/main.rs` to call `run_build_command()` in the Build command arm with:
- Ctrl+C handler for graceful cancellation
- Mode display for --once and --dry-run
- Appropriate exit codes

## Decisions Made

| ID | Decision | Rationale |
|----|----------|-----------|
| CHRONO-TIMESTAMP | Use chrono for iteration timestamps | Research document listed chrono as standard for iteration timestamps |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added chrono dependency**
- **Found during:** Task 2 implementation
- **Issue:** Research document stated chrono was already a dependency but it wasn't in Cargo.toml
- **Fix:** Added chrono 0.4 via `cargo add chrono`
- **Files modified:** Cargo.toml, Cargo.lock
- **Commit:** ccd96ca

## Test Coverage

Added 16 new tests in build module:
- State machine transition tests
- DoneReason display tests
- IterationResult tests
- RALPH_DONE detection tests
- All tasks complete detection tests
- Dry-run mode tests
- Echo mock subprocess tests
- Timeout handling tests
- Cancellation handling tests
- Missing file error tests

All 80 project tests pass. No clippy warnings.

## Technical Notes

### Claude CLI Args Pattern
Following existing pattern from planning/command.rs:
```rust
vec![
    "--internet".to_string(),      // WORKAROUND: Required to prevent hanging
    "-p".to_string(),              // Headless mode
    "--verbose".to_string(),       // Required for stream-json with -p
    "--output-format".to_string(),
    "stream-json".to_string(),
    "--system-prompt".to_string(),
    system_prompt,
    user_input,
]
```

### State Machine Flow
```
Starting -> Running{1}
Running{n} -> IterationComplete{n} | Done | Failed
IterationComplete{n} -> Running{n+1} | Done
Done -> return Ok(())
Failed -> return Err(...)
```

## Next Phase Readiness

### For Plan 02 (Termination Logic)
- BuildState and DoneReason enums ready
- IterationResult::Done variant established
- State transitions in place

### For Plan 03 (Dry-Run Enhancement)
- Basic dry-run implemented
- Needs enhancement per plan spec

## Commits

| Hash | Message |
|------|---------|
| 73ca81e | feat(04-01): add PROMPT_build.md and get_build_prompt function |
| ccd96ca | feat(04-01): create build module with state machine and iteration logic |
| 2d7b859 | feat(04-01): wire build command in main.rs |

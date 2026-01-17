---
phase: 02-subprocess-management
plan: 01
subsystem: subprocess
tags: [tokio, async, process, subprocess, streaming]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: Error types, lib.rs module structure
provides:
  - ClaudeRunner struct for async subprocess spawning
  - OutputLine enum for stdout/stderr discrimination
  - Concurrent stream reading without buffer deadlock
  - Process group isolation for signal handling
affects: [02-02-signals, 03-iteration-loop]

# Tech tracking
tech-stack:
  added: [tokio, tokio-util, nix]
  patterns: [tokio::select! for concurrent I/O, process_group(0) isolation]

key-files:
  created:
    - src/subprocess/mod.rs
    - src/subprocess/runner.rs
    - src/subprocess/output.rs
  modified:
    - Cargo.toml
    - src/lib.rs
    - src/error.rs

key-decisions:
  - "TOKIO-UTIL-NO-SYNC: tokio-util does not have a sync feature, CancellationToken is in base crate"
  - "EOF-STATE-STRUCT: Track stdout_done/stderr_done in struct, not local to next_output()"

patterns-established:
  - "Process isolation: Use process_group(0) to prevent signal inheritance from terminal"
  - "Concurrent streams: Use tokio::select! with conditional branches for EOF handling"
  - "Cancellation safety: Use Lines::next_line() which is cancellation-safe"

# Metrics
duration: 4min
completed: 2026-01-17
---

# Phase 2 Plan 1: Subprocess Spawning and Streaming Summary

**Async ClaudeRunner with tokio::process for subprocess spawning, concurrent stdout/stderr streaming via select!, and process group isolation**

## Performance

- **Duration:** 3m 38s
- **Started:** 2026-01-17T22:53:20Z
- **Completed:** 2026-01-17T22:56:58Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments
- ClaudeRunner struct that spawns subprocesses with piped I/O
- Concurrent stdout/stderr reading using tokio::select! without buffer deadlock
- Process group isolation via process_group(0) for future signal handling
- 5 comprehensive tests covering spawn, streaming, and error handling

## Task Commits

Each task was committed atomically:

1. **Task 1: Add dependencies and create subprocess module structure** - `c1cc117` (feat)
2. **Task 2: Implement ClaudeRunner spawn and streaming** - `14f866b` (feat)
3. **Task 3: Add tests for spawning and streaming** - `b156ca0` (test)

## Files Created/Modified
- `Cargo.toml` - Added tokio, tokio-util, nix dependencies
- `Cargo.lock` - Updated with new dependencies
- `src/lib.rs` - Added subprocess module export
- `src/error.rs` - Added Subprocess, Timeout, Cancelled error variants
- `src/subprocess/mod.rs` - Module exports for OutputLine and ClaudeRunner
- `src/subprocess/output.rs` - OutputLine enum with Stdout/Stderr variants
- `src/subprocess/runner.rs` - ClaudeRunner implementation with spawn, next_output, id, wait methods

## Decisions Made
- **TOKIO-UTIL-NO-SYNC:** tokio-util 0.7 does not have a "sync" feature (unlike what research docs suggested). CancellationToken is available in the base crate without feature flags.
- **EOF-STATE-STRUCT:** stdout_done and stderr_done flags must be stored in the struct, not as local variables in next_output(), to persist EOF state across multiple calls.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed tokio-util feature flag**
- **Found during:** Task 1 (cargo check)
- **Issue:** Plan specified `tokio-util = { version = "0.7", features = ["sync"] }` but tokio-util does not have a "sync" feature
- **Fix:** Changed to `tokio-util = "0.7"` (no features)
- **Files modified:** Cargo.toml
- **Verification:** cargo check passes
- **Committed in:** c1cc117 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (blocking issue)
**Impact on plan:** Research doc was slightly outdated on tokio-util features. CancellationToken is available without feature flags.

## Issues Encountered
None - all tasks executed smoothly after the tokio-util feature flag fix.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- ClaudeRunner ready for use in iteration loop
- Signal handling (Plan 02-02) can now add terminate_gracefully() method
- Timeout handling (Plan 02-02) can wrap next_output() calls with tokio::time::timeout

---
*Phase: 02-subprocess-management*
*Completed: 2026-01-17*

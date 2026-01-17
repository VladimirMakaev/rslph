---
phase: 02-subprocess-management
plan: 02
subsystem: subprocess-control
tags: [signals, cancellation, timeout, process-lifecycle]

dependency_graph:
  requires: ["02-01"]
  provides: ["signal-handling", "graceful-termination", "timeout-management"]
  affects: ["03-01", "04-01"]

tech_stack:
  added: []
  patterns:
    - "CancellationToken for cooperative cancellation"
    - "tokio::select! biased for priority-based branching"
    - "Process group signaling for child cleanup"

key_files:
  created:
    - src/subprocess/signals.rs
  modified:
    - src/subprocess/runner.rs
    - src/subprocess/mod.rs

decisions:
  - key: "PROCESS-GROUP-SIGTERM"
    value: "Send SIGTERM to negative PID to signal entire process group"
    rationale: "Claude may spawn child processes; signaling the group ensures they all receive termination"
  - key: "BIASED-SELECT-CANCEL"
    value: "Use biased select! to check cancellation before output"
    rationale: "Ensures prompt response to Ctrl+C even during heavy output"

metrics:
  duration: "2m 51s"
  completed: "2026-01-17"
---

# Phase 02 Plan 02: Signal Handling and Timeout Management Summary

Signal handling for graceful Ctrl+C shutdown via CancellationToken, plus timeout management to kill stuck Claude invocations.

## What Was Built

### 1. Signal Handling Module (src/subprocess/signals.rs)

- `setup_ctrl_c_handler()`: Spawns background task that listens for Ctrl+C and cancels a returned CancellationToken
- `is_cancelled()`: Utility function for quick cancellation status checks
- Exported from subprocess module for use in main execution loop

### 2. ClaudeRunner Termination Methods

- `terminate_gracefully(grace_period)`: Sends SIGTERM to process group, waits for grace period, then SIGKILL if needed
- `kill()`: Immediate termination without grace period
- Both methods call wait() to reap child process and prevent zombies

### 3. ClaudeRunner Execution Methods

- `run_to_completion(cancel_token)`: Collects output until EOF or cancellation, respects CancellationToken via biased select!
- `run_with_timeout(max_duration, cancel_token)`: Wraps run_to_completion with tokio::time::timeout, terminates and returns Timeout error on expiration

### 4. Comprehensive Test Suite

- 6 new tests covering termination, timeout, and cancellation scenarios
- Verify no zombie processes after any termination path
- All 34 project tests pass

## Key Implementation Details

- Process group (negative PID) used for SIGTERM to catch any children Claude spawns
- `biased` keyword in select! ensures cancellation is checked first
- Default 5-second grace period for terminate_gracefully when called from run_to_completion
- Error types Cancelled and Timeout already existed in RslphError

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Signal target | Process group (-PID) | Claude may spawn subprocesses that need cleanup |
| Select bias | Cancel-first | User expects immediate Ctrl+C response |
| Grace period | 5 seconds default | Enough time for Claude to save state |

## Deviations from Plan

None - plan executed exactly as written.

## Verification Results

```
cargo test subprocess
running 13 tests ... ok

cargo test
running 34 tests ... ok

ps aux | grep defunct | grep -v grep
# Pre-existing zombie (PID 38394 from 18:46), not from our tests
```

## Phase 2 Completion Status

With this plan complete, Phase 2 success criteria are all met:

1. [x] Claude CLI runs as subprocess with piped stdout/stderr - Plan 02-01
2. [x] Output streams line-by-line in real-time - Plan 02-01
3. [x] Ctrl+C gracefully terminates Claude - setup_ctrl_c_handler + terminate_gracefully
4. [x] Stuck Claude invocations timeout - run_with_timeout
5. [x] No zombie processes - wait() called in all termination paths

## Next Phase Readiness

Phase 3 (Planning Command) can begin. It will use:
- ClaudeRunner for spawning Claude CLI
- run_with_timeout for preventing stuck planning invocations
- CancellationToken for user interrupt handling

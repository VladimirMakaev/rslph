---
phase: quick
plan: 012
subsystem: subprocess-tui
tags: [stderr, debug-logging, subprocess, tui]

dependency-graph:
  requires: [tui-event-system, subprocess-runner]
  provides: [stderr-capture, subprocess-trace-logging]
  affects: [debugging, observability]

tech-stack:
  added: []
  patterns: [subprocess-event-forwarding, trace-logging]

key-files:
  created: []
  modified:
    - src/tui/event.rs
    - src/build/iteration.rs

decisions:
  - id: stderr-event-variant
    choice: "SubprocessEvent::Stderr(String) variant"
  - id: stderr-prefix
    choice: "[stderr] prefix in LogMessage for visibility"
  - id: trace-via-event
    choice: "Use SubprocessEvent::Log for streaming mode traces"

metrics:
  duration: "4m 2s"
  completed: "2026-01-31"
---

# Quick Task 012: Add stderr capture and debug logging for subprocess communication

**One-liner:** stderr from Claude CLI subprocess is now captured and displayed in TUI with [stderr] prefix, with enhanced trace logging for subprocess lifecycle.

## Summary

This task addresses the issue where stderr output from the Claude CLI was being ignored, making it impossible to diagnose subprocess issues. The changes surface stderr output to users and add comprehensive debug trace logging.

## Changes Made

### Task 1: Add Stderr event variant and forward stderr output

**Files modified:**
- `src/tui/event.rs` - Added `SubprocessEvent::Stderr(String)` variant with conversion to `AppEvent::LogMessage` prefixed with `[stderr]`
- `src/build/iteration.rs` - Added stderr handling in both streaming and non-streaming modes

**Key implementation:**
- New `Stderr` variant in `SubprocessEvent` enum
- Conversion maps `Stderr(s)` to `LogMessage("[stderr] {s}")`
- Streaming mode: sends `SubprocessEvent::Stderr` to TUI channel
- Non-streaming mode: logs via `ctx.log("[stderr] {}")`

### Task 2: Add enhanced debug logging for subprocess communication

**Files modified:**
- `src/build/iteration.rs` - Added comprehensive trace logging

**Trace points added:**
1. Full Claude command with all arguments at spawn time
2. Start of subprocess output streaming (`[TRACE] Starting subprocess output streaming`)
3. Stderr lines with content (`[TRACE] Received stderr: ...`)
4. End of streaming (`[TRACE] Subprocess output stream ended`)
5. Non-streaming mode start/completion with line counts

## Verification

All verifications pass:
- `cargo build` - compiles without errors
- `cargo build --release` - release build succeeds
- `cargo test` - all 300 unit tests + 99 e2e tests pass
- `cargo clippy -- -D warnings` - no warnings

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 646f67a | Add stderr capture and forward to TUI |
| 2 | bd3a70f | Add enhanced debug logging for subprocess communication |

## Deviations from Plan

None - plan executed exactly as written.

## Success Criteria Verification

1. When Claude CLI outputs to stderr, those messages appear in the TUI with [stderr] prefix - DONE
2. Debug logging shows the full subprocess command being executed - DONE
3. Debug logging shows when subprocess output stream starts and ends - DONE
4. All existing tests continue to pass - DONE (300 unit + 99 e2e tests pass)

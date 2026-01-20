---
phase: 08
plan: 02
subsystem: tui
tags: [token-tracking, status-bar, human-format]

dependency-graph:
  requires: []
  provides:
    - Token display in TUI status bar
    - Token tracking in App state
    - Token reporting in plan command
  affects:
    - 08-03 (E2E tests for token display)
    - 08-04 (Fake Claude token configuration)

tech-stack:
  added:
    - human_format: "1.2"
  patterns:
    - AppEvent for token state updates
    - SubprocessEvent for cross-component communication

key-files:
  created:
    - src/build/tokens.rs
  modified:
    - src/tui/app.rs
    - src/tui/event.rs
    - src/tui/widgets/status_bar.rs
    - src/planning/command.rs
    - src/build/iteration.rs
    - src/build/state.rs
    - src/subprocess/stream_json.rs
    - Cargo.toml

decisions:
  - id: token-display-format
    choice: "In: X | Out: Y | CacheW: Z | CacheR: W"
    alternatives: ["Compact I/O/CW/CR", "Single Cache value"]
    rationale: Per CONTEXT.md - show all 4 fields separately with readable labels

metrics:
  duration: 5m 16s
  completed: 2026-01-20
---

# Phase 8 Plan 02: Token Display Summary

**One-liner:** Status bar and plan command now display all 4 token fields using human-readable formatting (5.2k, 1.2M)

## What Was Built

1. **Token types module** (`src/build/tokens.rs`)
   - `TokenUsage` struct for cumulative token tracking
   - `IterationTokens` struct for per-iteration snapshots
   - `format_tokens()` helper for human-readable display (uses human_format crate)

2. **TUI token state** (`src/tui/app.rs`)
   - Added `total_tokens: TokenUsage` field to App struct
   - Added `AppEvent::TokenUsage` variant
   - Handler updates total_tokens on each token event

3. **Event routing** (`src/tui/event.rs`)
   - Added `SubprocessEvent::TokenUsage` variant
   - Conversion to AppEvent for TUI updates

4. **Status bar display** (`src/tui/widgets/status_bar.rs`)
   - Updated status line format to include all 4 token counts
   - Format: "Iter X/Y | Task X/Y | In: 5.2k | Out: 10.9k | CacheW: 2.1k | CacheR: 1.0k | [context bar]"

5. **Plan command output** (`src/planning/command.rs`)
   - Added token summary at completion for both basic and adaptive modes
   - Format: "Tokens used: In: X | Out: Y | CacheW: Z | CacheR: W"

6. **Build loop integration** (prerequisite work from 08-01)
   - Token fields added to BuildContext
   - SubprocessEvent::TokenUsage emitted from parse_and_stream_line
   - Per-iteration token accumulation in run_single_iteration
   - Cache token fields added to StreamResponse

## Commits

| Hash | Message |
|------|---------|
| 7fb41fa | feat(08-02): add token state to App and handle TokenUsage events |
| ceff336 | feat(08-02): display tokens in TUI status bar |
| a41c66b | feat(08-02): add token reporting to plan command and build loop |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Created tokens module (Plan 01 prerequisite)**
- **Found during:** Task 1
- **Issue:** Plan 02 depends on TokenUsage and format_tokens from Plan 01 which hadn't been executed
- **Fix:** Created src/build/tokens.rs with full token types and formatting
- **Files created:** src/build/tokens.rs, modified src/build/mod.rs, Cargo.toml
- **Commit:** 7fb41fa

**2. [Rule 3 - Blocking] Included build loop token integration**
- **Found during:** Task 3
- **Issue:** Pre-existing uncommitted changes from Plan 01 were needed for token events to flow correctly
- **Fix:** Committed state.rs, iteration.rs, stream_json.rs changes with Task 3
- **Files modified:** src/build/state.rs, src/build/iteration.rs, src/subprocess/stream_json.rs
- **Commit:** a41c66b

## Tests

- All 168 unit tests pass
- New test: `test_app_update_token_usage` verifies TokenUsage event handling
- Token formatting tests: `test_format_tokens_*` verify human-readable output

## Verification

1. `cargo check` - All code compiles with no errors
2. `cargo test --lib` - 168 tests pass
3. Status bar format matches CONTEXT.md specification: "In: X | Out: Y | CacheW: Z | CacheR: W"

## Next Phase Readiness

**Ready for:**
- Plan 08-03: E2E tests for token display
- Plan 08-04: Fake Claude token configuration
- Manual testing with real Claude CLI

**Dependencies satisfied:**
- Token types exported from build module
- SubprocessEvent::TokenUsage flows to TUI
- Status bar renders token counts
- Plan command outputs token summary

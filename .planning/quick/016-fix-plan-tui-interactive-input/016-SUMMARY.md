---
phase: quick
plan: 016
subsystem: tui
tags: [tui, planning, interactive, stdin, input-handling]
requires: [quick-015]
provides:
  - Interactive input in plan TUI mode
  - User can answer Claude's questions during planning
affects: []
tech-stack:
  added: []
  patterns:
    - crossterm EventStream for character input handling
    - Input mode state pattern for TUI
key-files:
  created: []
  modified:
    - src/planning/command.rs
    - src/tui/plan_tui.rs
decisions:
  - id: use-crossterm-eventstream
    choice: Direct crossterm EventStream instead of EventHandler
    rationale: EventHandler only emits AppEvent variants (Quit, ScrollUp, etc), doesn't support character passthrough needed for text input
  - id: input-channel-direction
    choice: Channel from TUI to main loop (not passing runner to TUI)
    rationale: Avoids ownership complexity, keeps runner in main loop where it's spawned
  - id: input-mode-priority
    choice: Input mode takes priority in header status display
    rationale: User needs immediate visual feedback that input is required
metrics:
  duration: 4m 21s
  completed: 2026-01-31
---

# Quick Task 016: Fix Plan TUI Interactive Input Summary

**One-liner:** Plan TUI now handles interactive input using spawn_interactive, EventStream for character capture, and input_tx channel for stdin relay.

## Completed Tasks

| Task | Commit | Files Modified |
|------|--------|----------------|
| 1: Use spawn_interactive and pass ClaudeRunner to TUI | 6789ff5 | src/planning/command.rs |
| 2: Add input handling to PlanTuiState and event loop | bad8f55 | src/tui/plan_tui.rs |
| 3: Render input prompt in plan TUI | bad8f55 | src/tui/plan_tui.rs |

## What Was Built

Fixed plan TUI to support interactive input when Claude CLI asks questions during planning.

**Key changes:**

1. **src/planning/command.rs:**
   - Changed `ClaudeRunner::spawn()` to `spawn_interactive()` to enable piped stdin
   - Created input channel: `(input_tx, mut input_rx) = mpsc::unbounded_channel::<String>()`
   - Added `input_rx.recv()` branch in stream loop to write user responses to Claude stdin
   - Detect input required: `if let Some(question) = event.is_input_required()`
   - Forward `InputRequired` events to TUI

2. **src/tui/plan_tui.rs:**
   - Added `InputRequired(String)` variant to `PlanTuiEvent` enum
   - Added input state fields: `input_mode`, `input_buffer`, `current_question`
   - Added methods: `enter_input_mode()`, `submit_input()`, `handle_input_char()`, `handle_input_backspace()`
   - Updated `run_plan_tui()` signature to accept `input_tx` channel
   - Replaced `EventHandler` with direct `crossterm::event::EventStream` for character input
   - Event handling: characters/backspace when `input_mode`, Enter to submit, Esc to cancel
   - Adjusted footer height to 6 when in input mode
   - Render input prompt: question + `> {buffer}_` with yellow border
   - Header shows "Waiting for your input..." when in input mode

## Deviations from Plan

None - plan executed exactly as written.

## Technical Details

**Architecture pattern:**

```
Claude subprocess (stdin piped)
         ^
         | write_stdin()
         |
    input_rx.recv()
         ^
         | mpsc channel
         |
  run_plan_tui (EventStream)
         |
    input_tx.send()
         ^
         | user types, presses Enter
         |
   Keyboard (character input)
```

**Input mode flow:**

1. Claude asks question → `is_input_required()` returns `Some(question)`
2. `InputRequired` event sent to TUI
3. TUI calls `enter_input_mode(question)` → sets `input_mode = true`
4. Render loop shows yellow input prompt in footer, status shows "Waiting for your input..."
5. User types → `handle_input_char()` appends to buffer
6. User presses Enter → `submit_input()` returns response, sends via `input_tx`
7. Main loop receives via `input_rx`, calls `runner.write_stdin(&text)`
8. Claude receives input on stdin, continues execution

**Why crossterm EventStream instead of EventHandler:**

The existing `EventHandler` only emits `AppEvent` variants like `Quit`, `ScrollUp`, `ScrollDown`. For character input, we need raw `KeyCode::Char(c)` events. Rather than extending `AppEvent` with character variants (which would affect build TUI too), we use `crossterm::event::EventStream` directly in plan TUI.

## Testing

**Unit tests added (6 tests):**
- `test_enter_input_mode()` - Verifies input mode initialization
- `test_handle_input_char()` - Character appending to buffer
- `test_handle_input_backspace()` - Buffer character removal
- `test_submit_input()` - Returns buffer and exits input mode
- `test_submit_input_not_in_input_mode()` - Returns None when not in input mode
- Updated `test_plan_tui_event_variants()` - Includes `InputRequired` variant

**All tests pass:**
- `cargo test --lib plan_tui` - 14 passed
- `cargo test` - 109 passed (full suite)
- `cargo check` - No errors or warnings

## Next Phase Readiness

**Ready for:**
- Interactive planning sessions with adaptive mode
- User can answer Claude's clarifying questions without leaving TUI

**Dependencies satisfied:**
- Quick task 015 verified `RSLPH_CLAUDE_CMD` E2E tests work
- `ClaudeRunner::spawn_interactive()` already implemented (quick-011)
- `ClaudeRunner::write_stdin()` already implemented (quick-011)
- `StreamEvent::is_input_required()` already implemented (quick-011)

**No blockers.**

## Session Notes

Execution was smooth and followed the plan precisely. The decision to use direct `crossterm::EventStream` instead of extending `EventHandler` was the right choice - it keeps plan TUI's input handling isolated and doesn't add complexity to the shared event system.

Duration: 4m 21s (faster than average for TUI changes due to clear pattern from quick-011).

---
phase: 15-interactive-planning
plan: 04
subsystem: tui
tags: [tui, interactive, input-mode, crossterm, keyboard]

dependency-graph:
  requires:
    - 15-01: session_id and AskUserQuestion extraction
    - 15-02: has_questions() detection and answer collection
  provides:
    - tui-input-mode: InputMode enum for TUI state machine
    - question-rendering: Visual display of questions in TUI
    - keyboard-input: Raw crossterm event handling for text input
    - tui-qa-integration: Full Q&A flow in TUI mode
  affects:
    - 15-03: Will use session resume with collected answers

tech-stack:
  added: []
  patterns:
    - crossterm-event-stream: Raw keyboard event handling for text input
    - tui-state-machine: InputMode enum for modal interface behavior
    - async-select-biased: Priority-based event handling in TUI loop

key-files:
  created: []
  modified:
    - src/tui/plan_tui.rs: InputMode, question rendering, keyboard input handling
    - src/tui/mod.rs: Export InputMode
    - src/planning/command.rs: TUI Q&A integration

decisions:
  - id: raw-crossterm-events
    choice: "Use crossterm EventStream directly instead of EventHandler"
    reason: "EventHandler is designed for navigation events (AppEvent), not raw text input. Raw crossterm events allow character-by-character text entry."
  - id: ctrl-enter-submit
    choice: "Use Ctrl+Enter or Ctrl+D to submit answers"
    reason: "Regular Enter adds newlines for multi-line answers. Consistent with CLI text editors."
  - id: answers-in-input-buffer
    choice: "Store answers in input_buffer field of TUI state"
    reason: "Allows run_plan_tui to return state with answers for command.rs to process"
  - id: todo-session-resume
    choice: "Add TODO for session resume pending Plan 15-03"
    reason: "Plan 15-03 adds resume_session() function. Integration ready but not activated."

metrics:
  duration: ~10m
  completed: 2026-02-01
---

# Phase 15 Plan 04: TUI Input Mode for Interactive Q&A Summary

**One-liner:** TUI input mode with InputMode enum, question rendering, and raw crossterm keyboard handling for collecting user answers.

## What Was Done

### Task 1: Add InputMode and input state to PlanTuiState (9ecfc03)
- Added `InputMode` enum with `Normal` and `AnsweringQuestions` variants
- Added input state fields to `PlanTuiState`:
  - `input_mode: InputMode` - current TUI mode
  - `pending_questions: Vec<String>` - questions waiting for answers
  - `input_buffer: String` - user's typed input
  - `session_id: Option<String>` - for session resume
  - `answers_submitted: bool` - flag for completion
- Added `PlanTuiEvent::QuestionsAsked` variant for triggering Q&A mode
- Added `PlanStatus::AwaitingInput` and `ResumingSession` status variants
- Added methods: `enter_question_mode()`, `exit_question_mode()`, `handle_input_*()`, `is_answering_questions()`, `get_session_id()`
- Added comprehensive tests for new functionality

### Task 2: Render question input area in TUI (3dc3f00)
- Added `render_question_input()` function for AnsweringQuestions mode
- Layout with 4 areas: header, questions box, input area, instructions footer
- Questions displayed with numbered format and yellow border
- Input area with cyan border and visual cursor indicator
- Placeholder text when input buffer is empty
- Instructions footer showing keybindings (Enter, Ctrl+Enter, Esc)
- Modified `render_plan_tui()` to switch layouts based on input_mode

### Task 3: Handle keyboard input for answer entry (8bbeff3)
- Replaced `EventHandler` with raw `crossterm::EventStream` for text input
- Added `handle_input_key()` for AnsweringQuestions mode:
  - Ctrl+Enter / Ctrl+D: submit answers
  - Enter: add newline to buffer
  - Backspace: delete last character
  - Esc: cancel and quit
  - Regular chars: append to buffer
- Added `handle_navigation_key()` for Normal mode scrolling
- TUI loop exits when `answers_submitted` to allow session resume
- TUI stays open after stream completes if in question mode

### Task 4: Integrate TUI input mode with planning command (711e000)
- Modified `run_tui_planning()` to detect questions after stream completes
- Send `QuestionsAsked` event to TUI when questions detected
- Wait for TUI to return with collected answers
- Added TODO for session resume integration (pending Plan 15-03)
- Added trace logging for debugging Q&A flow
- Proper cleanup on timeout/cancellation before checking questions

## Files Modified

| File | Changes |
|------|---------|
| src/tui/plan_tui.rs | InputMode enum, PlanTuiState fields, question rendering, keyboard handling |
| src/tui/mod.rs | Export InputMode |
| src/planning/command.rs | TUI Q&A integration in run_tui_planning |

## Deviations from Plan

None - plan executed exactly as written.

## Next Phase Readiness

### For Plan 15-03 (Session Resume)
The TUI integration is complete and ready. When Plan 15-03 adds `resume_session()`:
1. Remove the TODO comment in `run_tui_planning()`
2. Call `resume_session(session_id, formatted_answers, ...)`
3. Stream resume output to TUI
4. Handle potential additional questions (loop)

### Current Limitation
Without Plan 15-03, users can type answers but the session cannot be resumed. The TUI collects answers and logs them, but continues with incomplete output (likely failing to parse progress.md).

## Verification Results

- [x] cargo build compiles successfully
- [x] cargo test passes all plan_tui and planning tests
- [x] cargo clippy shows no new warnings (pre-existing only)
- [ ] Manual test pending (requires Claude CLI with question-asking prompt)

# Quick Task 011: Implement stdin relay for Claude CLI interactive questions

## One-liner

Stdin piping for Claude CLI with question detection and TUI input mode handling.

## Completed Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Enable stdin piping in ClaudeRunner | 26a4514 | src/subprocess/runner.rs |
| 2 | Add question detection to stream parser | 9c8e74c | src/subprocess/stream_json.rs, src/tui/event.rs |
| 3 | Wire question detection to TUI and add input handling | 23821a2 | src/tui/app.rs, src/build/iteration.rs |

## Changes Made

### Task 1: Enable stdin piping in ClaudeRunner
- Changed `.stdin(Stdio::null())` to `.stdin(Stdio::piped())` in spawn method
- Added `stdin: Option<ChildStdin>` field to ClaudeRunner struct
- Added `write_stdin(&mut self, response: &str)` method for sending responses to subprocess
- Updated spawn to capture stdin handle after process creation

### Task 2: Add question detection to stream parser
- Added `is_input_required(&self) -> Option<String>` method to StreamEvent
- Detects "result" type events with content containing "?" or "please"
- Detects tool_result blocks with "waiting for" or "input required"
- Added `InputRequired { question: String }` variant to SubprocessEvent
- Added From conversion mapping SubprocessEvent::InputRequired to AppEvent::InputRequired
- Added 4 unit tests for input detection scenarios

### Task 3: Wire question detection to TUI and add input handling
- Added `InputRequired { question: String }` variant to AppEvent
- Added input mode state fields to App:
  - `input_mode: bool` - whether waiting for user input
  - `input_buffer: String` - current input buffer
  - `current_question: Option<String>` - the question being answered
- Added input handling methods:
  - `enter_input_mode(question)` - enter input mode with a question
  - `submit_input() -> Option<String>` - exit input mode and return response
  - `handle_input_char(c)` - append character to buffer
  - `handle_input_backspace()` - remove last character from buffer
- Updated `parse_and_stream_line()` in iteration.rs to detect and send InputRequired events
- Added handling for InputRequired in App::update()

## Verification

- `cargo check` passes
- `cargo test` - all 300 tests pass
- `cargo clippy` - no warnings

## Notes

This task establishes the foundation for Claude CLI interactive question handling. The actual keyboard event handling and stdin writing in the TUI run loop (src/tui/run.rs) will need to be wired up in a follow-up task. This task provides:

1. The subprocess can now receive input via `write_stdin()`
2. The stream parser can detect when Claude asks for input
3. The TUI App has state and methods to manage input mode

The key integration points are:
- `StreamEvent::is_input_required()` for detecting questions
- `SubprocessEvent::InputRequired` for event propagation
- `App::enter_input_mode()` / `App::submit_input()` for state management
- `ClaudeRunner::write_stdin()` for sending responses back

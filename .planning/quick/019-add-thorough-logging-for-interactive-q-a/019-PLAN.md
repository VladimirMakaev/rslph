---
phase: quick
plan: 019
type: execute
wave: 1
depends_on: []
files_modified:
  - src/subprocess/stream_json.rs
  - src/planning/command.rs
  - src/tui/plan_tui.rs
  - src/subprocess/runner.rs
autonomous: true

must_haves:
  truths:
    - "When AskUserQuestion detected, log shows raw JSON event and parsed questions"
    - "Session ID extraction is logged at debug level when captured"
    - "TUI input mode transitions are logged (entering/exiting question mode)"
    - "All stdout/stderr from Claude subprocess is captured at trace level"
  artifacts:
    - path: "src/subprocess/stream_json.rs"
      provides: "Debug logging for AskUserQuestion extraction and session ID parsing"
    - path: "src/planning/command.rs"
      provides: "Trace logging for TUI planning flow and question detection"
    - path: "src/tui/plan_tui.rs"
      provides: "Debug logging for input mode transitions"
    - path: "src/subprocess/runner.rs"
      provides: "Trace logging for subprocess I/O"
  key_links:
    - from: "src/planning/command.rs"
      to: "stream_json::StreamEvent"
      via: "process_event logging"
      pattern: "debug!.*extract_ask_user_questions|trace!.*session_id"
---

<objective>
Add thorough debug/trace logging throughout the interactive Q&A flow.

Purpose: Diagnose why users aren't given opportunity to answer questions when running with real Claude CLI (vs fake_claude). The logging will capture the complete flow from subprocess stdout to TUI input mode transition.

Output: Enhanced logging at multiple levels (trace for raw I/O, debug for parsed events and state transitions) that can be enabled via RUST_LOG=rslph=debug or RUST_LOG=rslph=trace.
</objective>

<execution_context>
@/Users/vmakaev/.claude/get-shit-done/workflows/execute-plan.md
@/Users/vmakaev/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@src/subprocess/stream_json.rs
@src/planning/command.rs
@src/tui/plan_tui.rs
@src/subprocess/runner.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add debug logging to stream_json parser</name>
  <files>src/subprocess/stream_json.rs</files>
  <action>
Add tracing crate dependency logging (using log or tracing macros) to the stream_json parser:

1. At the top of stream_json.rs, add: `use tracing::{debug, trace};` (the project already uses tokio which includes tracing ecosystem)

2. In `StreamEvent::parse()`:
   - Add trace log of raw JSON line before parsing: `trace!(line = %line, "Parsing stream event");`
   - On parse success, add trace log with event_type: `trace!(event_type = %event.event_type, "Parsed stream event");`

3. In `StreamEvent::is_init_event()`:
   - Add debug log when returning true: `debug!(session_id = ?self.session_id, "Detected init event");`

4. In `StreamEvent::extract_session_id()`:
   - Add debug log when session_id is extracted: `debug!(session_id = %session_id, "Extracted session ID");`

5. In `StreamEvent::extract_ask_user_questions()`:
   - At entry, add trace log: `trace!("Checking for AskUserQuestion tool calls");`
   - When AskUserQuestion block found, add debug log: `debug!(questions = ?questions, "Found AskUserQuestion with {} question(s)", questions.len());`
   - When no AskUserQuestion found (before returning None), add trace: `trace!("No AskUserQuestion tool calls in event");`

6. In `StreamResponse::process_event()`:
   - Add debug log when session_id captured (first time): `debug!(session_id = %session_id, "Captured session ID (first wins)");`
   - Add debug log when AskUserQuestion collected: `debug!(count = %self.questions.len(), "AskUserQuestion collected, total so far");`

Also add `use tracing::{debug, trace};` at module top.
  </action>
  <verify>
Run `cargo build` to ensure the tracing macros are properly integrated. The build should succeed with no errors.
  </verify>
  <done>
stream_json.rs contains trace/debug logging for JSON parsing, session_id extraction, and AskUserQuestion detection.
  </done>
</task>

<task type="auto">
  <name>Task 2: Add debug logging to planning command TUI flow</name>
  <files>src/planning/command.rs</files>
  <action>
Enhance the existing eprintln TRACE logs in planning/command.rs with proper tracing macros and add new logging for question/session flow:

1. At top, add: `use tracing::{debug, trace, info};`

2. In `run_tui_planning()`:
   - After spawning Claude, add: `debug!(pid = ?runner.id(), "Spawned Claude for TUI planning");`
   - Inside the stream loop, when processing stdout:
     - Before attempting parse: `trace!(line_len = %s.len(), "Processing stdout line");`
     - After successful parse: `debug!(event_type = %event.event_type, has_tool_use = %event.has_tool_use(), "Parsed stream event in TUI mode");`
     - If event has AskUserQuestion: `debug!("Stream event contains AskUserQuestion, will check after stream completes");`
   - After stream completes, before checking has_questions:
     - `debug!(has_questions = %stream_response.has_questions(), session_id = ?stream_response.session_id, total_question_events = %stream_response.questions.len(), "Stream complete, checking for questions");`
   - When sending QuestionsAsked event to TUI:
     - `info!(session_id = %session_id, question_count = %questions.len(), "Sending questions to TUI for user input");`
   - After TUI returns with answers:
     - `debug!(should_quit = %tui_state.should_quit, answers_submitted = %tui_state.answers_submitted, input_len = %tui_state.input_buffer.len(), "TUI returned");`

3. In `resume_session()`:
   - Before spawning Claude for resume: `debug!(session_id = %session_id, message_len = %message.len(), "Resuming session with Claude");`
   - After collecting output: `debug!(has_questions = %stream_response.has_questions(), text_len = %stream_response.text.len(), "Resume session complete");`

4. Replace remaining eprintln! TRACE logs with proper tracing macros where they provide useful context (keeping eprintln for user-facing output like token summaries).
  </action>
  <verify>
Run `cargo build` to ensure tracing integration is correct. The build should succeed.
  </verify>
  <done>
planning/command.rs contains structured debug/trace logging for the TUI planning flow, including session ID handling and question detection.
  </done>
</task>

<task type="auto">
  <name>Task 3: Add debug logging to TUI input mode transitions</name>
  <files>src/tui/plan_tui.rs, src/subprocess/runner.rs</files>
  <action>
1. In src/tui/plan_tui.rs, add `use tracing::{debug, trace};` at top.

2. In `PlanTuiState::enter_question_mode()`:
   - Add: `debug!(question_count = %questions.len(), session_id = %session_id, "Entering question-answering mode");`

3. In `PlanTuiState::exit_question_mode()`:
   - Add: `debug!(answer_len = %self.input_buffer.len(), "Exiting question-answering mode, answers submitted");`

4. In `PlanTuiState::update()`:
   - When receiving QuestionsAsked event: `debug!("Received QuestionsAsked event, transitioning to input mode");`
   - When receiving Stream event: `trace!(has_items = %stream_event.extract_conversation_items().len(), "Received stream event in TUI");`

5. In `run_plan_tui()`:
   - When answers_submitted triggers break: `debug!("Answers submitted, exiting TUI event loop");`
   - When stream closed but in question mode: `debug!("Stream closed but in question mode, waiting for user input");`

6. In `handle_input_key()`:
   - When Ctrl+D or Ctrl+Enter submits: `debug!("Submit keybinding detected, exiting question mode");`

7. In src/subprocess/runner.rs, add `use tracing::trace;` at top.

8. In `ClaudeRunner::next_output()`:
   - When stdout line received: `trace!(line_len = %line.len(), "Received stdout from subprocess");`
   - When stderr line received: `trace!(line_len = %line.len(), "Received stderr from subprocess");`
   - When stdout/stderr done: `trace!("Subprocess stdout/stderr stream closed");`

9. In `ClaudeRunner::write_stdin()`:
   - Before write: `trace!(response_len = %response.len(), "Writing to subprocess stdin");`
  </action>
  <verify>
Run `cargo build` to ensure all tracing macros compile correctly.
Run `cargo test` to verify existing tests still pass.
  </verify>
  <done>
plan_tui.rs and runner.rs contain debug/trace logging for input mode transitions and subprocess I/O. Logging can be enabled with RUST_LOG=rslph=debug or RUST_LOG=rslph=trace.
  </done>
</task>

</tasks>

<verification>
1. `cargo build` succeeds with no errors
2. `cargo test` passes all tests
3. Running `RUST_LOG=rslph=trace rslph plan --tui --adaptive "test"` produces detailed trace output showing:
   - Raw stdout lines from Claude subprocess
   - Parsed stream events with types
   - Session ID extraction when it occurs
   - AskUserQuestion detection when Claude asks questions
   - Input mode transitions in TUI
</verification>

<success_criteria>
- All 3 tasks complete with no build errors
- Test suite continues to pass
- Debug/trace logging is available via RUST_LOG environment variable
- Logging captures the complete flow from subprocess stdout to TUI input mode
</success_criteria>

<output>
After completion, create `.planning/quick/019-add-thorough-logging-for-interactive-q-a/019-SUMMARY.md`
</output>

---
phase: quick-011
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/subprocess/runner.rs
  - src/subprocess/stream_json.rs
  - src/tui/event.rs
  - src/build/iteration.rs
autonomous: true

must_haves:
  truths:
    - "User questions from Claude CLI are detected in stream-json output"
    - "Questions are relayed to TUI for display"
    - "User can type responses that are sent back to Claude CLI"
  artifacts:
    - path: "src/subprocess/runner.rs"
      provides: "Stdin piping and write capability"
    - path: "src/subprocess/stream_json.rs"
      provides: "Question event detection"
    - path: "src/tui/event.rs"
      provides: "InputRequired event type"
  key_links:
    - from: "src/subprocess/stream_json.rs"
      to: "src/build/iteration.rs"
      via: "is_input_required() method"
    - from: "src/subprocess/runner.rs"
      to: "src/build/iteration.rs"
      via: "write_stdin() method"
---

<objective>
Implement stdin relay for Claude CLI interactive questions.

Purpose: When Claude CLI sends an interactive question (using AskUserQuestion tool or similar), rslph needs to capture it from stream-json output and relay it to the user via TUI, then send the user's response back to Claude CLI's stdin.

Output: Modified subprocess runner with stdin piping capability, question detection in stream parser, and TUI event for input requirements.
</objective>

<context>
@.planning/STATE.md
@src/subprocess/runner.rs
@src/subprocess/stream_json.rs
@src/tui/event.rs
@src/build/iteration.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Enable stdin piping in ClaudeRunner</name>
  <files>src/subprocess/runner.rs</files>
  <action>
  1. Change `.stdin(Stdio::null())` to `.stdin(Stdio::piped())` in the spawn method (line 62)

  2. Add a new field to ClaudeRunner struct to hold the stdin handle:
     ```rust
     stdin: Option<ChildStdin>,
     ```

  3. In spawn(), take the stdin handle after spawn:
     ```rust
     let stdin = child.stdin.take();
     ```

  4. Add a new method to write to stdin:
     ```rust
     /// Write a response to the subprocess stdin.
     ///
     /// Used to send user responses to Claude CLI when it asks interactive questions.
     /// Appends a newline after the response.
     pub async fn write_stdin(&mut self, response: &str) -> std::io::Result<()> {
         if let Some(ref mut stdin) = self.stdin {
             use tokio::io::AsyncWriteExt;
             stdin.write_all(response.as_bytes()).await?;
             stdin.write_all(b"\n").await?;
             stdin.flush().await?;
         }
         Ok(())
     }
     ```

  5. Import ChildStdin:
     ```rust
     use tokio::process::{Child, ChildStderr, ChildStdout, ChildStdin, Command};
     ```
  </action>
  <verify>
  Run `cargo check` - should compile without errors.
  Run `cargo test --lib subprocess::runner` - existing tests should pass.
  </verify>
  <done>ClaudeRunner can pipe stdin and write responses to subprocess</done>
</task>

<task type="auto">
  <name>Task 2: Add question detection to stream parser</name>
  <files>src/subprocess/stream_json.rs, src/tui/event.rs</files>
  <action>
  1. In stream_json.rs, add a method to StreamEvent to detect input-required events:
     ```rust
     /// Check if this event requires user input.
     ///
     /// Claude CLI uses various mechanisms to ask for user input:
     /// - "result" type events with content requesting input
     /// - Tool results that indicate waiting for user
     pub fn is_input_required(&self) -> Option<String> {
         // Check for result events that contain questions
         if self.event_type == "result" {
             if let Some(ref message) = self.message {
                 if let MessageContent::Text(text) = &message.content {
                     // Check if the text looks like a question
                     if text.contains("?") || text.to_lowercase().contains("please") {
                         return Some(text.clone());
                     }
                 }
             }
         }

         // Check for tool_result blocks that indicate input needed
         if let Some(ref message) = self.message {
             if let MessageContent::Blocks(blocks) = &message.content {
                 for block in blocks {
                     if block.block_type == "tool_result" {
                         if let Some(ref text) = block.text {
                             if text.contains("waiting for") || text.contains("input required") {
                                 return Some(text.clone());
                             }
                         }
                     }
                 }
             }
         }

         None
     }
     ```

  2. In src/tui/event.rs, add a new SubprocessEvent variant:
     ```rust
     /// Claude CLI is waiting for user input.
     InputRequired { question: String },
     ```

  3. Add the conversion in the From impl:
     ```rust
     SubprocessEvent::InputRequired { question } => AppEvent::InputRequired { question },
     ```

  4. In src/tui/mod.rs (or wherever AppEvent is defined), add corresponding AppEvent variant:
     ```rust
     /// Claude CLI requires user input.
     InputRequired { question: String },
     ```
  </action>
  <verify>
  Run `cargo check` - should compile without errors.
  Add a unit test in stream_json.rs:
  ```rust
  #[test]
  fn test_is_input_required() {
      let json = r#"{"type":"result","message":{"content":"What is your name?"}}"#;
      let event = StreamEvent::parse(json).expect("should parse");
      assert!(event.is_input_required().is_some());
  }
  ```
  </verify>
  <done>Stream parser can detect when Claude CLI asks for input</done>
</task>

<task type="auto">
  <name>Task 3: Wire question detection to TUI and add input handling</name>
  <files>src/build/iteration.rs, src/tui/app.rs</files>
  <action>
  1. In src/build/iteration.rs, modify parse_and_stream_line() to detect input requirements:
     After parsing the event, add:
     ```rust
     // Check if Claude is asking for user input
     if let Some(question) = event.is_input_required() {
         let _ = tui_tx.send(SubprocessEvent::InputRequired { question });
     }
     ```

  2. In src/tui/app.rs, add state for input mode:
     ```rust
     /// Whether we're currently waiting for user input
     pub input_mode: bool,
     /// The current input buffer
     pub input_buffer: String,
     /// The question being answered
     pub current_question: Option<String>,
     ```

  3. Add methods to App for input handling:
     ```rust
     /// Enter input mode to answer a question.
     pub fn enter_input_mode(&mut self, question: String) {
         self.input_mode = true;
         self.input_buffer.clear();
         self.current_question = Some(question);
     }

     /// Exit input mode and get the response.
     pub fn submit_input(&mut self) -> Option<String> {
         if self.input_mode {
             self.input_mode = false;
             let response = std::mem::take(&mut self.input_buffer);
             self.current_question = None;
             Some(response)
         } else {
             None
         }
     }

     /// Handle a character input in input mode.
     pub fn handle_input_char(&mut self, c: char) {
         if self.input_mode {
             self.input_buffer.push(c);
         }
     }

     /// Handle backspace in input mode.
     pub fn handle_input_backspace(&mut self) {
         if self.input_mode {
             self.input_buffer.pop();
         }
     }
     ```

  4. Note: The actual keyboard event handling and stdin writing will need to be wired in the TUI run loop (src/tui/run.rs), but that's a follow-up task. This task establishes the foundation.
  </action>
  <verify>
  Run `cargo check` - should compile without errors.
  Run `cargo test` - all tests should pass.
  </verify>
  <done>TUI has input mode state and methods for handling user input to Claude CLI questions</done>
</task>

</tasks>

<verification>
1. `cargo check` passes
2. `cargo test` passes
3. `cargo clippy` shows no new warnings
</verification>

<success_criteria>
- ClaudeRunner has stdin piped instead of null
- ClaudeRunner has write_stdin() method
- StreamEvent has is_input_required() method
- SubprocessEvent has InputRequired variant
- App has input_mode state and input handling methods
- All tests pass
</success_criteria>

<output>
After completion, create `.planning/quick/011-implement-stdin-relay-for-claude-cli-int/011-SUMMARY.md`
</output>

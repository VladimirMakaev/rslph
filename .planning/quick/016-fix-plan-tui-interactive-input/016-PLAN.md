---
phase: quick
plan: 016
type: execute
wave: 1
depends_on: []
files_modified:
  - src/planning/command.rs
  - src/tui/plan_tui.rs
autonomous: true

must_haves:
  truths:
    - "User can type responses when Claude asks questions in plan TUI"
    - "Typed response is sent to Claude subprocess stdin"
    - "Plan TUI displays input prompt when Claude asks a question"
  artifacts:
    - path: "src/planning/command.rs"
      provides: "spawn_interactive() call, ClaudeRunner passed to TUI"
    - path: "src/tui/plan_tui.rs"
      provides: "InputRequired event, input state, keyboard handling, input rendering"
  key_links:
    - from: "src/tui/plan_tui.rs"
      to: "ClaudeRunner.write_stdin()"
      via: "submit_input triggers write_stdin call"
---

<objective>
Fix plan TUI to handle interactive input when Claude asks questions during planning.

Purpose: Currently the plan TUI spawns Claude with stdin set to null and has no way to
send user responses when Claude asks clarifying questions. This blocks interactive planning.

Output: Working interactive input in plan TUI - user can type responses and submit with Enter.
</objective>

<execution_context>
@/Users/vmakaev/.claude/get-shit-done/workflows/execute-plan.md
@/Users/vmakaev/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@src/planning/command.rs - run_tui_planning() spawns Claude, needs spawn_interactive
@src/tui/plan_tui.rs - needs InputRequired event, input state, keyboard handling
@src/subprocess/runner.rs - has spawn_interactive() and write_stdin() ready to use
@src/subprocess/stream_json.rs - has is_input_required() for question detection
@src/tui/app.rs - reference for input_mode pattern (lines 356-394, 926-962)
</context>

<tasks>

<task type="auto">
  <name>Task 1: Use spawn_interactive and pass ClaudeRunner to TUI</name>
  <files>src/planning/command.rs</files>
  <action>
In run_tui_planning() (line 325):

1. Change ClaudeRunner::spawn() to ClaudeRunner::spawn_interactive() to enable piped stdin:
   ```rust
   let mut runner = ClaudeRunner::spawn_interactive(&config.claude_cmd.command, &combined_args, working_dir)
   ```

2. The stream loop needs to pass the runner to the TUI so it can call write_stdin().
   However, the current architecture spawns TUI as a separate task. Instead:
   - Keep the runner in the main loop (don't move to TUI)
   - Add a new channel for sending input responses FROM TUI TO main loop
   - Main loop receives responses and calls runner.write_stdin()

   Create a new channel pair before spawning TUI:
   ```rust
   let (input_tx, mut input_rx) = mpsc::unbounded_channel::<String>();
   ```

   Pass input_tx to run_plan_tui() alongside event_rx.

3. In the stream loop, add a branch to receive from input_rx and write to runner:
   ```rust
   response = input_rx.recv() => {
       if let Some(text) = response {
           if let Err(e) = runner.write_stdin(&text).await {
               logger.log(&format!("write_stdin error: {}", e));
           }
       }
   }
   ```

4. When forwarding StreamEvents, check is_input_required() and send InputRequired event:
   ```rust
   if let Some(question) = event.is_input_required() {
       let _ = event_tx.send(PlanTuiEvent::InputRequired(question));
   }
   ```
  </action>
  <verify>cargo check passes</verify>
  <done>run_tui_planning uses spawn_interactive, has input channel, forwards InputRequired events</done>
</task>

<task type="auto">
  <name>Task 2: Add input handling to PlanTuiState and event loop</name>
  <files>src/tui/plan_tui.rs</files>
  <action>
1. Add InputRequired variant to PlanTuiEvent enum:
   ```rust
   /// Claude CLI is asking for user input.
   InputRequired(String),
   ```

2. Add input state fields to PlanTuiState struct (follow app.rs pattern):
   ```rust
   /// Whether we're in input mode waiting for user response.
   pub input_mode: bool,
   /// Current input buffer for user typing.
   pub input_buffer: String,
   /// The question being answered.
   pub current_question: Option<String>,
   ```

3. Initialize these fields in PlanTuiState::new():
   ```rust
   input_mode: false,
   input_buffer: String::new(),
   current_question: None,
   ```

4. Add input mode methods to PlanTuiState (copy from app.rs pattern):
   ```rust
   pub fn enter_input_mode(&mut self, question: String) {
       self.input_mode = true;
       self.input_buffer.clear();
       self.current_question = Some(question);
   }

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

   pub fn handle_input_char(&mut self, c: char) {
       if self.input_mode {
           self.input_buffer.push(c);
       }
   }

   pub fn handle_input_backspace(&mut self) {
       if self.input_mode {
           self.input_buffer.pop();
       }
   }
   ```

5. In PlanTuiState::update(), handle InputRequired event:
   ```rust
   PlanTuiEvent::InputRequired(question) => {
       self.enter_input_mode(question);
   }
   ```

6. Update run_plan_tui() signature to accept input_tx:
   ```rust
   pub async fn run_plan_tui(
       event_rx: mpsc::UnboundedReceiver<PlanTuiEvent>,
       input_tx: mpsc::UnboundedSender<String>,
       cancel_token: CancellationToken,
   ) -> Result<PlanTuiState, RslphError>
   ```

7. In the keyboard event handler, add character input handling:
   - When in input_mode and a character key is pressed: call state.handle_input_char(c)
   - When in input_mode and Backspace is pressed: call state.handle_input_backspace()
   - When in input_mode and Enter is pressed: submit and send to input_tx

   Use crossterm's KeyCode matching. The EventHandler.next() returns AppEvent, so we need
   to check if we're in input_mode BEFORE processing as navigation. Add special handling:

   ```rust
   kbd_event = kbd_handler.next() => {
       if let Some(app_event) = kbd_event {
           // In input mode, intercept character events
           if state.input_mode {
               match app_event {
                   crate::tui::AppEvent::Quit => {
                       // Escape from input mode, or quit if pressed again
                       if state.input_mode {
                           state.input_mode = false;
                           state.input_buffer.clear();
                           state.current_question = None;
                       }
                   }
                   _ => {
                       // Other keys handled via raw crossterm below
                   }
               }
           } else {
               // Normal mode handling (existing code)
               match app_event {
                   crate::tui::AppEvent::Quit => { ... }
                   crate::tui::AppEvent::ScrollUp => { ... }
                   crate::tui::AppEvent::ScrollDown => { ... }
                   _ => {}
               }
           }
       }
   }
   ```

   IMPORTANT: The EventHandler only returns AppEvents (Quit, ScrollUp, ScrollDown, etc).
   For character input, we need to read raw crossterm events. Add a separate poll for
   raw keyboard events when in input_mode:

   ```rust
   // In the select! loop, add raw terminal event handling for input mode
   _ = async {
       if state.input_mode {
           if crossterm::event::poll(std::time::Duration::from_millis(10)).unwrap_or(false) {
               if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                   match key.code {
                       crossterm::event::KeyCode::Char(c) => state.handle_input_char(c),
                       crossterm::event::KeyCode::Backspace => state.handle_input_backspace(),
                       crossterm::event::KeyCode::Enter => {
                           if let Some(response) = state.submit_input() {
                               let _ = input_tx.send(response);
                           }
                       }
                       crossterm::event::KeyCode::Esc => {
                           state.input_mode = false;
                           state.input_buffer.clear();
                           state.current_question = None;
                       }
                       _ => {}
                   }
               }
           }
       }
   } => {}
   ```

   Actually, this won't work well with tokio::select! because crossterm::event::poll is blocking.

   Better approach: Use the existing EventHandler pattern but extract raw events.
   Check src/tui/event.rs - it already converts crossterm events. Modify to pass through
   Char events when in a mode that needs them.

   SIMPLEST approach: Check if EventHandler supports character passthrough. If not,
   poll crossterm directly in a non-blocking way using tokio::task::spawn_blocking or
   check if there's already async support.

   Looking at event.rs more carefully - EventHandler uses crossterm's EventStream which is async.
   The event loop already handles Key events. The issue is EventHandler.next() only returns
   AppEvent variants it knows about.

   Best solution: Add new AppEvent variants for character input, OR handle raw crossterm
   events in run_plan_tui directly instead of going through EventHandler.

   For simplicity, handle raw events directly in run_plan_tui when input_mode is true:
   - Add `use crossterm::event::{Event, KeyCode, KeyEvent};`
   - Use `crossterm::event::EventStream` directly for input mode

   Actually, even simpler: The kbd_handler already polls crossterm. We just need to
   NOT consume events through kbd_handler when in input_mode, and instead read them
   ourselves. But that's complicated.

   SIMPLEST: Add an InputChar(char), InputBackspace, InputSubmit variants to AppEvent,
   and modify EventHandler to emit these for Key events. Then handle them in plan_tui.

   OR: Just use crossterm's async EventStream directly in run_plan_tui, skip EventHandler.
   This is the cleanest since plan_tui is simpler than build_tui.

   Implement direct crossterm handling:
   ```rust
   use crossterm::event::{EventStream, Event, KeyCode, KeyModifiers};
   use futures::StreamExt;

   let mut event_stream = EventStream::new();

   // In select loop:
   term_event = event_stream.next() => {
       if let Some(Ok(Event::Key(key))) = term_event {
           if state.input_mode {
               match key.code {
                   KeyCode::Char(c) => state.handle_input_char(c),
                   KeyCode::Backspace => state.handle_input_backspace(),
                   KeyCode::Enter => {
                       if let Some(response) = state.submit_input() {
                           let _ = input_tx.send(response);
                       }
                   }
                   KeyCode::Esc => {
                       state.input_mode = false;
                       state.input_buffer.clear();
                       state.current_question = None;
                   }
                   _ => {}
               }
           } else {
               // Normal navigation mode
               match (key.code, key.modifiers) {
                   (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                       state.should_quit = true;
                       cancel_token.cancel();
                       break;
                   }
                   (KeyCode::PageUp, _) => {
                       state.scroll_offset = state.scroll_offset.saturating_sub(10);
                   }
                   (KeyCode::PageDown, _) => {
                       state.scroll_offset = (state.scroll_offset + 10)
                           .min(state.conversation.len().saturating_sub(1));
                   }
                   _ => {}
               }
           }
       }
   }
   ```

   Remove the kbd_handler entirely and use direct crossterm EventStream.
  </action>
  <verify>cargo check passes</verify>
  <done>PlanTuiState has input_mode/buffer/question fields and methods, event loop handles input</done>
</task>

<task type="auto">
  <name>Task 3: Render input prompt in plan TUI</name>
  <files>src/tui/plan_tui.rs</files>
  <action>
1. Modify render_footer() to show input prompt when in input_mode:
   ```rust
   fn render_footer(frame: &mut Frame, area: Rect, state: &PlanTuiState) {
       if state.input_mode {
           // Render input prompt
           let question = state.current_question.as_deref().unwrap_or("Input required:");
           let input_text = format!("{}\n> {}_", question, state.input_buffer);

           let footer = Paragraph::new(input_text)
               .style(Style::default().fg(Color::Yellow))
               .block(Block::default()
                   .borders(Borders::TOP)
                   .title("Answer (Enter to submit, Esc to cancel)")
                   .border_style(Style::default().fg(Color::Yellow)));
           frame.render_widget(footer, area);
       } else {
           // Existing plan preview rendering
           let preview_lines: Vec<Line> = state
               .plan_preview
               .lines()
               .rev()
               .take(3)
               ...
       }
   }
   ```

2. Update render_header() to show "Waiting for input..." status when in input_mode:
   ```rust
   // At the start of status determination:
   let (status_text, status_color) = if state.input_mode {
       ("Waiting for your input...", Color::Yellow)
   } else if state.stderr_without_stdout > 0 ... {
       // existing code
   }
   ```

3. Consider making footer taller when in input mode to show question + input. Adjust layout:
   ```rust
   let footer_height = if state.input_mode { 6 } else { 5 };
   let [header_area, main_area, footer_area] = Layout::vertical([
       Constraint::Length(3),
       Constraint::Min(10),
       Constraint::Length(footer_height),
   ])
   .areas(area);
   ```

   Actually, the layout is in render_plan_tui(), not render_footer(). Update render_plan_tui():
   ```rust
   pub fn render_plan_tui(frame: &mut Frame, state: &PlanTuiState) {
       let area = frame.area();
       let footer_height = if state.input_mode { 6 } else { 5 };

       let [header_area, main_area, footer_area] = Layout::vertical([
           Constraint::Length(3),
           Constraint::Min(10),
           Constraint::Length(footer_height),
       ])
       .areas(area);
       // ... rest unchanged
   }
   ```

4. Add tests for input mode rendering:
   ```rust
   #[test]
   fn test_enter_input_mode() {
       let mut state = PlanTuiState::new();
       state.enter_input_mode("What is your name?".to_string());
       assert!(state.input_mode);
       assert_eq!(state.current_question, Some("What is your name?".to_string()));
       assert!(state.input_buffer.is_empty());
   }

   #[test]
   fn test_handle_input_char() {
       let mut state = PlanTuiState::new();
       state.enter_input_mode("Question".to_string());
       state.handle_input_char('a');
       state.handle_input_char('b');
       assert_eq!(state.input_buffer, "ab");
   }

   #[test]
   fn test_handle_input_backspace() {
       let mut state = PlanTuiState::new();
       state.enter_input_mode("Question".to_string());
       state.handle_input_char('a');
       state.handle_input_char('b');
       state.handle_input_backspace();
       assert_eq!(state.input_buffer, "a");
   }

   #[test]
   fn test_submit_input() {
       let mut state = PlanTuiState::new();
       state.enter_input_mode("Question".to_string());
       state.handle_input_char('y');
       state.handle_input_char('e');
       state.handle_input_char('s');
       let response = state.submit_input();
       assert_eq!(response, Some("yes".to_string()));
       assert!(!state.input_mode);
       assert!(state.current_question.is_none());
   }

   #[test]
   fn test_submit_input_not_in_input_mode() {
       let mut state = PlanTuiState::new();
       let response = state.submit_input();
       assert_eq!(response, None);
   }
   ```
  </action>
  <verify>cargo test --lib plan_tui passes, cargo check passes</verify>
  <done>Input prompt renders when in input mode, tests pass</done>
</task>

</tasks>

<verification>
1. `cargo check` - compiles without errors
2. `cargo test --lib plan_tui` - all plan_tui tests pass including new input tests
3. `cargo test` - all tests pass
4. Manual test: Run `rslph plan --tui "something vague"` with adaptive mode that triggers questions
</verification>

<success_criteria>
- Plan TUI uses spawn_interactive() for piped stdin
- InputRequired event type exists and is forwarded when Claude asks questions
- PlanTuiState has input_mode, input_buffer, current_question fields
- Keyboard input (characters, backspace, enter, esc) works in input mode
- Input prompt renders in footer when in input mode
- Submitted input is sent to Claude subprocess via write_stdin()
- All existing tests continue to pass
</success_criteria>

<output>
After completion, create `.planning/quick/016-fix-plan-tui-interactive-input/016-SUMMARY.md`
</output>

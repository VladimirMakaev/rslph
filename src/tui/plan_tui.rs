//! TUI mode for the plan command.
//!
//! Provides streaming display of LLM output during planning, including
//! thinking blocks, tool calls, and generated plan preview.
//!
//! Also supports interactive Q&A when Claude asks clarifying questions.

use std::collections::HashMap;
use std::time::Instant;

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyModifiers};
use futures::StreamExt;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace};

use crate::error::RslphError;
use crate::subprocess::StreamEvent;
use crate::tui::conversation::{render_conversation, ConversationBuffer, ConversationItem};
use crate::tui::terminal::{init_terminal, restore_terminal};

/// Input mode for the plan TUI.
///
/// Determines how keyboard input is handled.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum InputMode {
    /// Normal viewing mode - keyboard navigates and scrolls.
    #[default]
    Normal,
    /// User is answering questions - keyboard input goes to text buffer.
    AnsweringQuestions,
}

/// Events that can be sent to the plan TUI.
#[derive(Debug, Clone)]
pub enum PlanTuiEvent {
    /// A parsed stream event from Claude.
    Stream(Box<StreamEvent>),
    /// Raw stdout line that couldn't be parsed as JSON.
    RawStdout(String),
    /// Stderr line from Claude.
    Stderr(String),
    /// Claude asked clarifying questions during planning.
    QuestionsAsked {
        /// The questions Claude is asking.
        questions: Vec<String>,
        /// Session ID for resuming the session with answers.
        session_id: String,
    },
}

/// Status of the planning operation.
#[derive(Debug, Clone)]
pub enum PlanStatus {
    /// Detecting project stack and technologies.
    StackDetection,
    /// Generating plan from user input.
    Planning,
    /// Generating a project name.
    GeneratingName,
    /// Awaiting user input (answering questions).
    AwaitingInput,
    /// Resuming session with user answers.
    ResumingSession,
    /// Planning complete.
    Complete,
    /// Planning failed with error.
    Failed(String),
}

/// State for the plan TUI.
#[derive(Debug)]
pub struct PlanTuiState {
    /// Conversation history buffer.
    pub conversation: ConversationBuffer,
    /// Scroll offset for conversation view.
    pub scroll_offset: usize,
    /// Generated plan preview (accumulated text output).
    pub plan_preview: String,
    /// Current planning status.
    pub status: PlanStatus,
    /// Start time for elapsed display.
    pub start_time: Instant,
    /// Flag indicating user requested quit.
    pub should_quit: bool,
    /// Count of stderr lines received without any stdout.
    pub stderr_without_stdout: usize,
    /// Whether we've received any stdout event.
    pub has_stdout: bool,

    // Input mode state for Q&A
    /// Current input mode (Normal or AnsweringQuestions).
    pub input_mode: InputMode,
    /// Questions waiting for answers.
    pub pending_questions: Vec<String>,
    /// Current user input buffer.
    pub input_buffer: String,
    /// Session ID for resuming after answers.
    pub session_id: Option<String>,
    /// Flag indicating answers have been submitted.
    pub answers_submitted: bool,
}

impl Default for PlanTuiState {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanTuiState {
    /// Create a new plan TUI state.
    pub fn new() -> Self {
        Self {
            conversation: ConversationBuffer::new(500),
            scroll_offset: 0,
            plan_preview: String::new(),
            status: PlanStatus::StackDetection,
            start_time: Instant::now(),
            should_quit: false,
            stderr_without_stdout: 0,
            has_stdout: false,
            input_mode: InputMode::Normal,
            pending_questions: Vec::new(),
            input_buffer: String::new(),
            session_id: None,
            answers_submitted: false,
        }
    }

    /// Update state from a plan TUI event.
    pub fn update(&mut self, event: &PlanTuiEvent) {
        match event {
            PlanTuiEvent::Stream(stream_event) => {
                self.has_stdout = true;
                let items = stream_event.extract_conversation_items();
                trace!(has_items = %items.len(), "Received stream event in TUI");
                // Extract conversation items from the stream event
                for item in items {
                    // If it's text, also append to plan preview
                    if let ConversationItem::Text(ref text) = item {
                        self.plan_preview.push_str(text);
                        self.plan_preview.push('\n');
                    }
                    self.conversation.push(item);
                }
            }
            PlanTuiEvent::RawStdout(line) => {
                self.has_stdout = true;
                // Display raw stdout as system message
                self.conversation
                    .push(ConversationItem::System(format!("[stdout] {}", line)));
            }
            PlanTuiEvent::Stderr(line) => {
                // Track stderr without stdout
                if !self.has_stdout {
                    self.stderr_without_stdout += 1;
                }
                // Display stderr as system message
                self.conversation
                    .push(ConversationItem::System(format!("[stderr] {}", line)));
            }
            PlanTuiEvent::QuestionsAsked {
                questions,
                session_id,
            } => {
                debug!("Received QuestionsAsked event, transitioning to input mode");
                self.enter_question_mode(questions.clone(), session_id.clone());
            }
        }

        // Auto-scroll to bottom (keep recent items visible)
        self.scroll_offset = self.conversation.len().saturating_sub(15);

        // Update status based on what we're seeing
        if matches!(self.status, PlanStatus::StackDetection) && !matches!(self.input_mode, InputMode::AnsweringQuestions) {
            self.status = PlanStatus::Planning;
        }
    }

    /// Enter question-answering mode.
    ///
    /// Switches TUI to input mode, stores questions and session ID.
    pub fn enter_question_mode(&mut self, questions: Vec<String>, session_id: String) {
        debug!(question_count = %questions.len(), session_id = %session_id, "Entering question-answering mode");
        self.input_mode = InputMode::AnsweringQuestions;
        self.pending_questions = questions;
        self.session_id = Some(session_id);
        self.input_buffer.clear();
        self.status = PlanStatus::AwaitingInput;
        self.answers_submitted = false;
    }

    /// Exit question-answering mode, keeping answers in input_buffer.
    ///
    /// Sets answers_submitted flag. The caller can access the answers
    /// via self.input_buffer.
    pub fn exit_question_mode(&mut self) {
        debug!(answer_len = %self.input_buffer.len(), "Exiting question-answering mode, answers submitted");
        self.input_mode = InputMode::Normal;
        self.answers_submitted = true;
        self.status = PlanStatus::ResumingSession;
        // Note: input_buffer is preserved for the command to read
    }

    /// Handle a character input in question mode.
    pub fn handle_input_char(&mut self, c: char) {
        if matches!(self.input_mode, InputMode::AnsweringQuestions) {
            self.input_buffer.push(c);
        }
    }

    /// Handle backspace in question mode.
    pub fn handle_input_backspace(&mut self) {
        if matches!(self.input_mode, InputMode::AnsweringQuestions) {
            self.input_buffer.pop();
        }
    }

    /// Handle enter (newline) in question mode.
    pub fn handle_input_newline(&mut self) {
        if matches!(self.input_mode, InputMode::AnsweringQuestions) {
            self.input_buffer.push('\n');
        }
    }

    /// Set status to failed with error message.
    pub fn set_failed(&mut self, error: String) {
        self.status = PlanStatus::Failed(error);
    }

    /// Set status to complete.
    pub fn set_complete(&mut self) {
        self.status = PlanStatus::Complete;
    }

    /// Check if currently in question-answering mode.
    pub fn is_answering_questions(&self) -> bool {
        matches!(self.input_mode, InputMode::AnsweringQuestions)
    }

    /// Get the session ID if available.
    pub fn get_session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
}

/// Render the plan TUI to the frame.
pub fn render_plan_tui(frame: &mut Frame, state: &PlanTuiState) {
    let area = frame.area();

    // If in AnsweringQuestions mode, render the question input interface
    if matches!(state.input_mode, InputMode::AnsweringQuestions) {
        render_question_input(frame, area, state);
        return;
    }

    // Normal mode: Split: top for status, middle for conversation, bottom for plan preview
    let [header_area, main_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(5),
    ])
    .areas(area);

    // Render header with status and elapsed time
    render_header(frame, header_area, state);

    // Render conversation view (plan TUI doesn't support collapse, pass empty map)
    let empty_collapsed: HashMap<usize, bool> = HashMap::new();
    render_conversation(
        frame,
        main_area,
        state.conversation.items(),
        state.scroll_offset,
        &empty_collapsed,
    );

    // Render plan preview footer
    render_footer(frame, footer_area, state);
}

/// Render the question input interface when in AnsweringQuestions mode.
fn render_question_input(frame: &mut Frame, area: Rect, state: &PlanTuiState) {
    // Layout for question input mode:
    // - Top: Header with status "Answering Questions..."
    // - Middle-top: Questions box (scrollable)
    // - Middle-bottom: Input text area with cursor
    // - Bottom: Instructions footer
    let [header_area, questions_area, input_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Percentage(40),
        Constraint::Percentage(40),
        Constraint::Length(3),
    ])
    .areas(area);

    // Render header with "Answering Questions" status
    let elapsed = state.start_time.elapsed().as_secs();
    let header = Paragraph::new(vec![Line::from(vec![
        Span::styled("Plan ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            "| Answering Questions... ",
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(format!("| {}s", elapsed), Style::default().fg(Color::Cyan)),
    ])])
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, header_area);

    // Render questions box with yellow border (indicating input mode)
    let question_lines: Vec<Line> = state
        .pending_questions
        .iter()
        .enumerate()
        .map(|(i, q)| {
            Line::from(vec![
                Span::styled(
                    format!("{}. ", i + 1),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(q),
            ])
        })
        .collect();

    let questions_box = Paragraph::new(question_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title("Questions from Claude"),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(questions_box, questions_area);

    // Render input area with cursor indicator
    let input_display = if state.input_buffer.is_empty() {
        vec![Line::from(Span::styled(
            "Type your answers here...",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        // Show the input buffer with a cursor at the end
        let mut lines: Vec<Line> = state
            .input_buffer
            .lines()
            .map(|l| Line::from(l.to_string()))
            .collect();

        // Add cursor to the last line
        if lines.is_empty() {
            lines.push(Line::from("_"));
        } else if let Some(last) = lines.last_mut() {
            *last = Line::from(format!("{}_", last.to_string()));
        }
        lines
    };

    let input_box = Paragraph::new(input_display)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title("Your Answers"),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(input_box, input_area);

    // Render footer with instructions
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": new line  "),
        Span::styled("Ctrl+Enter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" or "),
        Span::styled("Ctrl+D", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": submit  "),
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": cancel"),
    ]))
    .block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, footer_area);
}

/// Render the header showing status and elapsed time.
fn render_header(frame: &mut Frame, area: Rect, state: &PlanTuiState) {
    let elapsed = state.start_time.elapsed().as_secs();

    // Determine status text - show warning if stderr received but no stdout
    let (status_text, status_color) = if state.stderr_without_stdout > 0
        && !state.has_stdout
        && elapsed > 5
        && !matches!(state.status, PlanStatus::Complete | PlanStatus::Failed(_))
    {
        // Waiting scenario: received stderr but no stdout after 5 seconds
        ("Waiting for Claude... (check stderr above)", Color::Magenta)
    } else {
        match &state.status {
            PlanStatus::StackDetection => ("Detecting project stack...", Color::Yellow),
            PlanStatus::Planning => ("Generating plan...", Color::Yellow),
            PlanStatus::GeneratingName => ("Generating project name...", Color::Yellow),
            PlanStatus::AwaitingInput => ("Awaiting user input...", Color::Cyan),
            PlanStatus::ResumingSession => ("Resuming session...", Color::Yellow),
            PlanStatus::Complete => ("Complete!", Color::Green),
            PlanStatus::Failed(e) => (e.as_str(), Color::Red),
        }
    };

    let header = Paragraph::new(vec![Line::from(vec![
        Span::styled("Plan ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("| {} ", status_text),
            Style::default().fg(status_color),
        ),
        Span::styled(format!("| {}s", elapsed), Style::default().fg(Color::Cyan)),
    ])])
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, area);
}

/// Render the footer showing plan preview.
fn render_footer(frame: &mut Frame, area: Rect, state: &PlanTuiState) {
    // Get last few lines of the plan preview
    let preview_lines: Vec<Line> = state
        .plan_preview
        .lines()
        .rev()
        .take(3)
        .map(|l| Line::from(l.to_string()))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let footer = Paragraph::new(preview_lines).block(
        Block::default()
            .borders(Borders::TOP)
            .title("Plan Preview (q to quit, PgUp/PgDn to scroll)"),
    );
    frame.render_widget(footer, area);
}

/// Run the plan TUI event loop.
///
/// Receives stream events and renders them until the stream completes
/// or the user quits.
///
/// # Arguments
///
/// * `event_rx` - Receiver for plan TUI events from Claude
/// * `cancel_token` - Token for graceful cancellation
///
/// # Returns
///
/// The final TUI state, which includes whether the user quit.
/// If answers were submitted, state.answers_submitted will be true and
/// the answers can be retrieved via state.input_buffer (before exit_question_mode).
pub async fn run_plan_tui(
    event_rx: mpsc::UnboundedReceiver<PlanTuiEvent>,
    cancel_token: CancellationToken,
) -> Result<PlanTuiState, RslphError> {
    let mut terminal = init_terminal()
        .map_err(|e| RslphError::Subprocess(format!("Terminal init failed: {}", e)))?;

    let mut state = PlanTuiState::new();
    let mut event_rx = event_rx;

    // Create crossterm event stream directly for raw key handling
    let mut crossterm_events = EventStream::new();
    let tick_duration = std::time::Duration::from_millis(33); // ~30 FPS
    let mut render_interval = tokio::time::interval(tick_duration);
    let mut stream_closed = false; // Track when event channel is closed

    loop {
        // Render current state
        terminal
            .draw(|frame| render_plan_tui(frame, &state))
            .map_err(|e| RslphError::Subprocess(format!("Render failed: {}", e)))?;

        // Check if answers were submitted - exit the loop to allow command to resume
        if state.answers_submitted {
            debug!("Answers submitted, exiting TUI event loop");
            break;
        }

        tokio::select! {
            biased;

            // Check for cancellation
            _ = cancel_token.cancelled() => {
                state.set_failed("Cancelled".to_string());
                break;
            }

            // Stream events from Claude (only poll if not closed)
            stream_event = event_rx.recv(), if !stream_closed => {
                match stream_event {
                    Some(event) => {
                        state.update(&event);
                    }
                    None => {
                        // Stream complete - mark as closed to stop polling
                        stream_closed = true;
                        // If we're not in question mode, we're done
                        if !state.is_answering_questions() {
                            state.set_complete();
                            break;
                        }
                        // If in question mode, continue waiting for user input
                        debug!("Stream closed but in question mode, waiting for user input");
                    }
                }
            }

            // Raw keyboard events from crossterm
            crossterm_event = crossterm_events.next() => {
                match crossterm_event {
                    Some(Ok(CrosstermEvent::Key(key))) => {
                        if state.is_answering_questions() {
                            // Handle text input mode
                            handle_input_key(&mut state, key.code, key.modifiers);
                        } else {
                            // Handle normal navigation mode
                            handle_navigation_key(&mut state, key.code, key.modifiers, &cancel_token);
                        }

                        if state.should_quit {
                            cancel_token.cancel();
                            break;
                        }
                    }
                    Some(Ok(_)) => {
                        // Other events (mouse, resize) - ignore
                    }
                    Some(Err(_)) => {
                        // Event read error - continue
                    }
                    None => {
                        // Stream ended
                        break;
                    }
                }
            }

            // Render tick (keeps UI responsive)
            _ = render_interval.tick() => {
                // Just trigger re-render via loop
            }
        }
    }

    // Restore terminal before returning
    restore_terminal()
        .map_err(|e| RslphError::Subprocess(format!("Terminal restore failed: {}", e)))?;

    Ok(state)
}

/// Handle keyboard input when in AnsweringQuestions mode.
fn handle_input_key(state: &mut PlanTuiState, key_code: KeyCode, modifiers: KeyModifiers) {
    // Check for Ctrl+C first (always quit)
    if modifiers.contains(KeyModifiers::CONTROL) {
        match key_code {
            KeyCode::Char('c') => {
                state.should_quit = true;
                return;
            }
            KeyCode::Char('d') => {
                // Ctrl+D: submit answers
                debug!("Submit keybinding detected (Ctrl+D), exiting question mode");
                state.exit_question_mode();
                return;
            }
            KeyCode::Enter => {
                // Ctrl+Enter: submit answers
                debug!("Submit keybinding detected (Ctrl+Enter), exiting question mode");
                state.exit_question_mode();
                return;
            }
            _ => {}
        }
    }

    match key_code {
        KeyCode::Esc => {
            // Esc: cancel input and quit
            state.should_quit = true;
        }
        KeyCode::Backspace => {
            state.handle_input_backspace();
        }
        KeyCode::Enter => {
            // Regular Enter: add newline
            state.handle_input_newline();
        }
        KeyCode::Char(c) => {
            state.handle_input_char(c);
        }
        _ => {
            // Ignore other keys
        }
    }
}

/// Handle keyboard input when in Normal (navigation) mode.
fn handle_navigation_key(
    state: &mut PlanTuiState,
    key_code: KeyCode,
    modifiers: KeyModifiers,
    cancel_token: &CancellationToken,
) {
    // Check for Ctrl+C first (always quit)
    if modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('c') = key_code {
            state.should_quit = true;
            cancel_token.cancel();
            return;
        }
    }

    match key_code {
        KeyCode::Char('q') | KeyCode::Esc => {
            state.should_quit = true;
            cancel_token.cancel();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            state.scroll_offset = (state.scroll_offset + 1)
                .min(state.conversation.len().saturating_sub(1));
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.scroll_offset = state.scroll_offset.saturating_sub(1);
        }
        KeyCode::PageUp => {
            state.scroll_offset = state.scroll_offset.saturating_sub(10);
        }
        KeyCode::PageDown => {
            state.scroll_offset = (state.scroll_offset + 10)
                .min(state.conversation.len().saturating_sub(1));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_tui_state_new() {
        let state = PlanTuiState::new();
        assert!(state.conversation.items().is_empty());
        assert_eq!(state.scroll_offset, 0);
        assert!(state.plan_preview.is_empty());
        assert!(matches!(state.status, PlanStatus::StackDetection));
        assert!(!state.should_quit);
        assert_eq!(state.stderr_without_stdout, 0);
        assert!(!state.has_stdout);
        // New input mode fields
        assert!(matches!(state.input_mode, InputMode::Normal));
        assert!(state.pending_questions.is_empty());
        assert!(state.input_buffer.is_empty());
        assert!(state.session_id.is_none());
        assert!(!state.answers_submitted);
    }

    #[test]
    fn test_plan_tui_state_default() {
        let state = PlanTuiState::default();
        assert!(matches!(state.status, PlanStatus::StackDetection));
    }

    #[test]
    fn test_plan_status_variants() {
        let _ = PlanStatus::StackDetection;
        let _ = PlanStatus::Planning;
        let _ = PlanStatus::GeneratingName;
        let _ = PlanStatus::AwaitingInput;
        let _ = PlanStatus::ResumingSession;
        let _ = PlanStatus::Complete;
        let _ = PlanStatus::Failed("error".to_string());
    }

    #[test]
    fn test_input_mode_variants() {
        let normal = InputMode::Normal;
        let answering = InputMode::AnsweringQuestions;
        assert_ne!(normal, answering);
        assert_eq!(InputMode::default(), InputMode::Normal);
    }

    #[test]
    fn test_set_complete() {
        let mut state = PlanTuiState::new();
        state.set_complete();
        assert!(matches!(state.status, PlanStatus::Complete));
    }

    #[test]
    fn test_set_failed() {
        let mut state = PlanTuiState::new();
        state.set_failed("test error".to_string());
        assert!(matches!(state.status, PlanStatus::Failed(ref e) if e == "test error"));
    }

    #[test]
    fn test_update_with_raw_stdout() {
        let mut state = PlanTuiState::new();
        state.update(&PlanTuiEvent::RawStdout("raw output line".to_string()));
        assert_eq!(state.conversation.len(), 1);
        match &state.conversation.items()[0] {
            ConversationItem::System(s) => assert!(s.contains("[stdout]")),
            _ => panic!("Expected System item"),
        }
    }

    #[test]
    fn test_update_with_stderr() {
        let mut state = PlanTuiState::new();
        state.update(&PlanTuiEvent::Stderr("error message".to_string()));
        assert_eq!(state.conversation.len(), 1);
        match &state.conversation.items()[0] {
            ConversationItem::System(s) => assert!(s.contains("[stderr]")),
            _ => panic!("Expected System item"),
        }
        // Stderr without stdout should be tracked
        assert_eq!(state.stderr_without_stdout, 1);
        assert!(!state.has_stdout);
    }

    #[test]
    fn test_stderr_then_stdout_tracking() {
        let mut state = PlanTuiState::new();

        // Receive stderr first
        state.update(&PlanTuiEvent::Stderr("stderr line".to_string()));
        assert_eq!(state.stderr_without_stdout, 1);
        assert!(!state.has_stdout);

        // Receive stdout
        state.update(&PlanTuiEvent::RawStdout("stdout line".to_string()));
        assert!(state.has_stdout);

        // Receiving more stderr shouldn't increment counter since we have stdout
        state.update(&PlanTuiEvent::Stderr("more stderr".to_string()));
        assert_eq!(state.stderr_without_stdout, 1); // Still 1, not 2
    }

    #[test]
    fn test_plan_tui_event_variants() {
        use crate::subprocess::StreamEvent;
        let _ = PlanTuiEvent::Stream(Box::new(
            StreamEvent::parse(r#"{"type":"assistant"}"#).unwrap(),
        ));
        let _ = PlanTuiEvent::RawStdout("raw".to_string());
        let _ = PlanTuiEvent::Stderr("err".to_string());
        let _ = PlanTuiEvent::QuestionsAsked {
            questions: vec!["Q1?".to_string()],
            session_id: "session-123".to_string(),
        };
    }

    #[test]
    fn test_enter_question_mode() {
        let mut state = PlanTuiState::new();
        let questions = vec!["Q1?".to_string(), "Q2?".to_string()];

        state.enter_question_mode(questions.clone(), "session-abc".to_string());

        assert!(matches!(state.input_mode, InputMode::AnsweringQuestions));
        assert_eq!(state.pending_questions, questions);
        assert_eq!(state.session_id, Some("session-abc".to_string()));
        assert!(state.input_buffer.is_empty());
        assert!(matches!(state.status, PlanStatus::AwaitingInput));
        assert!(!state.answers_submitted);
        assert!(state.is_answering_questions());
        assert_eq!(state.get_session_id(), Some("session-abc"));
    }

    #[test]
    fn test_exit_question_mode() {
        let mut state = PlanTuiState::new();
        state.enter_question_mode(vec!["Q?".to_string()], "session-123".to_string());
        state.input_buffer = "My answer".to_string();

        state.exit_question_mode();

        // Answers remain in input_buffer for command to read
        assert_eq!(state.input_buffer, "My answer");
        assert!(matches!(state.input_mode, InputMode::Normal));
        assert!(state.answers_submitted);
        assert!(matches!(state.status, PlanStatus::ResumingSession));
        assert!(!state.is_answering_questions());
    }

    #[test]
    fn test_handle_input_char() {
        let mut state = PlanTuiState::new();

        // Normal mode - should not add chars
        state.handle_input_char('x');
        assert!(state.input_buffer.is_empty());

        // Enter question mode - now chars should be added
        state.enter_question_mode(vec!["Q?".to_string()], "session".to_string());
        state.handle_input_char('H');
        state.handle_input_char('i');
        assert_eq!(state.input_buffer, "Hi");
    }

    #[test]
    fn test_handle_input_backspace() {
        let mut state = PlanTuiState::new();
        state.enter_question_mode(vec!["Q?".to_string()], "session".to_string());
        state.input_buffer = "Hello".to_string();

        state.handle_input_backspace();
        assert_eq!(state.input_buffer, "Hell");

        // Backspace on empty buffer should be safe
        state.input_buffer.clear();
        state.handle_input_backspace();
        assert!(state.input_buffer.is_empty());
    }

    #[test]
    fn test_handle_input_newline() {
        let mut state = PlanTuiState::new();
        state.enter_question_mode(vec!["Q?".to_string()], "session".to_string());
        state.input_buffer = "Line1".to_string();

        state.handle_input_newline();
        assert_eq!(state.input_buffer, "Line1\n");
    }

    #[test]
    fn test_questions_asked_event() {
        let mut state = PlanTuiState::new();
        let questions = vec!["What is your project name?".to_string()];

        state.update(&PlanTuiEvent::QuestionsAsked {
            questions: questions.clone(),
            session_id: "test-session".to_string(),
        });

        assert!(state.is_answering_questions());
        assert_eq!(state.pending_questions, questions);
        assert_eq!(state.session_id, Some("test-session".to_string()));
    }
}

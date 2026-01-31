//! TUI mode for the plan command.
//!
//! Provides streaming display of LLM output during planning, including
//! thinking blocks, tool calls, and generated plan preview.

use std::collections::HashMap;
use std::time::Instant;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::RslphError;
use crate::subprocess::StreamEvent;
use crate::tui::conversation::{render_conversation, ConversationBuffer, ConversationItem};
use crate::tui::event::EventHandler;
use crate::tui::terminal::{init_terminal, restore_terminal};

/// Events that can be sent to the plan TUI.
#[derive(Debug, Clone)]
pub enum PlanTuiEvent {
    /// A parsed stream event from Claude.
    Stream(Box<StreamEvent>),
    /// Raw stdout line that couldn't be parsed as JSON.
    RawStdout(String),
    /// Stderr line from Claude.
    Stderr(String),
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
        }
    }

    /// Update state from a plan TUI event.
    pub fn update(&mut self, event: &PlanTuiEvent) {
        match event {
            PlanTuiEvent::Stream(stream_event) => {
                self.has_stdout = true;
                // Extract conversation items from the stream event
                for item in stream_event.extract_conversation_items() {
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
        }

        // Auto-scroll to bottom (keep recent items visible)
        self.scroll_offset = self.conversation.len().saturating_sub(15);

        // Update status based on what we're seeing
        if matches!(self.status, PlanStatus::StackDetection) {
            self.status = PlanStatus::Planning;
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
}

/// Render the plan TUI to the frame.
pub fn render_plan_tui(frame: &mut Frame, state: &PlanTuiState) {
    let area = frame.area();

    // Split: top for status, middle for conversation, bottom for plan preview
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
pub async fn run_plan_tui(
    event_rx: mpsc::UnboundedReceiver<PlanTuiEvent>,
    cancel_token: CancellationToken,
) -> Result<PlanTuiState, RslphError> {
    let mut terminal = init_terminal()
        .map_err(|e| RslphError::Subprocess(format!("Terminal init failed: {}", e)))?;

    let mut state = PlanTuiState::new();
    let mut event_rx = event_rx;

    // Create event handler for keyboard input (30 FPS for smooth rendering)
    let (mut kbd_handler, _subprocess_tx) = EventHandler::new(30);

    loop {
        // Render current state
        terminal
            .draw(|frame| render_plan_tui(frame, &state))
            .map_err(|e| RslphError::Subprocess(format!("Render failed: {}", e)))?;

        tokio::select! {
            biased;

            // Check for cancellation
            _ = cancel_token.cancelled() => {
                state.set_failed("Cancelled".to_string());
                break;
            }

            // Stream events from Claude
            stream_event = event_rx.recv() => {
                match stream_event {
                    Some(event) => {
                        state.update(&event);
                    }
                    None => {
                        // Stream complete
                        state.set_complete();
                        break;
                    }
                }
            }

            // Keyboard events
            kbd_event = kbd_handler.next() => {
                if let Some(app_event) = kbd_event {
                    match app_event {
                        crate::tui::AppEvent::Quit => {
                            state.should_quit = true;
                            cancel_token.cancel();
                            break;
                        }
                        crate::tui::AppEvent::ScrollUp => {
                            state.scroll_offset = state.scroll_offset.saturating_sub(1);
                        }
                        crate::tui::AppEvent::ScrollDown => {
                            state.scroll_offset = (state.scroll_offset + 1)
                                .min(state.conversation.len().saturating_sub(1));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Restore terminal before returning
    restore_terminal()
        .map_err(|e| RslphError::Subprocess(format!("Terminal restore failed: {}", e)))?;

    Ok(state)
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
        let _ = PlanStatus::Complete;
        let _ = PlanStatus::Failed("error".to_string());
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
    }
}

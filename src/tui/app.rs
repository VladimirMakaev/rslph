//! Application state for the TUI.
//!
//! Implements the Model part of The Elm Architecture (TEA).
//! Contains all state needed to render the UI and respond to events.

use std::fmt;
use std::path::PathBuf;

/// Message role/type in the conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    /// User prompt/input.
    User,
    /// Claude's text response.
    Assistant,
    /// Tool use (with tool name).
    Tool(String),
    /// System/log message.
    System,
}

impl MessageRole {
    /// Get display label for the role.
    pub fn label(&self) -> &str {
        match self {
            MessageRole::User => "You",
            MessageRole::Assistant => "Claude",
            MessageRole::Tool(_) => "Tool",
            MessageRole::System => "System",
        }
    }

    /// Parse from string (for backwards compatibility).
    pub fn from_str(s: &str) -> Self {
        match s {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            s if s.starts_with("tool:") => {
                MessageRole::Tool(s.strip_prefix("tool:").unwrap_or("unknown").to_string())
            }
            _ => MessageRole::System,
        }
    }
}

impl fmt::Display for MessageRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::Tool(name) => write!(f, "tool:{}", name),
            MessageRole::System => write!(f, "system"),
        }
    }
}

/// A message in the conversation history.
#[derive(Debug, Clone)]
pub struct Message {
    /// Role/type of the message.
    pub role: MessageRole,
    /// Content of the message.
    pub content: String,
    /// Which iteration this message belongs to.
    pub iteration: u32,
    /// Whether this message is collapsed in the view.
    pub collapsed: bool,
    /// Number of content lines (cached for performance).
    pub line_count: usize,
}

impl Message {
    /// Create a new message.
    pub fn new(role: impl Into<String>, content: impl Into<String>, iteration: u32) -> Self {
        let content_str: String = content.into();
        let line_count = content_str.lines().count().max(1);
        Self {
            role: MessageRole::from_str(&role.into()),
            content: content_str,
            iteration,
            collapsed: false,
            line_count,
        }
    }

    /// Create a new message with MessageRole directly.
    pub fn with_role(role: MessageRole, content: impl Into<String>, iteration: u32) -> Self {
        let content_str: String = content.into();
        let line_count = content_str.lines().count().max(1);
        // Tool messages start collapsed by default for cleaner output
        let collapsed = matches!(role, MessageRole::Tool(_));
        Self {
            role,
            content: content_str,
            iteration,
            collapsed,
            line_count,
        }
    }

    /// Toggle collapsed state.
    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
    }
}

/// Application state (TEA Model).
///
/// Contains all state needed to render the TUI and respond to events.
#[derive(Debug)]
pub struct App {
    // Status bar state
    /// Current iteration number (1-indexed).
    pub current_iteration: u32,
    /// Maximum number of iterations allowed.
    pub max_iterations: u32,
    /// Current task number within iteration.
    pub current_task: u32,
    /// Total number of tasks in current iteration.
    pub total_tasks: u32,
    /// Context usage as a ratio (0.0 to 1.0).
    pub context_usage: f64,
    /// Name of the model being used (e.g., "claude-sonnet-4-20250514").
    pub model_name: String,
    /// Name of the project.
    pub project_name: String,
    /// Path to the log file.
    pub log_path: Option<PathBuf>,

    // Output view state
    /// All messages in the conversation.
    pub messages: Vec<Message>,
    /// Current vertical scroll offset.
    pub scroll_offset: u16,
    /// Currently selected message index (for expand/collapse navigation).
    pub selected_message: Option<usize>,

    // Navigation state
    /// Which iteration we're currently viewing (for history browsing).
    pub viewing_iteration: u32,
    /// Whether the build is paused.
    pub is_paused: bool,
    /// Whether the application should quit.
    pub should_quit: bool,
    /// Maximum system messages to keep expanded (rolling limit).
    pub max_system_expanded: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            current_iteration: 0,
            max_iterations: 1,
            current_task: 0,
            total_tasks: 0,
            context_usage: 0.0,
            model_name: String::new(),
            project_name: String::new(),
            log_path: None,
            messages: Vec::new(),
            scroll_offset: 0,
            selected_message: None,
            viewing_iteration: 0,
            is_paused: false,
            should_quit: false,
            max_system_expanded: 5,
        }
    }
}

impl App {
    /// Create a new App with the given configuration.
    pub fn new(
        max_iterations: u32,
        model_name: impl Into<String>,
        project_name: impl Into<String>,
    ) -> Self {
        Self {
            max_iterations,
            model_name: model_name.into(),
            project_name: project_name.into(),
            ..Default::default()
        }
    }

    /// Update the app state based on an event.
    pub fn update(&mut self, event: AppEvent) {
        match event {
            AppEvent::ScrollUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            AppEvent::ScrollDown => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
            AppEvent::PrevIteration => {
                if self.viewing_iteration > 1 {
                    self.viewing_iteration -= 1;
                    self.selected_message = None;
                }
            }
            AppEvent::NextIteration => {
                if self.viewing_iteration < self.current_iteration {
                    self.viewing_iteration += 1;
                    self.selected_message = None;
                }
            }
            AppEvent::TogglePause => {
                self.is_paused = !self.is_paused;
            }
            AppEvent::Quit => {
                self.should_quit = true;
            }
            AppEvent::SelectPrevMessage => {
                self.select_prev_message();
            }
            AppEvent::SelectNextMessage => {
                self.select_next_message();
            }
            AppEvent::ToggleMessage => {
                self.toggle_selected_message();
            }
            AppEvent::ClaudeOutput(content) => {
                // Add assistant message for current iteration
                self.messages.push(Message::new(
                    "assistant",
                    content,
                    self.current_iteration,
                ));
            }
            AppEvent::ToolMessage { tool_name, content } => {
                // Add tool message for current iteration
                self.messages.push(Message::with_role(
                    MessageRole::Tool(tool_name),
                    content,
                    self.current_iteration,
                ));
            }
            AppEvent::ContextUsage(ratio) => {
                self.context_usage = ratio.clamp(0.0, 1.0);
            }
            AppEvent::IterationStart { iteration } => {
                self.current_iteration = iteration;
                self.viewing_iteration = iteration;
                self.scroll_offset = 0;
                self.selected_message = None;
            }
            AppEvent::IterationComplete { tasks_done } => {
                self.current_task = tasks_done;
                self.viewing_iteration = self.current_iteration;
                self.selected_message = None;
            }
            AppEvent::LogMessage(content) => {
                // Add system message for current iteration
                self.messages.push(Message::new(
                    "system",
                    content,
                    self.current_iteration,
                ));
                // Enforce rolling limit for system messages
                self.enforce_system_rolling_limit();
            }
            AppEvent::Render => {
                // Render events don't change state, just trigger redraw
            }
        }
    }

    /// Get messages for the currently viewed iteration.
    pub fn messages_for_viewing(&self) -> impl Iterator<Item = &Message> {
        self.messages
            .iter()
            .filter(|m| m.iteration == self.viewing_iteration)
    }

    /// Scroll down by one line, clamped to content length.
    pub fn scroll_down(&mut self, viewport_height: u16, content_height: u16) {
        let max_offset = content_height.saturating_sub(viewport_height);
        self.scroll_offset = (self.scroll_offset + 1).min(max_offset);
    }

    /// Scroll up by one line.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Auto-scroll to bottom (for following live output).
    pub fn scroll_to_bottom(&mut self, viewport_height: u16, content_height: u16) {
        let max_offset = content_height.saturating_sub(viewport_height);
        self.scroll_offset = max_offset;
    }

    /// Add a new message and auto-scroll if at bottom.
    pub fn add_message(&mut self, role: MessageRole, content: String, viewport_height: u16) {
        let was_at_bottom = self.is_at_bottom(viewport_height);
        let content_lines = content.lines().count().max(1);
        self.messages.push(Message {
            role,
            content,
            iteration: self.current_iteration,
            collapsed: false,
            line_count: content_lines,
        });
        // Auto-collapse old system messages beyond limit
        self.enforce_system_rolling_limit();
        if was_at_bottom {
            // Content height increased, but we handle this on next render
            // Just mark that we should scroll
        }
    }

    /// Add a message with role from string (backwards compatibility).
    pub fn add_message_str(&mut self, role: &str, content: String, viewport_height: u16) {
        self.add_message(MessageRole::from_str(role), content, viewport_height);
    }

    /// Add a tool use message.
    pub fn add_tool_message(&mut self, tool_name: String, content: String, viewport_height: u16) {
        self.add_message(MessageRole::Tool(tool_name), content, viewport_height);
    }

    /// Enforce rolling limit for system messages - collapse old ones beyond limit.
    fn enforce_system_rolling_limit(&mut self) {
        let iteration = self.current_iteration;
        let limit = self.max_system_expanded;

        // Find all system message indices for current iteration (newest first)
        let system_indices: Vec<usize> = self.messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m.iteration == iteration && matches!(m.role, MessageRole::System))
            .map(|(i, _)| i)
            .rev()
            .collect();

        // Auto-collapse messages beyond the limit
        for (i, &msg_idx) in system_indices.iter().enumerate() {
            if i >= limit {
                self.messages[msg_idx].collapsed = true;
            }
        }
    }

    /// Get indices of messages for the currently viewed iteration.
    pub fn message_indices_for_viewing(&self) -> Vec<usize> {
        self.messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m.iteration == self.viewing_iteration)
            .map(|(i, _)| i)
            .collect()
    }

    /// Toggle collapse state of the currently selected message.
    pub fn toggle_selected_message(&mut self) {
        if let Some(sel_idx) = self.selected_message {
            let indices = self.message_indices_for_viewing();
            if sel_idx < indices.len() {
                let msg_idx = indices[sel_idx];
                self.messages[msg_idx].toggle_collapsed();
            }
        }
    }

    /// Select next message in the current iteration.
    pub fn select_next_message(&mut self) {
        let indices = self.message_indices_for_viewing();
        if indices.is_empty() {
            return;
        }

        self.selected_message = match self.selected_message {
            None => Some(0),
            Some(i) if i + 1 < indices.len() => Some(i + 1),
            Some(i) => Some(i), // Stay at last
        };
    }

    /// Select previous message in the current iteration.
    pub fn select_prev_message(&mut self) {
        let indices = self.message_indices_for_viewing();
        if indices.is_empty() {
            return;
        }

        self.selected_message = match self.selected_message {
            None => Some(indices.len().saturating_sub(1)),
            Some(0) => Some(0), // Stay at first
            Some(i) => Some(i - 1),
        };
    }

    /// Check if scroll is at the bottom of content.
    fn is_at_bottom(&self, viewport_height: u16) -> bool {
        let content_height = self.content_height_for_iteration(self.viewing_iteration);
        let max_offset = content_height.saturating_sub(viewport_height);
        self.scroll_offset >= max_offset
    }

    /// Calculate the content height for a given iteration.
    /// Accounts for collapsed messages (1 line when collapsed).
    pub fn content_height_for_iteration(&self, iteration: u32) -> u16 {
        self.messages
            .iter()
            .filter(|m| m.iteration == iteration)
            .map(|m| {
                if m.collapsed {
                    1 // Collapsed message shows as single line "Role > (N lines)"
                } else {
                    m.line_count as u16 + 2 // +1 for role line, +1 for blank separator
                }
            })
            .sum()
    }
}

/// Events that can occur in the application.
///
/// These are converted from raw crossterm events or sent from subprocess handlers.
#[derive(Debug, Clone)]
pub enum AppEvent {
    // Keyboard/mouse navigation
    /// Scroll up one line.
    ScrollUp,
    /// Scroll down one line.
    ScrollDown,
    /// View previous iteration's messages.
    PrevIteration,
    /// View next iteration's messages.
    NextIteration,
    /// Toggle pause state.
    TogglePause,
    /// Request application quit.
    Quit,

    // Message selection (for collapse/expand)
    /// Select previous message.
    SelectPrevMessage,
    /// Select next message.
    SelectNextMessage,
    /// Toggle collapse state of selected message.
    ToggleMessage,

    // Subprocess events
    /// New output from Claude (text).
    ClaudeOutput(String),
    /// Tool use message (tool_name, content).
    ToolMessage { tool_name: String, content: String },
    /// Updated context usage ratio (0.0 to 1.0).
    ContextUsage(f64),
    /// New iteration is starting.
    IterationStart {
        /// The iteration number (1-indexed).
        iteration: u32,
    },
    /// An iteration completed with the given number of tasks done.
    IterationComplete {
        /// Number of tasks completed in this iteration.
        tasks_done: u32,
    },
    /// Log message from build loop (displayed as system message).
    LogMessage(String),

    // Timer events
    /// Time to render a new frame.
    Render,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_new() {
        let app = App::new(10, "claude-sonnet-4-20250514", "my-project");

        assert_eq!(app.max_iterations, 10);
        assert_eq!(app.model_name, "claude-sonnet-4-20250514");
        assert_eq!(app.project_name, "my-project");
        assert_eq!(app.current_iteration, 0);
        assert!(!app.is_paused);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_app_update_scroll() {
        let mut app = App::default();
        app.scroll_offset = 5;

        app.update(AppEvent::ScrollUp);
        assert_eq!(app.scroll_offset, 4);

        app.update(AppEvent::ScrollDown);
        assert_eq!(app.scroll_offset, 5);

        // Test saturation at 0
        app.scroll_offset = 0;
        app.update(AppEvent::ScrollUp);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_app_update_quit() {
        let mut app = App::default();
        assert!(!app.should_quit);

        app.update(AppEvent::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn test_app_update_toggle_pause() {
        let mut app = App::default();
        assert!(!app.is_paused);

        app.update(AppEvent::TogglePause);
        assert!(app.is_paused);

        app.update(AppEvent::TogglePause);
        assert!(!app.is_paused);
    }

    #[test]
    fn test_app_update_context_usage() {
        let mut app = App::default();

        app.update(AppEvent::ContextUsage(0.5));
        assert!((app.context_usage - 0.5).abs() < f64::EPSILON);

        // Test clamping
        app.update(AppEvent::ContextUsage(1.5));
        assert!((app.context_usage - 1.0).abs() < f64::EPSILON);

        app.update(AppEvent::ContextUsage(-0.5));
        assert!((app.context_usage - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_app_update_claude_output() {
        let mut app = App::default();
        app.current_iteration = 1;

        app.update(AppEvent::ClaudeOutput("Hello".to_string()));

        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::Assistant);
        assert_eq!(app.messages[0].content, "Hello");
        assert_eq!(app.messages[0].iteration, 1);
    }

    #[test]
    fn test_message_new() {
        let msg = Message::new("user", "test message", 3);

        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "test message");
        assert_eq!(msg.iteration, 3);
        assert!(!msg.collapsed);
        assert_eq!(msg.line_count, 1);
    }

    #[test]
    fn test_message_with_role() {
        let msg = Message::with_role(MessageRole::Tool("Read".to_string()), "file contents", 1);

        assert_eq!(msg.role, MessageRole::Tool("Read".to_string()));
        assert_eq!(msg.content, "file contents");
        // Tool messages start collapsed by default
        assert!(msg.collapsed);
    }

    #[test]
    fn test_message_with_role_non_tool_not_collapsed() {
        let msg = Message::with_role(MessageRole::Assistant, "response", 1);
        // Non-tool messages start expanded
        assert!(!msg.collapsed);
    }

    #[test]
    fn test_message_toggle_collapsed() {
        let mut msg = Message::new("assistant", "test", 1);
        assert!(!msg.collapsed);

        msg.toggle_collapsed();
        assert!(msg.collapsed);

        msg.toggle_collapsed();
        assert!(!msg.collapsed);
    }

    #[test]
    fn test_app_event_variants() {
        // Ensure all variants are constructible
        let _ = AppEvent::ScrollUp;
        let _ = AppEvent::ScrollDown;
        let _ = AppEvent::PrevIteration;
        let _ = AppEvent::NextIteration;
        let _ = AppEvent::TogglePause;
        let _ = AppEvent::Quit;
        let _ = AppEvent::SelectPrevMessage;
        let _ = AppEvent::SelectNextMessage;
        let _ = AppEvent::ToggleMessage;
        let _ = AppEvent::ClaudeOutput("test".to_string());
        let _ = AppEvent::ToolMessage { tool_name: "Read".to_string(), content: "file".to_string() };
        let _ = AppEvent::ContextUsage(0.5);
        let _ = AppEvent::IterationComplete { tasks_done: 3 };
        let _ = AppEvent::LogMessage("log".to_string());
        let _ = AppEvent::Render;
    }

    #[test]
    fn test_scroll_down_clamped() {
        let mut app = App::default();
        app.scroll_offset = 0;

        // Content of 10 lines, viewport of 5 lines => max offset is 5
        app.scroll_down(5, 10);
        assert_eq!(app.scroll_offset, 1);

        // Scroll to the edge
        app.scroll_offset = 5;
        app.scroll_down(5, 10);
        // Should stay at 5 (clamped)
        assert_eq!(app.scroll_offset, 5);
    }

    #[test]
    fn test_scroll_down_empty_content() {
        let mut app = App::default();
        app.scroll_offset = 0;

        // Empty content (0 lines), viewport of 10 lines => max offset is 0
        app.scroll_down(10, 0);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_up_saturating() {
        let mut app = App::default();
        app.scroll_offset = 0;

        app.scroll_up();
        assert_eq!(app.scroll_offset, 0); // Should not go negative

        app.scroll_offset = 5;
        app.scroll_up();
        assert_eq!(app.scroll_offset, 4);
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut app = App::default();
        app.scroll_offset = 0;

        // Content of 20 lines, viewport of 5 lines => max offset is 15
        app.scroll_to_bottom(5, 20);
        assert_eq!(app.scroll_offset, 15);
    }

    #[test]
    fn test_content_height_for_iteration() {
        let mut app = App::default();
        app.current_iteration = 1;

        // Add messages with multiline content
        app.messages.push(Message::new("assistant", "Line 1", 1));
        app.messages.push(Message::new("assistant", "Line 1\nLine 2\nLine 3", 1));
        app.messages.push(Message::new("assistant", "Different iteration", 2));

        // Iteration 1:
        // - "Line 1" = 1 line + 2 (role + blank) = 3
        // - "Line 1\nLine 2\nLine 3" = 3 lines + 2 (role + blank) = 5
        // Total = 8
        let height = app.content_height_for_iteration(1);
        assert_eq!(height, 8);
    }

    #[test]
    fn test_content_height_collapsed() {
        let mut app = App::default();
        app.current_iteration = 1;

        // Add a multiline message and collapse it
        app.messages.push(Message::new("assistant", "Line 1\nLine 2\nLine 3", 1));
        app.messages[0].collapsed = true;

        // Collapsed message shows as 1 line
        let height = app.content_height_for_iteration(1);
        assert_eq!(height, 1);
    }

    #[test]
    fn test_system_rolling_limit() {
        let mut app = App::default();
        app.current_iteration = 1;
        app.max_system_expanded = 3;

        // Add 5 system messages
        for i in 0..5 {
            app.messages.push(Message::new("system", format!("Log {}", i), 1));
        }
        app.enforce_system_rolling_limit();

        // First 2 should be collapsed, last 3 should be expanded
        assert!(app.messages[0].collapsed);
        assert!(app.messages[1].collapsed);
        assert!(!app.messages[2].collapsed);
        assert!(!app.messages[3].collapsed);
        assert!(!app.messages[4].collapsed);
    }

    #[test]
    fn test_select_next_prev_message() {
        let mut app = App::default();
        app.viewing_iteration = 1;
        app.messages.push(Message::new("assistant", "msg1", 1));
        app.messages.push(Message::new("assistant", "msg2", 1));
        app.messages.push(Message::new("assistant", "msg3", 1));

        // Initially no selection
        assert_eq!(app.selected_message, None);

        // Select next (first message)
        app.select_next_message();
        assert_eq!(app.selected_message, Some(0));

        // Select next (second message)
        app.select_next_message();
        assert_eq!(app.selected_message, Some(1));

        // Select prev (first message)
        app.select_prev_message();
        assert_eq!(app.selected_message, Some(0));

        // Select prev at boundary (stay at first)
        app.select_prev_message();
        assert_eq!(app.selected_message, Some(0));
    }

    #[test]
    fn test_toggle_selected_message() {
        let mut app = App::default();
        app.viewing_iteration = 1;
        app.messages.push(Message::new("assistant", "msg1", 1));

        app.selected_message = Some(0);
        assert!(!app.messages[0].collapsed);

        app.toggle_selected_message();
        assert!(app.messages[0].collapsed);
    }

    #[test]
    fn test_message_role_from_str() {
        assert_eq!(MessageRole::from_str("user"), MessageRole::User);
        assert_eq!(MessageRole::from_str("assistant"), MessageRole::Assistant);
        assert_eq!(MessageRole::from_str("system"), MessageRole::System);
        assert_eq!(MessageRole::from_str("tool:Read"), MessageRole::Tool("Read".to_string()));
        assert_eq!(MessageRole::from_str("unknown"), MessageRole::System); // Default
    }

    #[test]
    fn test_message_role_label() {
        assert_eq!(MessageRole::User.label(), "You");
        assert_eq!(MessageRole::Assistant.label(), "Claude");
        assert_eq!(MessageRole::System.label(), "System");
        assert_eq!(MessageRole::Tool("Read".to_string()).label(), "Tool");
    }
}

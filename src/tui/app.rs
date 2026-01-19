//! Application state for the TUI.
//!
//! Implements the Model part of The Elm Architecture (TEA).
//! Contains all state needed to render the UI and respond to events.

use std::path::PathBuf;

/// A message in the conversation history.
#[derive(Debug, Clone)]
pub struct Message {
    /// Role of the message sender ("user", "assistant", "system").
    pub role: String,
    /// Content of the message.
    pub content: String,
    /// Which iteration this message belongs to.
    pub iteration: u32,
}

impl Message {
    /// Create a new message.
    pub fn new(role: impl Into<String>, content: impl Into<String>, iteration: u32) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            iteration,
        }
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

    // Navigation state
    /// Which iteration we're currently viewing (for history browsing).
    pub viewing_iteration: u32,
    /// Whether the build is paused.
    pub is_paused: bool,
    /// Whether the application should quit.
    pub should_quit: bool,
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
            viewing_iteration: 0,
            is_paused: false,
            should_quit: false,
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
                }
            }
            AppEvent::NextIteration => {
                if self.viewing_iteration < self.current_iteration {
                    self.viewing_iteration += 1;
                }
            }
            AppEvent::TogglePause => {
                self.is_paused = !self.is_paused;
            }
            AppEvent::Quit => {
                self.should_quit = true;
            }
            AppEvent::ClaudeOutput(content) => {
                // Add assistant message for current iteration
                self.messages.push(Message::new(
                    "assistant",
                    content,
                    self.current_iteration,
                ));
            }
            AppEvent::ContextUsage(ratio) => {
                self.context_usage = ratio.clamp(0.0, 1.0);
            }
            AppEvent::IterationComplete { tasks_done } => {
                self.current_task = tasks_done;
                self.current_iteration += 1;
                self.viewing_iteration = self.current_iteration;
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

    // Subprocess events
    /// New output from Claude.
    ClaudeOutput(String),
    /// Updated context usage ratio (0.0 to 1.0).
    ContextUsage(f64),
    /// An iteration completed with the given number of tasks done.
    IterationComplete {
        /// Number of tasks completed in this iteration.
        tasks_done: u32,
    },

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
        assert_eq!(app.messages[0].role, "assistant");
        assert_eq!(app.messages[0].content, "Hello");
        assert_eq!(app.messages[0].iteration, 1);
    }

    #[test]
    fn test_message_new() {
        let msg = Message::new("user", "test message", 3);

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "test message");
        assert_eq!(msg.iteration, 3);
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
        let _ = AppEvent::ClaudeOutput("test".to_string());
        let _ = AppEvent::ContextUsage(0.5);
        let _ = AppEvent::IterationComplete { tasks_done: 3 };
        let _ = AppEvent::Render;
    }
}

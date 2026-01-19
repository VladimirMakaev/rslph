//! Event handling for the TUI.
//!
//! Provides an EventHandler that merges events from multiple async sources:
//! - Keyboard/mouse events from crossterm
//! - Subprocess events from a channel
//! - Render tick events from a timer

use std::time::Duration;

use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyModifiers, MouseEventKind,
};
use futures::StreamExt;
use tokio::sync::mpsc;

use super::AppEvent;

/// Subprocess event that can be sent to the TUI.
///
/// These events come from the ClaudeRunner or other subprocess components.
#[derive(Debug, Clone)]
pub enum SubprocessEvent {
    /// New line of output from Claude.
    Output(String),
    /// Updated context usage ratio.
    Usage(f64),
    /// Iteration completed.
    IterationDone { tasks_done: u32 },
}

impl From<SubprocessEvent> for AppEvent {
    fn from(event: SubprocessEvent) -> Self {
        match event {
            SubprocessEvent::Output(s) => AppEvent::ClaudeOutput(s),
            SubprocessEvent::Usage(ratio) => AppEvent::ContextUsage(ratio),
            SubprocessEvent::IterationDone { tasks_done } => {
                AppEvent::IterationComplete { tasks_done }
            }
        }
    }
}

/// Handles merging of async event sources into a unified event stream.
///
/// The EventHandler spawns an async task that uses `tokio::select!` to multiplex:
/// 1. Keyboard/mouse events from crossterm's EventStream
/// 2. Subprocess events from an mpsc channel
/// 3. Render tick events from a timer interval
///
/// Events are converted to `AppEvent` and sent through an internal channel.
pub struct EventHandler {
    /// Receiver for merged events.
    rx: mpsc::UnboundedReceiver<AppEvent>,
    /// Handle to the spawned event loop task.
    _task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    /// Create a new EventHandler.
    ///
    /// # Arguments
    ///
    /// * `subprocess_rx` - Receiver for subprocess events (from ClaudeRunner etc.)
    /// * `frame_rate` - Target frames per second for render ticks (typically 30)
    ///
    /// # Returns
    ///
    /// A tuple of (EventHandler, Sender for subprocess events).
    /// The caller should use the sender to forward subprocess events.
    pub fn new(frame_rate: u32) -> (Self, mpsc::UnboundedSender<SubprocessEvent>) {
        let (subprocess_tx, subprocess_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let task = tokio::spawn(Self::event_loop(event_tx, subprocess_rx, frame_rate));

        (
            Self {
                rx: event_rx,
                _task: task,
            },
            subprocess_tx,
        )
    }

    /// The main event loop that merges all event sources.
    async fn event_loop(
        tx: mpsc::UnboundedSender<AppEvent>,
        mut subprocess_rx: mpsc::UnboundedReceiver<SubprocessEvent>,
        frame_rate: u32,
    ) {
        let mut event_stream = EventStream::new();
        let tick_duration = Duration::from_millis(1000 / frame_rate as u64);
        let mut render_interval = tokio::time::interval(tick_duration);

        loop {
            tokio::select! {
                // Keyboard/mouse events from crossterm
                maybe_event = event_stream.next() => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            if let Some(app_event) = Self::convert_crossterm_event(event) {
                                if tx.send(app_event).is_err() {
                                    // Receiver dropped, exit loop
                                    break;
                                }
                            }
                        }
                        Some(Err(_)) => {
                            // Event read error, continue
                        }
                        None => {
                            // Stream ended, exit loop
                            break;
                        }
                    }
                }

                // Subprocess events from channel
                maybe_subprocess = subprocess_rx.recv() => {
                    match maybe_subprocess {
                        Some(event) => {
                            if tx.send(AppEvent::from(event)).is_err() {
                                // Receiver dropped, exit loop
                                break;
                            }
                        }
                        None => {
                            // Channel closed, but keep running for keyboard events
                        }
                    }
                }

                // Render tick
                _ = render_interval.tick() => {
                    if tx.send(AppEvent::Render).is_err() {
                        // Receiver dropped, exit loop
                        break;
                    }
                }
            }
        }
    }

    /// Convert a crossterm event to an AppEvent.
    ///
    /// Returns None for events we don't handle.
    fn convert_crossterm_event(event: CrosstermEvent) -> Option<AppEvent> {
        match event {
            CrosstermEvent::Key(key) => {
                // Check for Ctrl+C first
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let KeyCode::Char('c') = key.code {
                        return Some(AppEvent::Quit);
                    }
                }

                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => Some(AppEvent::ScrollDown),
                    KeyCode::Char('k') | KeyCode::Up => Some(AppEvent::ScrollUp),
                    KeyCode::Char('{') => Some(AppEvent::PrevIteration),
                    KeyCode::Char('}') => Some(AppEvent::NextIteration),
                    KeyCode::Char('p') => Some(AppEvent::TogglePause),
                    KeyCode::Char('q') => Some(AppEvent::Quit),
                    KeyCode::Esc => Some(AppEvent::Quit),
                    _ => None,
                }
            }
            CrosstermEvent::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => Some(AppEvent::ScrollUp),
                MouseEventKind::ScrollDown => Some(AppEvent::ScrollDown),
                _ => None,
            },
            CrosstermEvent::Resize(_, _) => {
                // Trigger a render on resize
                Some(AppEvent::Render)
            }
            _ => None,
        }
    }

    /// Get the next event.
    ///
    /// Returns None if the event channel is closed.
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subprocess_event_into_app_event() {
        let output = SubprocessEvent::Output("hello".to_string());
        let app_event: AppEvent = output.into();
        assert!(matches!(app_event, AppEvent::ClaudeOutput(s) if s == "hello"));

        let usage = SubprocessEvent::Usage(0.75);
        let app_event: AppEvent = usage.into();
        assert!(matches!(app_event, AppEvent::ContextUsage(r) if (r - 0.75).abs() < f64::EPSILON));

        let done = SubprocessEvent::IterationDone { tasks_done: 5 };
        let app_event: AppEvent = done.into();
        assert!(matches!(app_event, AppEvent::IterationComplete { tasks_done: 5 }));
    }
}

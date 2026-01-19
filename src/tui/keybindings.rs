//! Keyboard event handling for the TUI.
//!
//! Provides a handler function that processes AppEvents and updates App state.

use crate::tui::app::{App, AppEvent, MessageRole};

/// Handle an AppEvent and update App state.
///
/// Returns true if the app should quit.
pub fn handle_event(app: &mut App, event: AppEvent, viewport_height: u16) -> bool {
    match event {
        AppEvent::ScrollUp => app.scroll_up(),
        AppEvent::ScrollDown => {
            let content_height = app.content_height_for_iteration(app.viewing_iteration);
            app.scroll_down(viewport_height, content_height);
        }
        AppEvent::PrevIteration => {
            if app.viewing_iteration > 1 {
                app.viewing_iteration -= 1;
                app.scroll_offset = 0; // Reset scroll on iteration change
                app.selected_message = None; // Reset selection
            }
        }
        AppEvent::NextIteration => {
            if app.viewing_iteration < app.current_iteration {
                app.viewing_iteration += 1;
                app.scroll_offset = 0;
                app.selected_message = None;
            }
        }
        AppEvent::TogglePause => {
            app.is_paused = !app.is_paused;
            // TODO: Signal pause to build loop when implemented
        }
        AppEvent::Quit => {
            app.should_quit = true;
            return true;
        }
        AppEvent::SelectPrevMessage => {
            app.select_prev_message();
        }
        AppEvent::SelectNextMessage => {
            app.select_next_message();
        }
        AppEvent::ToggleMessage => {
            app.toggle_selected_message();
        }
        AppEvent::ClaudeOutput(line) => {
            // Add assistant message for current iteration
            app.add_message(MessageRole::Assistant, line, viewport_height);
        }
        AppEvent::ToolMessage { tool_name, content } => {
            // Add tool message for current iteration
            app.add_tool_message(tool_name, content, viewport_height);
        }
        AppEvent::ContextUsage(ratio) => {
            app.context_usage = ratio.clamp(0.0, 1.0);
        }
        AppEvent::IterationStart { iteration } => {
            app.current_iteration = iteration;
            app.viewing_iteration = iteration;
            app.scroll_offset = 0;
            app.selected_message = None;
        }
        AppEvent::IterationComplete { tasks_done } => {
            app.current_task += tasks_done;
            // Auto-advance viewing to current iteration
            app.viewing_iteration = app.current_iteration;
            app.selected_message = None;
        }
        AppEvent::LogMessage(line) => {
            // Add system message for current iteration
            app.add_message(MessageRole::System, line, viewport_height);
        }
        AppEvent::Render => {
            // Just triggers a render, no state change
        }
    }

    app.should_quit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_event_quit() {
        let mut app = App::default();
        assert!(!app.should_quit);

        let should_quit = handle_event(&mut app, AppEvent::Quit, 20);
        assert!(should_quit);
        assert!(app.should_quit);
    }

    #[test]
    fn test_handle_event_scroll_up() {
        let mut app = App::default();
        app.scroll_offset = 5;

        handle_event(&mut app, AppEvent::ScrollUp, 20);
        assert_eq!(app.scroll_offset, 4);
    }

    #[test]
    fn test_handle_event_toggle_pause() {
        let mut app = App::default();
        assert!(!app.is_paused);

        handle_event(&mut app, AppEvent::TogglePause, 20);
        assert!(app.is_paused);

        handle_event(&mut app, AppEvent::TogglePause, 20);
        assert!(!app.is_paused);
    }

    #[test]
    fn test_handle_event_prev_iteration() {
        let mut app = App::default();
        app.current_iteration = 3;
        app.viewing_iteration = 3;
        app.scroll_offset = 10;

        handle_event(&mut app, AppEvent::PrevIteration, 20);
        assert_eq!(app.viewing_iteration, 2);
        assert_eq!(app.scroll_offset, 0); // Reset on change

        // Already at iteration 1, should not go below
        app.viewing_iteration = 1;
        handle_event(&mut app, AppEvent::PrevIteration, 20);
        assert_eq!(app.viewing_iteration, 1);
    }

    #[test]
    fn test_handle_event_next_iteration() {
        let mut app = App::default();
        app.current_iteration = 3;
        app.viewing_iteration = 1;
        app.scroll_offset = 10;

        handle_event(&mut app, AppEvent::NextIteration, 20);
        assert_eq!(app.viewing_iteration, 2);
        assert_eq!(app.scroll_offset, 0);

        // Already at current iteration, should not go above
        app.viewing_iteration = 3;
        handle_event(&mut app, AppEvent::NextIteration, 20);
        assert_eq!(app.viewing_iteration, 3);
    }

    #[test]
    fn test_handle_event_context_usage() {
        let mut app = App::default();

        handle_event(&mut app, AppEvent::ContextUsage(0.75), 20);
        assert!((app.context_usage - 0.75).abs() < f64::EPSILON);

        // Test clamping
        handle_event(&mut app, AppEvent::ContextUsage(1.5), 20);
        assert!((app.context_usage - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_handle_event_iteration_complete() {
        let mut app = App::default();
        app.current_iteration = 1;
        app.current_task = 0;

        handle_event(&mut app, AppEvent::IterationComplete { tasks_done: 3 }, 20);
        assert_eq!(app.current_task, 3);
        assert_eq!(app.viewing_iteration, 1);
    }

    #[test]
    fn test_handle_event_render() {
        let mut app = App::default();
        let initial_offset = app.scroll_offset;

        // Render event should not change state
        handle_event(&mut app, AppEvent::Render, 20);
        assert_eq!(app.scroll_offset, initial_offset);
        assert!(!app.should_quit);
    }
}

//! Keyboard event handling for the TUI.
//!
//! Provides a handler function that processes AppEvents and updates App state.
//! Delegates to App::update() for most events, with special handling for
//! scroll (needs viewport_height) and quit (needs to return immediately).

use crate::tui::app::{App, AppEvent};

/// Handle an AppEvent and update App state.
///
/// Delegates to App::update() for event processing, with special handling for:
/// - ScrollDown: needs viewport_height to calculate max scroll
/// - Quit: needs to return true immediately
///
/// Returns true if the app should quit.
pub fn handle_event(app: &mut App, event: AppEvent, viewport_height: u16) -> bool {
    // Special case: ScrollDown needs viewport_height for clamping
    if let AppEvent::ScrollDown = &event {
        let content_height = app.content_height_for_iteration(app.viewing_iteration);
        app.scroll_down(viewport_height, content_height);
        return app.should_quit;
    }

    // Delegate all other events to App::update()
    app.update(event);

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

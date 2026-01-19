//! TUI snapshot tests using TestBackend and insta.
//!
//! Tests verify rendering and key handling by capturing the terminal buffer
//! and comparing against approved snapshots.

use insta::assert_snapshot;
use ratatui::{backend::TestBackend, Terminal};
use rslph::tui::{render, App, AppEvent};

/// Create a test terminal with fixed 80x24 dimensions for reproducible snapshots.
fn test_terminal() -> Terminal<TestBackend> {
    let backend = TestBackend::new(80, 24);
    Terminal::new(backend).unwrap()
}

/// Create an app with populated message state for testing.
fn app_with_messages() -> App {
    let mut app = App::new(5, "claude-sonnet-4", "test-project");

    // Start iteration 1
    app.update(AppEvent::IterationStart { iteration: 1 });

    // Add some tool messages
    app.update(AppEvent::ToolMessage {
        tool_name: "Read".to_string(),
        content: "src/main.rs\n1: fn main() {\n2:     println!(\"Hello\");\n3: }".to_string(),
    });

    // Add Claude output
    app.update(AppEvent::ClaudeOutput(
        "I've read the main.rs file. It contains a simple Hello World program.".to_string(),
    ));

    // Add another tool message
    app.update(AppEvent::ToolMessage {
        tool_name: "Write".to_string(),
        content: "src/lib.rs\n// New library file".to_string(),
    });

    // Add more Claude output
    app.update(AppEvent::ClaudeOutput(
        "I've created a new library file at src/lib.rs.".to_string(),
    ));

    app
}

// ============================================================================
// Rendering Snapshot Tests (Task 1)
// ============================================================================

#[test]
fn test_initial_render() {
    let mut terminal = test_terminal();
    let app = App::new(5, "claude-sonnet-4", "test-project");

    terminal
        .draw(|frame| render(frame, &app, 10))
        .unwrap();

    // TestBackend implements Display - outputs buffer content
    assert_snapshot!(terminal.backend());
}

#[test]
fn test_with_messages() {
    let mut terminal = test_terminal();
    let app = app_with_messages();

    terminal
        .draw(|frame| render(frame, &app, 10))
        .unwrap();

    assert_snapshot!(terminal.backend());
}

#[test]
fn test_paused_state() {
    let mut terminal = test_terminal();
    let mut app = app_with_messages();

    // Set paused state
    app.update(AppEvent::TogglePause);
    assert!(app.is_paused);

    terminal
        .draw(|frame| render(frame, &app, 10))
        .unwrap();

    // Snapshot should show PAUSED overlay
    assert_snapshot!(terminal.backend());
}

// ============================================================================
// Key Handling Tests (Task 2)
// ============================================================================

#[test]
fn test_scroll_navigation() {
    let mut terminal = test_terminal();
    let mut app = App::new(5, "claude-sonnet-4", "test-project");

    // Start iteration
    app.update(AppEvent::IterationStart { iteration: 1 });

    // Add many messages to create scrollable content
    for i in 0..15 {
        app.update(AppEvent::ClaudeOutput(format!(
            "Message line {} - this is some output from Claude.",
            i + 1
        )));
    }

    // Initial render
    terminal
        .draw(|frame| render(frame, &app, 20))
        .unwrap();

    // Scroll down several times
    app.update(AppEvent::ScrollDown);
    app.update(AppEvent::ScrollDown);
    app.update(AppEvent::ScrollDown);

    // Re-render after scrolling
    terminal
        .draw(|frame| render(frame, &app, 20))
        .unwrap();

    // Verify scroll offset changed (exact value depends on content height)
    assert!(app.scroll_offset > 0, "Scroll offset should have increased");

    assert_snapshot!(terminal.backend());
}

#[test]
fn test_iteration_navigation() {
    let mut terminal = test_terminal();
    let mut app = App::new(5, "claude-sonnet-4", "test-project");

    // Create iteration 1
    app.update(AppEvent::IterationStart { iteration: 1 });
    app.update(AppEvent::ClaudeOutput(
        "Iteration 1 message - first iteration content.".to_string(),
    ));
    app.update(AppEvent::IterationComplete { tasks_done: 1 });

    // Create iteration 2
    app.update(AppEvent::IterationStart { iteration: 2 });
    app.update(AppEvent::ClaudeOutput(
        "Iteration 2 message - second iteration content.".to_string(),
    ));

    // Currently viewing iteration 2
    assert_eq!(app.viewing_iteration, 2);

    // Navigate to previous iteration
    app.update(AppEvent::PrevIteration);
    assert_eq!(app.viewing_iteration, 1);

    // Render while viewing iteration 1
    terminal
        .draw(|frame| render(frame, &app, 10))
        .unwrap();

    assert_snapshot!(terminal.backend());
}

#[test]
fn test_quit_key() {
    let mut app = App::new(5, "claude-sonnet-4", "test-project");
    assert!(!app.should_quit);

    app.update(AppEvent::Quit);

    assert!(app.should_quit, "App should_quit should be true after Quit event");
}

#[test]
fn test_toggle_pause() {
    let mut terminal = test_terminal();
    let mut app = App::new(5, "claude-sonnet-4", "test-project");

    // Start with iteration for visual context
    app.update(AppEvent::IterationStart { iteration: 1 });
    app.update(AppEvent::ClaudeOutput("Some output message.".to_string()));

    assert!(!app.is_paused, "App should start unpaused");

    // Toggle pause on
    app.update(AppEvent::TogglePause);
    assert!(app.is_paused, "App should be paused after first toggle");

    // Render paused state
    terminal
        .draw(|frame| render(frame, &app, 10))
        .unwrap();

    // Toggle pause off
    app.update(AppEvent::TogglePause);
    assert!(!app.is_paused, "App should be unpaused after second toggle");

    // Render unpaused state
    terminal
        .draw(|frame| render(frame, &app, 10))
        .unwrap();

    // Snapshot the unpaused state (no overlay)
    assert_snapshot!(terminal.backend());
}

#[test]
fn test_context_usage_display() {
    let mut terminal = test_terminal();
    let mut app = App::new(5, "claude-sonnet-4", "test-project");

    // Start iteration
    app.update(AppEvent::IterationStart { iteration: 1 });
    app.update(AppEvent::ClaudeOutput("Testing context usage display.".to_string()));

    // Set context usage to 75%
    app.update(AppEvent::ContextUsage(0.75));

    terminal
        .draw(|frame| render(frame, &app, 10))
        .unwrap();

    assert_snapshot!(terminal.backend());
}

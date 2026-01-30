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
    let mut app = App::new(5, "claude-sonnet-4", "test-project");

    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

    // TestBackend implements Display - outputs buffer content
    assert_snapshot!(terminal.backend());
}

#[test]
fn test_with_messages() {
    let mut terminal = test_terminal();
    let mut app = app_with_messages();

    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

    assert_snapshot!(terminal.backend());
}

#[test]
fn test_paused_state() {
    let mut terminal = test_terminal();
    let mut app = app_with_messages();

    // Set paused state
    app.update(AppEvent::TogglePause);
    assert!(app.is_paused);

    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

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
    terminal.draw(|frame| render(frame, &mut app, 20)).unwrap();

    // Scroll down several times
    app.update(AppEvent::ScrollDown);
    app.update(AppEvent::ScrollDown);
    app.update(AppEvent::ScrollDown);

    // Re-render after scrolling
    terminal.draw(|frame| render(frame, &mut app, 20)).unwrap();

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
    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

    assert_snapshot!(terminal.backend());
}

#[test]
fn test_quit_key() {
    let mut app = App::new(5, "claude-sonnet-4", "test-project");
    assert!(!app.should_quit);

    app.update(AppEvent::Quit);

    assert!(
        app.should_quit,
        "App should_quit should be true after Quit event"
    );
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
    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

    // Toggle pause off
    app.update(AppEvent::TogglePause);
    assert!(!app.is_paused, "App should be unpaused after second toggle");

    // Render unpaused state
    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

    // Snapshot the unpaused state (no overlay)
    assert_snapshot!(terminal.backend());
}

#[test]
fn test_context_usage_display() {
    let mut terminal = test_terminal();
    let mut app = App::new(5, "claude-sonnet-4", "test-project");

    // Start iteration
    app.update(AppEvent::IterationStart { iteration: 1 });
    app.update(AppEvent::ClaudeOutput(
        "Testing context usage display.".to_string(),
    ));

    // Set context usage to 75%
    app.update(AppEvent::ContextUsage(0.75));

    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

    assert_snapshot!(terminal.backend());
}

// ============================================================================
// Token Display Tests (Task 3 - Plan 08-03)
// ============================================================================

/// Test that status bar displays token usage values.
///
/// Verifies the status bar shows "In: X | Out: Y | CacheW: Z | CacheR: W" format
/// with deterministic token values for reproducible snapshots.
#[test]
fn test_status_bar_displays_tokens() {
    let mut terminal = test_terminal();
    let mut app = App::new(10, "claude-opus-4-5", "test-project");

    // Start iteration
    app.update(AppEvent::IterationStart { iteration: 3 });
    app.update(AppEvent::ClaudeOutput("Testing token display.".to_string()));

    // Set task progress
    app.current_task = 2;
    app.total_tasks = 5;

    // Set deterministic token values for snapshot
    // Values chosen to test human_format abbreviations (5.2k, 10.9k, etc.)
    app.update(AppEvent::TokenUsage {
        input_tokens: 5200,
        output_tokens: 10900,
        cache_creation_input_tokens: 2100,
        cache_read_input_tokens: 1500,
    });

    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

    // Snapshot should show token values in abbreviated format
    assert_snapshot!(terminal.backend());
}

/// Test status bar with zero tokens (initial state).
///
/// Verifies the status bar shows "In: 0 | Out: 0 | CacheW: 0 | CacheR: 0"
/// when no token usage has been reported.
#[test]
fn test_status_bar_zero_tokens() {
    let mut terminal = test_terminal();
    let mut app = App::new(10, "claude-opus-4-5", "test-project");
    // Default app has zero tokens

    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

    // Snapshot should show zero token values
    assert_snapshot!(terminal.backend());
}

/// Test status bar with large token values (millions).
///
/// Verifies the human_format library correctly abbreviates large numbers.
#[test]
fn test_status_bar_large_tokens() {
    let mut terminal = test_terminal();
    let mut app = App::new(10, "claude-opus-4-5", "test-project");

    // Start iteration
    app.update(AppEvent::IterationStart { iteration: 1 });

    // Set large token values (millions)
    app.update(AppEvent::TokenUsage {
        input_tokens: 1_234_567,
        output_tokens: 567_890,
        cache_creation_input_tokens: 123_456,
        cache_read_input_tokens: 789_012,
    });

    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

    // Snapshot should show abbreviated values (e.g., 1.2M, 567.9k)
    assert_snapshot!(terminal.backend());
}

/// Test that token values ACCUMULATE correctly across iterations.
///
/// This verifies the += accumulation behavior (not = overwrite).
/// Iteration 1: 1000 in, 500 out
/// Iteration 2: +2500 in, +1300 out (additional)
/// Expected total: 3500 in, 1800 out, 500 cacheW, 1200 cacheR
#[test]
fn test_token_accumulation_across_iterations() {
    let mut terminal = test_terminal();
    let mut app = App::new(10, "claude-opus-4-5", "test-project");

    // First iteration with some tokens
    app.update(AppEvent::IterationStart { iteration: 1 });
    app.update(AppEvent::TokenUsage {
        input_tokens: 1000,
        output_tokens: 500,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
    });
    app.update(AppEvent::IterationComplete { tasks_done: 1 });

    // Second iteration with MORE tokens (these ADD to iteration 1)
    app.update(AppEvent::IterationStart { iteration: 2 });
    app.update(AppEvent::TokenUsage {
        input_tokens: 2500,  // Total now: 1000 + 2500 = 3500
        output_tokens: 1300, // Total now: 500 + 1300 = 1800
        cache_creation_input_tokens: 500,
        cache_read_input_tokens: 1200,
    });

    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

    // Snapshot should show CUMULATIVE values: 3.5k in, 1.8k out
    assert_snapshot!(terminal.backend());
}

// ============================================================================
// Eval-like Build Flow Tests (Plan 10-UAT)
// ============================================================================

/// Test TUI display during a calculator-like build flow.
///
/// Simulates what users would see during the build phase of an eval,
/// with tool use messages and completion output.
#[test]
fn test_eval_build_flow_display() {
    let mut terminal = test_terminal();
    let mut app = App::new(5, "claude-sonnet-4", "calculator-eval");

    // Start iteration (simulating build phase of eval)
    app.update(AppEvent::IterationStart { iteration: 1 });

    // Claude creates the calculator file
    app.update(AppEvent::ToolMessage {
        tool_name: "Write".to_string(),
        content: "main.py\n#!/usr/bin/env python3\nimport sys\nexpr = input().strip()\nresult = eval(expr)\nprint(int(result) if isinstance(result, float) and result.is_integer() else result)".to_string(),
    });

    // Claude makes it executable
    app.update(AppEvent::ToolMessage {
        tool_name: "Bash".to_string(),
        content: "chmod +x main.py\n(exit 0)".to_string(),
    });

    // Claude's completion message
    app.update(AppEvent::ClaudeOutput(
        "I've created a Python calculator that reads expressions from stdin and outputs the result.".to_string(),
    ));

    // Token usage for this iteration
    app.update(AppEvent::TokenUsage {
        input_tokens: 3500,
        output_tokens: 850,
        cache_creation_input_tokens: 200,
        cache_read_input_tokens: 0,
    });

    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

    assert_snapshot!(terminal.backend());
}

/// Test TUI display with multiple iterations (simulating complex eval build).
#[test]
fn test_eval_multi_iteration_display() {
    let mut terminal = test_terminal();
    let mut app = App::new(5, "claude-sonnet-4", "calculator-eval");

    // Iteration 1: Initial implementation attempt
    app.update(AppEvent::IterationStart { iteration: 1 });
    app.update(AppEvent::ToolMessage {
        tool_name: "Write".to_string(),
        content: "main.py\n# First attempt".to_string(),
    });
    app.update(AppEvent::ClaudeOutput(
        "Created initial calculator.".to_string(),
    ));
    app.update(AppEvent::TokenUsage {
        input_tokens: 2000,
        output_tokens: 500,
        cache_creation_input_tokens: 100,
        cache_read_input_tokens: 0,
    });
    app.update(AppEvent::IterationComplete { tasks_done: 0 });

    // Iteration 2: Fix and complete
    app.update(AppEvent::IterationStart { iteration: 2 });
    app.update(AppEvent::ToolMessage {
        tool_name: "Read".to_string(),
        content: "main.py\n# Reading to verify".to_string(),
    });
    app.update(AppEvent::ToolMessage {
        tool_name: "Write".to_string(),
        content: "main.py\n# Fixed version with integer division".to_string(),
    });
    app.update(AppEvent::ClaudeOutput(
        "Fixed the calculator to handle integer division correctly.".to_string(),
    ));
    app.update(AppEvent::TokenUsage {
        input_tokens: 2500,
        output_tokens: 600,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 100,
    });

    // Set task completion state
    app.current_task = 1;
    app.total_tasks = 1;

    terminal.draw(|frame| render(frame, &mut app, 10)).unwrap();

    // Snapshot should show iteration 2 with cumulative tokens (4.5k in, 1.1k out)
    assert_snapshot!(terminal.backend());
}

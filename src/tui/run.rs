//! Main TUI run loop.
//!
//! Provides the async run loop that ties together terminal, events, and rendering.

use crate::error::RslphError;
use crate::tui::app::App;
use crate::tui::event::{EventHandler, SubprocessEvent};
use crate::tui::keybindings::handle_event;
use crate::tui::terminal::{init_terminal, restore_terminal};
use crate::tui::ui::render;

use tokio::sync::mpsc;

/// Run the TUI event loop.
///
/// # Arguments
///
/// * `app` - Initial app state
/// * `recent_count` - Number of recent messages to display (from config)
///
/// # Returns
///
/// * `Ok(subprocess_tx)` - Returns the subprocess event sender so caller can forward events
/// * `Err` - Terminal or I/O error
///
/// Note: This function returns immediately after starting the TUI.
/// The actual event loop runs in the returned future.
pub async fn run_tui(
    mut app: App,
    recent_count: usize,
) -> Result<mpsc::UnboundedSender<SubprocessEvent>, RslphError> {
    let mut terminal = init_terminal()
        .map_err(|e| RslphError::Subprocess(format!("Terminal init failed: {}", e)))?;

    // Create event handler with 30 FPS render rate
    let (mut event_handler, subprocess_tx) = EventHandler::new(30);

    // Spawn the main event loop
    tokio::spawn(async move {
        loop {
            // Render current state
            let viewport_height = terminal
                .size()
                .map(|s| s.height.saturating_sub(4)) // Subtract header + footer
                .unwrap_or(20);

            if let Err(e) = terminal.draw(|frame| render(frame, &app, recent_count)) {
                eprintln!("[TUI] Render error: {}", e);
                break;
            }

            // Wait for next event
            if let Some(event) = event_handler.next().await {
                if handle_event(&mut app, event, viewport_height) {
                    break;
                }
            } else {
                // Event stream ended
                break;
            }

            if app.should_quit {
                break;
            }
        }

        // Restore terminal on exit
        if let Err(e) = restore_terminal() {
            eprintln!("[TUI] Terminal restore error: {}", e);
        }
    });

    Ok(subprocess_tx)
}

/// Run the TUI synchronously until quit.
///
/// This is a blocking wrapper around the async TUI loop for simpler usage.
///
/// # Arguments
///
/// * `app` - Initial app state
/// * `recent_count` - Number of recent messages to display
pub async fn run_tui_blocking(
    mut app: App,
    recent_count: usize,
) -> Result<(), RslphError> {
    let mut terminal = init_terminal()
        .map_err(|e| RslphError::Subprocess(format!("Terminal init failed: {}", e)))?;

    // Create event handler with 30 FPS render rate
    let (mut event_handler, _subprocess_tx) = EventHandler::new(30);

    loop {
        // Render current state
        let viewport_height = terminal
            .size()
            .map(|s| s.height.saturating_sub(4)) // Subtract header + footer
            .unwrap_or(20);

        terminal
            .draw(|frame| render(frame, &app, recent_count))
            .map_err(|e| RslphError::Subprocess(format!("Render failed: {}", e)))?;

        // Wait for next event
        if let Some(event) = event_handler.next().await {
            if handle_event(&mut app, event, viewport_height) {
                break;
            }
        } else {
            // Event stream ended
            break;
        }

        if app.should_quit {
            break;
        }
    }

    restore_terminal()
        .map_err(|e| RslphError::Subprocess(format!("Terminal restore failed: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    // Note: Testing the TUI run loop requires an actual TTY.
    // These functions are tested manually and through integration tests.
}

//! Terminal setup and restoration with panic safety.
//!
//! Provides functions to initialize the terminal for TUI mode (raw mode + alternate screen)
//! and restore it to normal state. Installs panic hooks to ensure terminal is restored
//! even on crash.

use std::io::{self, Stderr};
use std::panic;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

/// Type alias for our terminal backend.
pub type Tui = Terminal<CrosstermBackend<Stderr>>;

/// Initialize the terminal for TUI mode.
///
/// This function:
/// 1. Installs a panic hook that will restore the terminal (chains with existing hook)
/// 2. Enables raw mode (disables line buffering and echo)
/// 3. Enters alternate screen (preserves original terminal content)
/// 4. Enables mouse capture (for scroll events)
///
/// Returns a Terminal instance for rendering.
///
/// # Errors
///
/// Returns an error if terminal setup fails (e.g., not a TTY).
pub fn init_terminal() -> io::Result<Tui> {
    // Install panic hook BEFORE entering raw mode.
    // This ensures terminal is restored even if we panic during setup.
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal before showing panic message
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    // Enter raw mode - disables line buffering and key echo
    enable_raw_mode()?;

    // Use stderr for the terminal backend.
    // This keeps stdout available for non-TUI output if needed.
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stderr);
    Terminal::new(backend)
}

/// Restore the terminal to its normal state.
///
/// This function:
/// 1. Disables raw mode (re-enables line buffering and echo)
/// 2. Leaves alternate screen (restores original terminal content)
/// 3. Disables mouse capture
///
/// Should be called on normal exit and is also called by the panic hook.
///
/// # Errors
///
/// Returns an error if terminal restoration fails.
pub fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    // Note: Terminal tests are difficult to run in CI/test environments
    // because they require an actual TTY. The functions are tested
    // manually and through integration tests.
}

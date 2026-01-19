//! TUI (Terminal User Interface) module for rslph.
//!
//! Provides a rich terminal interface for monitoring build progress,
//! viewing Claude output in real-time, and navigating iteration history.

mod app;
mod event;
mod keybindings;
mod run;
mod terminal;
mod ui;
mod widgets;

pub use app::{App, AppEvent, Message};
pub use event::{EventHandler, SubprocessEvent};
pub use keybindings::handle_event;
pub use run::{run_tui, run_tui_blocking};
pub use terminal::{init_terminal, restore_terminal};
pub use ui::render;

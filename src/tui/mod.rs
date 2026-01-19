//! TUI (Terminal User Interface) module for rslph.
//!
//! Provides a rich terminal interface for monitoring build progress,
//! viewing Claude output in real-time, and navigating iteration history.

mod app;
mod event;
mod terminal;

pub use app::{App, AppEvent, Message};
pub use event::EventHandler;
pub use terminal::{init_terminal, restore_terminal};

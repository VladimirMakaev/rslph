//! TUI (Terminal User Interface) module for rslph.
//!
//! Provides a rich terminal interface for monitoring build progress,
//! viewing Claude output in real-time, and navigating iteration history.

mod app;
pub mod conversation;
pub mod dashboard;
mod event;
mod keybindings;
pub mod plan_tui;
mod run;
mod terminal;
pub mod theme;
mod ui;
mod widgets;

pub use app::{App, AppEvent, Message, MessageRole};
pub use conversation::{ConversationBuffer, ConversationItem};
pub use dashboard::{run_dashboard_tui, DashboardState, TrialProgress, TrialStatus};
pub use event::{EventHandler, SubprocessEvent};
pub use keybindings::handle_event;
pub use plan_tui::{run_plan_tui, PlanStatus, PlanTuiState};
pub use run::{run_tui, run_tui_blocking};
pub use terminal::{init_terminal, restore_terminal};
pub use ui::render;

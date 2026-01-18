//! Build loop module for autonomous task execution.
//!
//! Provides the core build loop that iterates through a progress file,
//! spawning Claude subprocesses to complete tasks one at a time.

mod command;
mod iteration;
mod state;

pub use command::run_build_command;
pub use state::{BuildContext, BuildState, DoneReason, IterationResult};

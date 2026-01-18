//! Planning command and stack detection.
//!
//! Provides the `plan` command handler and project stack auto-detection.

mod command;
mod stack;
mod vagueness;

pub use command::run_plan_command;
pub use stack::{detect_stack, DetectedStack, Language};
pub use vagueness::{assess_vagueness, VaguenessScore};

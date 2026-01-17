//! Planning command and stack detection.
//!
//! Provides the `plan` command handler and project stack auto-detection.

mod stack;

pub use stack::{detect_stack, DetectedStack, Language};

// E2E test module entry point
//
// This file serves as the integration test entry point for the e2e module.
// Rust requires either tests/e2e.rs or tests/e2e/main.rs to recognize
// the e2e directory as a test crate.

mod fixtures;
mod helpers;

// Re-export for test convenience
pub use fixtures::{Workspace, WorkspaceBuilder};
pub use helpers::*;

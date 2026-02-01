// E2E test module entry point
//
// This file serves as the integration test entry point for the e2e module.
// Rust requires either tests/e2e.rs or tests/e2e/main.rs to recognize
// the e2e directory as a test crate.

mod fixtures;
mod helpers;
mod scenario_tests;

// Infrastructure verification tests (Plan 07-04)
mod test_basic_loop;
mod test_edge_cases;

// True E2E integration tests (Plan 07-05)
mod test_rslph_integration;

// Token tracking tests (Plan 08-03)
mod test_token_tracking;

// TUI snapshot tests (Plan 07.1-01)
mod tui_tests;

// Eval command tests (Plan 09-03)
mod eval_command;

// Eval integration tests with fake Claude (Plan 10-UAT)
mod test_eval_integration;

// Interactive planning tests (Phase 15)
mod test_interactive_planning;

// Include fake_claude_lib for scenario builder access
#[path = "../fake_claude_lib/mod.rs"]
mod fake_claude_lib;

// Re-export for test convenience
pub use fixtures::{Workspace, WorkspaceBuilder};
pub use helpers::*;

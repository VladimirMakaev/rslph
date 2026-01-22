//! Prompt system for loading and managing Claude system prompts.
//!
//! Provides baked-in default prompts with optional file override via config.

mod defaults;
mod loader;
mod modes;

pub use defaults::test_discovery_prompt;
pub use loader::get_build_prompt;
pub use loader::get_build_prompt_for_mode;
pub use loader::get_plan_prompt;
pub use loader::get_plan_prompt_for_mode;
pub use modes::PromptMode;

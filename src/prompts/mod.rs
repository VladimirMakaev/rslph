//! Prompt system for loading and managing Claude system prompts.
//!
//! Provides baked-in default prompts with optional file override via config.

mod defaults;
mod loader;

pub use loader::get_plan_prompt;

//! Baked-in default prompts embedded at compile time.

use super::PromptMode;

// Basic mode prompts
const BASIC_PLAN: &str = include_str!("../../prompts/basic/PROMPT_plan.md");
const BASIC_BUILD: &str = include_str!("../../prompts/basic/PROMPT_build.md");

// GSD mode prompts
const GSD_PLAN: &str = include_str!("../../prompts/gsd/PROMPT_plan.md");
const GSD_BUILD: &str = include_str!("../../prompts/gsd/PROMPT_build.md");

/// Test discovery prompt (mode-independent)
pub const TEST_DISCOVERY_PROMPT: &str = include_str!("../../prompts/PROMPT_test_discovery.md");

impl PromptMode {
    /// Get the plan prompt for this mode.
    pub fn plan_prompt(&self) -> &'static str {
        match self {
            PromptMode::Basic => BASIC_PLAN,
            PromptMode::Gsd => GSD_PLAN,
        }
    }

    /// Get the build prompt for this mode.
    pub fn build_prompt(&self) -> &'static str {
        match self {
            PromptMode::Basic => BASIC_BUILD,
            PromptMode::Gsd => GSD_BUILD,
        }
    }
}

/// Get the test discovery prompt.
pub fn test_discovery_prompt() -> &'static str {
    TEST_DISCOVERY_PROMPT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_plan_prompt_exists() {
        let prompt = PromptMode::Basic.plan_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("Planning"));
    }

    #[test]
    fn test_basic_build_prompt_exists() {
        let prompt = PromptMode::Basic.build_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("RALPH_DONE"));
    }

    #[test]
    fn test_gsd_prompts_exist() {
        let plan = PromptMode::Gsd.plan_prompt();
        let build = PromptMode::Gsd.build_prompt();
        assert!(!plan.is_empty());
        assert!(!build.is_empty());
        // GSD prompts should have deviation handling
        assert!(build.contains("deviation") || build.contains("Deviation"));
    }

    #[test]
    fn test_discovery_prompt_exists() {
        let prompt = test_discovery_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("Test Runner Discovery"));
    }
}

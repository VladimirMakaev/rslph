//! Baked-in default prompts embedded at compile time.

/// Default planning prompt embedded at compile time.
pub const PLAN_PROMPT: &str = include_str!("../../prompts/PROMPT_plan.md");

/// Default build prompt embedded at compile time.
pub const BUILD_PROMPT: &str = include_str!("../../prompts/PROMPT_build.md");

/// Get the default planning prompt.
pub fn default_plan_prompt() -> &'static str {
    PLAN_PROMPT
}

/// Get the default build prompt.
pub fn default_build_prompt() -> &'static str {
    BUILD_PROMPT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_plan_prompt_exists() {
        let prompt = default_plan_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("Planning Assistant"));
        assert!(prompt.contains("## Output Format"));
    }

    #[test]
    fn test_default_build_prompt_exists() {
        let prompt = default_build_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("Build Agent"));
        assert!(prompt.contains("RALPH_DONE"));
        assert!(prompt.contains("ONE TASK PER ITERATION"));
    }
}

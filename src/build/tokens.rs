//! Token usage tracking types and formatting utilities.
//!
//! Provides data structures for tracking API token consumption across
//! iterations and helper functions for human-readable formatting.

use human_format::Formatter;

/// Cumulative token usage across all iterations.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

impl TokenUsage {
    /// Add usage from a stream event.
    pub fn add_from_usage(&mut self, usage: &crate::subprocess::Usage) {
        self.input_tokens += usage.input_tokens;
        self.output_tokens += usage.output_tokens;
        self.cache_creation_input_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
        self.cache_read_input_tokens += usage.cache_read_input_tokens.unwrap_or(0);
    }
}

/// Token usage for a single iteration.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct IterationTokens {
    pub iteration: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

/// Format token count for display (e.g., 5.2k, 1.2M).
pub fn format_tokens(count: u64) -> String {
    if count == 0 {
        return "0".to_string();
    }

    Formatter::new()
        .with_decimals(1)
        .with_separator("")
        .format(count as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tokens_zero() {
        assert_eq!(format_tokens(0), "0");
    }

    #[test]
    fn test_format_tokens_small() {
        // Small numbers should display as-is
        let result = format_tokens(500);
        assert!(result.contains("500"), "Expected 500 in {}", result);
    }

    #[test]
    fn test_format_tokens_thousands() {
        // Should abbreviate with k suffix
        let result = format_tokens(5200);
        assert!(
            result.contains("5.2") || result.contains("5,2"),
            "Expected 5.2k-like in {}",
            result
        );
    }

    #[test]
    fn test_format_tokens_millions() {
        // Should abbreviate with M suffix
        let result = format_tokens(1_234_567);
        assert!(
            result.contains("1.2") || result.contains("1."),
            "Expected 1.2M-like in {}",
            result
        );
    }

    #[test]
    fn test_token_usage_default() {
        let usage = TokenUsage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
        assert_eq!(usage.cache_creation_input_tokens, 0);
        assert_eq!(usage.cache_read_input_tokens, 0);
    }
}

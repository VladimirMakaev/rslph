//! Prompt mode selection for different agent philosophies.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

/// Available prompt modes for plan and build commands.
///
/// Each mode represents a coherent pair of plan + build prompts
/// designed to work together with a specific philosophy.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    EnumString,
    Display,
    ValueEnum,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[clap(rename_all = "snake_case")]
pub enum PromptMode {
    /// Current rslph prompts (default for backward compatibility)
    #[default]
    Basic,
    /// GSD-adapted prompts with XML structure and must-haves
    Gsd,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_default_mode_is_basic() {
        assert_eq!(PromptMode::default(), PromptMode::Basic);
    }

    #[test]
    fn test_parse_from_string() {
        // Use fully-qualified syntax to disambiguate from clap::ValueEnum::from_str
        assert_eq!(
            <PromptMode as FromStr>::from_str("basic").unwrap(),
            PromptMode::Basic
        );
        assert_eq!(
            <PromptMode as FromStr>::from_str("gsd").unwrap(),
            PromptMode::Gsd
        );
    }

    #[test]
    fn test_display() {
        assert_eq!(PromptMode::Basic.to_string(), "basic");
        assert_eq!(PromptMode::Gsd.to_string(), "gsd");
    }

    #[test]
    fn test_serde_roundtrip() {
        let mode = PromptMode::Gsd;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"gsd\"");
        let parsed: PromptMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, mode);
    }
}

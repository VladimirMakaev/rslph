//! Centralized theme module for TUI styling.
//!
//! This module provides a unified color palette, semantic styles, and symbols
//! for consistent visual presentation across the TUI. All colors are derived
//! from Claude's brand guidelines.

/// Color constants for the TUI.
///
/// Defines both Claude brand colors and semantic role colors for consistent
/// styling across all TUI components.
pub mod colors {
    use ratatui::style::Color;

    // Claude brand colors

    /// Crail - Claude's signature orange (#C15F3C)
    ///
    /// Primary accent color, used for assistant messages and emphasis.
    pub const CRAIL: Color = Color::Rgb(193, 95, 60);

    /// Cloudy - Warm gray accent (#B1ADA1)
    ///
    /// Secondary accent for system messages and muted elements.
    pub const CLOUDY: Color = Color::Rgb(177, 173, 161);

    /// Pampas - Light cream background (#F4F3EE)
    ///
    /// Light background color for contrast.
    pub const PAMPAS: Color = Color::Rgb(244, 243, 238);

    // Semantic role colors

    /// Color for thinking/reasoning text (internal monologue).
    pub const THINKING: Color = Color::DarkGray;

    /// Color for tool call headers.
    pub const TOOL_CALL: Color = Color::Yellow;

    /// Color for tool result output.
    pub const TOOL_RESULT: Color = Color::Cyan;

    /// Color for assistant responses (uses brand color).
    pub const ASSISTANT: Color = CRAIL;

    /// Color for user input.
    pub const USER: Color = Color::Cyan;

    /// Color for system messages.
    pub const SYSTEM: Color = CLOUDY;
}

/// Symbols for model tier indication.
///
/// Different symbols indicate the capability tier of the model being used.
pub mod symbols {
    /// Filled diamond - indicates highest tier (Opus).
    pub const TIER_HIGH: &str = "\u{25c6}"; // ◆

    /// Empty diamond - indicates mid tier (Sonnet).
    pub const TIER_MID: &str = "\u{25c7}"; // ◇

    /// Circle - indicates lower tier (Haiku) or unknown models.
    pub const TIER_LOW: &str = "\u{25cb}"; // ○

    /// Returns the appropriate tier symbol based on model name.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The name of the model (case-insensitive)
    ///
    /// # Returns
    ///
    /// A symbol indicating the model's tier:
    /// - `◆` for Opus models
    /// - `◇` for Sonnet models
    /// - `○` for Haiku or unknown models
    ///
    /// # Examples
    ///
    /// ```
    /// use rslph::tui::theme::symbols::model_tier_indicator;
    ///
    /// assert_eq!(model_tier_indicator("claude-opus-4"), "\u{25c6}");
    /// assert_eq!(model_tier_indicator("claude-sonnet-3"), "\u{25c7}");
    /// assert_eq!(model_tier_indicator("claude-haiku-3"), "\u{25cb}");
    /// ```
    pub fn model_tier_indicator(model_name: &str) -> &'static str {
        let name_lower = model_name.to_lowercase();

        if name_lower.contains("opus") {
            TIER_HIGH
        } else if name_lower.contains("sonnet") {
            TIER_MID
        } else {
            // Haiku and unknown models get the lower tier symbol
            TIER_LOW
        }
    }
}

/// Style functions for semantic UI elements.
///
/// These functions return pre-configured styles for different UI roles,
/// ensuring visual consistency throughout the application.
pub mod styles {
    use super::colors;
    use ratatui::style::{Modifier, Style};

    /// Style for assistant message text.
    ///
    /// Uses Claude's brand color (Crail) for recognition.
    pub fn assistant() -> Style {
        Style::default().fg(colors::ASSISTANT)
    }

    /// Style for thinking/reasoning text.
    ///
    /// Italic dark gray to visually distinguish internal monologue.
    pub fn thinking() -> Style {
        Style::default()
            .fg(colors::THINKING)
            .add_modifier(Modifier::ITALIC)
    }

    /// Style for tool call headers.
    ///
    /// Bold yellow for visibility and emphasis.
    pub fn tool_header() -> Style {
        Style::default()
            .fg(colors::TOOL_CALL)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for tool result output.
    ///
    /// Cyan color for tool execution results.
    pub fn tool_result() -> Style {
        Style::default().fg(colors::TOOL_RESULT)
    }

    /// Style for system messages.
    ///
    /// Uses Cloudy (warm gray) for muted system notifications.
    pub fn system() -> Style {
        Style::default().fg(colors::SYSTEM)
    }

    /// Style for user input text.
    ///
    /// Bold cyan to distinguish user messages.
    pub fn user() -> Style {
        Style::default()
            .fg(colors::USER)
            .add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;

    #[test]
    fn test_model_tier_indicator_opus() {
        assert_eq!(symbols::model_tier_indicator("claude-opus-4"), symbols::TIER_HIGH);
        assert_eq!(symbols::model_tier_indicator("claude-opus-4-5"), symbols::TIER_HIGH);
        assert_eq!(symbols::model_tier_indicator("OPUS"), symbols::TIER_HIGH);
    }

    #[test]
    fn test_model_tier_indicator_sonnet() {
        assert_eq!(symbols::model_tier_indicator("claude-sonnet-3"), symbols::TIER_MID);
        assert_eq!(symbols::model_tier_indicator("claude-sonnet-4"), symbols::TIER_MID);
        assert_eq!(symbols::model_tier_indicator("SONNET"), symbols::TIER_MID);
    }

    #[test]
    fn test_model_tier_indicator_haiku_and_unknown() {
        assert_eq!(symbols::model_tier_indicator("claude-haiku-3"), symbols::TIER_LOW);
        assert_eq!(symbols::model_tier_indicator("haiku"), symbols::TIER_LOW);
        assert_eq!(symbols::model_tier_indicator("unknown-model"), symbols::TIER_LOW);
        assert_eq!(symbols::model_tier_indicator(""), symbols::TIER_LOW);
    }

    #[test]
    fn test_styles_are_configured() {
        // Verify styles don't panic and have expected properties
        let assistant_style = styles::assistant();
        assert!(assistant_style.fg.is_some());

        let thinking_style = styles::thinking();
        assert!(thinking_style.fg.is_some());
        assert!(thinking_style.add_modifier.contains(Modifier::ITALIC));

        let tool_header_style = styles::tool_header();
        assert!(tool_header_style.fg.is_some());
        assert!(tool_header_style.add_modifier.contains(Modifier::BOLD));

        let user_style = styles::user();
        assert!(user_style.fg.is_some());
        assert!(user_style.add_modifier.contains(Modifier::BOLD));
    }
}

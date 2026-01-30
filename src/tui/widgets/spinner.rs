//! Spinner widget for LLM streaming indication.
//!
//! Provides an animated braille spinner using the throbber-widgets-tui crate.
//! The spinner indicates when Claude is actively streaming a response.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::Frame;
use throbber_widgets_tui::{Throbber, ThrobberState, BRAILLE_SIX};

use crate::tui::theme::colors::CRAIL;

/// Render an animated braille spinner.
///
/// The spinner uses the BRAILLE_SIX pattern (6-dot braille animation)
/// styled with Claude's brand color (Crail).
///
/// # Arguments
///
/// * `frame` - The ratatui Frame to render into
/// * `area` - The rectangular area to render the spinner
/// * `state` - Mutable reference to the spinner state (for animation)
/// * `label` - Text label to display next to the spinner
pub fn render_spinner(frame: &mut Frame, area: Rect, state: &mut ThrobberState, label: &str) {
    let spinner = Throbber::default()
        .throbber_set(BRAILLE_SIX)
        .label(label)
        .style(Style::default().fg(CRAIL));

    frame.render_stateful_widget(spinner, area, state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_state_creation() {
        let state = ThrobberState::default();
        // ThrobberState is created successfully
        assert!(std::mem::size_of_val(&state) > 0);
    }

    #[test]
    fn test_spinner_state_can_be_ticked() {
        let mut state = ThrobberState::default();
        // Tick the spinner (should not panic)
        state.calc_next();
    }
}

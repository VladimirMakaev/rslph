//! Context progress bar widget.
//!
//! Renders a gauge showing context window usage with traffic light colors:
//! - Green: <50% usage
//! - Yellow: 50-80% usage
//! - Red: >80% usage

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Gauge,
    Frame,
};

/// Determine the traffic light color based on context usage ratio.
///
/// - ratio < 0.5: Green (plenty of context remaining)
/// - ratio >= 0.5 and < 0.8: Yellow (getting close to limit)
/// - ratio >= 0.8: Red (near limit, may need to reset)
pub fn context_bar_color(ratio: f64) -> Color {
    if ratio < 0.5 {
        Color::Green
    } else if ratio < 0.8 {
        Color::Yellow
    } else {
        Color::Red
    }
}

/// Render the context usage progress bar.
pub fn render_context_bar(frame: &mut Frame, area: Rect, ratio: f64) {
    let ratio = ratio.clamp(0.0, 1.0);
    let color = context_bar_color(ratio);
    let percent = (ratio * 100.0) as u16;

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(color))
        .ratio(ratio)
        .label(format!("{}%", percent));

    frame.render_widget(gauge, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_bar_color_green() {
        assert_eq!(context_bar_color(0.0), Color::Green);
        assert_eq!(context_bar_color(0.25), Color::Green);
        assert_eq!(context_bar_color(0.49), Color::Green);
    }

    #[test]
    fn test_context_bar_color_yellow() {
        assert_eq!(context_bar_color(0.5), Color::Yellow);
        assert_eq!(context_bar_color(0.65), Color::Yellow);
        assert_eq!(context_bar_color(0.79), Color::Yellow);
    }

    #[test]
    fn test_context_bar_color_red() {
        assert_eq!(context_bar_color(0.8), Color::Red);
        assert_eq!(context_bar_color(0.9), Color::Red);
        assert_eq!(context_bar_color(1.0), Color::Red);
    }
}

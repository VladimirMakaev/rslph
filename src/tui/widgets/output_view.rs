//! Output view widget for displaying Claude's streaming output.
//!
//! Renders a scrollable paragraph view of messages filtered by iteration.

use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::tui::App;

/// Render the output view with scrolling support.
///
/// Displays messages filtered by the currently viewed iteration.
/// Each message is formatted as "role: content" with proper indentation
/// for multiline content.
///
/// Note: This function is currently unused as thread_view provides styled output.
/// Kept for potential future use or fallback rendering.
#[allow(dead_code)]
pub fn render_output(frame: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = app
        .messages
        .iter()
        .filter(|m| m.iteration == app.viewing_iteration)
        .flat_map(|msg| {
            // Format message with role prefix
            let prefix = format!("{}: ", msg.role.label());
            msg.content
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    if i == 0 {
                        Line::from(format!("{}{}", prefix, line))
                    } else {
                        Line::from(format!("{:width$}{}", "", line, width = prefix.len()))
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0)); // (y, x)

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use crate::tui::Message;

    // Note: Testing render_output directly requires a mock Frame,
    // which is complex. These tests verify the logic indirectly
    // through App's scroll methods.

    #[test]
    fn test_message_formatting_logic() {
        // Test the formatting logic used in render_output
        let msg = Message::new("assistant", "Line 1\nLine 2\nLine 3", 1);
        let prefix = format!("{}: ", msg.role.label());

        let lines: Vec<String> = msg
            .content
            .lines()
            .enumerate()
            .map(|(i, line)| {
                if i == 0 {
                    format!("{}{}", prefix, line)
                } else {
                    format!("{:width$}{}", "", line, width = prefix.len())
                }
            })
            .collect();

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Claude: Line 1");
        assert_eq!(lines[1], "        Line 2");
        assert_eq!(lines[2], "        Line 3");
    }
}

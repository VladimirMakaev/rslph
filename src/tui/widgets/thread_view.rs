//! Thread view widget for displaying Claude conversation with role styling.
//!
//! Renders messages in a style similar to Claude CLI output, with distinct
//! colors for user, assistant, and system roles.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, Message};

/// Role colors matching Claude CLI style.
fn role_style(role: &str) -> Style {
    match role {
        "user" => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        "assistant" => Style::default().fg(Color::Green),
        "system" => Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
        _ => Style::default(),
    }
}

/// Role display name.
fn role_label(role: &str) -> &str {
    match role {
        "user" => "You",
        "assistant" => "Claude",
        "system" => "System",
        _ => role,
    }
}

/// Render a single message with role styling.
pub fn format_message(msg: &Message) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Role header line
    let header = Line::from(vec![Span::styled(
        format!("{}: ", role_label(&msg.role)),
        role_style(&msg.role),
    )]);
    lines.push(header);

    // Content lines (indented)
    for line in msg.content.lines() {
        lines.push(Line::from(format!("  {}", line)));
    }

    // Blank line between messages
    lines.push(Line::from(""));

    lines
}

/// Render thread view for current iteration.
///
/// Shows messages for the viewing_iteration, limited to recent_count.
pub fn render_thread(frame: &mut Frame, area: Rect, app: &App, recent_count: usize) {
    let messages: Vec<&Message> = app
        .messages
        .iter()
        .filter(|m| m.iteration == app.viewing_iteration)
        .collect();

    // Take last N messages
    let display_messages: Vec<&Message> = if messages.len() > recent_count {
        messages[messages.len() - recent_count..].to_vec()
    } else {
        messages
    };

    let lines: Vec<Line> = display_messages
        .iter()
        .flat_map(|m| format_message(m))
        .collect();

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_style_user() {
        let style = role_style("user");
        assert_eq!(style.fg, Some(Color::Cyan));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_role_style_assistant() {
        let style = role_style("assistant");
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn test_role_style_system() {
        let style = role_style("system");
        assert_eq!(style.fg, Some(Color::Yellow));
        assert!(style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn test_role_style_unknown() {
        let style = role_style("unknown");
        assert_eq!(style, Style::default());
    }

    #[test]
    fn test_role_label() {
        assert_eq!(role_label("user"), "You");
        assert_eq!(role_label("assistant"), "Claude");
        assert_eq!(role_label("system"), "System");
        assert_eq!(role_label("other"), "other");
    }

    #[test]
    fn test_format_message_single_line() {
        let msg = Message::new("assistant", "Hello, world!", 1);
        let lines = format_message(&msg);

        // Should have: header, content line, blank line
        assert_eq!(lines.len(), 3);
        // First line is the header with role
        // Content is indented
        // Last line is blank
    }

    #[test]
    fn test_format_message_multiline() {
        let msg = Message::new("user", "Line 1\nLine 2\nLine 3", 1);
        let lines = format_message(&msg);

        // Should have: header + 3 content lines + blank line = 5 lines
        assert_eq!(lines.len(), 5);
    }
}

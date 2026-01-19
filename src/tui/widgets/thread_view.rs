//! Thread view widget for displaying Claude conversation with role styling.
//!
//! Renders messages in a style similar to Claude CLI output, with distinct
//! colors for user, assistant, system, and tool roles.
//! Supports collapsible messages with expand/collapse toggle.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, Message, MessageRole};

/// Role colors matching Claude CLI style.
fn role_style(role: &MessageRole) -> Style {
    match role {
        MessageRole::User => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        MessageRole::Assistant => Style::default().fg(Color::Green),
        MessageRole::System => Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
        MessageRole::Tool(_) => Style::default().fg(Color::Magenta),
    }
}

/// Role display name.
fn role_label(role: &MessageRole) -> String {
    match role {
        MessageRole::User => "You".to_string(),
        MessageRole::Assistant => "Claude".to_string(),
        MessageRole::System => "System".to_string(),
        MessageRole::Tool(name) => format!("Tool:{}", name),
    }
}

/// Render a collapsed message (single line with first line preview).
fn format_collapsed(msg: &Message, is_selected: bool) -> Vec<Line<'static>> {
    let mut style = role_style(&msg.role);
    if is_selected {
        style = style.add_modifier(Modifier::REVERSED);
    }

    let label = role_label(&msg.role);

    // Get first line of content, truncated if too long
    let first_line = msg.content.lines().next().unwrap_or("");
    let preview = if first_line.len() > 60 {
        format!("{}...", &first_line[..57])
    } else {
        first_line.to_string()
    };

    let indicator = format!("{} > {}", label, preview);

    vec![Line::from(vec![Span::styled(indicator, style)])]
}

/// Render an expanded message with role styling.
fn format_expanded(msg: &Message, is_selected: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let style = role_style(&msg.role);
    let label = role_label(&msg.role);

    // Selection highlighting on header
    let header_style = if is_selected {
        style.add_modifier(Modifier::REVERSED)
    } else {
        style
    };

    // Role header line with collapse indicator
    let header = Line::from(vec![Span::styled(
        format!("{}: ", label),
        header_style,
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

/// Render a single message with role styling.
/// Handles both collapsed and expanded states.
pub fn format_message(msg: &Message, is_selected: bool) -> Vec<Line<'static>> {
    if msg.collapsed {
        format_collapsed(msg, is_selected)
    } else {
        format_expanded(msg, is_selected)
    }
}

/// Render thread view for current iteration.
///
/// Shows messages for the viewing_iteration, limited to recent_count.
/// Supports collapsed/expanded messages and selection highlighting.
pub fn render_thread(frame: &mut Frame, area: Rect, app: &App, recent_count: usize) {
    let msg_indices = app.message_indices_for_viewing();

    // Take last N messages
    let display_indices: Vec<usize> = if msg_indices.len() > recent_count {
        msg_indices[msg_indices.len() - recent_count..].to_vec()
    } else {
        msg_indices
    };

    let lines: Vec<Line> = display_indices
        .iter()
        .enumerate()
        .flat_map(|(display_idx, &msg_idx)| {
            let msg = &app.messages[msg_idx];
            // Check if this message is selected
            let is_selected = app.selected_message == Some(display_idx);
            format_message(msg, is_selected)
        })
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
        let style = role_style(&MessageRole::User);
        assert_eq!(style.fg, Some(Color::Cyan));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_role_style_assistant() {
        let style = role_style(&MessageRole::Assistant);
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn test_role_style_system() {
        let style = role_style(&MessageRole::System);
        assert_eq!(style.fg, Some(Color::Yellow));
        assert!(style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn test_role_style_tool() {
        let style = role_style(&MessageRole::Tool("Read".to_string()));
        assert_eq!(style.fg, Some(Color::Magenta));
    }

    #[test]
    fn test_role_label() {
        assert_eq!(role_label(&MessageRole::User), "You");
        assert_eq!(role_label(&MessageRole::Assistant), "Claude");
        assert_eq!(role_label(&MessageRole::System), "System");
        assert_eq!(role_label(&MessageRole::Tool("Read".to_string())), "Tool:Read");
    }

    #[test]
    fn test_format_message_expanded_single_line() {
        let msg = Message::new("assistant", "Hello, world!", 1);
        let lines = format_message(&msg, false);

        // Should have: header, content line, blank line
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_format_message_expanded_multiline() {
        let msg = Message::new("user", "Line 1\nLine 2\nLine 3", 1);
        let lines = format_message(&msg, false);

        // Should have: header + 3 content lines + blank line = 5 lines
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn test_format_message_collapsed() {
        let mut msg = Message::new("assistant", "Line 1\nLine 2\nLine 3", 1);
        msg.collapsed = true;
        let lines = format_message(&msg, false);

        // Should have: single collapsed line
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_format_message_selected() {
        let msg = Message::new("assistant", "Hello", 1);
        let lines = format_message(&msg, true);

        // Check that we got lines (selection adds REVERSED modifier)
        assert!(!lines.is_empty());
    }
}

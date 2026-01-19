//! Thread view widget for displaying Claude conversation with role styling.
//!
//! Renders messages in a style similar to Claude CLI output, with distinct
//! colors for user, assistant, system, and tool roles.
//! Supports collapsible message groups with expand/collapse toggle.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, DisplayItem, Message, MessageGroup, MessageRole};

/// Role colors matching Claude CLI style.
fn role_style(role: &MessageRole) -> Style {
    match role {
        MessageRole::User => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        MessageRole::Assistant => Style::default().fg(Color::Green),
        MessageRole::System => Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
        MessageRole::Tool(_) => Style::default().fg(Color::Magenta),
    }
}

/// Format a tool message as a single line for grouped display.
fn format_tool_line(msg: &Message, _is_last: bool, _is_collapsed_more: bool) -> String {
    // All items use same prefix - tree connectors could be added later
    let prefix = "   ";

    match &msg.role {
        MessageRole::Tool(name) => {
            // Get first line of content, truncated
            let first_line = msg.content.lines().next().unwrap_or("");
            let preview = if first_line.len() > 50 {
                format!("{}...", &first_line[..47])
            } else {
                first_line.to_string()
            };
            format!("{}{}: {}", prefix, name, preview)
        }
        MessageRole::Assistant => {
            let first_line = msg.content.lines().next().unwrap_or("");
            let preview = if first_line.len() > 50 {
                format!("{}...", &first_line[..47])
            } else {
                first_line.to_string()
            };
            format!("{}Claude: {}", prefix, preview)
        }
        _ => {
            let first_line = msg.content.lines().next().unwrap_or("");
            format!("{}{}", prefix, first_line)
        }
    }
}

/// Render a message group with Claude CLI-like tree structure.
fn format_group(group: &MessageGroup, is_selected: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Group header style
    let header_style = if is_selected {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    };

    // Header line
    lines.push(Line::from(vec![Span::styled(
        group.header.clone(),
        header_style,
    )]));

    // Get visible messages
    let visible = group.visible_messages();
    let hidden = group.hidden_count();

    // Render each visible message as a tree item
    for (i, msg) in visible.iter().enumerate() {
        let is_last = hidden == 0 && i == visible.len() - 1;
        let line_text = format_tool_line(msg, is_last, false);
        let style = role_style(&msg.role);
        lines.push(Line::from(vec![Span::styled(line_text, style)]));
    }

    // Show "+N more" if collapsed with hidden items
    if hidden > 0 {
        let more_text = format!("   +{} more tool uses (Tab to expand)", hidden);
        lines.push(Line::from(vec![Span::styled(
            more_text,
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )]));
    }

    // Blank separator
    lines.push(Line::from(""));

    lines
}

/// Render a standalone system message.
fn format_system_message(msg: &Message, is_selected: bool) -> Vec<Line<'static>> {
    let mut style = role_style(&msg.role);
    if is_selected {
        style = style.add_modifier(Modifier::REVERSED);
    }

    let first_line = msg.content.lines().next().unwrap_or("");
    let text = format!("System: {}", first_line);

    vec![
        Line::from(vec![Span::styled(text, style)]),
        Line::from(""),
    ]
}

/// Role display name.
#[allow(dead_code)]
fn role_label(role: &MessageRole) -> String {
    match role {
        MessageRole::User => "You".to_string(),
        MessageRole::Assistant => "Claude".to_string(),
        MessageRole::System => "System".to_string(),
        MessageRole::Tool(name) => format!("Tool:{}", name),
    }
}

/// Render a collapsed message (single line with first line preview).
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn format_message(msg: &Message, is_selected: bool) -> Vec<Line<'static>> {
    if msg.collapsed {
        format_collapsed(msg, is_selected)
    } else {
        format_expanded(msg, is_selected)
    }
}

/// Render thread view using grouped display.
///
/// Shows display items (groups and system messages) for the viewing_iteration.
/// Groups show tool uses in a tree structure like Claude CLI.
pub fn render_thread(frame: &mut Frame, area: Rect, app: &App, _recent_count: usize) {
    let mut lines: Vec<Line> = Vec::new();

    // Collect display items for viewing iteration
    let items_for_iter: Vec<(usize, &DisplayItem)> = app.display_items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.iteration() == app.viewing_iteration)
        .collect();

    // Render each display item
    for (display_idx, (_, item)) in items_for_iter.iter().enumerate() {
        let is_selected = app.selected_group == Some(display_idx);

        match item {
            DisplayItem::Group(group) => {
                lines.extend(format_group(group, is_selected));
            }
            DisplayItem::System(msg) => {
                lines.extend(format_system_message(msg, is_selected));
            }
        }
    }

    // Render current in-progress group if viewing current iteration
    if let Some(current_group) = app.current_group_for_viewing() {
        let is_selected = app.selected_group == Some(items_for_iter.len());
        lines.extend(format_group(current_group, is_selected));
    }

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

    #[test]
    fn test_format_group_collapsed() {
        let mut group = MessageGroup::new(1);
        group.push(Message::with_role(MessageRole::Tool("Read".to_string()), "file1.rs", 1));
        group.push(Message::with_role(MessageRole::Tool("Read".to_string()), "file2.rs", 1));
        group.push(Message::with_role(MessageRole::Tool("Read".to_string()), "file3.rs", 1));
        group.push(Message::with_role(MessageRole::Tool("Bash".to_string()), "cargo build", 1));
        group.push(Message::with_role(MessageRole::Tool("Edit".to_string()), "file4.rs", 1));

        let lines = format_group(&group, false);

        // Should have: header + 3 visible messages + "+2 more" + blank = 6 lines
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn test_format_group_expanded() {
        let mut group = MessageGroup::new(1);
        group.push(Message::with_role(MessageRole::Tool("Read".to_string()), "file1.rs", 1));
        group.push(Message::with_role(MessageRole::Tool("Read".to_string()), "file2.rs", 1));
        group.push(Message::with_role(MessageRole::Tool("Read".to_string()), "file3.rs", 1));
        group.push(Message::with_role(MessageRole::Tool("Bash".to_string()), "cargo build", 1));
        group.push(Message::with_role(MessageRole::Tool("Edit".to_string()), "file4.rs", 1));
        group.expanded = true;

        let lines = format_group(&group, false);

        // Should have: header + 5 messages + blank = 7 lines (no "+N more")
        assert_eq!(lines.len(), 7);
    }

    #[test]
    fn test_format_group_small() {
        let mut group = MessageGroup::new(1);
        group.push(Message::with_role(MessageRole::Tool("Read".to_string()), "file1.rs", 1));
        group.push(Message::with_role(MessageRole::Tool("Bash".to_string()), "cargo build", 1));

        let lines = format_group(&group, false);

        // Should have: header + 2 messages + blank = 4 lines (no "+N more" needed)
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn test_format_system_message() {
        let msg = Message::new("system", "Build started", 1);
        let lines = format_system_message(&msg, false);

        // Should have: message line + blank = 2 lines
        assert_eq!(lines.len(), 2);
    }
}

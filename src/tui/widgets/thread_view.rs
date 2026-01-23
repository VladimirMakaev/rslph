//! Thread view widget for displaying Claude conversation with role styling.
//!
//! Renders messages in a style similar to Claude CLI output, with distinct
//! colors for user, assistant, system, and tool roles.
//! Supports collapsible message groups with expand/collapse toggle.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, DisplayItem, Message, MessageGroup, MessageRole, SystemGroup};
use crate::tui::theme::{colors, styles};

/// Role colors matching Claude CLI style.
fn role_style(role: &MessageRole) -> Style {
    match role {
        MessageRole::User => styles::user(),
        MessageRole::Assistant => styles::assistant(),
        MessageRole::System => styles::system(),
        MessageRole::Tool(_) => styles::tool_header(),
    }
}

/// Unicode box-drawing characters for message group borders.
mod box_chars {
    /// Top-left corner
    pub const TOP_LEFT: &str = "\u{250c}"; // ┌
    /// Horizontal line
    pub const HORIZONTAL: &str = "\u{2500}"; // ─
    /// Vertical line
    pub const VERTICAL: &str = "\u{2502}"; // │
    /// Bottom-left corner
    pub const BOTTOM_LEFT: &str = "\u{2514}"; // └
}

/// Format a tool message as a single line for grouped display.
fn format_tool_line(msg: &Message) -> String {
    match &msg.role {
        MessageRole::Tool(name) => {
            // Get first line of content, truncated
            let first_line = msg.content.lines().next().unwrap_or("");
            let preview = if first_line.len() > 50 {
                format!("{}...", &first_line[..47])
            } else {
                first_line.to_string()
            };
            format!("{}: {}", name, preview)
        }
        MessageRole::Assistant => {
            let first_line = msg.content.lines().next().unwrap_or("");
            let preview = if first_line.len() > 50 {
                format!("{}...", &first_line[..47])
            } else {
                first_line.to_string()
            };
            format!("Claude: {}", preview)
        }
        _ => {
            let first_line = msg.content.lines().next().unwrap_or("");
            first_line.to_string()
        }
    }
}

/// Render a message group with box-drawn borders.
fn format_group(group: &MessageGroup, is_selected: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Border color - Claude groups use assistant color
    let border_color = colors::ASSISTANT;
    let border_style = Style::default().fg(border_color);

    // Header style - bold for the text, with reversed if selected
    let header_style = if is_selected {
        Style::default()
            .fg(border_color)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default().fg(border_color).add_modifier(Modifier::BOLD)
    };

    // Header line: ┌─ {header} ────
    let header_text = format!(
        "{}{} {} {}{}{}",
        box_chars::TOP_LEFT,
        box_chars::HORIZONTAL,
        group.header,
        box_chars::HORIZONTAL,
        box_chars::HORIZONTAL,
        box_chars::HORIZONTAL
    );
    lines.push(Line::from(vec![Span::styled(header_text, header_style)]));

    // Get visible messages
    let visible = group.visible_messages();
    let hidden = group.hidden_count();

    // Render each visible message with vertical border
    for msg in visible.iter() {
        let line_text = format_tool_line(msg);
        let style = role_style(&msg.role);

        // │ {content}
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", box_chars::VERTICAL), border_style),
            Span::styled(line_text, style),
        ]));
    }

    // Show "+N more" if collapsed with hidden items
    if hidden > 0 {
        let more_text = format!("+{} more tool uses (Tab to expand)", hidden);
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", box_chars::VERTICAL), border_style),
            Span::styled(
                more_text,
                Style::default().fg(colors::THINKING).add_modifier(Modifier::ITALIC),
            ),
        ]));
    }

    // Footer line: └───────
    let footer_text = format!(
        "{}{}{}{}{}{}",
        box_chars::BOTTOM_LEFT,
        box_chars::HORIZONTAL,
        box_chars::HORIZONTAL,
        box_chars::HORIZONTAL,
        box_chars::HORIZONTAL,
        box_chars::HORIZONTAL
    );
    lines.push(Line::from(vec![Span::styled(footer_text, border_style)]));

    // Blank separator
    lines.push(Line::from(""));

    lines
}

/// Render a system message group with box-drawn borders.
fn format_system_group(group: &SystemGroup, is_selected: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Border color - System groups use CLOUDY color
    let border_color = colors::SYSTEM;
    let border_style = Style::default().fg(border_color);

    // Header style - bold for the text, with reversed if selected
    let header_style = if is_selected {
        Style::default()
            .fg(border_color)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default().fg(border_color).add_modifier(Modifier::BOLD)
    };

    // Header line: ┌─ System (Iteration N) ────
    let header_label = format!("System (Iteration {})", group.iteration);
    let header_text = format!(
        "{}{} {} {}{}{}",
        box_chars::TOP_LEFT,
        box_chars::HORIZONTAL,
        header_label,
        box_chars::HORIZONTAL,
        box_chars::HORIZONTAL,
        box_chars::HORIZONTAL
    );
    lines.push(Line::from(vec![Span::styled(header_text, header_style)]));

    // Get visible messages (most recent N)
    let visible = group.visible_messages();
    let hidden = group.hidden_count();

    // Render each visible message with vertical border
    for msg in visible.iter() {
        let first_line = msg.content.lines().next().unwrap_or("");
        let style = role_style(&msg.role);

        // │ {content}
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", box_chars::VERTICAL), border_style),
            Span::styled(first_line.to_string(), style),
        ]));
    }

    // Show "+N more" if collapsed with hidden items
    if hidden > 0 {
        let more_text = format!("+{} more system messages (Tab to expand)", hidden);
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", box_chars::VERTICAL), border_style),
            Span::styled(
                more_text,
                Style::default().fg(colors::THINKING).add_modifier(Modifier::ITALIC),
            ),
        ]));
    }

    // Footer line: └───────
    let footer_text = format!(
        "{}{}{}{}{}{}",
        box_chars::BOTTOM_LEFT,
        box_chars::HORIZONTAL,
        box_chars::HORIZONTAL,
        box_chars::HORIZONTAL,
        box_chars::HORIZONTAL,
        box_chars::HORIZONTAL
    );
    lines.push(Line::from(vec![Span::styled(footer_text, border_style)]));

    // Blank separator
    lines.push(Line::from(""));

    lines
}

/// Render a standalone system message (backwards compatibility, unused).
#[allow(dead_code)]
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
/// Shows display items (groups and system groups) for the viewing_iteration.
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
            DisplayItem::SystemGroup(group) => {
                lines.extend(format_system_group(group, is_selected));
            }
        }
    }

    // Render current in-progress groups if viewing current iteration
    let mut extra_idx = items_for_iter.len();

    if let Some(current_group) = app.current_group_for_viewing() {
        let is_selected = app.selected_group == Some(extra_idx);
        lines.extend(format_group(current_group, is_selected));
        extra_idx += 1;
    }

    if let Some(current_system_group) = app.current_system_group_for_viewing() {
        let is_selected = app.selected_group == Some(extra_idx);
        lines.extend(format_system_group(current_system_group, is_selected));
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
        assert_eq!(style.fg, Some(colors::USER));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_role_style_assistant() {
        let style = role_style(&MessageRole::Assistant);
        assert_eq!(style.fg, Some(colors::ASSISTANT));
    }

    #[test]
    fn test_role_style_system() {
        let style = role_style(&MessageRole::System);
        assert_eq!(style.fg, Some(colors::SYSTEM));
    }

    #[test]
    fn test_role_style_tool() {
        let style = role_style(&MessageRole::Tool("Read".to_string()));
        assert_eq!(style.fg, Some(colors::TOOL_CALL));
        assert!(style.add_modifier.contains(Modifier::BOLD));
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

        // Should have: header + 3 visible messages + "+2 more" + footer + blank = 7 lines
        assert_eq!(lines.len(), 7);
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

        // Should have: header + 5 messages + footer + blank = 8 lines (no "+N more")
        assert_eq!(lines.len(), 8);
    }

    #[test]
    fn test_format_group_small() {
        let mut group = MessageGroup::new(1);
        group.push(Message::with_role(MessageRole::Tool("Read".to_string()), "file1.rs", 1));
        group.push(Message::with_role(MessageRole::Tool("Bash".to_string()), "cargo build", 1));

        let lines = format_group(&group, false);

        // Should have: header + 2 messages + footer + blank = 5 lines (no "+N more" needed)
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn test_format_system_message() {
        let msg = Message::new("system", "Build started", 1);
        let lines = format_system_message(&msg, false);

        // Should have: message line + blank = 2 lines
        assert_eq!(lines.len(), 2);
    }
}

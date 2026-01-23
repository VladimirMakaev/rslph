//! Conversation display types and rendering for enhanced TUI.
//!
//! Provides types and functions for displaying the full LLM conversation
//! including thinking blocks, tool calls, text output, and tool results.

use std::collections::HashMap;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::theme::{colors, styles};

// Box-drawing characters for rounded borders (thinking blocks)
const BOX_ROUNDED_TOP_LEFT: &str = "\u{256d}"; // ╭
const BOX_ROUNDED_TOP_RIGHT: &str = "\u{256e}"; // ╮
const BOX_ROUNDED_BOTTOM_LEFT: &str = "\u{2570}"; // ╰
const BOX_ROUNDED_BOTTOM_RIGHT: &str = "\u{256f}"; // ╯

// Box-drawing characters for plain borders (tool calls)
const BOX_TOP_LEFT: &str = "\u{250c}"; // ┌
const BOX_TOP_RIGHT: &str = "\u{2510}"; // ┐
const BOX_BOTTOM_LEFT: &str = "\u{2514}"; // └
const BOX_BOTTOM_RIGHT: &str = "\u{2518}"; // ┘

// Shared box-drawing characters
const BOX_HORIZONTAL: &str = "\u{2500}"; // ─
const BOX_VERTICAL: &str = "\u{2502}"; // │

// Collapse indicators
const COLLAPSE_EXPANDED: &str = "\u{25bc}"; // ▼
const COLLAPSE_COLLAPSED: &str = "\u{25b6}"; // ▶

/// A displayable item from the LLM conversation.
#[derive(Debug, Clone)]
pub enum ConversationItem {
    /// Thinking block content (Claude's internal reasoning).
    Thinking(String),
    /// Text output from the assistant.
    Text(String),
    /// Tool invocation with name and formatted summary.
    ToolUse { name: String, summary: String },
    /// Tool result (truncated for display).
    ToolResult { name: String, output: String },
    /// System message or other event.
    System(String),
}

/// Conversation buffer with ring-buffer behavior for memory efficiency.
#[derive(Debug, Clone)]
pub struct ConversationBuffer {
    items: Vec<ConversationItem>,
    max_items: usize,
}

impl Default for ConversationBuffer {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl ConversationBuffer {
    /// Create a new conversation buffer with the given capacity.
    pub fn new(max_items: usize) -> Self {
        Self {
            items: Vec::with_capacity(max_items.min(1000)),
            max_items,
        }
    }

    /// Push a new item, removing oldest if at capacity.
    pub fn push(&mut self, item: ConversationItem) {
        if self.items.len() >= self.max_items {
            self.items.remove(0); // Ring buffer behavior
        }
        self.items.push(item);
    }

    /// Get all items in the buffer.
    pub fn items(&self) -> &[ConversationItem] {
        &self.items
    }

    /// Get the number of items in the buffer.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the buffer is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Render a conversation view with scrolling support.
///
/// # Arguments
///
/// * `frame` - The frame to render to
/// * `area` - The area to render in
/// * `items` - The conversation items to display
/// * `scroll_offset` - Number of items to skip from the beginning
/// * `thinking_collapsed` - Map of item indices to their collapsed state
pub fn render_conversation(
    frame: &mut Frame,
    area: Rect,
    items: &[ConversationItem],
    scroll_offset: usize,
    thinking_collapsed: &HashMap<usize, bool>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Conversation (c to toggle, PgUp/PgDn to scroll)");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Convert items to styled lines
    let mut lines: Vec<Line> = Vec::new();
    for (index, item) in items.iter().enumerate().skip(scroll_offset) {
        let is_collapsed = thinking_collapsed.get(&index).copied().unwrap_or(false);
        lines.extend(render_item(item, index, is_collapsed, inner.width as usize));
        lines.push(Line::from("")); // Separator
    }

    // Limit to visible area
    let visible_height = inner.height as usize;
    let visible_lines: Vec<Line> = lines.into_iter().take(visible_height).collect();

    let paragraph = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Create a horizontal border line for a box.
fn make_border_line(
    left: &str,
    right: &str,
    width: usize,
    style: Style,
    header: Option<&str>,
) -> Line<'static> {
    let header_len = header.map(|h| h.len() + 2).unwrap_or(0); // +2 for spaces around header
    let remaining_width = width.saturating_sub(2 + header_len); // -2 for corners

    let horizontal_part = BOX_HORIZONTAL.repeat(remaining_width);

    if let Some(h) = header {
        Line::from(vec![
            Span::styled(left.to_string(), style),
            Span::styled(format!(" {} ", h), style.add_modifier(Modifier::BOLD)),
            Span::styled(horizontal_part, style),
            Span::styled(right.to_string(), style),
        ])
    } else {
        Line::from(vec![
            Span::styled(left.to_string(), style),
            Span::styled(horizontal_part, style),
            Span::styled(right.to_string(), style),
        ])
    }
}

/// Wrap a content line with box vertical borders.
fn wrap_with_borders(content: &str, style: Style, content_style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{} ", BOX_VERTICAL), style),
        Span::styled(content.to_string(), content_style),
    ])
}

/// Render a single conversation item into styled lines with box-drawn containers.
fn render_item(
    item: &ConversationItem,
    index: usize,
    is_collapsed: bool,
    width: usize,
) -> Vec<Line<'static>> {
    match item {
        ConversationItem::Thinking(text) => {
            render_thinking_box(text, index, is_collapsed, width)
        }
        ConversationItem::Text(text) => {
            // Normal white text (no box)
            text.lines().map(|l| Line::from(l.to_string())).collect()
        }
        ConversationItem::ToolUse { name, summary } => {
            render_tool_use_box(name, summary, width)
        }
        ConversationItem::ToolResult { name, output } => {
            render_tool_result_box(name, output, width)
        }
        ConversationItem::System(text) => {
            // System messages use theme style
            let style = styles::system();
            vec![Line::from(Span::styled(text.clone(), style))]
        }
    }
}

/// Render a thinking block with rounded box and collapse support.
fn render_thinking_box(
    text: &str,
    _index: usize,
    is_collapsed: bool,
    width: usize,
) -> Vec<Line<'static>> {
    let style = styles::thinking();
    let border_style = Style::default().fg(colors::THINKING);

    let collapse_indicator = if is_collapsed {
        COLLAPSE_COLLAPSED
    } else {
        COLLAPSE_EXPANDED
    };

    let header = format!("{} thinking", collapse_indicator);
    let mut lines = Vec::new();

    // Top border with rounded corners and header
    lines.push(make_border_line(
        BOX_ROUNDED_TOP_LEFT,
        BOX_ROUNDED_TOP_RIGHT,
        width,
        border_style,
        Some(&header),
    ));

    if !is_collapsed {
        // Content lines
        for line in text.lines().take(20) {
            // Limit displayed lines
            lines.push(wrap_with_borders(line, border_style, style));
        }

        // Show truncation indicator if content was truncated
        let line_count = text.lines().count();
        if line_count > 20 {
            lines.push(wrap_with_borders(
                &format!("... ({} more lines)", line_count - 20),
                border_style,
                style,
            ));
        }
    }

    // Bottom border with rounded corners
    lines.push(make_border_line(
        BOX_ROUNDED_BOTTOM_LEFT,
        BOX_ROUNDED_BOTTOM_RIGHT,
        width,
        border_style,
        None,
    ));

    lines
}

/// Render a tool use block with plain box.
fn render_tool_use_box(name: &str, summary: &str, width: usize) -> Vec<Line<'static>> {
    let style = styles::tool_header();
    let border_style = Style::default().fg(colors::TOOL_CALL);

    let mut lines = Vec::new();

    // Top border with tool name header
    lines.push(make_border_line(
        BOX_TOP_LEFT,
        BOX_TOP_RIGHT,
        width,
        border_style,
        Some(name),
    ));

    // Summary content
    for line in summary.lines() {
        lines.push(wrap_with_borders(line, border_style, style));
    }

    // Bottom border
    lines.push(make_border_line(
        BOX_BOTTOM_LEFT,
        BOX_BOTTOM_RIGHT,
        width,
        border_style,
        None,
    ));

    lines
}

/// Render a tool result block with plain box.
fn render_tool_result_box(name: &str, output: &str, width: usize) -> Vec<Line<'static>> {
    let style = styles::tool_result();
    let border_style = Style::default().fg(colors::TOOL_RESULT);

    let header = format!("{} result", name);
    let mut lines = Vec::new();

    // Top border with result header
    lines.push(make_border_line(
        BOX_TOP_LEFT,
        BOX_TOP_RIGHT,
        width,
        border_style,
        Some(&header),
    ));

    // Output content (limited to 5 lines)
    for line in output.lines().take(5) {
        lines.push(wrap_with_borders(line, border_style, style));
    }

    // Show truncation indicator if content was truncated
    let line_count = output.lines().count();
    if line_count > 5 {
        lines.push(wrap_with_borders(
            &format!("... ({} more lines)", line_count - 5),
            border_style,
            style,
        ));
    }

    // Bottom border
    lines.push(make_border_line(
        BOX_BOTTOM_LEFT,
        BOX_BOTTOM_RIGHT,
        width,
        border_style,
        None,
    ));

    lines
}

/// Calculate scroll offset to keep recent items visible.
#[allow(dead_code)]
pub fn calculate_scroll(item_count: usize, visible_lines: usize) -> usize {
    item_count.saturating_sub(visible_lines / 2) // Approximate lines per item
}

/// Truncate a string to max length with ellipsis.
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_buffer_new() {
        let buffer = ConversationBuffer::new(100);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_conversation_buffer_push() {
        let mut buffer = ConversationBuffer::new(100);
        buffer.push(ConversationItem::Text("hello".to_string()));
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_conversation_buffer_ring_behavior() {
        let mut buffer = ConversationBuffer::new(3);
        buffer.push(ConversationItem::Text("first".to_string()));
        buffer.push(ConversationItem::Text("second".to_string()));
        buffer.push(ConversationItem::Text("third".to_string()));
        assert_eq!(buffer.len(), 3);

        // Add fourth - should remove first
        buffer.push(ConversationItem::Text("fourth".to_string()));
        assert_eq!(buffer.len(), 3);

        // Verify first item is now "second"
        match &buffer.items()[0] {
            ConversationItem::Text(s) => assert_eq!(s, "second"),
            _ => panic!("Expected Text item"),
        }
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("", 10), "");
    }

    #[test]
    fn test_calculate_scroll() {
        assert_eq!(calculate_scroll(100, 20), 90);
        assert_eq!(calculate_scroll(10, 20), 0);
        assert_eq!(calculate_scroll(0, 20), 0);
    }

    #[test]
    fn test_conversation_item_variants() {
        let _ = ConversationItem::Thinking("thinking...".to_string());
        let _ = ConversationItem::Text("output".to_string());
        let _ = ConversationItem::ToolUse {
            name: "Read".to_string(),
            summary: "/path/to/file".to_string(),
        };
        let _ = ConversationItem::ToolResult {
            name: "Read".to_string(),
            output: "file contents".to_string(),
        };
        let _ = ConversationItem::System("system message".to_string());
    }

    #[test]
    fn test_render_thinking_box_collapsed() {
        let lines = render_thinking_box("Test thinking content", 0, true, 40);
        // Collapsed should have top and bottom border only (2 lines)
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_render_thinking_box_expanded() {
        let lines = render_thinking_box("Line 1\nLine 2\nLine 3", 0, false, 40);
        // Expanded: top border + 3 content lines + bottom border = 5 lines
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn test_render_tool_use_box() {
        let lines = render_tool_use_box("Read", "/path/to/file", 40);
        // Top border + content line + bottom border = 3 lines
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_render_tool_result_box() {
        let lines = render_tool_result_box("Read", "file contents", 40);
        // Top border + content line + bottom border = 3 lines
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_render_tool_result_box_truncation() {
        let long_output = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7";
        let lines = render_tool_result_box("Read", long_output, 40);
        // Top border + 5 content lines + truncation indicator + bottom border = 8 lines
        assert_eq!(lines.len(), 8);
    }
}

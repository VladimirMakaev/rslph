//! Conversation display types and rendering for enhanced TUI.
//!
//! Provides types and functions for displaying the full LLM conversation
//! including thinking blocks, tool calls, text output, and tool results.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

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
pub fn render_conversation(
    frame: &mut Frame,
    area: Rect,
    items: &[ConversationItem],
    scroll_offset: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Conversation (c to toggle, PgUp/PgDn to scroll)");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Convert items to styled lines
    let mut lines: Vec<Line> = Vec::new();
    for item in items.iter().skip(scroll_offset) {
        lines.extend(render_item(item));
        lines.push(Line::from("")); // Separator
    }

    // Limit to visible area
    let visible_height = inner.height as usize;
    let visible_lines: Vec<Line> = lines.into_iter().take(visible_height).collect();

    let paragraph = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Render a single conversation item into styled lines.
fn render_item(item: &ConversationItem) -> Vec<Line<'static>> {
    match item {
        ConversationItem::Thinking(text) => {
            // Gray italic for thinking
            let style = Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC);
            let prefix = Span::styled("[thinking] ", style.add_modifier(Modifier::BOLD));

            // Split into lines, each with thinking style
            text.lines()
                .enumerate()
                .map(|(i, line)| {
                    if i == 0 {
                        Line::from(vec![prefix.clone(), Span::styled(line.to_string(), style)])
                    } else {
                        Line::from(Span::styled(format!("  {}", line), style))
                    }
                })
                .collect()
        }
        ConversationItem::Text(text) => {
            // Normal white text
            text.lines().map(|l| Line::from(l.to_string())).collect()
        }
        ConversationItem::ToolUse { name, summary } => {
            // Yellow for tool use
            let style = Style::default().fg(Color::Yellow);
            vec![Line::from(vec![
                Span::styled(format!("[{}] ", name), style.add_modifier(Modifier::BOLD)),
                Span::styled(summary.clone(), style),
            ])]
        }
        ConversationItem::ToolResult { name, output } => {
            // Cyan for tool result
            let style = Style::default().fg(Color::Cyan);
            let mut lines = vec![Line::from(Span::styled(
                format!("[{} result]", name),
                style.add_modifier(Modifier::BOLD),
            ))];
            for line in output.lines().take(5) {
                // Limit result display
                lines.push(Line::from(Span::styled(format!("  {}", line), style)));
            }
            lines
        }
        ConversationItem::System(text) => {
            // Magenta for system messages
            let style = Style::default().fg(Color::Magenta);
            vec![Line::from(Span::styled(text.clone(), style))]
        }
    }
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
}

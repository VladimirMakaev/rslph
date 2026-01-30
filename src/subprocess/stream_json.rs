//! Parser for Claude CLI stream-json output format.
//!
//! The stream-json format outputs JSONL (one JSON object per line) with message events
//! containing content blocks, usage information, and other metadata.

use serde::Deserialize;

use crate::tui::conversation::ConversationItem;

/// A single line from Claude CLI's stream-json output.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamEvent {
    /// Event type: "user", "assistant", "system", "summary", etc.
    #[serde(rename = "type")]
    pub event_type: String,

    /// The message content (for user/assistant events).
    #[serde(default)]
    pub message: Option<Message>,

    /// Event UUID.
    #[serde(default)]
    pub uuid: Option<String>,

    /// Timestamp of the event.
    #[serde(default)]
    pub timestamp: Option<String>,
}

/// A message within a stream event.
#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    /// Message ID.
    #[serde(default)]
    pub id: Option<String>,

    /// Message role: "user" or "assistant".
    #[serde(default)]
    pub role: Option<String>,

    /// Content blocks (array of text, tool_use, thinking, etc.).
    #[serde(default)]
    pub content: MessageContent,

    /// Model used for this message.
    #[serde(default)]
    pub model: Option<String>,

    /// Stop reason: "end_turn", "tool_use", etc.
    #[serde(default)]
    pub stop_reason: Option<String>,

    /// Token usage statistics.
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// Content can be a string (for user messages) or an array of content blocks (for assistant messages).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple string content (typically user messages).
    Text(String),

    /// Array of content blocks (typically assistant messages).
    Blocks(Vec<ContentBlock>),

    /// Empty content.
    #[default]
    Empty,
}

/// A content block within a message.
#[derive(Debug, Clone, Deserialize)]
pub struct ContentBlock {
    /// Block type: "text", "tool_use", "thinking", "tool_result".
    #[serde(rename = "type")]
    pub block_type: String,

    /// Text content (for "text" blocks).
    #[serde(default)]
    pub text: Option<String>,

    /// Thinking content (for "thinking" blocks).
    #[serde(default)]
    pub thinking: Option<String>,

    /// Tool name (for "tool_use" blocks).
    #[serde(default)]
    pub name: Option<String>,

    /// Tool input (for "tool_use" blocks).
    #[serde(default)]
    pub input: Option<serde_json::Value>,

    /// Tool use ID (for "tool_use" and "tool_result" blocks).
    #[serde(default)]
    pub id: Option<String>,
}

/// Format tool input JSON into a human-readable summary.
///
/// Extracts key fields based on tool type:
/// - Read: shows file_path
/// - Bash: shows command
/// - Edit/Write: shows file_path
/// - Grep/Glob: shows pattern
/// - Other: shows first few fields
pub fn format_tool_summary(tool_name: &str, input_json: &str) -> String {
    // Parse JSON input
    let value: serde_json::Value = match serde_json::from_str(input_json) {
        Ok(v) => v,
        Err(_) => return input_json.to_string(), // Fallback to raw if unparseable
    };

    let obj = match value.as_object() {
        Some(o) => o,
        None => return input_json.to_string(),
    };

    match tool_name {
        "Read" => {
            if let Some(path) = obj.get("file_path").and_then(|v| v.as_str()) {
                return path.to_string();
            }
        }
        "Bash" => {
            if let Some(cmd) = obj.get("command").and_then(|v| v.as_str()) {
                // Truncate long commands
                let display = if cmd.len() > 80 {
                    format!("{}...", &cmd[..77])
                } else {
                    cmd.to_string()
                };
                return display;
            }
        }
        "Edit" => {
            if let Some(path) = obj.get("file_path").and_then(|v| v.as_str()) {
                return format!("Edit {}", path);
            }
        }
        "Write" => {
            if let Some(path) = obj.get("file_path").and_then(|v| v.as_str()) {
                return format!("Write {}", path);
            }
        }
        "Grep" => {
            let pattern = obj.get("pattern").and_then(|v| v.as_str()).unwrap_or("?");
            let path = obj.get("path").and_then(|v| v.as_str());
            return if let Some(p) = path {
                format!("Grep: {} in {}", pattern, p)
            } else {
                format!("Grep: {}", pattern)
            };
        }
        "Glob" => {
            let pattern = obj.get("pattern").and_then(|v| v.as_str()).unwrap_or("?");
            let path = obj.get("path").and_then(|v| v.as_str());
            return if let Some(p) = path {
                format!("Glob: {} in {}", pattern, p)
            } else {
                format!("Glob: {}", pattern)
            };
        }
        _ => {}
    }

    // Fallback: show compact JSON, truncated
    let compact = serde_json::to_string(&value).unwrap_or_else(|_| input_json.to_string());
    if compact.len() > 80 {
        format!("{}...", &compact[..77])
    } else {
        compact
    }
}

/// Truncate a string to max length with ellipsis for display.
fn truncate_for_display(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Usage {
    /// Input tokens used.
    #[serde(default)]
    pub input_tokens: u64,

    /// Output tokens generated.
    #[serde(default)]
    pub output_tokens: u64,

    /// Cache creation tokens (if applicable).
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,

    /// Cache read tokens (if applicable).
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

impl StreamEvent {
    /// Parse a single JSON line into a StreamEvent.
    pub fn parse(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }

    /// Extract conversation items from this event for TUI display.
    ///
    /// Returns a vector of ConversationItems representing the content blocks
    /// in this event (thinking, text, tool_use).
    pub fn extract_conversation_items(&self) -> Vec<ConversationItem> {
        let message = match &self.message {
            Some(m) => m,
            None => return vec![],
        };

        let blocks = match &message.content {
            MessageContent::Text(s) => {
                // User message as system-type display
                return vec![ConversationItem::System(format!(
                    "[User] {}",
                    truncate_for_display(s, 200)
                ))];
            }
            MessageContent::Blocks(blocks) => blocks,
            MessageContent::Empty => return vec![],
        };

        blocks
            .iter()
            .filter_map(|block| match block.block_type.as_str() {
                "thinking" => block
                    .thinking
                    .clone()
                    .map(|t| ConversationItem::Thinking(truncate_for_display(&t, 500))),
                "text" => block.text.clone().map(ConversationItem::Text),
                "tool_use" => {
                    let name = block.name.clone().unwrap_or_else(|| "unknown".to_string());
                    let input_json = block
                        .input
                        .as_ref()
                        .map(|v| serde_json::to_string(v).unwrap_or_default())
                        .unwrap_or_default();
                    let summary = format_tool_summary(&name, &input_json);
                    Some(ConversationItem::ToolUse { name, summary })
                }
                "tool_result" => {
                    // Tool results come in different events - skip for now
                    None
                }
                _ => None,
            })
            .collect()
    }

    /// Check if this is an assistant message.
    pub fn is_assistant(&self) -> bool {
        self.event_type == "assistant"
    }

    /// Extract text content from an assistant message.
    ///
    /// Returns all text blocks concatenated together.
    pub fn extract_text(&self) -> Option<String> {
        let message = self.message.as_ref()?;
        let blocks = match &message.content {
            MessageContent::Text(s) => return Some(s.clone()),
            MessageContent::Blocks(blocks) => blocks,
            MessageContent::Empty => return None,
        };

        let text: String = blocks
            .iter()
            .filter_map(|block| {
                if block.block_type == "text" {
                    block.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// Get token usage from this event.
    pub fn usage(&self) -> Option<&Usage> {
        self.message.as_ref()?.usage.as_ref()
    }

    /// Extract tool use blocks from an assistant message.
    ///
    /// Returns a vector of (tool_name, input_json) tuples.
    pub fn extract_tool_uses(&self) -> Vec<(String, String)> {
        let message = match &self.message {
            Some(m) => m,
            None => return vec![],
        };
        let blocks = match &message.content {
            MessageContent::Blocks(blocks) => blocks,
            _ => return vec![],
        };

        blocks
            .iter()
            .filter(|block| block.block_type == "tool_use")
            .filter_map(|block| {
                let name = block.name.clone()?;
                let input = block
                    .input
                    .as_ref()
                    .map(|v| {
                        // Format the input as compact JSON
                        serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string())
                    })
                    .unwrap_or_else(|| "{}".to_string());
                Some((name, input))
            })
            .collect()
    }

    /// Check if this event contains tool use.
    pub fn has_tool_use(&self) -> bool {
        let message = match &self.message {
            Some(m) => m,
            None => return false,
        };
        match &message.content {
            MessageContent::Blocks(blocks) => blocks.iter().any(|b| b.block_type == "tool_use"),
            _ => false,
        }
    }
}

/// Accumulated response from parsing multiple stream events.
#[derive(Debug, Clone, Default)]
pub struct StreamResponse {
    /// Concatenated text from all assistant messages.
    pub text: String,

    /// Total input tokens used.
    pub input_tokens: u64,

    /// Total output tokens generated.
    pub output_tokens: u64,

    /// Cache creation input tokens.
    pub cache_creation_input_tokens: u64,

    /// Cache read input tokens.
    pub cache_read_input_tokens: u64,

    /// Model used.
    pub model: Option<String>,

    /// Final stop reason.
    pub stop_reason: Option<String>,
}

impl StreamResponse {
    /// Create a new empty response.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a stream event and accumulate its content.
    pub fn process_event(&mut self, event: &StreamEvent) {
        if event.is_assistant() {
            if let Some(text) = event.extract_text() {
                self.text.push_str(&text);
            }

            if let Some(message) = &event.message {
                if self.model.is_none() {
                    self.model = message.model.clone();
                }

                if let Some(stop_reason) = &message.stop_reason {
                    self.stop_reason = Some(stop_reason.clone());
                }

                if let Some(usage) = &message.usage {
                    // Accumulate tokens from all messages in this response
                    self.input_tokens += usage.input_tokens;
                    self.output_tokens += usage.output_tokens;
                    self.cache_creation_input_tokens +=
                        usage.cache_creation_input_tokens.unwrap_or(0);
                    self.cache_read_input_tokens += usage.cache_read_input_tokens.unwrap_or(0);
                }
            }
        }
    }

    /// Parse a line and process it if valid JSON.
    ///
    /// Returns true if the line was successfully parsed and processed.
    pub fn process_line(&mut self, line: &str) -> bool {
        match StreamEvent::parse(line) {
            Ok(event) => {
                self.process_event(&event);
                true
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_assistant_text_message() {
        let json = r#"{"type":"assistant","message":{"id":"123","role":"assistant","content":[{"type":"text","text":"Hello world"}],"model":"claude-opus-4.5","stop_reason":"end_turn","usage":{"input_tokens":100,"output_tokens":50}},"uuid":"abc","timestamp":"2026-01-18T00:00:00Z"}"#;

        let event = StreamEvent::parse(json).expect("should parse");
        assert!(event.is_assistant());
        assert_eq!(event.extract_text(), Some("Hello world".to_string()));

        let usage = event.usage().expect("should have usage");
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
    }

    #[test]
    fn test_parse_user_message() {
        let json = r#"{"type":"user","message":{"role":"user","content":"Hello"},"uuid":"abc","timestamp":"2026-01-18T00:00:00Z"}"#;

        let event = StreamEvent::parse(json).expect("should parse");
        assert!(!event.is_assistant());
        assert_eq!(event.event_type, "user");
    }

    #[test]
    fn test_parse_tool_use_message() {
        let json = r#"{"type":"assistant","message":{"id":"123","role":"assistant","content":[{"type":"thinking","thinking":"Let me read the file"},{"type":"tool_use","id":"tool1","name":"Read","input":{"file_path":"/tmp/test"}}],"model":"claude-opus-4.5","stop_reason":"tool_use","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let event = StreamEvent::parse(json).expect("should parse");
        assert!(event.is_assistant());
        // Tool use messages don't have text content
        assert_eq!(event.extract_text(), None);
    }

    #[test]
    fn test_stream_response_accumulation() {
        let mut response = StreamResponse::new();

        let json1 = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello "}],"model":"claude-opus-4.5","usage":{"input_tokens":50,"output_tokens":10}}}"#;
        let json2 = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"world!"}],"stop_reason":"end_turn","usage":{"input_tokens":50,"output_tokens":20}}}"#;

        response.process_line(json1);
        response.process_line(json2);

        assert_eq!(response.text, "Hello world!");
        assert_eq!(response.model, Some("claude-opus-4.5".to_string()));
        assert_eq!(response.stop_reason, Some("end_turn".to_string()));
        // Tokens are accumulated across all messages
        assert_eq!(response.input_tokens, 100); // 50 + 50
        assert_eq!(response.output_tokens, 30); // 10 + 20
    }

    #[test]
    fn test_parse_empty_content() {
        let json = r#"{"type":"system","message":{"content":[]}}"#;

        let event = StreamEvent::parse(json).expect("should parse");
        assert_eq!(event.extract_text(), None);
    }

    #[test]
    fn test_process_invalid_json() {
        let mut response = StreamResponse::new();
        let result = response.process_line("not json");
        assert!(!result);
    }

    #[test]
    fn test_extract_tool_uses() {
        let json = r#"{"type":"assistant","message":{"id":"123","role":"assistant","content":[{"type":"thinking","thinking":"Let me read the file"},{"type":"tool_use","id":"tool1","name":"Read","input":{"file_path":"/tmp/test"}},{"type":"tool_use","id":"tool2","name":"Write","input":{"file_path":"/tmp/out","content":"hello"}}],"model":"claude-opus-4.5","stop_reason":"tool_use","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let event = StreamEvent::parse(json).expect("should parse");
        assert!(event.has_tool_use());

        let tool_uses = event.extract_tool_uses();
        assert_eq!(tool_uses.len(), 2);
        assert_eq!(tool_uses[0].0, "Read");
        assert!(tool_uses[0].1.contains("/tmp/test"));
        assert_eq!(tool_uses[1].0, "Write");
    }

    #[test]
    fn test_has_tool_use_false_for_text() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello"}]}}"#;
        let event = StreamEvent::parse(json).expect("should parse");
        assert!(!event.has_tool_use());
        assert!(event.extract_tool_uses().is_empty());
    }

    #[test]
    fn test_format_tool_summary_read() {
        let input = r#"{"file_path":"/Users/test/project/Cargo.toml"}"#;
        let result = format_tool_summary("Read", input);
        assert_eq!(result, "/Users/test/project/Cargo.toml");
    }

    #[test]
    fn test_format_tool_summary_bash() {
        let input = r#"{"command":"cargo build --release"}"#;
        let result = format_tool_summary("Bash", input);
        assert_eq!(result, "cargo build --release");
    }

    #[test]
    fn test_format_tool_summary_bash_long() {
        let long_cmd = "a".repeat(100);
        let input = format!(r#"{{"command":"{}"}}"#, long_cmd);
        let result = format_tool_summary("Bash", &input);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 83); // 80 + "..."
    }

    #[test]
    fn test_format_tool_summary_edit() {
        let input = r#"{"file_path":"/src/main.rs","old_string":"old","new_string":"new"}"#;
        let result = format_tool_summary("Edit", input);
        assert_eq!(result, "Edit /src/main.rs");
    }

    #[test]
    fn test_format_tool_summary_write() {
        let input = r#"{"file_path":"/src/lib.rs","content":"fn main() {}"}"#;
        let result = format_tool_summary("Write", input);
        assert_eq!(result, "Write /src/lib.rs");
    }

    #[test]
    fn test_format_tool_summary_grep() {
        let input = r#"{"pattern":"TODO","path":"/src"}"#;
        let result = format_tool_summary("Grep", input);
        assert_eq!(result, "Grep: TODO in /src");
    }

    #[test]
    fn test_format_tool_summary_grep_no_path() {
        let input = r#"{"pattern":"FIXME"}"#;
        let result = format_tool_summary("Grep", input);
        assert_eq!(result, "Grep: FIXME");
    }

    #[test]
    fn test_format_tool_summary_glob() {
        let input = r#"{"pattern":"*.rs","path":"/src"}"#;
        let result = format_tool_summary("Glob", input);
        assert_eq!(result, "Glob: *.rs in /src");
    }

    #[test]
    fn test_format_tool_summary_unknown_tool() {
        let input = r#"{"foo":"bar"}"#;
        let result = format_tool_summary("UnknownTool", input);
        assert_eq!(result, r#"{"foo":"bar"}"#);
    }

    #[test]
    fn test_format_tool_summary_invalid_json() {
        let input = "not json";
        let result = format_tool_summary("Read", input);
        assert_eq!(result, "not json");
    }
}

//! Parser for Claude CLI stream-json output format.
//!
//! The stream-json format outputs JSONL (one JSON object per line) with message events
//! containing content blocks, usage information, and other metadata.

use serde::Deserialize;

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
                    self.input_tokens = usage.input_tokens;
                    self.output_tokens = usage.output_tokens;
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
        assert_eq!(response.output_tokens, 20); // Last usage wins
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
}

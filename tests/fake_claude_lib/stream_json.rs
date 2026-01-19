//! Serializable stream-json event types for fake Claude binary output.
//!
//! These types mirror the deserialization types in `src/subprocess/stream_json.rs`
//! but with Serialize derive for output generation. When Claude CLI output format
//! changes, both files should be updated together.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global counter for generating unique tool IDs.
static TOOL_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Generate a unique tool ID for tool_use blocks.
pub fn next_tool_id() -> String {
    let id = TOOL_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("toolu_{:04}", id)
}

/// A single stream-json event output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEventOutput {
    /// Event type: "user", "assistant", "system", "summary", etc.
    #[serde(rename = "type")]
    pub event_type: String,

    /// The message content (for user/assistant events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<MessageOutput>,

    /// Event UUID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,

    /// Timestamp of the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// A message within a stream event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageOutput {
    /// Message ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Message role: "user" or "assistant".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Content blocks.
    pub content: MessageContentOutput,

    /// Model used for this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Stop reason: "end_turn", "tool_use", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,

    /// Token usage statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageOutput>,
}

/// Content can be a string (for user messages) or an array of content blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContentOutput {
    /// Simple string content (typically user messages).
    Text(String),

    /// Array of content blocks (typically assistant messages).
    Blocks(Vec<ContentBlockOutput>),
}

/// A content block within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlockOutput {
    /// Block type: "text", "tool_use", "thinking", "tool_result".
    #[serde(rename = "type")]
    pub block_type: String,

    /// Text content (for "text" blocks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Thinking content (for "thinking" blocks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,

    /// Tool name (for "tool_use" blocks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Tool input (for "tool_use" blocks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,

    /// Tool use ID (for "tool_use" and "tool_result" blocks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageOutput {
    /// Input tokens used.
    pub input_tokens: u64,

    /// Output tokens generated.
    pub output_tokens: u64,

    /// Cache creation tokens (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u64>,

    /// Cache read tokens (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u64>,
}

impl ContentBlockOutput {
    /// Create a text content block.
    pub fn text(text: &str) -> Self {
        Self {
            block_type: "text".to_string(),
            text: Some(text.to_string()),
            thinking: None,
            name: None,
            input: None,
            id: None,
        }
    }

    /// Create a thinking content block.
    pub fn thinking(thinking: &str) -> Self {
        Self {
            block_type: "thinking".to_string(),
            text: None,
            thinking: Some(thinking.to_string()),
            name: None,
            input: None,
            id: None,
        }
    }

    /// Create a tool_use content block.
    pub fn tool_use(id: &str, name: &str, input: serde_json::Value) -> Self {
        Self {
            block_type: "tool_use".to_string(),
            text: None,
            thinking: None,
            name: Some(name.to_string()),
            input: Some(input),
            id: Some(id.to_string()),
        }
    }

    /// Create a tool_result content block.
    pub fn tool_result(tool_use_id: &str, content: &str) -> Self {
        Self {
            block_type: "tool_result".to_string(),
            text: Some(content.to_string()),
            thinking: None,
            name: None,
            input: None,
            id: Some(tool_use_id.to_string()),
        }
    }
}

impl StreamEventOutput {
    /// Create an assistant text response event.
    pub fn assistant_text(text: &str) -> Self {
        let content_block = ContentBlockOutput {
            block_type: "text".to_string(),
            text: Some(text.to_string()),
            thinking: None,
            name: None,
            input: None,
            id: None,
        };

        Self {
            event_type: "assistant".to_string(),
            message: Some(MessageOutput {
                id: Some(format!("msg_{}", uuid_v4_simple())),
                role: Some("assistant".to_string()),
                content: MessageContentOutput::Blocks(vec![content_block]),
                model: Some("claude-opus-4-5-20251101".to_string()),
                stop_reason: Some("end_turn".to_string()),
                usage: Some(UsageOutput {
                    input_tokens: 100,
                    output_tokens: 50,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                }),
            }),
            uuid: Some(uuid_v4_simple()),
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Create a system init event (first event of session).
    pub fn system_init() -> Self {
        Self {
            event_type: "system".to_string(),
            message: None,
            uuid: Some(uuid_v4_simple()),
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Create a result/summary event.
    pub fn result(cost_usd: f64) -> Self {
        // The result event type is "result" in the real Claude CLI
        Self {
            event_type: "result".to_string(),
            message: Some(MessageOutput {
                id: None,
                role: None,
                content: MessageContentOutput::Text(format!("Cost: ${:.4}", cost_usd)),
                model: None,
                stop_reason: None,
                usage: Some(UsageOutput {
                    input_tokens: 100,
                    output_tokens: 50,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                }),
            }),
            uuid: Some(uuid_v4_simple()),
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Create a tool_use assistant event.
    ///
    /// Generates a unique tool ID automatically.
    pub fn tool_use(name: &str, input: serde_json::Value) -> Self {
        let id = next_tool_id();
        Self::assistant_with_blocks(
            vec![ContentBlockOutput::tool_use(&id, name, input)],
            Some("tool_use"),
        )
    }

    /// Create an assistant event with multiple content blocks.
    pub fn assistant_with_blocks(blocks: Vec<ContentBlockOutput>, stop_reason: Option<&str>) -> Self {
        Self {
            event_type: "assistant".to_string(),
            message: Some(MessageOutput {
                id: Some(format!("msg_{}", uuid_v4_simple())),
                role: Some("assistant".to_string()),
                content: MessageContentOutput::Blocks(blocks),
                model: Some("claude-opus-4-5-20251101".to_string()),
                stop_reason: stop_reason.map(|s| s.to_string()),
                usage: Some(UsageOutput {
                    input_tokens: 100,
                    output_tokens: 50,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                }),
            }),
            uuid: Some(uuid_v4_simple()),
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        }
    }
}

/// Generate a simple UUID-like string for testing purposes.
fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:032x}", nanos)
}

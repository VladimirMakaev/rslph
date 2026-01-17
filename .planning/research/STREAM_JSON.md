# Claude CLI Stream-JSON Research

**Researched:** 2026-01-17
**Confidence:** MEDIUM (public docs, couldn't run CLI directly)

## Overview

Claude CLI supports `--output-format stream-json` for headless/programmatic use, providing real-time JSON streaming of Claude's output suitable for parsing by automation tools like rslph.

## Usage

```bash
claude -p "Your prompt here" --output-format stream-json | while read line; do
    # Process each JSON line
    echo "Progress: $line"
done
```

## JSON Fields Available

Based on statusline script analysis and documentation:

### Context Window Information
```json
{
  "context_window": {
    "total_input_tokens": 12500,
    "total_output_tokens": 3200,
    "context_window_size": 200000
  }
}
```

### Cost & Duration
```json
{
  "cost": {
    "total_cost_usd": 2.50,
    "total_duration_ms": 45000,
    "total_lines_added": 128,
    "total_lines_removed": 51
  }
}
```

### Model Information
```json
{
  "model": {
    "display_name": "Opus 4.5"
  }
}
```

## Stream Event Types (Inferred)

Based on JSONL transcript format and streaming patterns:

- **Text output events**: Assistant responses as they stream
- **Tool use events**: When Claude invokes tools (Read, Write, Bash, etc.)
- **Tool result events**: Results from tool execution
- **Thinking events**: Claude's reasoning (if enabled)
- **System events**: Status changes, errors
- **Summary events**: Session summaries

## Key Benefits for rslph

1. **Context usage tracking**: Can display progress bar showing `total_input_tokens / context_window_size`
2. **Model display**: Get `model.display_name` for status bar
3. **Cost tracking**: Track `cost.total_cost_usd` per iteration
4. **Duration**: Track `cost.total_duration_ms` for timing
5. **Real-time output**: Parse streaming JSON for live TUI updates

## Integration Pattern

```rust
// Pseudocode for rslph integration
let mut child = Command::new("claude")
    .args(["-p", &prompt, "--output-format", "stream-json"])
    .stdout(Stdio::piped())
    .spawn()?;

let stdout = BufReader::new(child.stdout.take().unwrap());
let lines = LinesStream::new(stdout.lines());

while let Some(line) = lines.next().await {
    let event: ClaudeEvent = serde_json::from_str(&line?)?;
    match event {
        ClaudeEvent::Text { content } => update_tui_output(content),
        ClaudeEvent::ContextWindow { tokens, limit } => update_progress_bar(tokens, limit),
        ClaudeEvent::ToolUse { tool, input } => log_tool_use(tool, input),
        // ...
    }
}
```

## Open Questions

1. **Exact event schema**: Need to capture actual stream-json output to document full schema
2. **Event type field**: What field distinguishes event types?
3. **Partial text streaming**: How are partial text chunks delivered?
4. **Error events**: How are errors represented in stream-json?

## Recommendations for rslph

1. **Parse stream-json for structured data**: Use `--output-format stream-json` instead of raw text
2. **Extract context usage**: Display `context_window.total_input_tokens / context_window_size` as percentage
3. **Show model**: Display `model.display_name` in status bar
4. **Track costs**: Accumulate `cost.total_cost_usd` across iterations
5. **Define event types**: Create Rust enums for each event type with serde deserialization

## Sources

- [Claude Code Headless Docs](https://code.claude.com/docs/en/headless)
- [Claude Code Statusline Gist](https://gist.github.com/Mohamed3on/70780575570a07985916e5f50e290382) - Shows JSON field extraction
- [Claude Code Log Tool](https://github.com/daaain/claude-code-log) - JSONL transcript processing
- [Adriano Melo Blog](https://adrianomelo.com/posts/claude-code-headless.html) - stream-json usage example

## Next Steps

During implementation, capture actual stream-json output to document:
1. Full event type enumeration
2. Complete field schemas
3. Edge cases (errors, interrupts, tool failures)

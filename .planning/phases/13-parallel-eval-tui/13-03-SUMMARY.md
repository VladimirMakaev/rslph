---
phase: 13-parallel-eval-tui
plan: 03
subsystem: tui
tags: [tui, conversation, ratatui, streaming, llm-output]
dependency-graph:
  requires: [13-01]
  provides: [conversation-view, enhanced-tui]
  affects: []
tech-stack:
  added: []
  patterns: [ring-buffer, TEA-state-management, split-view-layout]
key-files:
  created:
    - src/tui/conversation.rs
  modified:
    - src/tui/mod.rs
    - src/tui/app.rs
    - src/tui/ui.rs
    - src/tui/event.rs
    - src/subprocess/stream_json.rs
decisions:
  - id: conversation-max-items
    choice: "1000 item ring buffer limit for memory efficiency"
  - id: conversation-toggle-key
    choice: "'c' key toggles split conversation view"
  - id: conversation-scroll-keys
    choice: "PageUp/PageDown scroll by 10 items"
  - id: split-view-ratio
    choice: "50/50 horizontal split for conversation and main view"
metrics:
  duration: ~6 minutes
  completed: 2026-01-22
---

# Phase 13 Plan 03: Enhanced TUI with LLM Conversation Display Summary

**One-liner:** Added scrollable conversation view to build TUI showing thinking (gray italic), tool calls (yellow), and text with 'c' toggle and PageUp/PageDown navigation.

## What Was Built

### Task 1: ConversationItem Types and Extraction
- Created `src/tui/conversation.rs` with `ConversationItem` enum:
  - `Thinking(String)` - Claude's internal reasoning
  - `Text(String)` - Text output from assistant
  - `ToolUse { name, summary }` - Tool invocation with formatted summary
  - `ToolResult { name, output }` - Tool result (truncated for display)
  - `System(String)` - System messages or events
- Created `ConversationBuffer` ring-buffer struct (max 1000 items)
- Added `extract_conversation_items()` method to `StreamEvent` in stream_json.rs
- Added helper `truncate()` function for display limiting

### Task 2: Conversation Rendering with Styled Content
- Added `render_conversation()` function for scrollable conversation display
- Added `render_item()` function with color-coded styles:
  - Thinking: gray italic with `[thinking]` prefix
  - Text: normal white
  - ToolUse: yellow with bold name
  - ToolResult: cyan with indented output
  - System: magenta
- Block borders with "Conversation" title

### Task 3: Build TUI Integration
- Added conversation state fields to `App` struct:
  - `conversation: ConversationBuffer`
  - `conversation_scroll: usize`
  - `show_conversation: bool`
- Added new `AppEvent` variants:
  - `ToggleConversation`
  - `ConversationScrollUp(usize)`
  - `ConversationScrollDown(usize)`
  - `StreamEvent(StreamEvent)`
- Added keyboard mappings in event.rs:
  - 'c' -> ToggleConversation
  - PageUp -> ConversationScrollUp(10)
  - PageDown -> ConversationScrollDown(10)
- Updated `render()` in ui.rs for split-view when conversation enabled
- Updated footer key hints with "c:conversation"
- Added unit tests for conversation functionality

## Key Technical Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Buffer size | 1000 items max | Balance between history depth and memory usage |
| Toggle key | 'c' | Mnemonic for "conversation" |
| Scroll amount | 10 items | Reasonable page jump |
| Split ratio | 50/50 horizontal | Equal visibility for both views |
| Color scheme | gray/yellow/cyan/magenta | Matches common terminal conventions |

## Files Changed

| File | Change |
|------|--------|
| src/tui/conversation.rs | New file with ConversationItem, ConversationBuffer, render functions |
| src/tui/mod.rs | Export conversation module and types |
| src/subprocess/stream_json.rs | Added extract_conversation_items() method |
| src/tui/app.rs | Added conversation state, events, and handlers |
| src/tui/event.rs | Added keyboard mappings for 'c', PageUp, PageDown |
| src/tui/ui.rs | Added split-view rendering with render_conversation |
| tests/e2e/snapshots/*.snap | Updated footer text with "c:conversation" |

## Commits

| Hash | Message |
|------|---------|
| 9a746f7 | feat(13-03): add ConversationItem types and extraction from stream-json |
| 813c554 | feat(13-03): integrate conversation view into build TUI |

## Deviations from Plan

None - plan executed exactly as written. Task 2's rendering was implemented as part of Task 1's conversation.rs file creation.

## Verification

- [x] `cargo build --release` succeeds without warnings
- [x] `cargo test` passes (266 lib tests, 99 e2e tests)
- [x] 'c' key toggles conversation view in build TUI
- [x] Thinking blocks render in gray italic
- [x] Tool calls render in yellow with bold name
- [x] PageUp/PageDown scroll conversation history
- [x] Footer shows updated key hints

## Next Phase Readiness

**Integration with 13-02 Dashboard TUI:**
- ConversationBuffer can be reused for dashboard conversation display
- render_conversation can render in dashboard panels

**Integration with 13-04 Plan TUI:**
- Same conversation infrastructure available for plan command TUI

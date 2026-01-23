---
phase: 14
plan: 04
subsystem: tui
tags: [box-drawing, containers, conversation, thinking, tool-calls]

dependency-graph:
  requires: ["14-01"]
  provides: ["box-drawn-containers", "thinking-collapse-state"]
  affects: ["14-06"]

tech-stack:
  added: []
  patterns: ["box-drawing-unicode", "collapsible-blocks"]

key-files:
  created: []
  modified:
    - src/tui/app.rs
    - src/tui/conversation.rs
    - src/tui/ui.rs
    - src/tui/plan_tui.rs

decisions:
  box-drawing-chars-unicode: "Use Unicode box-drawing characters instead of BorderType::Rounded widget for line-based rendering compatibility"
  collapse-indicator-symbols: "Use triangle symbols for collapse state: ▼ (expanded) ▶ (collapsed)"
  rounded-vs-plain-borders: "Thinking blocks use rounded corners (╭╮╰╯), tool calls use plain corners (┌┐└┘)"
  thinking-collapse-tracking: "HashMap<usize, bool> by item index for efficient collapse state lookup"

metrics:
  duration: "~5m"
  completed: "2026-01-23"
---

# Phase 14 Plan 04: Box-Drawn Containers Summary

Unicode box-drawn containers for thinking blocks and tool calls with collapsible thinking support.

## What Was Built

### Task 1: Thinking Collapse State in App
- Added `thinking_collapsed: HashMap<usize, bool>` field to App struct
- Implemented `toggle_thinking_collapse(index)` method
- Implemented `is_thinking_collapsed(index)` method
- Implemented `toggle_all_thinking_collapsed()` method
- Added unit tests for collapse functionality

### Task 2: Box-Drawn Containers in Conversation View
- Added box-drawing character constants for rounded borders (thinking): ╭ ╮ ╰ ╯
- Added box-drawing character constants for plain borders (tools): ┌ ┐ └ ┘
- Added collapse indicator constants: ▼ (expanded), ▶ (collapsed)
- Implemented `render_thinking_box()` with collapsible content
- Implemented `render_tool_use_box()` with tool name header
- Implemented `render_tool_result_box()` with result truncation
- Added helper functions `make_border_line()` and `wrap_with_borders()`
- Updated `render_item()` to dispatch to appropriate box renderer
- Added unit tests for box rendering functions

### Task 3: Updated Function Signatures and Callers
- Extended `render_conversation()` signature with `thinking_collapsed: &HashMap<usize, bool>`
- Updated `render_item()` signature with `index`, `is_collapsed`, and `width` parameters
- Updated ui.rs caller to pass `&app.thinking_collapsed`
- Updated plan_tui.rs caller to pass empty HashMap (collapse not supported in plan TUI)

## Key Implementation Details

### Box Drawing Approach
Used Unicode box-drawing characters for line-based rendering:
```rust
// Rounded (thinking blocks)
╭─ ▼ thinking ─────────────────────╮
│ content line 1                    │
│ content line 2                    │
╰──────────────────────────────────╯

// Plain (tool calls)
┌─ Read ───────────────────────────┐
│ /path/to/file                     │
└──────────────────────────────────┘
```

### Collapse Behavior
- Expanded (default): Shows full content with ▼ indicator
- Collapsed: Shows only header with ▶ indicator and no content
- Thinking blocks limited to 20 lines with truncation indicator
- Tool results limited to 5 lines with truncation indicator

## Commits

| Hash | Description |
|------|-------------|
| 8bf8d68 | feat(14-04): add thinking collapse state to App |
| 01dfb09 | feat(14-04): add box-drawn containers for conversation items |

## Files Modified

| File | Changes |
|------|---------|
| src/tui/app.rs | Added thinking_collapsed HashMap and toggle methods |
| src/tui/conversation.rs | Box-drawing constants, render functions for thinking/tool boxes |
| src/tui/ui.rs | Updated render_conversation caller to pass thinking_collapsed |
| src/tui/plan_tui.rs | Updated render_conversation caller with empty HashMap |

## Deviations from Plan

### [Rule 2 - Missing Critical] Empty HashMap import in plan_tui.rs
- **Found during:** Task 3
- **Issue:** plan_tui.rs needed HashMap import to pass empty map
- **Fix:** Added `use std::collections::HashMap;` import
- **Files modified:** src/tui/plan_tui.rs

## Next Phase Readiness

- Box containers are fully functional
- Collapse state is tracked but toggle keybinding not yet implemented (Plan 14-06)
- Theme colors from 14-01 are properly used for borders and content

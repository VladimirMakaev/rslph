---
phase: 14
plan: 06
subsystem: tui
tags: [integration, spinner, streaming, keybinding, visual-parity]

dependency-graph:
  requires: ["14-02", "14-03", "14-04", "14-05"]
  provides: ["spinner-integration", "streaming-state-events", "thinking-toggle-keybinding"]
  affects: []

tech-stack:
  added: []
  patterns: ["event-driven-state", "stateful-widget-rendering"]

key-files:
  created: []
  modified:
    - src/tui/ui.rs
    - src/tui/run.rs
    - src/tui/app.rs
    - src/tui/event.rs
    - src/tui/widgets/spinner.rs

decisions:
  spinner-area-position: "Right side of header area, 20 chars wide when streaming"
  streaming-state-trigger: "Start streaming on first non-empty StreamEvent content, stop on IterationComplete"
  thinking-toggle-key: "Key 't' toggles all thinking blocks collapsed/expanded"
  render-mutability: "Changed render signature from &App to &mut App for stateful spinner widget"

metrics:
  duration: "~4m"
  completed: "2026-01-23"
---

# Phase 14 Plan 06: Integration of Visual Parity Components Summary

Wired together all visual parity components: spinner displays during streaming, streaming state driven by events, and key binding for thinking block toggle.

## What Was Built

### Task 1: Integrate Spinner into Status Bar Area
- Added `render_spinner` import to ui.rs
- Changed `render` function signature from `&App` to `&mut App` for stateful widget access
- Added spinner rendering in header area when `app.is_streaming` is true
- Spinner appears at right side of header (20 chars wide)
- Updated run.rs callers to pass `&mut app`

### Task 2: Connect Streaming State to Events
- Modified StreamEvent handler to detect content and call `start_streaming()`
- Modified IterationComplete handler to call `stop_streaming()`
- Streaming state now driven by actual LLM events

### Task 3: Add Key Binding for Thinking Toggle
- Added `ToggleThinkingCollapse` variant to AppEvent enum
- Added handler in App::update that calls `toggle_all_thinking_collapsed()`
- Added key mapping: `'t'` -> `ToggleThinkingCollapse` in event.rs
- Updated footer help text to include "t:thinking" hint

### Cleanup
- Removed unused `SpinnerState` re-export from spinner.rs
- Updated spinner.rs tests to use `ThrobberState` directly

## Key Implementation Details

### Spinner Integration
The spinner uses `render_stateful_widget` which requires mutable state access. To accommodate this:
```rust
// ui.rs signature changed
pub fn render(frame: &mut Frame, app: &mut App, recent_count: usize)

// run.rs calls updated
terminal.draw(|frame| render(frame, &mut app, recent_count))
```

### Event-Driven Streaming
```rust
// Start on content
AppEvent::StreamEvent(stream_event) => {
    let items = stream_event.extract_conversation_items();
    if !items.is_empty() && !self.is_streaming {
        self.start_streaming();
    }
    // ... push items
}

// Stop on iteration complete
AppEvent::IterationComplete { tasks_done } => {
    // ... finalization
    self.stop_streaming();
}
```

### Key Binding
```rust
KeyCode::Char('t') => Some(AppEvent::ToggleThinkingCollapse),
```

## Commits

| Hash | Description |
|------|-------------|
| 5ef2cd6 | feat(14-06): integrate visual parity components into TUI |

## Files Modified

| File | Changes |
|------|---------|
| src/tui/ui.rs | Added spinner import, changed render signature, added spinner rendering, updated footer hints |
| src/tui/run.rs | Updated render calls to pass &mut app |
| src/tui/app.rs | Added streaming detection in StreamEvent, stop_streaming in IterationComplete, ToggleThinkingCollapse event and handler |
| src/tui/event.rs | Added 't' key mapping to ToggleThinkingCollapse |
| src/tui/widgets/spinner.rs | Removed unused SpinnerState re-export, updated tests |

## Deviations from Plan

### [Rule 2 - Missing Critical] Cleanup of unused re-export
- **Found during:** Build verification
- **Issue:** SpinnerState re-export in spinner.rs was unused (app.rs imports ThrobberState directly)
- **Fix:** Removed the unused re-export and updated tests to use ThrobberState
- **Files modified:** src/tui/widgets/spinner.rs
- **Commit:** 5ef2cd6

## Phase 14 Complete

All 6 plans in Phase 14 (TUI Visual Parity with Claude Code) are now complete:

1. 14-01: Centralized theme module with Claude brand colors
2. 14-02: Animated braille spinner widget
3. 14-03: Enhanced status bar with model tier and timer
4. 14-04: Box-drawn containers for thinking and tool calls
5. 14-05: Themed borders for thread view groups
6. 14-06: Integration of all components (this plan)

The TUI now features:
- Claude brand colors throughout
- Model tier indicators (opus/sonnet/haiku symbols)
- Session timer in status bar
- Animated spinner during LLM streaming
- Box-drawn containers for thinking blocks and tool calls
- Collapsible thinking blocks with 't' key toggle
- Themed group borders in thread view

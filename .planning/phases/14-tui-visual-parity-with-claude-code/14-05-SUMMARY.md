---
phase: 14
plan: 05
subsystem: tui
tags: [theme, colors, box-drawing, visual-parity]
dependency_graph:
  requires: [14-01]
  provides: "thread_view with theme colors and box-drawn message groups"
  affects: []
tech_stack:
  added: []
  patterns: ["theme-based-styling", "unicode-box-drawing"]
key_files:
  created: []
  modified:
    - src/tui/widgets/thread_view.rs
decisions:
  - id: group-border-colors
    choice: "Claude groups use ASSISTANT (Crail), system groups use SYSTEM (Cloudy)"
  - id: box-char-module
    choice: "Inline box_chars module with Unicode constants for borders"
metrics:
  duration: 12m
  completed: 2026-01-23
---

# Phase 14 Plan 05: Thread View Theme Integration Summary

Thread view now uses centralized theme colors and displays message groups with box-drawn borders using Unicode characters.

## What Was Built

### Task 1: Theme Color Integration

Replaced all hardcoded color references with theme module imports:

**Import added:**
```rust
use crate::tui::theme::{colors, styles};
```

**role_style function updated:**
```rust
fn role_style(role: &MessageRole) -> Style {
    match role {
        MessageRole::User => styles::user(),
        MessageRole::Assistant => styles::assistant(),
        MessageRole::System => styles::system(),
        MessageRole::Tool(_) => styles::tool_header(),
    }
}
```

**Color replacements:**
- Group headers: `Color::Green` replaced with `colors::ASSISTANT` (Crail)
- System groups: `Color::DarkGray` replaced with `colors::SYSTEM` (Cloudy)
- "+N more" hints: Use `colors::THINKING` with italic modifier

### Task 2: Box-Drawn Borders

Added Unicode box-drawing characters for message group containers:

**Box character module:**
```rust
mod box_chars {
    pub const TOP_LEFT: &str = "\u{250c}"; // ┌
    pub const HORIZONTAL: &str = "\u{2500}"; // ─
    pub const VERTICAL: &str = "\u{2502}"; // │
    pub const BOTTOM_LEFT: &str = "\u{2514}"; // └
}
```

**Message group structure:**
```
┌─ Claude (Iteration 1) ───
│ Read: file1.rs
│ Edit: file2.rs
│ Bash: cargo build
└──────
```

**Border colors are type-specific:**
- Claude groups: `colors::ASSISTANT` (Crail orange)
- System groups: `colors::SYSTEM` (Cloudy gray)

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | cf2db56 | Replace hardcoded colors with theme colors in thread_view |
| 2 | f7dbeb1 | Add box-drawn borders to message groups |

## Files Modified

- `src/tui/widgets/thread_view.rs`: Theme imports, role_style with styles, box_chars module, format_group and format_system_group with borders

## Verification Results

```
cargo check: OK (2 unrelated warnings in spinner.rs)
Theme import: Line 16 - use crate::tui::theme::{colors, styles}
Hardcoded colors: 0 (none remain)
Theme usage: 12 references to colors:: and styles::
Box characters: 10 usages of box_chars constants
Tests: All 13 thread_view tests pass
```

## Success Criteria Met

1. [x] thread_view.rs imports theme::{colors, styles}
2. [x] role_style function uses styles::user(), styles::assistant(), etc.
3. [x] No hardcoded Color::Green, Color::Yellow, Color::Magenta remain
4. [x] Group headers use colors::ASSISTANT (Crail) for Claude styling
5. [x] System groups use colors::SYSTEM (Cloudy) for styling
6. [x] Message groups display with Unicode box-drawing characters
7. [x] Box border colors are type-specific (assistant=Crail, system=Cloudy)
8. [x] cargo build succeeds

## Deviations from Plan

None - plan executed exactly as written.

## TUI-06 Status

This plan satisfies TUI-06 (box-drawn message borders with type-specific styling) at the message group level:
- Groups are enclosed in box borders (header, vertical sides, footer)
- Border colors are type-specific (ASSISTANT for Claude, SYSTEM for system)
- This matches Claude Code's visual pattern where iterations/groups are bordered rather than individual messages

## Next Phase Readiness

Thread view is now fully themed and displays visually distinct message groups. Ready for:
- 14-02: Spinner widget with brand animation
- 14-06: Box-drawn elements in conversation view

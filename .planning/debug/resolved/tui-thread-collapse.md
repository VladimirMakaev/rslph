---
status: resolved
trigger: "TUI messages not collapsible - showing flat with raw JSON content instead of collapsed 'Role > first line...'"
created: 2026-01-19T10:00:00Z
updated: 2026-01-19T10:20:00Z
---

## Current Focus

hypothesis: CONFIRMED - Two issues: 1) Tool messages default to expanded, 2) Collapsed format shows "(N lines)" not first line preview
test: Run TUI and verify Tool messages show collapsed with first line preview
expecting: Tool:Bash > cargo new calc-app..., Tool:Read > /Users/.../file.txt
next_action: Verify visually or run the app

## Symptoms

expected: Messages render as "Tool:Read > first line of content..." when collapsed, with ability to expand via Tab/Enter
actual: Each message shows flat with full raw JSON content visible ({"file_path":...} etc.)
errors: None - visual/rendering issue
reproduction: Run `cargo run -- build progress.md` with any progress file
started: After Phase 6 TUI implementation - collapse feature was supposed to be added

## Eliminated

## Evidence

- timestamp: 2026-01-19T10:05:00Z
  checked: thread_view.rs rendering logic
  found: format_message correctly checks msg.collapsed and calls format_collapsed or format_expanded
  implication: Rendering logic is correct - issue is in message state, not rendering

- timestamp: 2026-01-19T10:06:00Z
  checked: app.rs Message::new and Message::with_role
  found: Both set collapsed=false as default (line 82, 95)
  implication: All messages start expanded by default

- timestamp: 2026-01-19T10:07:00Z
  checked: app.rs enforce_system_rolling_limit
  found: Only auto-collapses SYSTEM messages when count exceeds max_system_expanded (5)
  implication: Tool messages have no auto-collapse logic

- timestamp: 2026-01-19T10:08:00Z
  checked: format_collapsed function in thread_view.rs
  found: Shows "{label} > ({line_count} lines)" format - NOT showing first line of content
  implication: Even if collapsed, it would show "(3 lines)" not "first line of content..."

## Resolution

root_cause: |
  Two issues causing the bug:
  1. Tool messages created with collapsed=false by default - no auto-collapse logic
  2. format_collapsed shows "{label} > ({line_count} lines)" instead of "{label} > first_line_preview..."

  Messages display expanded because they're never collapsed. Even if collapsed, the
  format would be wrong (showing line count, not content preview).
fix: |
  1. Modified format_collapsed() in thread_view.rs to show first line of content (truncated at 60 chars)
     instead of "(N lines)"
  2. Modified Message::with_role() in app.rs to set collapsed=true for Tool messages by default
verification: |
  - All 148 tests pass including updated test_message_with_role
  - New test_message_with_role_non_tool_not_collapsed confirms non-tool messages stay expanded
  - Tool messages now start collapsed and show first line preview
files_changed:
  - src/tui/widgets/thread_view.rs
  - src/tui/app.rs

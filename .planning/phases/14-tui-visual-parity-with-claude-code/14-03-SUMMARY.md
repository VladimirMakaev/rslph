---
phase: 14-tui-visual-parity
plan: 03
subsystem: tui
tags: [status-bar, timer, model-tier, ratatui]

dependency-graph:
  requires:
    - 14-01 (centralized theme module)
  provides:
    - Enhanced status bar with model tier indicator
    - Session timer display (HH:MM:SS or MM:SS)
    - session_start field in App state
  affects:
    - Future status bar enhancements (context usage styling)

tech-stack:
  added: []
  patterns:
    - Session timing with std::time::Instant
    - Theme symbol functions for visual indicators

key-files:
  created: []
  modified:
    - src/tui/app.rs
    - src/tui/widgets/status_bar.rs
    - tests/e2e/snapshots/*.snap (13 updated)

decisions:
  - id: status-bar-format
    choice: "tier_symbol model_name | HH:MM:SS"
    rationale: Matches Claude Code UI layout with model info and session time
  - id: timer-format
    choice: "HH:MM:SS for hours > 0, otherwise MM:SS"
    rationale: Compact display for typical sessions, full format for long runs

metrics:
  duration: 3m
  completed: 2026-01-23
---

# Phase 14 Plan 03: Enhanced Status Bar with Model Tier and Timer Summary

JWT auth with refresh rotation using jose library — *Enhanced status bar showing model tier indicator (diamond/circle symbols) and live session timer in branding line*

## What Was Built

### App State Enhancement
- Added `session_start: Instant` field to App struct
- Initialized to `Instant::now()` in Default impl
- Tracks when TUI session started for timer display

### Status Bar Enhancements
- Replaced "project (model)" with "tier_symbol model | HH:MM:SS" format
- Added `format_session_time` helper function for duration formatting
- Integrated `model_tier_indicator` from theme module
- Left side shows "rslph" (bold), right side shows model info and timer

### Display Examples
- Opus model: `rslph` | `filled_diamond claude-opus-4 | 05:23`
- Sonnet model: `rslph` | `empty_diamond claude-sonnet-4 | 03:15`
- Haiku model: `rslph` | `circle claude-haiku-3 | 00:45`

## Technical Decisions

1. **Timer Format**: MM:SS for short sessions, HH:MM:SS when hours > 0
2. **Tier Detection**: Leverages theme module's `model_tier_indicator` function
3. **Session Tracking**: Uses Instant for accurate elapsed time measurement

## Deviations from Plan

None - plan executed exactly as written.

## Testing

- 13 TUI snapshot tests updated to reflect new branding line format
- All 99 tests pass after snapshot update
- Snapshots correctly show tier indicator and timer format

## Next Phase Readiness

Ready for next wave of visual parity work (14-04, 14-05, 14-06).

Theme integration tested and working - status bar now uses centralized theme module for model tier symbols.

## Commits

| Hash | Message |
|------|---------|
| 9da566f | feat(14-03): add session_start field to App |
| 9e5928f | feat(14-03): enhance status bar with tier indicator and timer |
| 8c283b6 | test(14-03): update TUI snapshots for new status bar format |

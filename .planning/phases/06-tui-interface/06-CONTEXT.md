# Phase 6: TUI Interface - Context

**Gathered:** 2026-01-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Rich terminal UI for monitoring build execution — displays status information, live Claude output, and allows navigation through conversation history. Users can view current and previous iterations, pause/resume execution, and navigate with keyboard and mouse.

</domain>

<decisions>
## Implementation Decisions

### Layout Structure
- Top header + main body layout with footer
- Header is 2 lines tall
- Line 1: "rslph" branding on left, project name + model name on right
- Line 2: Iteration metrics, task metrics, and context usage bar
- Footer: Key binding hints (p:pause, j/k:scroll, etc.)
- Main area: Shows current iteration view (single iteration at a time, not a list)

### Thread Display
- Display messages like Claude CLI does (matching Claude's role indicators and formatting)
- Current iteration is always expanded and visible (live output)
- Previous iterations are navigable via `{`/`}` keys but not shown as a list
- Show last 10 messages by default for each iteration (configurable)
- Status header updates to show "Iteration X of Y" as you navigate

### Status Bar Content
- Line 1: "rslph" on left | "project-name (model-name)" on right
- Line 2: "Iter X/Y | Task X/Y | 45% [████████░░░░░░░░░░░]"
- Context bar shows percentage number + visual bar
- Context bar color: traffic light style (green → yellow → red as it fills)

### Keyboard Navigation
- Vim-style key bindings
- `j`/`k`: Scroll line-by-line within current iteration's messages
- `{`/`}` (Shift+[/]): Switch between iterations
- `p`: Pause/resume build execution
- `Ctrl+C`: Kill the application
- No `q` key — use Ctrl+C to exit

### Pause/Resume Behavior
- `p` toggles pause/resume
- When paused: Overlay message appears "PAUSED - press p to resume"
- Pause stops current iteration gracefully
- Resume continues from progress file (like running build again)

### Mouse Support
- Click to expand/collapse threads
- Scroll wheel for scrolling through messages
- No drag selection needed

### Claude's Discretion
- Exact styling and colors beyond traffic light context bar
- Loading states and transitions
- Error display formatting
- Exact key hints text in footer

</decisions>

<specifics>
## Specific Ideas

- Messages should look "like Claude displays messages" — match Claude CLI output formatting
- Iteration view should feel like a single-pane browser with navigation, not a stacked list
- Pause overlay should be clear and unobtrusive

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 06-tui-interface*
*Context gathered: 2026-01-19*

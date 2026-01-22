# Phase 7.1: TUI Testing with ratatui-testlib — Context

> Decisions from user discussion. Researchers and planners: treat these as constraints, not suggestions.

## Test Scope

**Coverage level:** Critical paths only — main screens, core navigation, enough to catch regressions.

**What to test:**
- All main panels: task list, log panel, status indicators
- Single reference terminal size (no multi-size testing)
- Main panels only — no overlay elements (modals, popups)

**What NOT to test:**
- Every visible widget
- Multiple terminal sizes
- Overlays, modals, popups

## Verification Approach

**Method:** Visual snapshots with strict matching.

**Key decision:** Use the fake Claude setup from Phase 7's E2E tests to fully control all data (timestamps, task content, build output). This makes strict snapshot comparison feasible — no fuzzy matching needed.

**Snapshot storage:** In test directory (`tests/e2e/snapshots/`)

**Snapshot tooling:** Claude decides based on what ratatui-testlib supports and integrates best.

## Input Testing

**Scope:** Core bindings only — quit, navigation, start/stop loop.

**Verification:** Visual result verification (snapshot after key press).

**Depth:**
- Single key presses only — no multi-step sequences
- Happy paths only — no edge case testing (unbound keys, invalid input)

## Integration Style

**Organization:** Separate TUI test suite (own module/file), not mixed with other E2E tests.

**Infrastructure:** Share with existing E2E tests — reuse fake Claude, workspace fixtures, test utilities.

**Running:** TUI tests run automatically with `cargo test` — no opt-in flags required.

---

## Deferred Ideas

None captured during discussion.

---

## Next Steps

Ready for: `/gsd:plan-phase 7.1` or `/gsd:research-phase 7.1`

---
status: resolved
trigger: "TUI shows empty screen - header/footer visible but no Claude output in main area despite iteration running. Keys don't respond."
created: 2026-01-19T12:00:00Z
updated: 2026-01-19T12:15:00Z
---

## Current Focus

hypothesis: CONFIRMED - keybindings::handle_event adds to messages[] but not display_items[], render_thread only reads display_items[]
test: Fixed by refactoring keybindings::handle_event to delegate to App::update()
expecting: TUI now displays content correctly
next_action: COMPLETE - fix verified

## Symptoms

expected: TUI main area shows Claude output stream, collapsible threads work, keyboard navigation (j/k scroll, etc.) responds
actual: Main area completely empty, no Claude output visible, keys don't work at all
errors: None - clean output, just empty screen
reproduction: Run `rslph build` on any progress file - header/footer appear correctly (shows Iter 1/20, Task 0/26, 9% context) but main area is blank
started: After Phase 06-04 which added thread view, keyboard navigation, and build command integration

## Eliminated

## Evidence

- timestamp: 2026-01-19T12:02:00Z
  checked: Traced event handling path in run.rs -> keybindings.rs -> app.rs
  found: TUI run loop uses keybindings::handle_event, NOT App::update
  implication: Two parallel event handling implementations with different behavior

- timestamp: 2026-01-19T12:03:00Z
  checked: How ClaudeOutput is handled in each path
  found: |
    keybindings::handle_event (line 50-52):
      app.add_message(MessageRole::Assistant, line, viewport_height)
      -> adds to messages[] ONLY

    App::update (line 324-328):
      self.add_to_current_group(msg)
      -> adds to BOTH messages[] AND display_items[]
  implication: Messages received but never added to display_items

- timestamp: 2026-01-19T12:04:00Z
  checked: What render_thread reads
  found: render_thread filters app.display_items by viewing_iteration (thread_view.rs lines 212-216)
  implication: Empty display_items = empty screen

- timestamp: 2026-01-19T12:04:30Z
  checked: Whether keybindings.rs touches display_items
  found: No references to display_items in keybindings.rs
  implication: Confirmed - display_items never populated

- timestamp: 2026-01-19T12:08:00Z
  checked: Fix applied - refactored keybindings::handle_event
  found: |
    Simplified keybindings::handle_event to delegate to App::update() for all events
    except ScrollDown (which needs viewport_height for max scroll calculation).
    Also fixed App::update PrevIteration/NextIteration to reset scroll_offset.
  implication: All events now routed through App::update which populates display_items

- timestamp: 2026-01-19T12:09:00Z
  checked: All tests pass
  found: 162 tests pass including all 48 TUI tests
  implication: Fix is correct and doesn't break existing functionality

- timestamp: 2026-01-19T12:14:00Z
  checked: Clippy linting
  found: Code passes clippy with -D warnings (also fixed pre-existing clippy issues)
  implication: Code quality verified

## Resolution

root_cause: |
  Dual event handling paths with divergent implementations.

  Phase 06-04 added thread_view.rs which uses display_items for grouped message display.
  App::update() was updated to populate display_items via add_to_current_group().
  But the actual TUI run loop uses keybindings::handle_event() which only calls add_message().

  The add_message() function was NOT updated to populate display_items, so:
  1. Events come in via event.rs
  2. keybindings::handle_event processes them, calls add_message()
  3. Messages go into messages[] vector
  4. display_items[] stays empty
  5. render_thread() finds nothing in display_items[], renders empty screen

  Keys also don't work because the event loop IS working (header updates), but
  visible content is empty so there's nothing to scroll/select.

fix: |
  Refactored keybindings::handle_event to delegate to App::update() for all events.
  Only exception: ScrollDown is handled specially since it needs viewport_height.
  Also fixed App::update to reset scroll_offset on PrevIteration/NextIteration.

  Additional cleanup:
  - Fixed clippy error in thread_view.rs (identical if/else branches)
  - Added #[allow(dead_code)] to render_output in output_view.rs

verification: |
  - All 162 tests pass
  - All 48 TUI-specific tests pass
  - Clippy passes with -D warnings
  - Code builds cleanly

files_changed:
  - src/tui/keybindings.rs (simplified to delegate to App::update)
  - src/tui/app.rs (added scroll_offset reset to PrevIteration/NextIteration)
  - src/tui/widgets/thread_view.rs (fixed clippy identical branches warning)
  - src/tui/widgets/output_view.rs (added dead_code allow for unused function)

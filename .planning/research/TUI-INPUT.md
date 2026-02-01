# TUI Multiline Text Input Research

**Project:** rslph v1.3 Hardening
**Dimension:** TUI Input Patterns
**Researched:** 2026-02-01
**Overall Confidence:** MEDIUM

## Executive Summary

The rslph TUI currently has a basic multiline input implementation in `plan_tui.rs` that manually handles character-by-character input with a simple underscore cursor. This approach has known issues with cursor display, multiline navigation, and lacks standard text editing features (word movement, undo/redo, selection).

The recommended solution is **tui-textarea**, a mature multiline text editor widget for ratatui. However, there is a version compatibility gap: the current tui-textarea 0.7.0 supports ratatui 0.29, while rslph uses ratatui 0.30. A draft PR (#119) exists to add ratatui 0.30 support.

## Key Findings

### Current State Analysis

The existing rslph TUI input implementation in `src/tui/plan_tui.rs`:

- Uses a simple `String` buffer (`input_buffer`)
- Character-by-character handling via `handle_input_char()`, `handle_input_backspace()`, `handle_input_newline()`
- Cursor display: appends `_` to the last line for visual indication
- No cursor positioning within text (only append/delete from end)
- No word movement, selection, copy/paste, or undo/redo
- Testing: Uses ratatui's `TestBackend` + insta snapshots (established pattern)

**Problems identified:**
1. No cursor navigation within existing text
2. No word-level operations (Ctrl+arrows, Alt+arrows)
3. No undo/redo support
4. Cursor display is a simple underscore appended to text
5. No text selection support

### Recommended Solution: tui-textarea

**Crate:** [tui-textarea](https://github.com/rhysd/tui-textarea)
**Latest Version:** 0.7.0 (October 2024)
**Supports:** ratatui 0.29 (PR #119 in draft for 0.30 support)

#### Features Provided

| Feature | Description |
|---------|-------------|
| Multiline editing | Full multi-line text support with proper cursor tracking |
| Cursor handling | Row/column positioning, visual cursor styles |
| Emacs keybindings | Ctrl+N/P/F/B navigation, Ctrl+A/E line ends, Ctrl+K delete to end |
| Word movement | Alt+F/B for word-level navigation |
| Undo/redo | Ctrl+U (undo), Ctrl+R (redo) |
| Text selection | Visual selection with copy/cut/paste |
| Line numbers | Optional line number display |
| Search | Optional regex-based search (feature flag) |
| Mouse support | Scroll and click positioning |

#### API Overview

```rust
use tui_textarea::TextArea;

// Creation
let mut textarea = TextArea::default();
let mut textarea = TextArea::new(vec!["line1".into(), "line2".into()]);

// Input handling - handles crossterm KeyEvent directly
textarea.input(crossterm_key_event);  // With Emacs-like shortcuts
textarea.input_without_shortcuts(crossterm_key_event);  // Basic only

// Content access
let lines: &[String] = textarea.lines();
let owned: Vec<String> = textarea.into_lines();

// Cursor
let (row, col) = textarea.cursor();
textarea.move_cursor(CursorMove::Forward);
textarea.move_cursor(CursorMove::Jump(row, col));

// Rendering (implements Widget trait)
frame.render_widget(textarea.widget(), area);
```

### Version Compatibility Strategy

**Current situation:**
- rslph uses ratatui 0.30
- tui-textarea 0.7.0 supports ratatui 0.29
- PR #119 (draft) adds ratatui 0.30 support

**Options (in order of preference):**

1. **Wait for tui-textarea 0.8.0** (if released soon)
   - Cleanest solution
   - Check PR #119 status before implementation

2. **Use git dependency temporarily**
   ```toml
   [dependencies]
   tui-textarea = { git = "https://github.com/rhysd/tui-textarea", branch = "main" }
   ```
   - Risk: unstable API
   - Pro: immediate access to 0.30 support if merged

3. **Fork and patch**
   - Fork tui-textarea, apply PR #119 changes
   - Maintain until upstream releases

4. **Downgrade ratatui to 0.29**
   - Not recommended: regresses other features

**Recommendation:** Check PR #119 status at implementation time. If merged to main, use git dependency. If released as 0.8.0, use crates.io version. Otherwise, consider fork.

## Integration Approach

### Integration with Existing Code

The current `PlanTuiState` in `src/tui/plan_tui.rs` maintains:
- `input_mode: InputMode` - Normal vs AnsweringQuestions
- `input_buffer: String` - Current user input
- `pending_questions: Vec<String>` - Questions being answered

**Proposed changes:**

```rust
// Replace input_buffer with TextArea
pub struct PlanTuiState {
    // ... existing fields ...

    // Replace: pub input_buffer: String,
    pub textarea: Option<TextArea<'static>>,

    // Keep existing
    pub input_mode: InputMode,
    pub pending_questions: Vec<String>,
    pub session_id: Option<String>,
}
```

**Key integration points:**

1. **Enter question mode:**
   ```rust
   fn enter_question_mode(&mut self, questions: Vec<String>, session_id: String) {
       self.textarea = Some(TextArea::default());
       // ... rest unchanged
   }
   ```

2. **Handle input:**
   ```rust
   fn handle_input_key(&mut self, key_event: KeyEvent) {
       if let Some(ref mut textarea) = self.textarea {
           textarea.input(key_event);
       }
   }
   ```

3. **Get answers:**
   ```rust
   fn get_answers(&self) -> String {
       self.textarea
           .as_ref()
           .map(|ta| ta.lines().join("\n"))
           .unwrap_or_default()
   }
   ```

4. **Render:**
   ```rust
   fn render_input_area(&self, frame: &mut Frame, area: Rect) {
       if let Some(ref textarea) = self.textarea {
           frame.render_widget(textarea.widget(), area);
       }
   }
   ```

### Same Pattern for Main TUI

The main TUI in `src/tui/app.rs` also has basic input handling:
- `input_mode: bool`
- `input_buffer: String`
- `current_question: Option<String>`

Apply the same pattern: replace `input_buffer: String` with `textarea: Option<TextArea<'static>>`.

## Testing Strategy

### Existing Test Pattern

The project already has TUI snapshot tests in `tests/e2e/tui_tests.rs`:

```rust
use ratatui::{backend::TestBackend, Terminal};
use insta::assert_snapshot;

fn test_terminal() -> Terminal<TestBackend> {
    let backend = TestBackend::new(80, 24);
    Terminal::new(backend).unwrap()
}

#[test]
fn test_render_snapshot() {
    let mut terminal = test_terminal();
    // ... set up state ...
    terminal.draw(|frame| render(frame, &state)).unwrap();
    assert_snapshot!(terminal.backend());
}
```

### Testing TextArea Integration

**Unit tests for input state:**
```rust
#[test]
fn test_textarea_input_handling() {
    let mut state = PlanTuiState::new();
    state.enter_question_mode(vec!["Q?".into()], "session".into());

    // Simulate typing
    let key = KeyEvent::new(KeyCode::Char('H'), KeyModifiers::empty());
    state.handle_input_key(key);

    let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty());
    state.handle_input_key(key);

    assert_eq!(state.get_answers(), "Hi");
}

#[test]
fn test_textarea_multiline() {
    let mut state = PlanTuiState::new();
    state.enter_question_mode(vec!["Q?".into()], "session".into());

    // Type "Line1\nLine2"
    for c in "Line1".chars() {
        state.handle_input_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
    }
    state.handle_input_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    for c in "Line2".chars() {
        state.handle_input_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
    }

    assert_eq!(state.get_answers(), "Line1\nLine2");
}
```

**Snapshot tests for rendering:**
```rust
#[test]
fn test_input_mode_with_textarea() {
    let mut terminal = test_terminal();
    let mut state = PlanTuiState::new();
    state.enter_question_mode(vec!["What is your answer?".into()], "session".into());

    // Add some text
    state.textarea.as_mut().unwrap().insert_str("My multiline\nanswer here");

    terminal.draw(|frame| render_plan_tui(frame, &state)).unwrap();
    assert_snapshot!(terminal.backend());
}
```

### ratatui-testlib (Optional Enhancement)

The [ratatui-testlib](https://docs.rs/ratatui-testlib/latest/ratatui_testlib/) crate provides:
- PTY-based testing with real terminal emulation
- Keyboard event injection via `harness.send_text("hello")`
- Screen state capture for assertions

**For rslph, this is optional.** The existing TestBackend + insta pattern is sufficient for:
- Rendering verification (visual snapshots)
- State management tests (unit tests)

ratatui-testlib would be useful for:
- End-to-end integration tests with actual key sequences
- Testing complex multi-key interactions

**Recommendation:** Keep existing TestBackend pattern. Add ratatui-testlib only if complex key sequence testing becomes necessary.

## Alternatives Considered

### edtui

[edtui](https://github.com/preiter93/edtui) - Vim-inspired editor widget

**Pros:**
- Vim modal editing
- Syntax highlighting
- Full code editor features

**Cons:**
- More complex than needed for Q&A input
- Vim mode may confuse non-vim users
- Overkill for simple text input

**Verdict:** Not recommended. tui-textarea's Emacs-style bindings are more intuitive for general users.

### ratatui-code-editor

[ratatui-code-editor](https://github.com/vipmax/ratatui-code-editor) - Code editor widget

**Pros:**
- Syntax highlighting
- Line numbers

**Cons:**
- Designed for code, not general text
- Less mature than tui-textarea

**Verdict:** Not recommended. Over-engineered for Q&A input use case.

### Manual Implementation (Current Approach)

Keep and improve the existing character-by-character handling.

**Pros:**
- No new dependency
- Full control

**Cons:**
- Significant effort to implement cursor navigation, word movement, undo/redo
- Reinventing the wheel
- More code to maintain

**Verdict:** Not recommended. tui-textarea provides all needed features out of the box.

## Implementation Checklist

- [ ] Check tui-textarea PR #119 status / 0.8.0 release
- [ ] Add tui-textarea dependency (with appropriate version/git ref)
- [ ] Add feature flags: `crossterm` (default), optionally `search`
- [ ] Replace `input_buffer: String` with `textarea: Option<TextArea<'static>>` in PlanTuiState
- [ ] Update `enter_question_mode()` to create TextArea
- [ ] Update `handle_input_key()` to delegate to TextArea
- [ ] Update `get_answers()` to extract from TextArea
- [ ] Update `render_question_input()` to render TextArea widget
- [ ] Apply same pattern to main TUI's App struct
- [ ] Add unit tests for TextArea input handling
- [ ] Add snapshot tests for TextArea rendering
- [ ] Update keybinding documentation in footer

## Cargo.toml Changes

```toml
[dependencies]
# Add when 0.8.0 releases with ratatui 0.30 support:
tui-textarea = "0.8"

# Or use git if PR #119 merged but not released:
# tui-textarea = { git = "https://github.com/rhysd/tui-textarea" }

# Current dependencies (unchanged)
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
```

## Sources

- [tui-textarea GitHub](https://github.com/rhysd/tui-textarea) - PRIMARY
- [tui-textarea docs.rs](https://docs.rs/tui-textarea/0.6.1/tui_textarea/) - API reference
- [tui-textarea PR #119](https://github.com/rhysd/tui-textarea/pulls) - ratatui 0.30 support (draft)
- [edtui GitHub](https://github.com/preiter93/edtui) - Alternative considered
- [ratatui-testlib docs](https://docs.rs/ratatui-testlib/latest/ratatui_testlib/) - Testing utility
- [lib.rs tui-textarea](https://lib.rs/crates/tui-textarea) - Crate info

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| tui-textarea features | HIGH | Official docs, verified API |
| ratatui 0.30 compatibility | MEDIUM | Draft PR exists, not released |
| Integration approach | HIGH | Based on existing rslph code patterns |
| Testing approach | HIGH | Extends existing TestBackend + insta pattern |
| Alternative assessment | MEDIUM | Based on README review, not hands-on testing |

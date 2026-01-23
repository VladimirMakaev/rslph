# Phase 14: TUI Visual Parity with Claude Code - Research

**Researched:** 2026-01-23
**Domain:** Terminal UI styling, ratatui custom theming, Claude Code visual design
**Confidence:** HIGH

## Summary

This research investigates how to achieve visual parity between rslph's TUI and Claude Code's terminal interface. The existing TUI (ratatui 0.30) provides a solid foundation with TEA pattern, widget architecture, and proper state management. The visual enhancement requires: custom RGB colors matching Claude's brand palette, box-drawn containers for thinking blocks and tool calls, animated braille spinners during LLM streaming, and an enhanced status bar with model tier indicators.

The standard approach is to add `throbber-widgets-tui` for animated spinners (compatible with ratatui 0.30), create a centralized `theme.rs` module with color constants, and implement box-drawing using ratatui's built-in border sets with custom title positioning.

**Primary recommendation:** Add throbber-widgets-tui v0.10.0 for spinners, create src/tui/theme.rs for centralized colors, and extend existing widget files to use box-drawn containers with Claude brand colors.

## Standard Stack

The established libraries/tools for this domain:

### Core (Already Present)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30 | TUI framework | Immediate-mode rendering, widget system, styling |
| crossterm | 0.29 | Terminal backend | Event handling, cross-platform support |

### Supporting (New Addition)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| throbber-widgets-tui | 0.10.0 | Animated spinners | LLM streaming states, loading indicators |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| throbber-widgets-tui | Manual spinner implementation | Manual requires frame timing logic; throbber handles tick/state |
| Custom color module | Inline colors | Inline is scattered and hard to maintain |

**Installation:**
```bash
cargo add throbber-widgets-tui@0.10.0
```

## Architecture Patterns

### Recommended Project Structure
```
src/tui/
├── theme.rs          # NEW: Centralized color palette and styles
├── mod.rs            # Add theme export
├── widgets/
│   ├── thread_view.rs    # MODIFY: Use themed colors, box containers
│   ├── status_bar.rs     # MODIFY: Model tier indicator, session timer
│   ├── spinner.rs        # NEW: Throbber wrapper with braille patterns
│   └── ...
├── conversation.rs   # MODIFY: Box-drawn thinking/tool containers
└── ...
```

### Pattern 1: Centralized Theme Module
**What:** Single source of truth for all colors and styles
**When to use:** Any component needing consistent styling
**Example:**
```rust
// src/tui/theme.rs
use ratatui::style::{Color, Modifier, Style};

/// Claude brand colors from official palette
pub mod colors {
    use ratatui::style::Color;

    /// Crail - Claude's signature orange/terracotta
    pub const CRAIL: Color = Color::Rgb(193, 95, 60);    // #C15F3C

    /// Cloudy - Warm gray accent
    pub const CLOUDY: Color = Color::Rgb(177, 173, 161); // #B1ADA1

    /// Pampas - Light cream background
    pub const PAMPAS: Color = Color::Rgb(244, 243, 238); // #F4F3EE

    /// White - Clean white
    pub const WHITE: Color = Color::Rgb(255, 255, 255);  // #FFFFFF

    // Semantic colors
    pub const THINKING: Color = Color::DarkGray;
    pub const TOOL_CALL: Color = Color::Yellow;
    pub const TOOL_RESULT: Color = Color::Cyan;
    pub const ASSISTANT: Color = CRAIL;  // Use brand color
    pub const USER: Color = Color::Cyan;
    pub const SYSTEM: Color = CLOUDY;
}

/// Model tier symbols
pub mod symbols {
    /// Diamond for highest tier (Opus)
    pub const TIER_HIGH: &str = "◆";
    /// Half-diamond for mid tier (Sonnet)
    pub const TIER_MID: &str = "◇";
    /// Circle for base tier (Haiku)
    pub const TIER_LOW: &str = "○";
}

/// Predefined styles for common use cases
pub mod styles {
    use super::colors;
    use ratatui::style::{Modifier, Style};

    pub fn assistant() -> Style {
        Style::default().fg(colors::ASSISTANT)
    }

    pub fn thinking() -> Style {
        Style::default()
            .fg(colors::THINKING)
            .add_modifier(Modifier::ITALIC)
    }

    pub fn tool_header() -> Style {
        Style::default()
            .fg(colors::TOOL_CALL)
            .add_modifier(Modifier::BOLD)
    }
}
```

### Pattern 2: Box-Drawn Containers with Titles
**What:** Unicode box drawing for thinking blocks and tool calls
**When to use:** Grouping related content visually
**Example:**
```rust
use ratatui::widgets::{Block, Borders, BorderType};
use ratatui::symbols::border;

// Rounded box with title
let thinking_block = Block::default()
    .title("─ Thinking ")
    .title_style(Style::default().fg(Color::DarkGray))
    .borders(Borders::ALL)
    .border_type(BorderType::Rounded)
    .border_style(Style::default().fg(Color::DarkGray));

// Tool call box with tool name header
let tool_block = Block::default()
    .title(format!(" {} ", tool_name))
    .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    .borders(Borders::ALL)
    .border_type(BorderType::Plain)
    .border_style(Style::default().fg(Color::Yellow));
```

### Pattern 3: Animated Spinner with Throbber
**What:** Braille spinner during LLM streaming
**When to use:** Indicating active processing without progress percentage
**Example:**
```rust
use throbber_widgets_tui::{Throbber, ThrobberState, BRAILLE_SIX};

// In app state
pub struct App {
    spinner_state: ThrobberState,
    is_streaming: bool,
    // ...
}

// Tick on each frame (in event loop)
if app.is_streaming {
    app.spinner_state.calc_next();
}

// Render spinner
let spinner = Throbber::default()
    .throbber_set(BRAILLE_SIX)
    .label("Processing...")
    .style(Style::default().fg(colors::CRAIL));

frame.render_stateful_widget(spinner, spinner_area, &mut app.spinner_state);
```

### Anti-Patterns to Avoid
- **Hardcoded colors in widgets:** Use theme.rs constants instead
- **Building custom spinner logic:** Use throbber-widgets-tui
- **Inline style definitions:** Extract to theme.rs for reuse
- **RGB string parsing at render time:** Pre-define Color::Rgb constants

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Animated spinner | Manual tick counting + character cycling | throbber-widgets-tui | Frame timing, 25+ symbol sets, state management |
| Box drawing | Manual Unicode character concatenation | ratatui Block with BorderType | Handles corners, edges, titles correctly |
| Color constants | Scattered Color::Rgb() calls | Centralized theme.rs | Maintainability, consistent branding |
| Model tier detection | String matching on model names | Pattern match with fallback | Model names may include versions |

**Key insight:** ratatui already provides BorderType enum with PLAIN, ROUNDED, DOUBLE, THICK variants - no custom box drawing needed.

## Common Pitfalls

### Pitfall 1: RGB Colors on Limited Terminals
**What goes wrong:** Color::Rgb doesn't render correctly on terminals with limited color support
**Why it happens:** Not all terminals support true color (16M colors)
**How to avoid:** Use crossterm's feature detection or provide fallback colors
**Warning signs:** Colors appear wrong or missing on certain terminals

```rust
// Fallback pattern
fn themed_color(rgb: Color, fallback: Color) -> Color {
    // In practice, most modern terminals support RGB
    // Could add detection if needed for legacy support
    rgb
}
```

### Pitfall 2: Spinner Not Animating
**What goes wrong:** Spinner displays static character
**Why it happens:** Missing `calc_next()` call in event loop tick
**How to avoid:** Ensure tick handler advances spinner state on every frame
**Warning signs:** Static braille character instead of animation

### Pitfall 3: Theme Import Across Modules
**What goes wrong:** Inconsistent color usage after adding theme module
**Why it happens:** Some widgets still use inline Color definitions
**How to avoid:** Search-and-replace all Color::Rgb/Color::* to use theme constants
**Warning signs:** Mix of brand colors and default colors in UI

### Pitfall 4: Box Title Truncation
**What goes wrong:** Long titles overflow box boundaries
**Why it happens:** ratatui title doesn't auto-truncate
**How to avoid:** Truncate title strings before passing to Block::title()
**Warning signs:** Titles overlapping content or extending past borders

## Code Examples

Verified patterns from official sources:

### Custom RGB Color Definition
```rust
// Source: ratatui Color enum documentation
use ratatui::style::Color;

// Method 1: Direct RGB
const CRAIL: Color = Color::Rgb(193, 95, 60);

// Method 2: Hex string (runtime)
let color: Color = "#C15F3C".parse().unwrap();
```

### BorderType Variants
```rust
// Source: ratatui::widgets::BorderType
use ratatui::widgets::BorderType;

BorderType::Plain    // ┌──────┐
BorderType::Rounded  // ╭──────╮
BorderType::Double   // ╔══════╗
BorderType::Thick    // ┏━━━━━━┓
```

### Throbber Braille Sets
```rust
// Source: throbber-widgets-tui v0.10.0
use throbber_widgets_tui::{BRAILLE_ONE, BRAILLE_SIX, BRAILLE_EIGHT};

// Available braille sets:
// BRAILLE_ONE through BRAILLE_EIGHT - different animation patterns
// Also: ASCII, ARROW, BLOCK, CIRCLE, DOUBLE_CIRCLE, and more
```

### Status Bar Model Tier Indicator
```rust
// Source: Custom pattern based on requirements
fn model_tier_indicator(model_name: &str) -> &'static str {
    if model_name.contains("opus") {
        "◆"  // Filled diamond - highest
    } else if model_name.contains("sonnet") {
        "◇"  // Empty diamond - mid
    } else {
        "○"  // Circle - base (haiku or unknown)
    }
}
```

### Collapsible Thinking Block
```rust
// Source: Custom pattern for TUI-02
use ratatui::widgets::{Block, Borders, BorderType, Paragraph};

fn render_thinking_block(
    frame: &mut Frame,
    area: Rect,
    content: &str,
    collapsed: bool,
) {
    let display_content = if collapsed {
        format!("▶ {} chars", content.len())  // Collapsed view
    } else {
        content.to_string()  // Full content
    };

    let block = Block::default()
        .title("─ Thinking ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(display_content)
        .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))
        .block(block);

    frame.render_widget(paragraph, area);
}
```

### Tool Call Container
```rust
// Source: Custom pattern for TUI-03
fn render_tool_call(
    frame: &mut Frame,
    area: Rect,
    tool_name: &str,
    params: &str,
) {
    let block = Block::default()
        .title(format!(" {} ", tool_name))
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Indented parameters
    let param_text = Paragraph::new(params)
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(param_text, inner);
}
```

### Enhanced Status Bar with Timer
```rust
// Source: Custom pattern for TUI-05
use std::time::{Duration, Instant};

fn format_session_time(start: Instant) -> String {
    let elapsed = start.elapsed();
    let secs = elapsed.as_secs();
    let mins = secs / 60;
    let hours = mins / 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins % 60, secs % 60)
    } else {
        format!("{:02}:{:02}", mins, secs % 60)
    }
}

fn render_enhanced_status(frame: &mut Frame, area: Rect, app: &App) {
    let tier = model_tier_indicator(&app.model_name);
    let session_time = format_session_time(app.session_start);

    let status = format!(
        "{} {} | {} | Iter {}/{} | ...",
        tier,
        app.model_name,
        session_time,
        app.current_iteration,
        app.max_iterations,
    );

    // ... render with token bar
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Generic terminal colors | Brand-specific RGB palette | Phase 14 | Consistent Claude identity |
| Text-only thinking blocks | Box-drawn collapsible containers | Phase 14 | Visual hierarchy |
| No streaming indicator | Animated braille spinner | Phase 14 | User feedback during waits |
| Basic status bar | Model tier + timer + token bar | Phase 14 | Rich status information |

**Deprecated/outdated:**
- Default ratatui colors for roles: Replace with theme constants
- Inline Style::default().fg() calls: Use theme::styles module

## Open Questions

Things that couldn't be fully resolved:

1. **Exact Claude Code color values**
   - What we know: Crail (#C15F3C) and Cloudy (#B1ADA1) from brand guidelines
   - What's unclear: Exact shades used in Claude Code TUI (may differ slightly)
   - Recommendation: Use documented brand colors, adjust if visual feedback suggests

2. **Terminal true color support**
   - What we know: crossterm supports RGB, most modern terminals do too
   - What's unclear: Fallback strategy for limited terminals (tmux, older SSH)
   - Recommendation: Use RGB colors as primary, document terminal requirements

3. **Spinner frame rate**
   - What we know: throbber uses calc_next() per tick
   - What's unclear: Optimal tick rate for smooth animation vs CPU usage
   - Recommendation: Use default tick rate (typically ~100ms), adjust if needed

## Sources

### Primary (HIGH confidence)
- ratatui 0.30 documentation via Context7 - Color, Style, Block, BorderType
- throbber-widgets-tui v0.10.0 - Spinner widget patterns
- Claude brand guidelines - Crail #C15F3C, Cloudy #B1ADA1

### Secondary (MEDIUM confidence)
- Existing codebase analysis - src/tui/ module patterns
- Phase 6 TUI research - Architecture patterns

### Tertiary (LOW confidence)
- WebSearch for Claude Code appearance - Visual reference only

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - ratatui 0.30 is current, throbber-widgets-tui confirmed compatible
- Architecture: HIGH - Patterns derived from existing codebase + official docs
- Pitfalls: MEDIUM - Based on common terminal UI issues

**Research date:** 2026-01-23
**Valid until:** 60 days (ratatui and throbber are stable libraries)

## Implementation Notes

### Files to Modify

| File | Changes |
|------|---------|
| `Cargo.toml` | Add throbber-widgets-tui = "0.10.0" |
| `src/tui/mod.rs` | Export theme module |
| `src/tui/theme.rs` | NEW: Color constants, style functions |
| `src/tui/widgets/thread_view.rs` | Use theme colors, may add box containers |
| `src/tui/widgets/status_bar.rs` | Add model tier, session timer |
| `src/tui/conversation.rs` | Box-drawn thinking/tool containers |
| `src/tui/app.rs` | Add spinner_state, session_start fields |

### Current Code Patterns to Preserve

The existing TUI follows consistent patterns that should be maintained:

1. **TEA architecture:** App state, event handling, render functions
2. **Widget separation:** Each widget in widgets/ module
3. **Collapsible groups:** MessageGroup with expanded flag
4. **Ring buffer:** ConversationBuffer with 1000 item limit
5. **Layout composition:** Layout::vertical/horizontal for area splitting

### Migration Path

1. Create theme.rs with color constants
2. Update existing widgets to use theme imports
3. Add throbber dependency and spinner widget
4. Enhance status bar (incremental)
5. Add box containers to conversation.rs
6. Add message type-specific borders to thread_view.rs

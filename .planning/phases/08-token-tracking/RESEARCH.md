# Phase 8: Token Tracking - Research

**Researched:** 2026-01-20
**Domain:** Token parsing, TUI status bar updates, human-readable number formatting
**Confidence:** HIGH

## Summary

This research investigates the implementation approach for real-time token tracking during plan/build execution. The existing codebase already has 90% of the token parsing infrastructure in place - the `Usage` struct in `stream_json.rs` captures all four token fields (`input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`) and `StreamResponse` already accumulates usage from events.

The main implementation work involves:
1. Adding token accumulation state to `BuildContext` (not `BuildState` which is an enum for state machine states)
2. Creating a new `SubprocessEvent` variant for token updates
3. Adding token fields to `App` state for TUI display
4. Implementing human-readable number formatting (use `human_format` crate)
5. Extending `ScenarioBuilder` to configure token usage in fake Claude responses

**Primary recommendation:** Follow the existing event streaming pattern - add `SubprocessEvent::TokenUsage` variant, route through existing channel to App, render in status bar. Use `human_format` crate for number abbreviation.

## Standard Stack

The established libraries/tools for this domain:

### Core (Already in Project)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde | 1.0 | JSON serialization of Usage struct | Already used for stream-json parsing |
| tokio | 1.49 | Async streaming and channel communication | Project foundation |
| ratatui | 0.30 | TUI framework for status bar display | Already used for TUI |

### Supporting (New Addition)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| human_format | 1.2 | Number abbreviation (5.2k, 1.2M) | Token count display |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| human_format | Hand-rolled formatting | human_format is well-maintained (383k downloads/month), handles edge cases, supports custom decimals |
| human_format | number_prefix | number_prefix is less flexible, human_format has better API |

**Installation:**
```bash
cargo add human_format@1.2
```

## Architecture Patterns

### Recommended Data Flow
```
┌─────────────────────────────────────────────────────────────────────┐
│                         Build Loop (iteration.rs)                   │
│                                                                     │
│  StreamEvent → parse_and_stream_line() → extract Usage             │
│                                         │                           │
│                          ┌──────────────┴──────────────┐            │
│                          ▼                              ▼           │
│            SubprocessEvent::TokenUsage           Update totals in   │
│            (send to TUI channel)                 BuildContext       │
└─────────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         TUI App (app.rs)                            │
│                                                                     │
│  AppEvent::TokenUsage → Update App.total_tokens                     │
│                        (running cumulative total)                   │
│                                                                     │
│  Render status bar with formatted token counts                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Pattern 1: Token State Structs
**What:** Define reusable token tracking structs
**When to use:** Both in BuildContext (persistence) and App (display)
**Example:**
```rust
// Source: CONTEXT.md design decision
/// Token usage for a single iteration
#[derive(Debug, Clone, Default)]
pub struct IterationTokens {
    pub iteration: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

/// Cumulative token usage across all iterations
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

impl TokenUsage {
    /// Add usage from a stream event
    pub fn add_from_usage(&mut self, usage: &crate::subprocess::Usage) {
        self.input_tokens += usage.input_tokens;
        self.output_tokens += usage.output_tokens;
        self.cache_creation_input_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
        self.cache_read_input_tokens += usage.cache_read_input_tokens.unwrap_or(0);
    }
}
```

### Pattern 2: Event Routing for Token Updates
**What:** Extend existing SubprocessEvent enum for token updates
**When to use:** When streaming token updates from iteration to TUI
**Example:**
```rust
// Source: Existing pattern in src/tui/event.rs
pub enum SubprocessEvent {
    // ... existing variants ...

    /// Token usage update from stream event
    TokenUsage {
        input_tokens: u64,
        output_tokens: u64,
        cache_creation_input_tokens: u64,
        cache_read_input_tokens: u64,
    },
}
```

### Pattern 3: Human-Readable Number Formatting
**What:** Format large numbers with SI suffixes
**When to use:** Displaying token counts in status bar
**Example:**
```rust
// Source: https://docs.rs/human_format
use human_format::Formatter;

/// Format token count for display (e.g., 5.2k, 1.2M)
pub fn format_tokens(count: u64) -> String {
    if count == 0 {
        return "0".to_string();
    }

    Formatter::new()
        .with_decimals(1)
        .with_separator("")  // No space: "5.2k" not "5.2 k"
        .format(count as f64)
}

// Examples:
// 500 -> "500"
// 5200 -> "5.2k"
// 1234567 -> "1.2M"
```

### Anti-Patterns to Avoid
- **Storing tokens in BuildState enum:** `BuildState` is the state machine enum (Starting, Running, Done). Store tokens in `BuildContext` which holds mutable build data.
- **Parsing usage on every line:** Only assistant events have usage data. Check `event.is_assistant()` first.
- **Blocking on token updates:** Token display is informational. Never block build loop waiting for TUI acknowledgment.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Number abbreviation | Custom divide-by-1000 logic | `human_format` crate | Handles edge cases (0, negative, very large), proper rounding, standard SI suffixes |
| Token struct serialization | Manual JSON building | serde derives | Already in project, consistent with existing patterns |
| Async channel for TUI | Custom shared state | `mpsc::UnboundedSender` | Already used throughout project, non-blocking |

**Key insight:** The existing codebase has established patterns for event streaming (SubprocessEvent), state management (App), and rendering (ratatui widgets). Follow these patterns rather than inventing new approaches.

## Common Pitfalls

### Pitfall 1: Token Double-Counting
**What goes wrong:** Same usage event counted multiple times when multiple text blocks exist
**Why it happens:** Claude can send multiple assistant events in a single response (thinking + text + tool_use)
**How to avoid:** Usage is cumulative per message - only use the FINAL usage from an assistant message, not intermediate
**Warning signs:** Token counts much higher than expected, growing faster than API billing

**Pattern:**
```rust
// WRONG: Accumulates every event
if let Some(usage) = event.usage() {
    totals.add_from_usage(usage);
}

// RIGHT: Track per-message, update on message completion
// The existing StreamResponse.process_event already handles this correctly -
// it overwrites rather than accumulates, keeping only the final usage
```

### Pitfall 2: Cache Token Misattribution
**What goes wrong:** Cache costs calculated incorrectly, not matching Anthropic billing
**Why it happens:** Confusing `input_tokens`, `cache_creation_input_tokens`, and `cache_read_input_tokens`
**How to avoid:** Display all four fields separately, don't combine cache tokens with input tokens
**Warning signs:** Displayed totals don't match API billing

**Semantics (from Anthropic docs):**
- `input_tokens`: Regular prompt tokens (NOT including cached tokens)
- `cache_creation_input_tokens`: Tokens written to cache (25% premium pricing)
- `cache_read_input_tokens`: Tokens read from cache (90% discount pricing)
- `output_tokens`: Response tokens generated

### Pitfall 3: TUI Not Updating in Real-Time
**What goes wrong:** Token counts only update at end of iteration, not during streaming
**Why it happens:** Sending events only after subprocess completes
**How to avoid:** Send TokenUsage event immediately when parsing usage from stream
**Warning signs:** Status bar shows stale values during long Claude responses

### Pitfall 4: Missing Usage in Fake Claude
**What goes wrong:** E2E tests fail or show 0 tokens when testing token display
**Why it happens:** Fake Claude responses don't include configured token values
**How to avoid:** Extend ScenarioBuilder with `.with_token_usage()` method
**Warning signs:** TUI snapshot tests show "In: 0 | Out: 0"

## Code Examples

Verified patterns from official sources:

### Status Bar Token Display
```rust
// Source: Existing pattern from src/tui/widgets/status_bar.rs
fn render_status_line(frame: &mut Frame, area: Rect, app: &App) {
    // Existing: "Iter 1/10 | Task 2/5 | [context bar]"
    // New format: "Iter 1/10 | Task 2/5 | In: 5.2k | Out: 10.9k | [context bar]"

    let status_text = format!(
        "Iter {}/{} | Task {}/{} | In: {} | Out: {} | CacheW: {} | CacheR: {} | ",
        app.current_iteration, app.max_iterations,
        app.current_task, app.total_tasks,
        format_tokens(app.total_tokens.input_tokens),
        format_tokens(app.total_tokens.output_tokens),
        format_tokens(app.total_tokens.cache_creation_input_tokens),
        format_tokens(app.total_tokens.cache_read_input_tokens),
    );

    // ... render text and context bar ...
}
```

### Extending parse_and_stream_line for Tokens
```rust
// Source: Existing pattern from src/build/iteration.rs
fn parse_and_stream_line(
    line: &str,
    tui_tx: &mpsc::UnboundedSender<SubprocessEvent>,
) -> Option<StreamEvent> {
    let event = StreamEvent::parse(line).ok()?;

    // ... existing text and tool_use handling ...

    // Send token usage if available (existing code sends context ratio)
    if event.is_assistant() {
        if let Some(usage) = event.usage() {
            // Send full token usage for status bar
            let _ = tui_tx.send(SubprocessEvent::TokenUsage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cache_creation_input_tokens: usage.cache_creation_input_tokens.unwrap_or(0),
                cache_read_input_tokens: usage.cache_read_input_tokens.unwrap_or(0),
            });

            // Existing context usage calculation
            let ratio = (usage.input_tokens + usage.output_tokens) as f64 / 200_000.0;
            let _ = tui_tx.send(SubprocessEvent::Usage(ratio.min(1.0)));
        }
    }

    Some(event)
}
```

### ScenarioBuilder Token Extension
```rust
// Source: Pattern from tests/fake_claude_lib/scenario.rs
impl ScenarioBuilder {
    /// Set token usage for the current invocation's responses.
    ///
    /// All assistant events in this invocation will use these token values.
    pub fn with_token_usage(
        mut self,
        input_tokens: u64,
        output_tokens: u64,
        cache_creation: u64,
        cache_read: u64,
    ) -> Self {
        self.current_invocation.token_config = Some(TokenConfig {
            input_tokens,
            output_tokens,
            cache_creation_input_tokens: cache_creation,
            cache_read_input_tokens: cache_read,
        });
        self
    }
}

// Test usage:
let handle = ScenarioBuilder::new()
    .respond_with_text("Working on task...")
    .with_token_usage(5000, 1500, 2000, 1000)
    .build();
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual token counting | API provides usage in response | Always available | No need to tokenize locally |
| Separate cache/input tracking | Unified Usage struct with all fields | Prompt caching (Aug 2024) | Must track 4 fields separately |

**Deprecated/outdated:**
- None identified - the current Usage struct format is current with Anthropic API

## Open Questions

Things that couldn't be fully resolved:

1. **Stream-json event timing**
   - What we know: Usage appears in assistant events, final message has cumulative usage
   - What's unclear: Does Claude CLI emit partial usage during streaming, or only at end?
   - Recommendation: Use final usage value per assistant message (existing StreamResponse pattern)

2. **Cache token appearance frequency**
   - What we know: Cache tokens appear when prompt caching is active
   - What's unclear: Does Claude CLI always pass these through, or does it depend on model/flags?
   - Recommendation: Handle as Option<u64>, display 0 if absent

3. **Status bar width constraints**
   - What we know: Current status bar shows "Iter X/Y | Task X/Y | [bar]"
   - What's unclear: Will adding 4 token fields fit in narrow terminals?
   - Recommendation: Start with full display, consider abbreviation (I/O/CW/CR) if too wide

## Sources

### Primary (HIGH confidence)
- **Codebase analysis:** `src/subprocess/stream_json.rs` - Usage struct verified with all 4 token fields
- **Codebase analysis:** `src/tui/event.rs` - SubprocessEvent pattern for extending
- **Codebase analysis:** `src/build/iteration.rs` - parse_and_stream_line pattern
- **Codebase analysis:** `tests/fake_claude_lib/scenario.rs` - ScenarioBuilder extension pattern
- [human_format crate](https://lib.rs/crates/human_format) - Number formatting API

### Secondary (MEDIUM confidence)
- [Anthropic Prompt Caching Blog](https://claude.com/blog/prompt-caching) - Cache token semantics
- [tokscale GitHub](https://github.com/junhoyeo/tokscale) - Claude CLI JSONL format reference

### Tertiary (LOW confidence)
- Claude CLI stream-json format - Inferred from existing code and tokscale, no official docs found

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All dependencies verified, human_format has strong adoption
- Architecture: HIGH - Follows existing codebase patterns exactly
- Token semantics: MEDIUM - Based on Anthropic blog post, may need validation
- TUI updates: HIGH - Follows existing SubprocessEvent -> App -> render pattern

**Research date:** 2026-01-20
**Valid until:** 2026-02-20 (30 days - stable domain, dependencies well-established)

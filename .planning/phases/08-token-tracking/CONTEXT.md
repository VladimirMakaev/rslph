# Phase 8: Token Tracking — Context

**Phase goal:** Users can observe token consumption during plan/build execution
**Created:** 2026-01-20
**Status:** Decisions locked

---

## Decisions

### 1. Display Format

| Decision | Choice |
|----------|--------|
| When to display | Real-time calculation with running total |
| TUI placement | Both status bar AND iteration summary |
| Format style | Abbreviated: "In: 5.2k \| Out: 10.9k \| Cache: 2.1k" |
| Non-TUI mode | Print per-iteration to stdout |
| Persistence | Per-iteration breakdown stored in BuildState |

**Implementation notes:**
- Status bar shows running cumulative total
- Iteration summary shows that iteration's token usage
- Both update in real-time as stream events arrive

### 2. Token Breakdown

| Decision | Choice |
|----------|--------|
| Fields tracked | All 4: input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens |
| Display grouping | All 4 fields shown separately (no logical grouping) |
| Number formatting | Auto-scale: 5.2k for thousands, 1.2M for millions |

**Display labels:**
- `In:` — input_tokens
- `Out:` — output_tokens
- `CacheW:` — cache_creation_input_tokens
- `CacheR:` — cache_read_input_tokens

### 3. Storage Approach

| Decision | Choice |
|----------|--------|
| Storage location | BuildState struct (add token fields) |
| Progress.md | Keep separate — do NOT add tokens to progress file |
| Granularity | Both totals AND per-iteration breakdown |

**BuildState additions:**
```rust
pub struct BuildState {
    // ... existing fields ...
    pub iteration_tokens: Vec<IterationTokens>,  // Per-iteration
    pub total_tokens: TokenUsage,                 // Running totals
}

pub struct IterationTokens {
    pub iteration: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}
```

### 4. Plan vs Build

| Decision | Choice |
|----------|--------|
| Plan command | Yes, track tokens |
| Relationship | Separate counts per command, combined in eval only |
| Plan display | Show at end of plan command (not real-time) |

**Implementation notes:**
- `rslph plan` returns final token count on completion
- `rslph build` tracks per-iteration and cumulative
- `rslph eval` (Phase 9+) combines plan + build totals

### 5. Testing Requirements

| Decision | Choice |
|----------|--------|
| E2E verification | Fake Claude returns realistic token counts in Usage events |
| TUI snapshots | Both status bar AND iteration summary with token display |
| Test data | Per-scenario configurable token values |

**Fake Claude enhancement:**
- ScenarioBuilder gains `.with_token_usage(input, output, cache_create, cache_read)` method
- Each response in fake Claude includes Usage event with configured tokens
- Default values if not configured (e.g., 1000 input, 500 output)

**TUI snapshot tests:**
- Snapshot of status bar showing token totals
- Snapshot of iteration summary showing per-iteration tokens
- Use fixed token values for deterministic snapshots

---

## Scope Boundaries

**In scope (Phase 8):**
- Token parsing from stream-json (already exists in `Usage` struct)
- Token accumulation per iteration
- Token display in TUI (status bar + iteration summary)
- Token display in non-TUI mode
- Token persistence in BuildState
- Plan command token tracking
- E2E and TUI tests for token display

**Out of scope (later phases):**
- Cost estimation from tokens → Future
- Token display as separate TUI widget → Future
- Token limits/warnings → Future
- Eval command token aggregation → Phase 9

---

## Technical Context

**Existing infrastructure:**
- `src/subprocess/stream_json.rs` has `Usage` struct with all 4 token fields
- `StreamEvent::Usage(Usage)` variant exists
- `StreamResponse` already captures usage on final event
- `BuildState` exists in `src/build/state.rs`

**Key files to modify:**
- `src/build/state.rs` — Add token fields to BuildState
- `src/build/iteration.rs` — Accumulate tokens from StreamEvent::Usage
- `src/tui/status_bar.rs` — Display running total
- `src/tui/threads.rs` or equivalent — Display per-iteration tokens
- `src/plan/command.rs` — Track and report plan tokens
- `tests/e2e/` — Add token verification tests
- Fake Claude binary — Return Usage events with configurable tokens

---

## Deferred Ideas

*None captured during discussion*

---

*Downstream agents: Use these decisions directly. Do not re-ask the user about display format, token fields, storage, or testing approach.*

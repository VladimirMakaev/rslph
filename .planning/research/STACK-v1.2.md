# Stack Research: v1.2 Context Engineering Milestone

**Project:** rslph v1.2 "Context Engineering"
**Researched:** 2026-01-20
**Focus:** Token tracking, eval framework infrastructure, test-driven iteration flows

---

## Executive Summary

The v1.2 milestone requires minimal stack additions. The existing codebase already has:
- `serde_json` for JSON parsing (stream-json already implemented in `src/subprocess/stream_json.rs`)
- `std::time::Instant/Duration` for timing (already used in build/command.rs)
- `tempfile` crate as dev-dependency (already used in fixtures.rs and tests)
- `chrono` for timestamps (already in dependencies)

**Key finding:** The token tracking infrastructure is already 90% complete. The `Usage` struct in `stream_json.rs` captures `input_tokens`, `output_tokens`, `cache_creation_input_tokens`, and `cache_read_input_tokens`. The `StreamResponse` type already accumulates these values.

**Recommendation:** Promote `tempfile` from dev-dependency to regular dependency for eval command isolation. Everything else exists.

---

## Stack Analysis by Feature Area

### 1. Token Consumption Tracking

**Status:** Already implemented, needs activation

**Existing infrastructure:**
- `src/subprocess/stream_json.rs` - `Usage` struct with all token fields
- `StreamResponse::process_event()` - Already extracts usage from stream events
- `StreamEvent::usage()` - Getter method for usage data

**Token fields available in Claude CLI stream-json:**
```rust
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}
```

**What's missing:**
1. Aggregation across iterations (trivial: sum up `StreamResponse` usage after each iteration)
2. Display in TUI status bar (integration task, not stack)
3. Persistence to results file (use existing `serde_json` for JSON output)

**Stack additions needed:** None

**Confidence:** HIGH - Verified by reading existing `src/subprocess/stream_json.rs` and `tests/fake_claude_lib/stream_json.rs` which mirror the exact Claude CLI format.

---

### 2. Timing/Benchmarking

**Status:** Already available, needs structured collection

**Existing infrastructure:**
- `std::time::Instant` - Already used in `build/state.rs` for `iteration_start`
- `std::time::Duration` - Already used throughout for timeouts
- `chrono` - Already in Cargo.toml for timestamps

**Usage pattern (already in codebase):**
```rust
// From src/build/command.rs
ctx.iteration_start = Some(std::time::Instant::now());

// To measure:
let elapsed = ctx.iteration_start.unwrap().elapsed();
```

**For eval framework, need:**
1. Total plan duration
2. Per-iteration timing
3. First-passing-test timing (for TDD metrics)

**Stack additions needed:** None

**Recommendation:** Create a `Metrics` struct to aggregate timing:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalMetrics {
    pub total_duration_secs: f64,
    pub iterations: u32,
    pub input_tokens_total: u64,
    pub output_tokens_total: u64,
    pub first_test_pass_iteration: Option<u32>,
    pub tests_passed: u32,
    pub tests_total: u32,
}
```

This is pure Rust with existing `serde` - no new dependencies.

**Confidence:** HIGH - `std::time` is stable Rust stdlib, `chrono` already in deps.

---

### 3. Temp Directory Management (Eval Isolation)

**Status:** Available as dev-dependency, needs promotion

**Current state:**
```toml
[dev-dependencies]
tempfile = "3"
```

**Required change:**
```toml
[dependencies]
tempfile = "3"
```

**Why promote to regular dependency:**
- `rslph eval` command needs to run built-in projects in isolated temp directories
- Must clean up after eval run (TempDir RAII pattern)
- `keep()` method allows preserving workspace for debugging

**TempDir API (verified from docs.rs):**

| Method | Purpose | Use in rslph eval |
|--------|---------|-------------------|
| `TempDir::new()` | Create in system temp | Default isolation |
| `TempDir::new_in(dir)` | Create in specific parent | Custom isolation location |
| `TempDir::with_prefix(prefix)` | Named temp dirs | `rslph-eval-{project}` |
| `.path()` | Get Path reference | Pass to subprocess |
| `.keep()` | Persist on drop | `--keep-workspace` flag |
| `.close()` | Explicit cleanup with error | Capture cleanup failures |

**Usage pattern for eval:**
```rust
pub struct EvalWorkspace {
    temp_dir: TempDir,
    project_name: String,
}

impl EvalWorkspace {
    pub fn new(project_name: &str) -> std::io::Result<Self> {
        let temp_dir = TempDir::with_prefix(&format!("rslph-eval-{}-", project_name))?;
        Ok(Self { temp_dir, project_name: project_name.to_string() })
    }

    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    pub fn keep(self) -> PathBuf {
        self.temp_dir.keep()
    }
}
```

**Stack change required:** Promote `tempfile = "3"` to regular dependencies

**Confidence:** HIGH - Already using `tempfile` extensively in test code (`tests/e2e/fixtures.rs`), API is well-understood.

---

### 4. Results Storage Format

**Status:** Use JSON (already have `serde_json`)

**Existing infrastructure:**
- `serde_json = "1.0"` in dependencies
- `serde = { features = ["derive"] }` for serialization
- All relevant types already have `Serialize`/`Deserialize`

**Recommended results format:**
```json
{
  "project": "calculator",
  "timestamp": "2026-01-20T10:30:00Z",
  "success": true,
  "metrics": {
    "total_duration_secs": 145.5,
    "iterations": 3,
    "input_tokens_total": 45000,
    "output_tokens_total": 12000,
    "cache_read_tokens": 8000,
    "first_test_pass_iteration": 2,
    "tests_passed": 5,
    "tests_total": 5
  },
  "iterations": [
    {
      "number": 1,
      "duration_secs": 60.2,
      "input_tokens": 15000,
      "output_tokens": 4000,
      "tests_passed": 2,
      "tests_total": 5
    }
  ]
}
```

**Why JSON over TOML:**
1. Better for nested structures (iteration arrays)
2. Standard format for benchmarking results
3. Easy to aggregate multiple runs
4. Compatible with existing tooling (jq, dashboards)

**Stack additions needed:** None

**Confidence:** HIGH - `serde_json` already a dependency, JSON is standard for metrics.

---

### 5. Built-in Eval Projects Storage

**Status:** Use `include_str!()` macro

**Approach:** Embed project templates directly in the binary using Rust's `include_str!()` macro. No additional dependencies needed.

**Structure:**
```
src/
  eval/
    mod.rs
    projects/
      mod.rs
      calculator.rs      # Contains TEMPLATE constant
      todo_app.rs
```

**Example:**
```rust
// src/eval/projects/calculator.rs
pub const PROMPT: &str = include_str!("calculator/PROMPT.md");
pub const HIDDEN_TESTS: &str = include_str!("calculator/tests.py");
pub const EXPECTED_FILES: &[&str] = &["calculator.py"];
```

**Stack additions needed:** None

**Confidence:** HIGH - Standard Rust pattern for embedding resources.

---

## What NOT to Add

### Avoid: External benchmarking crates (criterion, hyperfine)

**Reason:** These are for micro-benchmarks and statistical analysis. rslph eval runs take minutes, not nanoseconds. Simple `Instant::elapsed()` is appropriate.

### Avoid: Database for results (sqlite, sled)

**Reason:** Premature optimization. JSON files suffice for:
- Single eval runs
- Comparing a few runs
- v1.x scope

If v2 needs historical analysis across hundreds of runs, reconsider then.

### Avoid: uuid crate

**Reason:** Already using `chrono` timestamps for uniqueness. `format!("eval-{}-{}", project, timestamp)` is sufficient for eval run identification.

### Avoid: Metrics/tracing crates (metrics, prometheus)

**Reason:** Eval framework doesn't need real-time metrics export. We capture metrics at run end and write to JSON. If future dashboards need this, add then.

---

## Final Cargo.toml Changes

**Before (current state):**
```toml
[dependencies]
# ... existing deps ...

[dev-dependencies]
tempfile = "3"
# ... other dev deps ...
```

**After (for v1.2):**
```toml
[dependencies]
# ... existing deps unchanged ...
tempfile = "3"  # PROMOTED from dev-dependencies

[dev-dependencies]
# tempfile removed, now in regular deps
# ... other dev deps unchanged ...
```

**Total changes:** Move one line. Everything else already exists.

---

## Integration Notes

### Token Tracking Integration

The existing `StreamResponse` in `stream_json.rs` accumulates tokens per Claude invocation. To track per-iteration:

1. After each iteration's `run_iteration()` completes, extract `stream_response.input_tokens` and `stream_response.output_tokens`
2. Sum into a running total in `BuildContext` or a new `EvalContext`
3. For TUI display, add to `StatusBar` widget state

### Timing Integration

Already have `iteration_start: Option<std::time::Instant>` in `BuildState`. For eval:

1. Record `Instant::now()` at eval start
2. Per iteration, record start/end times
3. At eval completion, calculate durations

### Temp Directory Integration

The existing `WorkspaceBuilder` in `tests/e2e/fixtures.rs` is an excellent template. For eval:

1. Create `EvalWorkspace` wrapping `TempDir`
2. Extract project files from `include_str!()` constants
3. Write to temp directory
4. Run rslph build in that directory
5. After build, run hidden tests
6. Cleanup happens automatically on drop (unless `--keep-workspace`)

---

## Confidence Assessment

| Component | Confidence | Rationale |
|-----------|------------|-----------|
| Token tracking | HIGH | Already implemented in stream_json.rs, just needs wiring |
| Timing | HIGH | Using std::time which is stable stdlib |
| Temp directories | HIGH | tempfile already used extensively in tests |
| Results format | HIGH | serde_json already a dependency |
| Project embedding | HIGH | Standard Rust pattern with include_str!() |

**Overall confidence:** HIGH - This milestone requires almost no new dependencies, just wiring existing capabilities together.

---

## Sources

### Official Documentation
- [tempfile crate docs](https://docs.rs/tempfile/latest/tempfile/) - TempDir API, keep() method
- [std::time::Instant](https://doc.rust-lang.org/std/time/struct.Instant.html) - Timing primitives
- [serde_json](https://docs.rs/serde_json/latest/serde_json/) - JSON serialization

### Existing Codebase (Primary Source)
- `/Users/vmakaev/NonWork/rslph/src/subprocess/stream_json.rs` - Token usage structs
- `/Users/vmakaev/NonWork/rslph/tests/e2e/fixtures.rs` - WorkspaceBuilder pattern
- `/Users/vmakaev/NonWork/rslph/tests/fake_claude_lib/stream_json.rs` - Stream event generation
- `/Users/vmakaev/NonWork/rslph/src/build/state.rs` - BuildState with timing

### Previous Research
- `/Users/vmakaev/NonWork/rslph/.planning/research/STREAM_JSON.md` - Claude CLI output format
- `/Users/vmakaev/NonWork/rslph/.planning/research/STACK.md` - Base stack decisions

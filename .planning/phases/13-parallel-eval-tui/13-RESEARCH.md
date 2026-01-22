# Phase 13: Parallel Eval TUI - Research

**Researched:** 2026-01-22
**Domain:** Parallel async execution, TUI dashboards, real-time streaming
**Confidence:** HIGH

## Summary

Phase 13 requires four major capabilities: (1) parallel eval execution across multiple prompt modes, (2) a TUI dashboard showing real-time progress of parallel runs, (3) enhanced conversation display showing full LLM output including thinking blocks and tool calls, and (4) TUI mode for the plan command.

The existing codebase provides strong foundations to build on. The TUI infrastructure in `src/tui/` uses ratatui with The Elm Architecture (TEA) pattern, tokio for async, and mpsc channels for subprocess-to-TUI communication. The eval command in `src/eval/command.rs` already runs sequential trials. The stream-json parser in `src/subprocess/stream_json.rs` extracts thinking blocks, tool uses, and text content from Claude's output.

**Primary recommendation:** Use `tokio::spawn` to parallelize trials, one channel per trial/mode combination, with a multi-pane dashboard layout that aggregates events from all concurrent runs.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.x | Async runtime | Already used throughout codebase |
| ratatui | 0.29.0 | TUI framework | Already used for build TUI |
| crossterm | 0.28.1 | Terminal backend | Already used with ratatui |
| tokio::sync::mpsc | 1.x | Async channels | Already used for subprocess events |
| tokio_util::sync::CancellationToken | 0.3.1 | Graceful shutdown | Already used in build/eval commands |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| futures | 0.3.x | Stream utilities | For `select!` and stream combinators |
| tokio::sync::broadcast | 1.x | Multi-consumer events | If multiple UI panels need same events |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| mpsc per trial | Single aggregated channel | Simpler but loses trial context |
| tokio::spawn per trial | tokio::JoinSet | JoinSet tracks all spawned tasks better |
| ratatui stateful widgets | Custom render logic | Stateful widgets cleaner for scrolling |

**Installation:**
```bash
# Already in Cargo.toml - no new dependencies needed
```

## Architecture Patterns

### Recommended Project Structure
```
src/
├── eval/
│   ├── command.rs        # Add run_parallel_eval_command
│   ├── parallel.rs       # NEW: Parallel execution orchestrator
│   └── mod.rs            # Export new types
├── tui/
│   ├── app.rs            # Extend App state for parallel/conversation
│   ├── widgets/
│   │   ├── dashboard.rs  # NEW: Multi-trial progress dashboard
│   │   ├── conversation.rs # NEW: Full LLM conversation view
│   │   └── ...
│   └── ...
└── prompts/
    └── modes.rs          # Already has PromptMode enum
```

### Pattern 1: Channel-per-Trial Architecture
**What:** Each parallel trial gets its own mpsc sender, events tagged with (mode, trial_num)
**When to use:** Parallel execution where you need to track source of each event
**Example:**
```rust
// Source: Existing pattern in src/tui/run.rs
pub struct TrialEvent {
    pub mode: PromptMode,
    pub trial_num: u32,
    pub event: SubprocessEvent,
}

// Orchestrator spawns trials and forwards tagged events
async fn spawn_trial(
    mode: PromptMode,
    trial_num: u32,
    aggregator_tx: mpsc::UnboundedSender<TrialEvent>,
) {
    let (trial_tx, mut trial_rx) = mpsc::unbounded_channel();

    // Spawn the trial with its own sender
    tokio::spawn(run_single_trial_with_events(mode, trial_num, trial_tx));

    // Forward events with trial context
    while let Some(event) = trial_rx.recv().await {
        let _ = aggregator_tx.send(TrialEvent { mode, trial_num, event });
    }
}
```

### Pattern 2: Dashboard State Machine
**What:** App state tracks multiple concurrent trials with aggregated progress
**When to use:** Multi-pane TUI showing parallel execution
**Example:**
```rust
// Source: Pattern from src/tui/app.rs App struct
pub struct ParallelEvalState {
    /// All trials being tracked
    pub trials: Vec<TrialProgress>,
    /// Which trial is currently focused (for detailed view)
    pub focused_trial: Option<(PromptMode, u32)>,
    /// Aggregated statistics per mode
    pub mode_stats: HashMap<PromptMode, ModeProgress>,
}

pub struct TrialProgress {
    pub mode: PromptMode,
    pub trial_num: u32,
    pub current_iteration: u32,
    pub max_iterations: u32,
    pub current_task: u32,
    pub total_tasks: u32,
    pub elapsed_secs: f64,
    pub status: TrialStatus,
    pub pass_rate: Option<f64>,
}

pub enum TrialStatus {
    Planning,
    Building,
    Testing,
    Complete,
    Failed(String),
}
```

### Pattern 3: Conversation Display with ContentBlocks
**What:** Parse stream-json to extract thinking, tool_use, text blocks for display
**When to use:** Enhanced TUI showing full LLM conversation
**Example:**
```rust
// Source: src/subprocess/stream_json.rs ContentBlock
pub enum ConversationItem {
    Thinking(String),
    Text(String),
    ToolUse { name: String, summary: String },
    ToolResult { name: String, truncated_output: String },
}

// Extract from StreamEvent
fn extract_conversation_items(event: &StreamEvent) -> Vec<ConversationItem> {
    let message = match &event.message {
        Some(m) => m,
        None => return vec![],
    };

    match &message.content {
        MessageContent::Blocks(blocks) => {
            blocks.iter().filter_map(|block| {
                match block.block_type.as_str() {
                    "thinking" => block.thinking.clone().map(ConversationItem::Thinking),
                    "text" => block.text.clone().map(ConversationItem::Text),
                    "tool_use" => {
                        let name = block.name.clone().unwrap_or_default();
                        let input_json = block.input.as_ref()
                            .map(|v| serde_json::to_string(v).unwrap_or_default())
                            .unwrap_or_default();
                        let summary = format_tool_summary(&name, &input_json);
                        Some(ConversationItem::ToolUse { name, summary })
                    }
                    _ => None,
                }
            }).collect()
        }
        _ => vec![],
    }
}
```

### Pattern 4: Multi-Pane Dashboard Layout
**What:** Split terminal into grid showing all trials with progress bars
**When to use:** Parallel eval dashboard
**Example:**
```rust
// Source: Pattern from src/tui/ui.rs Layout usage
fn render_dashboard(frame: &mut Frame, area: Rect, state: &ParallelEvalState) {
    // Grid layout: 3 columns for 3 modes, N rows for N trials
    let mode_count = state.mode_stats.len();
    let trial_count = state.trials.len() / mode_count;

    // Header row + trial rows
    let row_constraints: Vec<Constraint> = std::iter::once(Constraint::Length(2))
        .chain((0..trial_count).map(|_| Constraint::Length(3)))
        .collect();

    let rows = Layout::vertical(row_constraints).split(area);

    // Render header with mode names
    render_mode_headers(frame, rows[0], &state.mode_stats);

    // Render each trial row
    for (row_idx, trial_row) in rows.iter().skip(1).enumerate() {
        render_trial_row(frame, *trial_row, &state.trials, row_idx);
    }
}
```

### Anti-Patterns to Avoid
- **Shared mutable state across trials:** Don't use Arc<Mutex<_>> for trial state - use channels instead
- **Blocking in async context:** Don't use std::sync primitives - use tokio equivalents
- **Single event loop for all trials:** Don't poll sequentially - use tokio::select! with channel per trial
- **Unbounded buffers:** Consider bounded channels if memory is a concern for long-running evals

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Stream-json parsing | Custom JSON parser | Existing StreamEvent/ContentBlock | Already handles all message types |
| Tool input formatting | Raw JSON display | format_tool_summary() | Produces readable summaries |
| Async channel merging | Manual polling | tokio::select! macro | Built-in, efficient |
| Terminal raw mode | Direct crossterm calls | src/tui/terminal.rs setup/teardown | Already handles cleanup |
| Token formatting | Manual formatting | format_tokens() | Handles K/M suffixes |
| Progress tracking | New struct | Extend existing App state | Patterns established |

**Key insight:** The codebase has mature patterns for TUI + async integration. Extend, don't rewrite.

## Common Pitfalls

### Pitfall 1: Deadlock on Channel Close
**What goes wrong:** Sender drops before receiver processes all messages, causing lost events
**Why it happens:** Trial completes but TUI still expects events
**How to avoid:** Send explicit completion event before dropping sender:
```rust
// Send completion before task ends
trial_tx.send(SubprocessEvent::TrialComplete { mode, trial_num, result }).ok();
// Now sender can drop safely
```
**Warning signs:** Missing final results, dashboard shows "running" forever

### Pitfall 2: Context Deadline Exceeded on Many Parallel Runs
**What goes wrong:** Too many parallel Claude CLI processes overwhelm API rate limits
**Why it happens:** Unbounded parallelism with API-backed execution
**How to avoid:** Limit parallelism with semaphore:
```rust
use tokio::sync::Semaphore;

static PARALLEL_LIMIT: usize = 3; // Or configurable

let semaphore = Arc::new(Semaphore::new(PARALLEL_LIMIT));
for trial in trials {
    let permit = semaphore.clone().acquire_owned().await?;
    tokio::spawn(async move {
        run_trial(...).await;
        drop(permit); // Release when done
    });
}
```
**Warning signs:** API errors, 429 responses, hung processes

### Pitfall 3: Terminal Corruption on Panic
**What goes wrong:** Panic in spawned task doesn't clean up terminal raw mode
**Why it happens:** ratatui setup not restored before exit
**How to avoid:** Use existing teardown_terminal() in Drop or panic hook:
```rust
// Already handled in src/tui/terminal.rs
// But for parallel, ensure main coordinator catches panics:
let result = tokio::spawn(trial_task).await;
if result.is_err() {
    // Log but don't crash - other trials continue
}
```
**Warning signs:** Terminal shows escape codes after crash

### Pitfall 4: Event Ordering in Multi-Source Dashboard
**What goes wrong:** Events from different trials interleave confusingly
**Why it happens:** Async execution order is non-deterministic
**How to avoid:** Tag all events with source, render in separate panes:
```rust
// TrialEvent already has mode + trial_num
// Dashboard renders each trial in its own cell
```
**Warning signs:** Progress jumping around, confusion about which trial is which

### Pitfall 5: Memory Growth from Conversation History
**What goes wrong:** Storing full conversation for every trial exhausts memory
**Why it happens:** Long evals generate megabytes of output
**How to avoid:** Limit conversation buffer per trial:
```rust
const MAX_CONVERSATION_ITEMS: usize = 1000;

fn add_conversation_item(&mut self, item: ConversationItem) {
    if self.conversation.len() >= MAX_CONVERSATION_ITEMS {
        self.conversation.remove(0); // Ring buffer behavior
    }
    self.conversation.push(item);
}
```
**Warning signs:** Memory usage grows unbounded during long evals

## Code Examples

Verified patterns from official sources:

### Running Multiple Trials in Parallel with JoinSet
```rust
// Source: tokio JoinSet documentation pattern
use tokio::task::JoinSet;

async fn run_parallel_evals(
    modes: Vec<PromptMode>,
    trials_per_mode: u32,
    aggregator_tx: mpsc::UnboundedSender<TrialEvent>,
) -> Vec<EvalResult> {
    let mut set = JoinSet::new();

    for mode in &modes {
        for trial_num in 1..=trials_per_mode {
            let tx = aggregator_tx.clone();
            let mode = *mode;
            set.spawn(async move {
                run_trial_with_events(mode, trial_num, tx).await
            });
        }
    }

    let mut results = Vec::new();
    while let Some(result) = set.join_next().await {
        match result {
            Ok(Ok(eval_result)) => results.push(eval_result),
            Ok(Err(e)) => eprintln!("Trial failed: {}", e),
            Err(e) => eprintln!("Task panicked: {}", e),
        }
    }

    results
}
```

### Multi-Pane Dashboard with Ratatui
```rust
// Source: Pattern from src/tui/ui.rs
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

fn render_trial_cell(frame: &mut Frame, area: Rect, trial: &TrialProgress) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("{} Trial {}", trial.mode, trial.trial_num));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [status_area, progress_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
    ]).areas(inner);

    // Status line: "Building 3/10 | Task 2/5"
    let status = format!(
        "{:?} | Iter {}/{} | Task {}/{}",
        trial.status,
        trial.current_iteration,
        trial.max_iterations,
        trial.current_task,
        trial.total_tasks,
    );
    frame.render_widget(Paragraph::new(status), status_area);

    // Progress bar
    let progress = if trial.max_iterations > 0 {
        trial.current_iteration as f64 / trial.max_iterations as f64
    } else {
        0.0
    };
    let gauge = Gauge::default()
        .ratio(progress)
        .gauge_style(Style::default().fg(Color::Green));
    frame.render_widget(gauge, progress_area);
}
```

### Conversation View with Colored Content Types
```rust
// Source: Pattern from src/tui/widgets/thread_view.rs
fn render_conversation_item(item: &ConversationItem) -> Vec<Line<'static>> {
    match item {
        ConversationItem::Thinking(text) => {
            // Gray italic for thinking
            let style = Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC);
            vec![Line::from(vec![
                Span::styled("[thinking] ", style),
                Span::styled(truncate(text, 200), style),
            ])]
        }
        ConversationItem::Text(text) => {
            // Normal text
            text.lines().map(|l| Line::from(l.to_string())).collect()
        }
        ConversationItem::ToolUse { name, summary } => {
            // Yellow for tool use
            let style = Style::default().fg(Color::Yellow);
            vec![Line::from(vec![
                Span::styled(format!("[{}] ", name), style.add_modifier(Modifier::BOLD)),
                Span::styled(summary.clone(), style),
            ])]
        }
        ConversationItem::ToolResult { name, truncated_output } => {
            // Cyan for tool result
            let style = Style::default().fg(Color::Cyan);
            vec![Line::from(vec![
                Span::styled(format!("[{} result] ", name), style),
                Span::styled(truncated_output.clone(), style),
            ])]
        }
    }
}
```

### Event Aggregation with tokio::select!
```rust
// Source: Pattern from src/tui/event.rs EventHandler::event_loop
async fn aggregate_events(
    mut trial_receivers: Vec<mpsc::UnboundedReceiver<TrialEvent>>,
    aggregated_tx: mpsc::UnboundedSender<TrialEvent>,
) {
    // Use futures::stream::select_all for dynamic number of streams
    use futures::stream::{SelectAll, StreamExt};
    use tokio_stream::wrappers::UnboundedReceiverStream;

    let streams: SelectAll<_> = trial_receivers
        .drain(..)
        .map(|rx| UnboundedReceiverStream::new(rx))
        .collect();

    futures::pin_mut!(streams);

    while let Some(event) = streams.next().await {
        if aggregated_tx.send(event).is_err() {
            break; // Aggregated receiver dropped
        }
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Sequential trial loop | Parallel with JoinSet | Phase 13 | 3x+ speedup for multi-mode evals |
| Text-only TUI output | ContentBlock-aware display | Phase 13 | Rich conversation view |
| Build-only TUI | Plan + Build TUI | Phase 13 | Consistent UX |

**Deprecated/outdated:**
- Sequential `for trial_num in 1..=trials` loop: Replace with parallel spawn
- Plain text subprocess output: Parse stream-json for structured content

## Open Questions

Things that couldn't be fully resolved:

1. **Optimal parallelism limit for Claude API**
   - What we know: Too many parallel calls hit rate limits
   - What's unclear: Exact limit varies by account tier
   - Recommendation: Make configurable with default of 3, document in --help

2. **Dashboard layout for many trials (e.g., 3 modes x 10 trials = 30 cells)**
   - What we know: Terminal size limits visible cells
   - What's unclear: Best UX for scrolling vs pagination vs summary view
   - Recommendation: Show summary grid, allow focus on single trial for detail

3. **Memory budget for conversation history**
   - What we know: Long evals generate lots of output
   - What's unclear: Optimal buffer size vs UX tradeoff
   - Recommendation: Start with 1000 items per trial, make configurable

## Sources

### Primary (HIGH confidence)
- `src/tui/app.rs` - App state, AppEvent enum, TEA pattern
- `src/tui/run.rs` - TUI spawn pattern with mpsc channels
- `src/tui/event.rs` - EventHandler, SubprocessEvent, tokio::select! usage
- `src/tui/ui.rs` - Layout patterns with ratatui
- `src/eval/command.rs` - Trial execution, run_single_trial
- `src/subprocess/stream_json.rs` - StreamEvent, ContentBlock, tool extraction

### Secondary (MEDIUM confidence)
- tokio JoinSet documentation - Parallel task management pattern
- ratatui Gauge widget - Progress bar rendering

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Using existing dependencies
- Architecture: HIGH - Extending proven codebase patterns
- Pitfalls: MEDIUM - Based on async/TUI experience, not production tested
- Conversation display: HIGH - stream_json.rs already parses all block types

**Research date:** 2026-01-22
**Valid until:** 2026-02-22 (30 days - stable domain)

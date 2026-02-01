# Phase 15: Interactive Planning Input - Gap Closure Research

**Researched:** 2026-02-01
**Domain:** Claude CLI stream-json format, AskUserQuestion tool, fake_claude test simulation
**Confidence:** HIGH (based on existing codebase implementation, verified by unit tests)

## Summary

This research documents the exact formats and protocols needed to close the Phase 15 gaps. The root cause of all 6 UAT failures is that prompts explicitly forbid Claude from asking questions ("Do NOT ask clarifying questions"), making the entire Q&A infrastructure dead code.

The gap closure requires:
1. Modifying prompts to allow questions in adaptive mode
2. Extending fake_claude to simulate AskUserQuestion tool_use events
3. Adding E2E tests that exercise the full Q&A flow

**Primary recommendation:** Remove the "Do NOT ask clarifying questions" instruction from prompts when `--adaptive` flag is used, and add fake_claude scenarios that emit AskUserQuestion tool_use events for testing.

## Claude CLI AskUserQuestion Format

### Init Event Structure (INTER-01)

The session_id is captured from the init event. Based on the existing implementation in `src/subprocess/stream_json.rs`:

```json
{
  "type": "system",
  "subtype": "init",
  "session_id": "fa0f513d-ca3f-447f-aaa3-9d12ffb6a75f",
  "tools": [...]
}
```

**Detection logic (already implemented):**
```rust
pub fn is_init_event(&self) -> bool {
    self.event_type == "system"
        && self.subtype.as_deref() == Some("init")
}

pub fn extract_session_id(&self) -> Option<&str> {
    if self.is_init_event() {
        self.session_id.as_deref()
    } else {
        None
    }
}
```

**Confidence:** HIGH - This is already implemented and tested in `stream_json.rs`.

### AskUserQuestion Tool Use Event (INTER-02, INTER-03)

When Claude uses the AskUserQuestion tool, it emits an assistant event with a tool_use content block:

```json
{
  "type": "assistant",
  "message": {
    "id": "msg_abc123",
    "role": "assistant",
    "content": [
      {
        "type": "tool_use",
        "id": "toolu_0001",
        "name": "AskUserQuestion",
        "input": {
          "questions": [
            "What programming language do you want to use?",
            "What database backend should we use?"
          ]
        }
      }
    ],
    "model": "claude-opus-4-5-20251101",
    "stop_reason": "tool_use",
    "usage": {
      "input_tokens": 1000,
      "output_tokens": 150
    }
  },
  "uuid": "event-uuid-here",
  "timestamp": "2026-02-01T10:00:00Z"
}
```

**Key points:**
- Event type is `"assistant"`
- Content is an array with a `tool_use` block
- Tool name is exactly `"AskUserQuestion"`
- Input contains a `questions` array of strings
- Stop reason is `"tool_use"` (not `"end_turn"`)

**Detection logic (already implemented):**
```rust
pub fn extract_ask_user_questions(&self) -> Option<AskUserQuestion> {
    let message = self.message.as_ref()?;
    let blocks = match &message.content {
        MessageContent::Blocks(blocks) => blocks,
        _ => return None,
    };

    for block in blocks {
        if block.block_type == "tool_use" && block.name.as_deref() == Some("AskUserQuestion") {
            if let Some(input) = &block.input {
                if let Some(questions_value) = input.get("questions") {
                    if let Some(questions_array) = questions_value.as_array() {
                        let questions: Vec<String> = questions_array
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();

                        if !questions.is_empty() {
                            return Some(AskUserQuestion { questions });
                        }
                    }
                }
            }
        }
    }

    None
}
```

**Confidence:** HIGH - Implementation exists with unit tests passing.

## Session Resume Protocol (INTER-05)

### Resume Command

To resume a session with answers:

```bash
claude -p --verbose --output-format stream-json \
    --resume fa0f513d-ca3f-447f-aaa3-9d12ffb6a75f \
    "Here are my answers to your questions:

Q1: What programming language do you want to use?
Q2: What database backend should we use?

My answers:
I want to use Rust.
PostgreSQL for the database."
```

**Key points:**
- Use `--resume SESSION_ID` to continue a previous session
- The positional argument after --resume is the user's message
- Claude receives this as a new user turn in the conversation
- The same output format (stream-json) is used for the resumed session

**Answer formatting (already implemented):**
```rust
fn format_answers_for_resume(questions: &[String], answers: &str) -> String {
    let mut formatted = String::from("Here are my answers to your questions:\n\n");

    for (i, question) in questions.iter().enumerate() {
        formatted.push_str(&format!("Q{}: {}\n", i + 1, question));
    }

    formatted.push_str(&format!("\nMy answers:\n{}\n", answers));

    formatted
}
```

**Confidence:** HIGH - Implementation exists and is tested.

### Multi-Round Support (INTER-06)

The existing implementation handles multi-round Q&A with a loop:

```rust
const MAX_QUESTION_ROUNDS: u32 = 5;
let mut round = 0;

while stream_response.has_questions() {
    round += 1;
    if round > MAX_QUESTION_ROUNDS {
        break;
    }

    // ... collect answers, resume session ...
    stream_response = resume_session(...).await?;
}
```

**Confidence:** HIGH - Implementation exists.

## Fake Claude Extension Design

### Current Architecture

The fake_claude binary reads configuration from `FAKE_CLAUDE_CONFIG` environment variable:

```
tests/
├── fake_claude.rs           # Binary entry point
└── fake_claude_lib/
    ├── mod.rs               # Module declarations
    ├── config.rs            # FakeClaudeConfig, InvocationConfig
    ├── stream_json.rs       # StreamEventOutput, ContentBlockOutput
    ├── scenario.rs          # ScenarioBuilder, FakeClaudeHandle
    └── prebuilt.rs          # Pre-made scenarios
```

### Required Extensions

#### 1. Add System Init Event with Session ID

**File:** `tests/fake_claude_lib/stream_json.rs`

Add a new constructor for system init events with session_id:

```rust
impl StreamEventOutput {
    /// Create a system init event with session_id for testing session resume.
    pub fn system_init_with_session(session_id: &str) -> Self {
        Self {
            event_type: "system".to_string(),
            subtype: Some("init".to_string()),  // Add subtype field
            session_id: Some(session_id.to_string()),  // Add session_id field
            message: None,
            uuid: Some(uuid_v4_simple()),
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        }
    }
}
```

**Also add these fields to StreamEventOutput struct:**
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub subtype: Option<String>,

#[serde(skip_serializing_if = "Option::is_none")]
pub session_id: Option<String>,
```

#### 2. Add AskUserQuestion Helper

**File:** `tests/fake_claude_lib/stream_json.rs`

```rust
impl StreamEventOutput {
    /// Create an AskUserQuestion tool_use event.
    pub fn ask_user_question(questions: Vec<&str>) -> Self {
        let questions_json: Vec<serde_json::Value> = questions
            .into_iter()
            .map(|q| serde_json::Value::String(q.to_string()))
            .collect();

        let input = serde_json::json!({
            "questions": questions_json
        });

        let id = next_tool_id();
        Self::assistant_with_blocks(
            vec![ContentBlockOutput::tool_use(&id, "AskUserQuestion", input)],
            Some("tool_use"),  // stop_reason
        )
    }
}
```

#### 3. Add ScenarioBuilder Methods

**File:** `tests/fake_claude_lib/scenario.rs`

```rust
impl ScenarioBuilder {
    /// Start this invocation with a system init event containing a session ID.
    ///
    /// Use this for scenarios that need session resume testing.
    pub fn with_session_id(mut self, session_id: &str) -> Self {
        self.current_invocation.events.push(
            StreamEventOutput::system_init_with_session(session_id)
        );
        self
    }

    /// Add an AskUserQuestion tool_use event to current invocation.
    ///
    /// The event will have stop_reason="tool_use" to indicate
    /// Claude is waiting for input.
    pub fn asks_questions(mut self, questions: Vec<&str>) -> Self {
        self.current_invocation.events.push(
            StreamEventOutput::ask_user_question(questions)
        );
        self
    }
}
```

#### 4. Handle --resume Flag in fake_claude Binary

**File:** `tests/fake_claude.rs`

The fake_claude binary needs to detect the `--resume` flag to know which invocation to use:

```rust
fn main() {
    // ... existing config loading ...

    // Check if this is a resume invocation
    let args: Vec<String> = env::args().collect();
    let is_resume = args.iter().any(|a| a == "--resume");

    // Read and increment counter
    let invocation = increment_counter(&config.counter_path);

    // For resume invocations, the counter will be incremented naturally
    // so invocation 1 is the first resume after invocation 0's questions

    // ... rest of existing logic ...
}
```

No special handling needed - the counter naturally handles invocations:
- Invocation 0: Initial call, emits AskUserQuestion
- Invocation 1: Resume call with --resume flag, emits response

### Prebuilt Interactive Scenario

**File:** `tests/fake_claude_lib/prebuilt.rs`

```rust
/// Create a fake Claude scenario with AskUserQuestion flow.
///
/// This scenario simulates:
/// 1. Initial call: Emits system init + AskUserQuestion
/// 2. Resume call: Receives answers, produces progress file
pub fn interactive_planning() -> ScenarioBuilder {
    let progress = r#"# Progress: Interactive Test

## Status

In Progress

## Tasks

### Phase 1: Setup

- [ ] Configure project

## Testing Strategy

Basic tests.
"#;

    ScenarioBuilder::new()
        // Invocation 0: Ask questions
        .with_session_id("test-session-123")
        .asks_questions(vec![
            "What programming language?",
            "What database?"
        ])
        .next_invocation()
        // Invocation 1: Resume with answers, produce progress file
        .with_session_id("test-session-123")  // Same session ID
        .respond_with_text(progress)
}

/// Create a multi-round Q&A scenario.
pub fn multi_round_qa() -> ScenarioBuilder {
    let progress = r#"# Progress: Multi-Round Test

## Status

RALPH_DONE

## Tasks

### Phase 1: Done

- [x] Task complete
"#;

    ScenarioBuilder::new()
        // Round 1: First questions
        .with_session_id("multi-session-456")
        .asks_questions(vec!["Question round 1?"])
        .next_invocation()
        // Round 2: Follow-up questions
        .with_session_id("multi-session-456")
        .asks_questions(vec!["Question round 2?"])
        .next_invocation()
        // Round 3: Final response
        .with_session_id("multi-session-456")
        .respond_with_text(progress)
}
```

## Test Cases by Requirement

### INTER-01: Session ID Capture

**Unit Test (existing in stream_json.rs):**
```rust
#[test]
fn test_stream_response_session_id_extraction() {
    let mut response = StreamResponse::new();
    let init_json = r#"{"type":"system","subtype":"init","session_id":"test-session-123","tools":[]}"#;
    response.process_line(init_json);
    assert_eq!(response.session_id, Some("test-session-123".to_string()));
}

#[test]
fn test_stream_response_session_id_first_wins() {
    // Already exists - verifies first init event's session_id is kept
}
```

**E2E Test (new):**
```rust
#[tokio::test]
async fn test_session_id_captured_from_fake_claude() {
    let handle = ScenarioBuilder::new()
        .with_session_id("e2e-session-789")
        .respond_with_text("# Progress: Test\n\n## Status\nIn Progress\n\n## Tasks\n...")
        .build();

    // Run rslph plan with fake claude
    // Assert session_id was captured
}
```

### INTER-02: AskUserQuestion Detection

**Unit Test (existing):**
```rust
#[test]
fn test_extract_ask_user_questions() {
    let json = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"AskUserQuestion","input":{"questions":["Q1?","Q2?"]}}]}}"#;
    let event = StreamEvent::parse(json).expect("should parse");
    let ask = event.extract_ask_user_questions().expect("should have AskUserQuestion");
    assert_eq!(ask.questions.len(), 2);
}
```

**E2E Test (new):**
```rust
#[tokio::test]
async fn test_asks_questions_detected_from_fake_claude() {
    let handle = prebuilt::interactive_planning().build();
    // Run rslph and assert questions were detected
}
```

### INTER-03: Question Parsing

**Unit Test (existing):**
```rust
#[test]
fn test_stream_response_questions_accumulation() {
    // Already exists - verifies questions are accumulated correctly
}
```

**E2E Test (new):**
```rust
#[tokio::test]
async fn test_questions_parsed_correctly() {
    let handle = ScenarioBuilder::new()
        .with_session_id("parse-test")
        .asks_questions(vec!["Question 1?", "Question 2?", "Question 3?"])
        .build();
    // Run and verify all 3 questions are parsed
}
```

### INTER-04: User Input Collection

**Unit Test (CLI mode - mock stdin):**
This requires testing the `read_multiline_input()` function. Since it reads from stdin directly, testing requires either:
- Refactoring to accept a reader trait
- Using process-based testing

**E2E Test:** Not directly testable without stdin mocking infrastructure.

### INTER-05: Session Resume

**Integration Test (new):**
```rust
#[tokio::test]
async fn test_session_resume_produces_valid_output() {
    let handle = prebuilt::interactive_planning().build();

    // Run rslph plan --adaptive with fake claude
    // Provide answers via stdin mock or headless mode
    // Assert progress.md is created and valid
}
```

### INTER-06: Multi-Round Support

**E2E Test (new):**
```rust
#[tokio::test]
async fn test_multi_round_qa_completes() {
    let handle = prebuilt::multi_round_qa().build();

    // Run rslph plan --adaptive
    // Provide answers for each round
    // Assert final progress.md is valid
    // Assert token accumulation message includes round count
}
```

### INTER-07: Fallback Handling

**E2E Test (new - based on existing calculator scenario):**
```rust
#[tokio::test]
async fn test_no_questions_proceeds_normally() {
    let handle = prebuilt::calculator().build();  // This doesn't ask questions

    // Run rslph plan
    // Assert progress.md created without Q&A loop
}
```

## Prompt Modification Strategy

### Root Cause

The following instruction appears in planning prompts and prevents Claude from asking questions:

**`prompts/gsd/PROMPT_plan.md` line 177:**
```
7. Do NOT ask clarifying questions - make reasonable assumptions
```

**`prompts/PROMPT_plan.md` (basic) line 87:**
```
7. Do NOT ask clarifying questions - make reasonable assumptions
```

### Solution Options

#### Option A: Conditional Prompt (Recommended)

Create a conditional instruction that allows questions in adaptive mode:

**Modify the guideline to:**
```markdown
7. In standard mode: Make reasonable assumptions rather than asking questions.
   In adaptive mode: Use the AskUserQuestion tool to gather critical missing information.
```

**Implementation:**
The `run_plan_command` function already knows if `adaptive` is true. Modify `get_plan_prompt_for_mode()` to accept an `adaptive` parameter and return a different prompt section.

#### Option B: Separate Prompt Files

Create `PROMPT_plan_adaptive.md` variants that encourage questions:

**Pros:** Clean separation, no conditional logic in prompts
**Cons:** Prompt duplication, maintenance burden

#### Option C: Prompt Append

Keep base prompts unchanged, append an "adaptive mode" section when `--adaptive` flag is used:

```rust
let system_prompt = if adaptive {
    format!("{}\n\n## Adaptive Mode Instructions\nYou MAY use AskUserQuestion tool...", base_prompt)
} else {
    base_prompt
};
```

**Pros:** Minimal changes to existing prompts
**Cons:** Prompt structure is less clear

### Recommendation

Use **Option A** with prompt modification. The change is minimal:

1. In `src/prompts/mod.rs`, add `get_plan_prompt_for_mode_and_adaptive(mode, adaptive)`
2. Modify the single line in each PROMPT_plan.md to be conditional
3. When adaptive=true, include instruction to use AskUserQuestion for critical questions
4. When adaptive=false, keep the "Do NOT ask questions" instruction

## Recommendations for Gap Closure Plans

### Plan Structure

Create a single gap closure plan with 4 tasks:

1. **Task 1: Extend fake_claude for AskUserQuestion**
   - Add session_id and subtype fields to StreamEventOutput
   - Add `system_init_with_session()` constructor
   - Add `ask_user_question()` constructor
   - Add `with_session_id()` and `asks_questions()` to ScenarioBuilder
   - Add `interactive_planning()` and `multi_round_qa()` prebuilt scenarios
   - Unit tests for new functionality

2. **Task 2: Modify prompts to allow questions in adaptive mode**
   - Update `prompts/gsd/PROMPT_plan.md` guideline 7
   - Update `prompts/PROMPT_plan.md` guideline 7
   - Update `get_plan_prompt_for_mode()` to accept adaptive parameter
   - Modify callers to pass adaptive flag

3. **Task 3: Add E2E tests for INTER-01 through INTER-07**
   - Test session ID capture with fake_claude
   - Test AskUserQuestion detection
   - Test session resume produces valid output
   - Test multi-round Q&A
   - Test fallback (no questions) works

4. **Task 4: Run UAT and verify gaps closed**
   - Re-run all 10 UAT tests
   - Document results
   - Address any remaining issues

### Verification Approach

Each task should verify:
1. Unit tests pass: `cargo test`
2. Clippy passes: `cargo clippy`
3. Specific tests pass: `cargo test [test_name]`

Final verification: Re-run UAT protocol from 15-UAT.md.

## Common Pitfalls

### Pitfall 1: Forgetting to Add Fields to Serialize

**What goes wrong:** New fields in StreamEventOutput don't appear in output
**Prevention:** Add `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields
**Detection:** Unit test fails to find expected fields in JSON

### Pitfall 2: Counter Race Conditions

**What goes wrong:** Parallel tests interfere with invocation counter
**Prevention:** Each ScenarioBuilder creates its own temp directory with isolated counter
**Detection:** Tests pass individually but fail in parallel

### Pitfall 3: Prompt Changes Break Existing Behavior

**What goes wrong:** Changing prompts affects non-adaptive mode
**Prevention:** Make changes conditional on adaptive flag
**Detection:** Existing tests fail after prompt changes

## Sources

### Primary (HIGH confidence)
- `/Users/vmakaev/Non-Work/rslph/src/subprocess/stream_json.rs` - Existing implementation and tests
- `/Users/vmakaev/Non-Work/rslph/src/planning/command.rs` - Session resume implementation
- `/Users/vmakaev/Non-Work/rslph/tests/fake_claude_lib/` - Test infrastructure

### Secondary (MEDIUM confidence)
- [ClaudeLog: --resume flag](https://claudelog.com/faqs/what-is-resume-flag-in-claude-code/) - Resume behavior
- [GitHub: claude-ask-user-demo](https://github.com/oneryalcin/claude-ask-user-demo) - AskUserQuestion patterns

### Tertiary (LOW confidence)
- [Claude Code Guide](https://github.com/Cranot/claude-code-guide) - General CLI usage

## Metadata

**Confidence breakdown:**
- AskUserQuestion format: HIGH - verified against existing implementation
- Session resume protocol: HIGH - verified against existing implementation
- Fake Claude extension: HIGH - based on existing architecture
- Prompt modification: MEDIUM - straightforward but untested approach

**Research date:** 2026-02-01
**Valid until:** 2026-03-01 (stable implementation patterns)

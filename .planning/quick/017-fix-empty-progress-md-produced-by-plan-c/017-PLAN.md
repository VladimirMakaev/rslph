---
phase: quick
plan: 017
type: execute
wave: 1
depends_on: []
files_modified:
  - tests/e2e/test_rslph_integration.rs
  - src/progress.rs
autonomous: true

must_haves:
  truths:
    - "Plan command with --adaptive and --mode=gsd produces non-empty progress.md"
    - "ProgressFile::parse returns error when required sections are missing"
    - "E2E test using fake Claude validates plan command end-to-end"
  artifacts:
    - path: "tests/e2e/test_rslph_integration.rs"
      provides: "E2E test for plan command with adaptive mode"
      contains: "test_rslph_plan_adaptive_mode"
    - path: "src/progress.rs"
      provides: "Validation in parse() to reject empty progress files"
      contains: "is_empty"
  key_links:
    - from: "src/planning/command.rs"
      to: "src/progress.rs"
      via: "ProgressFile::parse"
      pattern: "ProgressFile::parse"
---

<objective>
Fix empty progress.md produced by `ralph plan --mode=gsd --adaptive INITIAL.md` command.

Purpose: When user runs plan command with adaptive mode, the output progress.md file is empty. This is caused by either:
1. StreamResponse not accumulating text correctly from Claude's stream-json output
2. ProgressFile::parse being too lenient and returning empty struct instead of error
3. The fake Claude scenario not producing valid progress file content

Output: Working plan command that produces valid progress.md with tasks, and a test to prevent regression.
</objective>

<execution_context>
@/Users/vmakaev/.claude/get-shit-done/workflows/execute-plan.md
@/Users/vmakaev/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@src/planning/command.rs - run_plan_command, run_adaptive_planning, run_basic_planning
@src/progress.rs - ProgressFile::parse, to_markdown
@tests/e2e/test_rslph_integration.rs - existing plan E2E tests
@tests/fake_claude_lib/prebuilt.rs - example scenarios with progress file content
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add E2E test for plan command with adaptive mode using fake Claude</name>
  <files>tests/e2e/test_rslph_integration.rs</files>
  <action>
Add a new test `test_rslph_plan_adaptive_mode_produces_valid_progress` that:

1. Creates a ScenarioBuilder with proper progress file content for adaptive mode:
   - First invocation: clarifying questions response (any text)
   - Second invocation: testing strategy response (any text)
   - Third invocation: final plan with VALID progress file markdown content
   - Fourth invocation (optional): project name response if name is empty

2. The third invocation response should include valid progress file format:
```markdown
# Progress: Test Project

## Status

In Progress

## Tasks

### Phase 1: Setup

- [ ] Create configuration file
- [ ] Set up project structure

## Testing Strategy

Run unit tests to verify functionality.
```

3. Use WorkspaceBuilder with a simple input file (e.g., "Build a test project")

4. Run `rslph plan --adaptive --no-tui` with the fake Claude

5. Assert:
   - Command succeeds (exit code 0)
   - progress.md file exists in workspace
   - progress.md content is NOT empty
   - progress.md contains expected sections (Status, Tasks)

Reference existing test `test_rslph_plan_single_response` for pattern.
  </action>
  <verify>
Run: `cargo test test_rslph_plan_adaptive_mode_produces_valid_progress -- --nocapture`
Test should pass (or fail revealing the actual bug if it exists).
  </verify>
  <done>E2E test exists that validates adaptive mode plan command produces valid progress.md</done>
</task>

<task type="auto">
  <name>Task 2: Add validation to ProgressFile::parse to reject empty progress files</name>
  <files>src/progress.rs</files>
  <action>
Modify `ProgressFile::parse` to validate the parsed result before returning:

1. After parsing completes (before line 316), add validation:
   - If `pf.name.is_empty() && pf.status.is_empty() && pf.tasks.is_empty()`, the parse failed to extract meaningful content

2. Return an error for empty/invalid progress files:
   ```rust
   // Validate that we parsed something meaningful
   if pf.name.is_empty() && pf.status.is_empty() && pf.tasks.is_empty() && pf.analysis.is_empty() {
       return Err(RslphError::Parse(
           "Failed to parse progress file: no valid sections found".to_string()
       ));
   }
   ```

3. Ensure RslphError::Parse exists in error.rs (it should already exist based on codebase patterns)

4. Add a unit test `test_parse_empty_content_returns_error` that verifies:
   - Empty string returns error
   - String without progress sections returns error
   - Valid progress file still parses correctly
  </action>
  <verify>
Run: `cargo test progress::tests:: -- --nocapture`
All existing tests should pass, plus new validation test.
  </verify>
  <done>ProgressFile::parse returns error for empty/invalid content instead of empty struct</done>
</task>

<task type="auto">
  <name>Task 3: Debug and fix the root cause in planning command</name>
  <files>src/planning/command.rs</files>
  <action>
Based on what the test reveals, fix the actual bug. Likely issues:

1. **If StreamResponse.text is empty**: The stream-json parsing may not be extracting text correctly from Claude's adaptive mode responses. Check that:
   - Events are being parsed correctly
   - Assistant messages with text blocks are being accumulated
   - The response_text variable has content before parse

2. **If parse fails with new validation**: The Claude response format may not match expected progress file format. Add better error messages that show what was received:
   ```rust
   eprintln!("[TRACE] Response text length: {} chars", response_text.len());
   if response_text.len() < 100 {
       eprintln!("[TRACE] Full response: {}", response_text);
   }
   ```

3. **Propagate parse errors properly**: Ensure parse errors bubble up with context about what failed:
   ```rust
   let progress_file = ProgressFile::parse(&response_text)
       .map_err(|e| RslphError::Parse(format!(
           "Failed to parse Claude response as progress file: {}. Response length: {} chars",
           e, response_text.len()
       )))?;
   ```

The actual fix depends on what the E2E test reveals. Focus on making the error visible so users know what went wrong.
  </action>
  <verify>
Run: `cargo test test_rslph_plan -- --nocapture`
All plan-related tests should pass.
  </verify>
  <done>Plan command with adaptive mode produces valid progress.md, or returns meaningful error if Claude response is invalid</done>
</task>

</tasks>

<verification>
1. `cargo test test_rslph_plan -- --nocapture` - all plan tests pass
2. `cargo test progress::tests:: -- --nocapture` - progress parsing tests pass
3. `cargo clippy -- -D warnings` - no new warnings
4. `cargo build --release` - builds successfully
</verification>

<success_criteria>
- E2E test validates plan command with adaptive mode
- ProgressFile::parse rejects empty/invalid content with error
- Plan command produces valid progress.md or meaningful error
- All tests pass
</success_criteria>

<output>
After completion, create `.planning/quick/017-fix-empty-progress-md-produced-by-plan-c/017-SUMMARY.md`
</output>

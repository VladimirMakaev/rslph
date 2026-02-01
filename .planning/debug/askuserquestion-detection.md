---
status: diagnosed
trigger: "Investigate why AskUserQuestion detection is not working in rslph"
created: 2026-02-01T00:00:00Z
updated: 2026-02-01T00:01:00Z
---

## Current Focus

hypothesis: CONFIRMED - Multiple compounding issues preventing AskUserQuestion from working
test: Traced entire flow from prompts through parsing
expecting: N/A - root cause found
next_action: Report findings

## Symptoms

expected: When Claude uses AskUserQuestion tool, questions should be detected and displayed
actual: Parse error in normal mode, no questions in adaptive mode, TUI doesn't show questions
errors: Parse error (unspecified)
reproduction:
  - `rslph plan --mode=gsd INITIAL.md` fails with parse error
  - `rslph plan --mode=gsd --adaptive --no-tui INITIAL.md` doesn't ask questions
  - TUI mode doesn't show questions
started: Phase 15 implementation (recent)

## Eliminated

## Evidence

- timestamp: 2026-02-01T00:00:30Z
  checked: prompts/gsd/PROMPT_plan.md and prompts/basic/PROMPT_plan.md
  found: Line 177 in GSD prompt says "7. Do NOT ask clarifying questions - make reasonable assumptions"
  implication: The prompt EXPLICITLY tells Claude NOT to ask questions

- timestamp: 2026-02-01T00:00:35Z
  checked: src/subprocess/stream_json.rs
  found: extract_ask_user_questions() implementation looks correct - parses tool_use with name="AskUserQuestion"
  implication: Parsing logic is not the issue - tests pass

- timestamp: 2026-02-01T00:00:40Z
  checked: Grep for "AskUserQuestion" in prompts/
  found: Zero matches - AskUserQuestion tool is NOT defined in any prompt
  implication: Claude doesn't know this tool exists - it's a Claude CLI built-in, not a prompt-defined tool

- timestamp: 2026-02-01T00:00:45Z
  checked: .planning/research/claude-cli-interactive-input.md
  found: Research confirms AskUserQuestion is auto-responded with error in -p mode
  implication: Even if Claude uses AskUserQuestion, CLI auto-responds before we can capture it

- timestamp: 2026-02-01T00:00:50Z
  checked: src/planning/command.rs lines 181-192
  found: has_questions() check exists, display_questions() is called if true
  implication: Code path exists but never executes because Claude never uses AskUserQuestion

- timestamp: 2026-02-01T00:00:55Z
  checked: Unit tests for questions accumulation
  found: test_stream_response_questions_accumulation passes
  implication: Parsing works correctly when questions ARE present in stream

## Resolution

root_cause: |
  COMPOUND ISSUE with 3 factors:

  1. PROMPT INSTRUCTS NO QUESTIONS: Both GSD and Basic prompts explicitly say
     "Do NOT ask clarifying questions - make reasonable assumptions" (line 177 in GSD prompt).
     Claude is following instructions and NOT asking questions.

  2. ASKUSERQUESTION NOT DEFINED: The AskUserQuestion tool is a Claude CLI built-in,
     not defined in rslph prompts. When using custom --system-prompt, Claude may not
     have access to AskUserQuestion tool at all.

  3. CLI AUTO-RESPONDS: Per research, even if Claude DID use AskUserQuestion in -p mode,
     the CLI auto-generates an error response within ~80ms:
     "tool_result: is_error=true, content='Answer questions?'"
     Claude then falls back to plain text questions, which are NOT detected by extract_ask_user_questions().

  The parse error for `rslph plan --mode=gsd INITIAL.md` is UNRELATED to questions -
  it occurs because Claude's output (when instructed to make assumptions instead of asking)
  may not parse as a valid progress file. This is a separate issue.

fix: |
  To enable interactive questions, the following changes would be needed:

  1. MODIFY PROMPTS: Remove "Do NOT ask clarifying questions" instruction from prompts
     or create new prompts that explicitly encourage questions.

  2. DEFINE ASKUSERQUESTION TOOL: Either:
     a) Add AskUserQuestion tool definition to the system prompt, OR
     b) Use Claude CLI's native interactive mode (not -p mode)

  3. HANDLE PLAIN TEXT QUESTIONS: Since CLI auto-responds to AskUserQuestion,
     detect questions in plain text output instead of tool_use events.
     Look for "?" in output or specific patterns like "Please provide..." or numbered questions.

  4. INVESTIGATE PARSE ERROR: The parse error is separate - likely Claude's output
     when using assumptions doesn't conform to expected progress file format.

verification:
files_changed: []

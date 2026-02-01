# Claude CLI Interactive Input Research

**Date**: 2026-02-01
**Context**: Investigating why plan TUI hangs when Claude asks questions

## Problem Statement

When Claude uses `AskUserQuestion` tool during planning, users cannot enter responses. The TUI either:
1. Hangs indefinitely (with piped stdin)
2. Shows questions but doesn't wait for input (with null stdin)

## Key Findings

### 1. Claude CLI Stdin Behavior

| Stdin Type | Behavior |
|------------|----------|
| `Stdio::null()` | EOF immediately, Claude proceeds |
| `Stdio::piped()` (open) | Claude hangs waiting for EOF |
| `Stdio::piped()` + close | EOF sent, Claude proceeds |
| Pipe with content + EOF | Claude reads content, then proceeds |

**Conclusion**: Claude CLI in `-p` mode waits for stdin EOF before starting to process.

### 2. AskUserQuestion Auto-Response

When Claude calls `AskUserQuestion` in `-p` mode:

```
[   8.542] tool_use: AskUserQuestion with questions
[   8.623] tool_result: is_error=true, content="Answer questions?"
```

- Claude CLI auto-generates an error response within ~80ms
- It does NOT wait for external input
- After the error, Claude falls back to asking questions as plain text

### 3. Session Resume Capability

Claude CLI supports session continuation:

```bash
# Capture session_id from JSON output
sid=$(claude -p "prompt" --output-format json | jq -r '.session_id')

# Resume with new message
claude -p --resume "$sid" "User's answers: ..."
```

Flags:
- `--resume` / `-r` - Resume a specific session by ID
- `--continue` / `-c` - Resume most recent session

### 4. Tested Approaches

| Approach | Result |
|----------|--------|
| `spawn()` with `Stdio::null()` | Works, but can't send input later |
| `spawn_interactive()` without closing stdin | Hangs - Claude waits for EOF |
| `spawn_interactive()` + `close_stdin()` | Works - same as null stdin |
| PTY (pseudo-terminal) | Not tested - Claude might behave differently |

## Proposed Solutions

### Option 1: Session Resume (Recommended)

1. Run Claude with `-p`, capture `session_id` from init event
2. When session completes, check if `AskUserQuestion` was called
3. If yes, parse the questions and show them to user
4. Get user answers via TUI input
5. Resume session: `claude -p --resume $session_id "Answers: ..."`

**Pros**: Uses official API, maintains conversation context
**Cons**: Not real-time interactive, requires re-API-call

### Option 2: Re-prompt with Answers

1. Run Claude, detect `AskUserQuestion` in stream
2. When session completes with questions, get user answers
3. Start NEW session with: original prompt + "User's answers to previous questions: ..."

**Pros**: Simple implementation
**Cons**: Loses session context, uses more tokens

### Option 3: PTY-based Approach

Use a pseudo-terminal instead of pipes. Claude CLI might:
- Behave as if connected to real terminal
- Wait for interactive input on AskUserQuestion

**Pros**: Potentially real-time interactive
**Cons**: Complex implementation, uncertain behavior

## Implementation Notes

### Current State (after fix)

```rust
// In run_tui_planning():
let mut runner = ClaudeRunner::spawn_interactive(...).await?;
runner.close_stdin();  // Send EOF immediately
```

This prevents hanging but doesn't enable interactive input.

### Session ID Extraction

The session_id appears in the init event:

```json
{"type":"system","subtype":"init","session_id":"fa0f513d-ca3f-447f-aaa3-9d12ffb6a75f",...}
```

### AskUserQuestion Detection

Use existing `StreamEvent::is_input_required()` or check for:

```json
{"type":"assistant","content":[{"type":"tool_use","name":"AskUserQuestion","input":{...}}]}
```

## Files Modified

- `src/subprocess/runner.rs` - Added `close_stdin()` method
- `src/planning/command.rs` - Call `close_stdin()` after spawn

## Related Commits

- `5fbcdeb` - fix(plan-tui): close stdin immediately to prevent Claude CLI hang
- `bfc40d6` - revert(quick-016): restore plan TUI to working state

## References

- [Mastering Claude Code Sessions](https://www.vibesparking.com/en/blog/ai/claude-code/docs/cli/2025-08-28-mastering-claude-code-sessions-continue-resume-automate/)
- [Claude Code Guide](https://github.com/Cranot/claude-code-guide)

## Next Steps

1. Implement session resume approach for AskUserQuestion handling
2. Parse questions from tool_use event
3. Add TUI input mode for collecting answers
4. Test with adaptive planning mode

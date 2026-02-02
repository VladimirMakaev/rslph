# Phase 15: Interactive Planning Input

**Goal**: Enable users to answer Claude's clarifying questions during planning via session resume

**Depends on**: Phase 13.1 complete (current state)

**Research**: `.planning/research/claude-cli-interactive-input.md`

## Problem Statement

When Claude uses `AskUserQuestion` during planning in `-p` mode:
1. Claude CLI auto-responds with an error within ~80ms
2. Claude falls back to asking questions as plain text
3. The output is not a valid progress file format
4. Parse fails with "no valid sections found"

Users want to actually provide answers to Claude's questions (API keys, architecture decisions, etc.) rather than having Claude make assumptions.

## Proposed Solution: Session Resume

Based on research, the recommended approach is:

1. **Run initial planning** with `-p`, capture `session_id` from init event
2. **Detect AskUserQuestion** in stream via tool_use event
3. **Parse and display questions** to user via TUI or CLI
4. **Collect user answers** via interactive input
5. **Resume session**: `claude -p --resume $session_id "User answers: ..."`
6. **Parse final output** as progress file

## Requirements

| ID | Requirement | Description |
|----|-------------|-------------|
| INTER-01 | Session ID capture | Extract session_id from init event in stream-json output |
| INTER-02 | AskUserQuestion detection | Detect tool_use with name="AskUserQuestion" in stream |
| INTER-03 | Question parsing | Parse questions array from AskUserQuestion input |
| INTER-04 | User input collection | Collect answers via CLI prompt or TUI input mode |
| INTER-05 | Session resume | Resume session with `--resume $sid` and formatted answers |
| INTER-06 | Multi-round support | Handle multiple rounds of questions if needed |
| INTER-07 | Fallback handling | If no questions asked, proceed normally |

## Success Criteria

1. User can run `ralph plan --mode=gsd --adaptive INITIAL.md`
2. When Claude asks questions, they are displayed to the user
3. User can type answers in the terminal
4. Claude receives answers and produces a valid progress file
5. Parse succeeds and progress.md is written

## Technical Notes

### Session ID Location
```json
{"type":"system","subtype":"init","session_id":"fa0f513d-ca3f-447f-aaa3-9d12ffb6a75f",...}
```

### AskUserQuestion Detection
```json
{"type":"assistant","content":[{"type":"tool_use","name":"AskUserQuestion","input":{...}}]}
```

### Resume Command
```bash
claude -p --resume "$session_id" "User's answers: ..."
```

## Plans

- [ ] 15-01: Session ID capture and AskUserQuestion detection
- [ ] 15-02: Interactive input collection (CLI mode)
- [ ] 15-03: Session resume and response parsing
- [ ] 15-04: TUI mode support for question/answer flow

## Files to Modify

- `src/subprocess/stream_json.rs` - Add session_id extraction
- `src/planning/command.rs` - Add question detection and resume logic
- `src/cli.rs` - Add interactive input mode (or reuse multiline input)

## Dependencies

- Claude CLI `--resume` flag support (confirmed available)
- StreamResponse already captures events (extend for session_id)

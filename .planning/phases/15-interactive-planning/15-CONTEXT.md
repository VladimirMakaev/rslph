# Phase 15: Interactive Planning Input - Gap Closure Context

**Gathered:** 2026-02-01
**Status:** Ready for planning (gap closure)

<domain>
## Phase Boundary

Enable users to answer Claude's clarifying questions during planning via session resume. The infrastructure was built (15-01 through 15-04) but UAT revealed the prompts forbid questions, making the feature dead code.

**Gap closure scope:** Make the existing infrastructure work. Not adding new capabilities.

</domain>

<decisions>
## Implementation Decisions

### Testing Approach
- **Research first**: Understand how real Claude CLI emits AskUserQuestion events in stream-json
- **Fake Claude support**: Extend fake_claude to simulate AskUserQuestion tool_use events
- **Test coverage**: Every Phase 15 requirement (INTER-01 through INTER-07) must have a passing test
- **Test-driven**: Write tests that currently fail, then fix implementation to make them pass

### Fake Claude AskUserQuestion Simulation
- Research the exact stream-json format for AskUserQuestion tool_use
- Add scenario builder support for injecting questions
- Simulate multi-round Q&A (question → answer → follow-up question)
- Support session resume simulation (`--resume` flag handling)

### Prompt Modification
- Prompts need to allow questions in adaptive mode
- Specific approach TBD after research (separate files vs conditionals)

### Claude's Discretion
- Exact fake_claude implementation details
- Test organization (new file vs extend existing)
- Naming conventions for test scenarios

</decisions>

<specifics>
## Specific Ideas

- User explicitly wants research on "how claude process works when it asks for clarifications"
- Tests should cover: session ID capture, question detection, parsing, input collection, session resume, multi-round, fallback
- Research should inform fake_claude implementation

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within gap closure scope

</deferred>

---

*Phase: 15-interactive-planning (gap closure)*
*Context gathered: 2026-02-01*

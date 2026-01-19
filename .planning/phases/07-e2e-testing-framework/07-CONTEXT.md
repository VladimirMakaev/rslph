# Phase 7: E2E Testing Framework - Context

**Gathered:** 2026-01-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Comprehensive testing infrastructure with fake Claude simulation for deterministic, reproducible testing of the Ralph Wiggum Loop. Includes fake Claude process, scenario API, workspace fixtures, and test assertion helpers.

</domain>

<decisions>
## Implementation Decisions

### Fake Claude API Design
- **Builder pattern** with factory function entry: `fake_claude()`
- **Multi-invocation builder**: Single builder with `.next_invocation()` to chain responses for multi-iteration scenarios
- **Invocation order matching**: Responses matched by invocation number (1st call, 2nd call), not prompt content
- **Silent pass-through**: Unconfigured invocations return empty/noop response, don't fail
- **Returns executable path**: Builder returns path to fake executable, test passes it to rslph
- **In-memory invocation log**: Scenario tracks all invocations for test assertions
- **Manual cleanup**: Test calls `.cleanup()` when done

### Output Simulation Fidelity
- **Exact stream-json match**: Fake Claude output must match real Claude CLI format exactly
- **Configurable delays**: Streaming timing is configurable between output chunks (default instant)
- **Configurable crash**: Builder can configure exit mid-stream to simulate crash/timeout
- **Raw escape hatch**: `send_raw(...)` method for one-off malformed output tests (not a first-class API)

### Tool Call Simulation
- **Output simulation only**: Fake Claude outputs tool call JSON, does not intercept actual tool execution
- **Both typed helpers and generic**: `.uses_read('path')`, `.uses_write('path', 'content')` for common tools, `.tool_call('ToolName', {...})` for edge cases
- **Strict interleaving order**: Text, then tool, then text, then tool — as written in builder
- **Configurable tool results**: Fake outputs tool_result with configurable content (what Claude "sees" back)

### Test Workspace Behavior
- **Fixture-managed**: Pytest fixture creates isolated workspace, test just uses it
- **Per-test isolation**: Fresh temp directory for every test function
- **Minimal valid default**: Workspace starts with valid rslph setup (config file, git init)
- **Fluent builder API**: Customize workspace setup via builder pattern
- **Cleanup policy**: Default cleanup on success, keep on failure, configurable via env var

### Test Authoring Language
- **Requires research**: Investigate cram, Python/pytest, Rust cargo test, and TUI testing approaches
- **TUI testing complexity**: Research needed to determine feasibility and approach
- **Assertion focus**: Verify user-facing behavior (progress file, file system, git commits, TUI if feasible)

### Claude's Discretion
- Exact stream-json format details (reverse-engineer from real Claude CLI)
- Python package structure and naming
- Specific pytest fixture implementation
- Error message wording

</decisions>

<specifics>
## Specific Ideas

- Fake Claude should be a Python package — same ecosystem simplicity as many test tools
- Builder API should feel natural to Python developers
- Test language selection (cram vs Python vs Rust) should be researched during planning phase
- TUI testing approach needs investigation — may affect test language choice

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 07-e2e-testing-framework*
*Context gathered: 2026-01-19*

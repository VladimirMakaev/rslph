# Phase 10: Eval Projects and Testing - Context

**Gathered:** 2026-01-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can evaluate agent performance against built-in projects with hidden tests. This phase delivers the calculator eval project, a test runner for stdin/stdout testing, hidden test data, a second eval project, and integration with the eval command from Phase 9.

</domain>

<decisions>
## Implementation Decisions

### Calculator project scope
- Scientific operations: add, subtract, multiply, divide, powers, roots, modulo, parentheses, order of operations
- CLI with stdin/stdout invocation (e.g., `echo '2+2' | calc`)
- Language agnostic — agent chooses implementation language
- Output format: number only (just the numeric result, no expression echo)

### Test coverage
- Target: ~100 test cases for calculator project
- Cover all supported operations with varying complexity levels
- Mix of simple (single operation), medium (2-3 operations), and complex (nested parentheses, multiple operations)
- Use Python3 during research/planning to validate expected outputs are correct
- Ensure edge cases: negative numbers, zero division handling, floating-point precision

### Results presentation
- Summary only for pass/fail counts (e.g., "Tests: 8/10 passed (80%)")
- Live progress during test execution (dots or names as each test runs)
- Output to both terminal and JSON file
- Explicit PASS/FAIL message for overall result
- Floating-point comparison with threshold of 0.001 for numeric results

### Claude's Discretion
- Test data format (structure of input/output pairs file)
- Second eval project choice and scope
- Exact progress indicator style (dots vs names vs spinner)
- JSON results file naming and structure

</decisions>

<specifics>
## Specific Ideas

- Calculator should handle scientific operations which is moderately challenging — tests the agent's ability to parse and evaluate expressions
- Threshold-based comparison prevents false failures from floating-point precision issues

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 10-eval-projects-and-testing*
*Context gathered: 2026-01-20*

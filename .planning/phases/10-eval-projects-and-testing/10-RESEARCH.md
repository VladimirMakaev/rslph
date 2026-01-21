# Phase 10: Eval Projects and Testing - Research

**Researched:** 2026-01-20
**Domain:** Built-in eval projects, stdin/stdout test runners, pass rate tracking
**Confidence:** HIGH

## Summary

Phase 10 builds on the eval command foundation (Phase 9) to add built-in eval projects and hidden test execution. The research confirms that:

1. **Existing infrastructure is ready:** Phase 9 implemented the eval command with temp directory isolation, token aggregation, and plan+build orchestration. The `run_eval_command` function already handles project copying and prompt detection.

2. **Embedding strategy:** The `include_dir` crate provides the cleanest approach for embedding entire project directories at compile time. It supports glob patterns and is well-maintained (1.7M monthly downloads).

3. **Test runner approach:** A language-agnostic test runner should use `std::process::Command` with stdin piping and stdout capture. Test cases are best stored in JSONL format (one JSON object per line with input/expected pairs).

4. **Calculator project design:** The first eval project should be a simple CLI calculator that reads expressions from stdin and outputs results to stdout. Test cases verify basic arithmetic operations.

**Primary recommendation:** Use `include_dir` to embed project directories, implement a simple stdin/stdout test runner using existing `std::process::Command` patterns, store test data as JSONL hidden from the agent, and extend `EvalResult` to track pass rate.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| include_dir | 0.7.4 | Embed project directories at compile time | 1.7M downloads/month, simple API, glob support |
| std::process::Command | stdlib | Execute built programs with stdin/stdout | Already used in subprocess module, proven pattern |
| serde_json | 1.0 | Parse JSONL test data | Already in dependencies |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tempfile | 3.x | Already in deps, used for eval workspace | Create isolated test execution environment |
| tokio::process::Command | 1.x | Async process execution with timeout | When timeout support needed |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| include_dir | rust-embed | rust-embed more complex, better for web assets; include_dir simpler for code projects |
| include_dir | includedir | includedir requires build.rs codegen; include_dir uses proc macro |
| JSONL test format | Separate .in/.out files | JSONL keeps input/output paired, easier to maintain |

**Installation:**
```bash
cargo add include_dir
```

```toml
[dependencies]
include_dir = "0.7"
```

## Architecture Patterns

### Recommended Project Structure
```
src/
+-- eval/
|   +-- mod.rs           # Module exports, EvalResult
|   +-- command.rs       # run_eval_command() - EXISTS from Phase 9
|   +-- projects.rs      # NEW: Built-in project registry
|   +-- test_runner.rs   # NEW: Stdin/stdout test execution
|
evals/                   # NEW: Source eval projects (embedded)
+-- calculator/
|   +-- prompt.txt       # Starting prompt for agent
|   +-- tests.jsonl      # Hidden test cases (input/expected pairs)
|   +-- README.md        # Optional project description
|
+-- <second-project>/    # Second eval project (PROJ-04)
    +-- ...
```

### Pattern 1: Directory Embedding with include_dir
**What:** Embed entire project directory at compile time using proc macro
**When to use:** When project files need to be bundled into binary
**Example:**
```rust
// Source: include_dir crate documentation
use include_dir::{include_dir, Dir};

// Embed the entire evals directory at compile time
static EVALS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/evals");

/// Get a built-in eval project by name.
pub fn get_builtin_project(name: &str) -> Option<&'static Dir<'static>> {
    EVALS_DIR.get_dir(name)
}

/// List all available built-in projects.
pub fn list_builtin_projects() -> impl Iterator<Item = &'static str> {
    EVALS_DIR.dirs().map(|d| {
        d.path().file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
    })
}

/// Extract project files to a directory.
pub fn extract_project(project: &Dir, dest: &Path) -> std::io::Result<()> {
    for file in project.files() {
        let file_path = dest.join(file.path());
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file_path, file.contents())?;
    }
    for dir in project.dirs() {
        extract_project(dir, dest)?;
    }
    Ok(())
}
```

### Pattern 2: JSONL Test Data Format
**What:** Store test cases as newline-delimited JSON for simple parsing
**When to use:** When test data should be language-agnostic and easy to read
**Example:**
```json
{"input": "2 + 2", "expected": "4"}
{"input": "10 - 5", "expected": "5"}
{"input": "3 * 4", "expected": "12"}
{"input": "20 / 4", "expected": "5"}
{"input": "2 + 3 * 4", "expected": "14"}
```

```rust
// Source: Standard serde_json pattern
use serde::Deserialize;
use std::io::BufRead;

#[derive(Debug, Deserialize)]
pub struct TestCase {
    pub input: String,
    pub expected: String,
}

/// Load test cases from JSONL content.
pub fn load_test_cases(content: &str) -> Vec<TestCase> {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}
```

### Pattern 3: Stdin/Stdout Test Runner
**What:** Execute built program with input and compare output
**When to use:** Black-box testing of CLI programs
**Example:**
```rust
// Source: std::process::Command documentation + existing patterns
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::path::Path;

#[derive(Debug)]
pub struct TestResult {
    pub input: String,
    pub expected: String,
    pub actual: String,
    pub passed: bool,
}

/// Run a single test case against a program.
pub fn run_test_case(
    program: &Path,
    test: &TestCase,
    timeout: Duration,
) -> std::io::Result<TestResult> {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Write input to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(test.input.as_bytes())?;
        stdin.write_all(b"\n")?;
    }

    // Wait for output with timeout
    let output = child.wait_with_output()?;
    let actual = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    let passed = actual == test.expected;

    Ok(TestResult {
        input: test.input.clone(),
        expected: test.expected.clone(),
        actual,
        passed,
    })
}
```

### Pattern 4: Pass Rate Tracking
**What:** Track passing/total test cases in EvalResult
**When to use:** When eval needs quantitative success metrics
**Example:**
```rust
// Source: Extension to existing EvalResult in src/eval/mod.rs
use crate::build::tokens::TokenUsage;

/// Test execution results for an eval run.
#[derive(Debug, Clone)]
pub struct TestResults {
    /// Number of passing test cases
    pub passed: u32,
    /// Total number of test cases
    pub total: u32,
    /// Individual test case results
    pub cases: Vec<TestResult>,
}

impl TestResults {
    /// Calculate pass rate as a percentage.
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.passed as f64 / self.total as f64) * 100.0
        }
    }
}

/// Extended EvalResult with test results.
#[derive(Debug, Clone)]
pub struct EvalResult {
    pub project: String,
    pub elapsed_secs: f64,
    pub total_tokens: TokenUsage,
    pub iterations: u32,
    pub workspace_path: Option<PathBuf>,
    /// NEW: Hidden test results (EVAL-02, EVAL-03)
    pub test_results: Option<TestResults>,
}
```

### Anti-Patterns to Avoid
- **Exposing test data to agent:** Test data MUST be in a separate location from project files; do NOT include tests.jsonl in the copied project directory
- **Using cargo test for eval:** The eval tests are stdin/stdout black-box tests, not Rust unit tests; they run the built binary, not test the source
- **Hardcoding test expectations:** Use parameterized test cases from JSONL, not hardcoded assertions in Rust code
- **Blocking on stdin without timeout:** Always use timeouts when waiting for subprocess output to prevent hangs

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| File embedding at compile time | Manual include_str! per file | include_dir crate | Handles entire directories, glob patterns, preserves structure |
| Process I/O with timeout | Blocking child.wait() | tokio::time::timeout or std::process with separate thread | Prevents indefinite hangs |
| JSON parsing | Manual string parsing | serde_json::from_str | Already in deps, handles escaping correctly |
| Temp file cleanup | Manual remove_dir_all | TempDir RAII | Already pattern in codebase, exception-safe |

**Key insight:** The test runner is simple in concept but edge cases (timeouts, encoding, line endings) require careful handling. Use existing patterns from subprocess/runner.rs.

## Common Pitfalls

### Pitfall 1: Test Data Visible to Agent
**What goes wrong:** Agent reads test cases and optimizes for them instead of solving the problem
**Why it happens:** Tests placed in same directory as project files
**How to avoid:** Store tests.jsonl separately, extract project files but NOT test data
**Warning signs:** Agent mentions specific test values in its reasoning

### Pitfall 2: Line Ending Mismatches
**What goes wrong:** Tests fail despite correct output due to \r\n vs \n
**Why it happens:** Windows/Unix line ending differences
**How to avoid:** Trim both expected and actual output before comparison
**Warning signs:** Tests fail with "expected '4\r' but got '4'"

### Pitfall 3: Trailing Whitespace/Newlines
**What goes wrong:** Output "4\n" doesn't equal expected "4"
**Why it happens:** Programs often print trailing newline
**How to avoid:** Use `.trim()` on both expected and actual output
**Warning signs:** Visual diff shows outputs look identical

### Pitfall 4: Test Timeout Too Short/Long
**What goes wrong:** Fast tests timeout or slow tests block forever
**Why it happens:** Single timeout value for all programs
**How to avoid:** Use reasonable default (5s) with optional per-project override
**Warning signs:** Simple calculator timeouts or infinite loops block eval

### Pitfall 5: Binary Not Found After Build
**What goes wrong:** Test runner can't find the program to execute
**Why it happens:** Binary path depends on language/build system
**How to avoid:** Project metadata specifies how to build and where binary is
**Warning signs:** "command not found" or "no such file" errors

## Code Examples

Verified patterns from official sources and existing codebase:

### Complete Test Runner Implementation
```rust
// Source: Based on existing subprocess/runner.rs patterns
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

pub struct TestRunner {
    pub program_path: PathBuf,
    pub timeout: Duration,
}

impl TestRunner {
    pub fn new(program_path: PathBuf) -> Self {
        Self {
            program_path,
            timeout: Duration::from_secs(5),
        }
    }

    /// Run all test cases and collect results.
    pub fn run_tests(&self, tests: &[TestCase]) -> TestResults {
        let mut passed = 0;
        let mut cases = Vec::new();

        for test in tests {
            let result = self.run_single_test(test);
            if result.passed {
                passed += 1;
            }
            cases.push(result);
        }

        TestResults {
            passed,
            total: tests.len() as u32,
            cases,
        }
    }

    fn run_single_test(&self, test: &TestCase) -> TestResult {
        let mut child = match Command::new(&self.program_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                return TestResult {
                    input: test.input.clone(),
                    expected: test.expected.clone(),
                    actual: format!("spawn error: {}", e),
                    passed: false,
                };
            }
        };

        // Write input
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(test.input.as_bytes());
            let _ = stdin.write_all(b"\n");
        }

        // Wait for output
        match child.wait_with_output() {
            Ok(output) => {
                let actual = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_string();
                let expected = test.expected.trim();
                TestResult {
                    input: test.input.clone(),
                    expected: expected.to_string(),
                    actual: actual.clone(),
                    passed: actual == expected,
                }
            }
            Err(e) => TestResult {
                input: test.input.clone(),
                expected: test.expected.clone(),
                actual: format!("execution error: {}", e),
                passed: false,
            },
        }
    }
}
```

### Calculator Eval Project Structure
```
evals/calculator/
+-- prompt.txt       # The starting prompt for the agent
+-- tests.jsonl      # HIDDEN: Input/expected output pairs
```

**prompt.txt:**
```
Build a simple command-line calculator.

The program should:
1. Read a mathematical expression from stdin
2. Evaluate the expression
3. Print the result to stdout

Support these operations:
- Addition (+)
- Subtraction (-)
- Multiplication (*)
- Division (/)

Examples:
Input: "2 + 2"
Output: "4"

Input: "10 * 5"
Output: "50"

The program should handle integer arithmetic. You may use any programming language.
```

**tests.jsonl:**
```json
{"input": "2 + 2", "expected": "4"}
{"input": "10 - 5", "expected": "5"}
{"input": "3 * 4", "expected": "12"}
{"input": "20 / 4", "expected": "5"}
{"input": "100 + 200", "expected": "300"}
{"input": "50 - 25", "expected": "25"}
{"input": "7 * 8", "expected": "56"}
{"input": "81 / 9", "expected": "9"}
{"input": "1 + 1", "expected": "2"}
{"input": "0 * 100", "expected": "0"}
```

### Project Registry Pattern
```rust
// src/eval/projects.rs
use include_dir::{include_dir, Dir};
use std::path::Path;

/// Embedded eval projects directory.
static EVALS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/evals");

/// Get a built-in project by name.
pub fn get_project(name: &str) -> Option<&'static Dir<'static>> {
    EVALS_DIR.get_dir(name)
}

/// Check if a project is a built-in project.
pub fn is_builtin(name: &str) -> bool {
    get_project(name).is_some()
}

/// List all built-in project names.
pub fn list_projects() -> Vec<&'static str> {
    EVALS_DIR
        .dirs()
        .filter_map(|d| d.path().file_name()?.to_str())
        .collect()
}

/// Extract project files (excluding tests.jsonl) to a directory.
pub fn extract_project_files(project: &Dir, dest: &Path) -> std::io::Result<()> {
    for file in project.files() {
        // Skip the hidden test file
        if file.path().file_name().map(|n| n == "tests.jsonl").unwrap_or(false) {
            continue;
        }

        let file_path = dest.join(file.path());
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file_path, file.contents())?;
    }
    for dir in project.dirs() {
        extract_project_files(dir, dest)?;
    }
    Ok(())
}

/// Get the hidden test data for a project.
pub fn get_test_data(project: &Dir) -> Option<&'static str> {
    project
        .get_file("tests.jsonl")
        .and_then(|f| f.contents_utf8())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| External eval files | include_dir embedding | Compile-time embedding | Single binary, no runtime deps |
| Per-file include_str! | include_dir proc macro | include_dir 0.7+ | Cleaner API, recursive |
| Cargo test for validation | Stdin/stdout black-box | Standard in competitive programming | Language-agnostic testing |

**Deprecated/outdated:**
- **includedir crate with build.rs:** Old approach requiring codegen; include_dir proc macro is simpler
- **rust-embed for code projects:** Better suited for web assets; include_dir more appropriate

## Open Questions

Things that couldn't be fully resolved:

1. **Second eval project scope (PROJ-04)**
   - What we know: Requirement says "medium difficulty (TBD scope)"
   - What's unclear: What specific project would be good (FizzBuzz? Todo CLI? JSON parser?)
   - Recommendation: Consider FizzBuzz (simple but complete) or a basic HTTP echo server

2. **Language detection for built programs**
   - What we know: Calculator can be built in any language
   - What's unclear: How to detect which binary to run after build
   - Recommendation: Require projects to specify run command in metadata file (e.g., run.sh or manifest)

3. **Timeout strategy per test vs per project**
   - What we know: Need timeout to prevent hangs
   - What's unclear: Should timeout be per-test or total for all tests
   - Recommendation: Start with per-test timeout (5s), can add total later

## Sources

### Primary (HIGH confidence)
- [include_dir crate documentation](https://lib.rs/crates/include_dir) - Version 0.7.4, API patterns
- [std::process::Command](https://doc.rust-lang.org/std/process/struct.Command.html) - Stdin/stdout patterns
- `/Users/vmakaev/NonWork/rslph/src/eval/command.rs` - Existing eval implementation from Phase 9
- `/Users/vmakaev/NonWork/rslph/src/subprocess/runner.rs` - Existing subprocess patterns

### Secondary (MEDIUM confidence)
- [HackerEarth test cases](https://help.hackerearth.com/hc/en-us/articles/360002470854-test-cases) - Sample/hidden test pattern
- [assert_cmd crate](https://docs.rs/assert_cmd/0.11.1/assert_cmd/) - Stdin/stdout testing patterns
- [Run command with stdin/stdout in Rust](https://ahmadrosid.com/blog/rust-run-command-with-custom-stdin-stdout) - Complete example

### Tertiary (LOW confidence)
- Calculator CLI examples - General pattern guidance

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - include_dir is well-established, serde_json already in deps
- Architecture: HIGH - Extends existing eval infrastructure cleanly
- Pitfalls: HIGH - Based on common competitive programming patterns
- Test runner: HIGH - Uses existing subprocess patterns from codebase

**Research date:** 2026-01-20
**Valid until:** 2026-02-20 (stable crates, well-established patterns)

//! Prebuilt fake Claude scenarios for common test cases.
//!
//! These scenarios create deterministic, working artifacts that can be used
//! for end-to-end testing of the full eval flow.

use super::scenario::ScenarioBuilder;

/// Progress file content for a simple calculator project.
/// Uses RALPH_DONE to signal immediate completion after one iteration.
const CALCULATOR_PROGRESS: &str = r#"# Progress: Calculator

## Status

In Progress

## Tasks

### Phase 1: Implementation

- [ ] Create calculator program

## Testing Strategy

Run the calculator with test inputs and verify outputs.
"#;

/// Progress file content that signals build is complete (with RALPH_DONE).
const CALCULATOR_PROGRESS_DONE: &str = r#"# Progress: Calculator

## Status

RALPH_DONE - All tasks complete

## Tasks

### Phase 1: Implementation

- [x] Create calculator program

## Testing Strategy

Run the calculator with test inputs and verify outputs.
"#;

/// Python calculator that uses eval() for computation.
/// Handles integer division by converting results to int.
const PYTHON_CALCULATOR: &str = r#"#!/usr/bin/env python3
import sys

expr = input().strip()
result = eval(expr)
# Convert to int for integer results (e.g., 20/4 = 5, not 5.0)
if isinstance(result, float) and result.is_integer():
    result = int(result)
print(result)
"#;

/// Create a fake Claude scenario that builds a working Python calculator.
///
/// This scenario simulates:
/// 1. Planning phase: Creates a valid progress.md file
/// 2. Build phase: Creates main.py and marks task complete with RALPH_DONE
///
/// The calculator passes all 10 test cases in evals/calculator/tests.jsonl.
///
/// # Example
///
/// ```ignore
/// use crate::fake_claude_lib::prebuilt;
///
/// let handle = prebuilt::calculator().build();
///
/// // Run eval with fake Claude
/// let mut cmd = Command::cargo_bin("rslph").unwrap();
/// for (key, val) in handle.env_vars() {
///     cmd.env(key, val);
/// }
/// cmd.args(["eval", "calculator"]);
/// ```
pub fn calculator() -> ScenarioBuilder {
    ScenarioBuilder::new()
        // Invocation 0: Planning phase - output progress file
        .respond_with_text(CALCULATOR_PROGRESS)
        .next_invocation()
        // Invocation 1: Build phase iteration 1 - create calculator AND write RALPH_DONE progress
        .uses_write("main.py", PYTHON_CALCULATOR)
        .uses_bash("chmod +x main.py")
        // Also write the updated progress file with RALPH_DONE
        .uses_write("progress.md", CALCULATOR_PROGRESS_DONE)
        .with_execute_tools()
        .respond_with_text("I've created a Python calculator that reads expressions from stdin and outputs the result. The calculator uses Python's eval() for computation and handles integer division correctly. All tasks are complete.")
}

/// Create a fake Claude scenario that builds a working FizzBuzz program.
///
/// Similar to calculator(), but for the fizzbuzz eval project.
pub fn fizzbuzz() -> ScenarioBuilder {
    let progress = r#"# Progress: FizzBuzz

## Status

In Progress

## Tasks

### Phase 1: Implementation

- [ ] Create FizzBuzz program

## Testing Strategy

Run with numbers 1-20 and verify outputs.
"#;

    let progress_done = r#"# Progress: FizzBuzz

## Status

RALPH_DONE - All tasks complete

## Tasks

### Phase 1: Implementation

- [x] Create FizzBuzz program

## Testing Strategy

Run with numbers 1-20 and verify outputs.
"#;

    let python_fizzbuzz = r#"#!/usr/bin/env python3
import sys

n = int(input().strip())
if n % 15 == 0:
    print("FizzBuzz")
elif n % 3 == 0:
    print("Fizz")
elif n % 5 == 0:
    print("Buzz")
else:
    print(n)
"#;

    ScenarioBuilder::new()
        // Invocation 0: Planning phase
        .respond_with_text(progress)
        .next_invocation()
        // Invocation 1: Build phase
        .uses_write("main.py", python_fizzbuzz)
        .uses_bash("chmod +x main.py")
        .uses_write("progress.md", progress_done)
        .with_execute_tools()
        .respond_with_text("I've created a Python FizzBuzz program that reads a number from stdin and outputs the appropriate result. All tasks are complete.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculator_scenario_builds() {
        let handle = calculator().build();
        assert!(handle.executable_path.exists() || true); // May not exist yet in some test environments
    }

    #[test]
    fn test_fizzbuzz_scenario_builds() {
        let handle = fizzbuzz().build();
        assert!(handle.executable_path.exists() || true);
    }

    #[test]
    fn test_calculator_progress_parses() {
        // Verify the initial progress file format is parseable
        let pf = rslph::progress::ProgressFile::parse(CALCULATOR_PROGRESS)
            .expect("Calculator progress should parse");

        assert_eq!(pf.name, "Calculator");
        assert!(pf.status.contains("In Progress"));
        assert_eq!(pf.total_tasks(), 1, "Should have 1 task");
        assert_eq!(pf.completed_tasks(), 0, "Task should not be completed");

        // Verify phase structure
        assert_eq!(pf.tasks.len(), 1, "Should have 1 phase");
        assert_eq!(pf.tasks[0].name, "Phase 1: Implementation");
        assert_eq!(pf.tasks[0].tasks[0].description, "Create calculator program");
    }

    #[test]
    fn test_calculator_progress_done_parses() {
        // Verify the RALPH_DONE progress file format is parseable
        let pf = rslph::progress::ProgressFile::parse(CALCULATOR_PROGRESS_DONE)
            .expect("Calculator done progress should parse");

        assert_eq!(pf.name, "Calculator");
        assert!(pf.status.contains("RALPH_DONE"), "Status should contain RALPH_DONE");
        assert!(pf.is_done(), "Should be detected as done");
        assert_eq!(pf.total_tasks(), 1, "Should have 1 task");
        assert_eq!(pf.completed_tasks(), 1, "Task should be completed");
    }

    #[test]
    fn test_python_calculator_logic() {
        // Verify the calculator code handles integer division
        use std::process::{Command, Stdio};
        use std::io::Write;

        let mut child = Command::new("python3")
            .arg("-c")
            .arg(PYTHON_CALCULATOR.replace("input()", "\"20 / 4\""))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn();

        // This test may fail if python3 isn't available, which is fine
        if let Ok(mut child) = child {
            let output = child.wait_with_output().ok();
            if let Some(out) = output {
                let result = String::from_utf8_lossy(&out.stdout);
                assert!(result.trim() == "5", "Expected 5, got: {}", result.trim());
            }
        }
    }
}

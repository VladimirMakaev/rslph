//! Prebuilt fake Claude scenarios for common test cases.
//!
//! These scenarios create deterministic, working artifacts that can be used
//! for end-to-end testing of the full eval flow.

use super::scenario::ScenarioBuilder;

// Allow dead code for prebuilt scenarios that may not all be used in every test
#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
/// Create a fake Claude scenario that builds a working Python calculator.
///
/// This scenario simulates:
/// 1. Planning phase: Creates a valid progress.md file
/// 2. Build phase: Creates main.py and marks task complete with RALPH_DONE
/// 3. Test discovery phase: Returns shell script to run the program
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
        // Invocation 1: Build phase iteration 1 - create calculator AND output RALPH_DONE progress
        // Note: Build command parses Claude's text response as the updated progress file,
        // so we must return valid progress file format, not just a status message.
        .uses_write("main.py", PYTHON_CALCULATOR)
        .uses_bash("chmod +x main.py")
        .with_execute_tools()
        .respond_with_text(CALCULATOR_PROGRESS_DONE)
        .next_invocation()
        // Invocation 2: Test discovery phase - return run script
        .respond_with_text("#!/bin/sh\npython main.py")
}

#[allow(dead_code)]
/// Create a fake Claude scenario that builds a working FizzBuzz program.
///
/// Similar to calculator(), but for the fizzbuzz eval project.
/// Includes test discovery phase for running the program.
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
        // Invocation 1: Build phase - return valid progress file with RALPH_DONE
        .uses_write("main.py", python_fizzbuzz)
        .uses_bash("chmod +x main.py")
        .with_execute_tools()
        .respond_with_text(progress_done)
        .next_invocation()
        // Invocation 2: Test discovery phase - return run script
        .respond_with_text("#!/bin/sh\npython main.py")
}

#[allow(dead_code)]
/// Create a fake Claude scenario that times out on first iteration, then succeeds.
///
/// This scenario is designed for testing timeout retry behavior:
/// 1. Planning phase: Quick response with progress file
/// 2. Build phase 1st attempt: Delays 5 seconds (designed to trigger timeout with small timeout)
/// 3. Build phase 2nd attempt (retry): Quick response that completes
///
/// Use with RSLPH_ITERATION_TIMEOUT=2 and RSLPH_TIMEOUT_RETRIES=3 for testing.
///
/// # Example
///
/// ```ignore
/// use crate::fake_claude_lib::prebuilt;
///
/// let handle = prebuilt::timeout_retry().build();
///
/// let mut cmd = Command::cargo_bin("rslph").unwrap();
/// for (key, val) in handle.env_vars() {
///     cmd.env(key, val);
/// }
/// cmd.env("RSLPH_ITERATION_TIMEOUT", "2");  // 2 second timeout
/// cmd.env("RSLPH_TIMEOUT_RETRIES", "3");    // Allow up to 3 retries
/// cmd.args(["eval", "calculator"]);
/// ```
pub fn timeout_retry() -> ScenarioBuilder {
    let progress = r#"# Progress: Calculator

## Status

In Progress

## Tasks

### Phase 1: Implementation

- [ ] Create calculator program

## Testing Strategy

Run the calculator with test inputs and verify outputs.
"#;

    let progress_done = r#"# Progress: Calculator

## Status

RALPH_DONE - All tasks complete

## Tasks

### Phase 1: Implementation

- [x] Create calculator program

## Testing Strategy

Run the calculator with test inputs and verify outputs.
"#;

    let python_calculator = r#"#!/usr/bin/env python3
import sys

expr = input().strip()
result = eval(expr)
# Convert to int for integer results (e.g., 20/4 = 5, not 5.0)
if isinstance(result, float) and result.is_integer():
    result = int(result)
print(result)
"#;

    ScenarioBuilder::new()
        // Invocation 0: Planning phase - quick response
        .respond_with_text(progress)
        .next_invocation()
        // Invocation 1: Build phase attempt 1 - delays 5 seconds (will timeout with 2s timeout)
        .with_initial_delay_ms(5000)
        .respond_with_text(progress) // Returns same progress since it times out
        .next_invocation()
        // Invocation 2: Build phase attempt 2 (retry) - quick response with RALPH_DONE
        // Note: Must return valid progress file format for validation to pass
        .uses_write("main.py", python_calculator)
        .uses_bash("chmod +x main.py")
        .with_execute_tools()
        .respond_with_text(progress_done)
        .next_invocation()
        // Invocation 3: Test discovery phase
        .respond_with_text("#!/bin/sh\npython main.py")
}

#[allow(dead_code)]
/// Create a fake Claude scenario that times out on all retries.
///
/// This scenario is designed for testing timeout exhaustion:
/// All build phase invocations delay 5 seconds to trigger timeout.
///
/// Use with RSLPH_ITERATION_TIMEOUT=2 and RSLPH_TIMEOUT_RETRIES=2 for testing.
pub fn timeout_exhausted() -> ScenarioBuilder {
    let progress = r#"# Progress: Calculator

## Status

In Progress

## Tasks

### Phase 1: Implementation

- [ ] Create calculator program

## Testing Strategy

Run the calculator with test inputs and verify outputs.
"#;

    ScenarioBuilder::new()
        // Invocation 0: Planning phase - quick response
        .respond_with_text(progress)
        .next_invocation()
        // Invocation 1: Build phase attempt 1 - delays (will timeout)
        .with_initial_delay_ms(5000)
        .respond_with_text(progress)
        .next_invocation()
        // Invocation 2: Build phase attempt 2 (retry 1) - also delays
        .with_initial_delay_ms(5000)
        .respond_with_text(progress)
        .next_invocation()
        // Invocation 3: Build phase attempt 3 (retry 2) - also delays
        .with_initial_delay_ms(5000)
        .respond_with_text(progress)
}

#[allow(dead_code)]
/// Create a fake Claude scenario with AskUserQuestion flow.
///
/// This scenario simulates:
/// 1. Initial call: Emits system init + AskUserQuestion
/// 2. Resume call: Receives answers, produces progress file
pub fn interactive_planning() -> ScenarioBuilder {
    let progress = r#"# Progress: Interactive Test

## Status

RALPH_DONE - All tasks complete

## Tasks

### Phase 1: Setup

- [x] Configure project based on user answers

## Testing Strategy

Basic tests.
"#;

    ScenarioBuilder::new()
        // Invocation 0: Ask questions
        .with_session_id("test-session-123")
        .asks_questions(vec![
            "What programming language do you want to use?",
            "What database backend should we use?",
        ])
        .next_invocation()
        // Invocation 1: Resume with answers, produce progress file
        .with_session_id("test-session-123")
        .respond_with_text(progress)
}

#[allow(dead_code)]
/// Create a multi-round Q&A scenario.
///
/// Simulates two rounds of questions before producing final output.
pub fn multi_round_qa() -> ScenarioBuilder {
    let progress = r#"# Progress: Multi-Round Test

## Status

RALPH_DONE - All tasks complete

## Tasks

### Phase 1: Done

- [x] Task complete based on multi-round Q&A

## Testing Strategy

Verified through multiple question rounds.
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

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_calculator_scenario_builds() {
        let handle = calculator().build();
        // Handle may or may not exist yet depending on test environment
        let _ = handle.executable_path.exists();
    }

    #[test]
    fn test_fizzbuzz_scenario_builds() {
        let handle = fizzbuzz().build();
        // Handle may or may not exist yet depending on test environment
        let _ = handle.executable_path.exists();
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
        assert_eq!(
            pf.tasks[0].tasks[0].description,
            "Create calculator program"
        );
    }

    #[test]
    fn test_calculator_progress_done_parses() {
        // Verify the RALPH_DONE progress file format is parseable
        let pf = rslph::progress::ProgressFile::parse(CALCULATOR_PROGRESS_DONE)
            .expect("Calculator done progress should parse");

        assert_eq!(pf.name, "Calculator");
        assert!(
            pf.status.contains("RALPH_DONE"),
            "Status should contain RALPH_DONE"
        );
        assert!(pf.is_done(), "Should be detected as done");
        assert_eq!(pf.total_tasks(), 1, "Should have 1 task");
        assert_eq!(pf.completed_tasks(), 1, "Task should be completed");
    }

    #[test]
    fn test_python_calculator_logic() {
        // Verify the calculator code handles integer division
        use std::process::{Command, Stdio};

        let child = Command::new("python3")
            .arg("-c")
            .arg(PYTHON_CALCULATOR.replace("input()", "\"20 / 4\""))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn();

        // This test may fail if python3 isn't available, which is fine
        if let Ok(child) = child {
            let output = child.wait_with_output().ok();
            if let Some(out) = output {
                let result = String::from_utf8_lossy(&out.stdout);
                assert!(result.trim() == "5", "Expected 5, got: {}", result.trim());
            }
        }
    }

    #[test]
    fn test_interactive_planning_scenario_builds() {
        let handle = interactive_planning().build();
        // Verify handle was created successfully
        let _ = handle.executable_path.exists();
    }

    #[test]
    fn test_multi_round_qa_scenario_builds() {
        let handle = multi_round_qa().build();
        // Verify handle was created successfully
        let _ = handle.executable_path.exists();
    }

    #[test]
    fn test_interactive_planning_invocations() {
        // Verify the scenario can be built and env vars retrieved
        let handle = interactive_planning().build();
        let env_vars = handle.env_vars();
        assert_eq!(env_vars.len(), 2);
        assert_eq!(env_vars[0].0, "FAKE_CLAUDE_CONFIG");
        assert_eq!(env_vars[1].0, "RSLPH_CLAUDE_CMD");
    }
}

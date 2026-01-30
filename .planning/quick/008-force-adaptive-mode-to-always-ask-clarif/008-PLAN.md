---
phase: quick-008
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/planning/personas.rs
  - src/planning/command.rs
autonomous: true

must_haves:
  truths:
    - "Adaptive mode always asks clarifying questions when vagueness threshold is met"
    - "There is no escape hatch that skips user interaction"
  artifacts:
    - path: "src/planning/personas.rs"
      provides: "Requirements clarifier persona without REQUIREMENTS_CLEAR escape"
    - path: "src/planning/command.rs"
      provides: "Adaptive planning that always presents questions to user"
  key_links:
    - from: "src/planning/command.rs"
      to: "src/planning/personas.rs"
      via: "REQUIREMENTS_CLARIFIER_PERSONA constant"
      pattern: "REQUIREMENTS_CLARIFIER_PERSONA"
---

<objective>
Force adaptive mode to always ask clarifying questions when vagueness is detected.

Purpose: Users expect adaptive mode to engage them in requirements gathering. Currently, Claude can skip this by returning "REQUIREMENTS_CLEAR" for well-known concepts like "todo app".

Output: Modified persona prompt and command handler that always present questions to the user when vagueness triggers clarification.
</objective>

<context>
@.planning/STATE.md
@src/planning/personas.rs
@src/planning/command.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Remove REQUIREMENTS_CLEAR escape from persona</name>
  <files>src/planning/personas.rs</files>
  <action>
    In REQUIREMENTS_CLARIFIER_PERSONA constant (lines 9-32):

    1. Remove the "If requirements are clear enough, respond with: REQUIREMENTS_CLEAR" instruction from the Output Format section (lines 23-24)

    2. Update the Output Format section to only describe the numbered questions format:
       ```
       ## Output Format

       Output numbered questions:
       1. [First question about unclear aspect]
       2. [Second question about unclear aspect]
       ...

       Keep questions focused and actionable. Maximum 5 questions.
       ```

    The persona should ALWAYS generate questions - there is no escape hatch.
  </action>
  <verify>
    Run: `grep -n "REQUIREMENTS_CLEAR" src/planning/personas.rs`
    Expected: No matches (exit code 1)
  </verify>
  <done>REQUIREMENTS_CLARIFIER_PERSONA no longer mentions REQUIREMENTS_CLEAR</done>
</task>

<task type="auto">
  <name>Task 2: Remove REQUIREMENTS_CLEAR check from command handler</name>
  <files>src/planning/command.rs</files>
  <action>
    In run_adaptive_planning function (around line 443):

    1. Find the condition `if !questions.contains("REQUIREMENTS_CLEAR")`

    2. Remove the entire if/else block that checks for REQUIREMENTS_CLEAR:
       - Remove the `if !questions.contains("REQUIREMENTS_CLEAR") { ... } else { ... }` structure
       - Keep ONLY the body of the if-branch (print questions, get input, store clarifications)
       - Remove the else-branch that prints "Requirements are clear enough, skipping clarification"

    The resulting code should unconditionally:
    - Print the questions from Claude
    - Prompt user for answers
    - Store clarifications

    Before (lines 443-454):
    ```rust
    if !questions.contains("REQUIREMENTS_CLEAR") {
        // Print questions and get user input
        println!("Clarifying Questions:\n");
        println!("{}", questions);
        println!("\nPlease answer the questions above (type your answers, then Enter twice to submit):\n");
        clarifications = read_multiline_input()?;
        println!("\nGathered clarifications. Continuing...\n");
    } else {
        println!("Requirements are clear enough, skipping clarification.\n");
    }
    ```

    After:
    ```rust
    // Print questions and get user input
    println!("Clarifying Questions:\n");
    println!("{}", questions);
    println!("\nPlease answer the questions above (type your answers, then Enter twice to submit):\n");
    clarifications = read_multiline_input()?;
    println!("\nGathered clarifications. Continuing...\n");
    ```
  </action>
  <verify>
    Run: `grep -n "REQUIREMENTS_CLEAR" src/planning/command.rs`
    Expected: No matches (exit code 1)

    Run: `cargo check`
    Expected: Compiles without errors
  </verify>
  <done>Adaptive planning always asks for clarifications when vagueness threshold is met</done>
</task>

</tasks>

<verification>
1. `cargo check` - compiles without errors
2. `cargo test` - all tests pass
3. `grep -r "REQUIREMENTS_CLEAR" src/` - returns no matches
</verification>

<success_criteria>
- REQUIREMENTS_CLEAR string removed from both personas.rs and command.rs
- Adaptive mode always presents questions and waits for user input when vagueness is detected
- All existing tests pass
</success_criteria>

<output>
After completion, create `.planning/quick/008-force-adaptive-mode-to-always-ask-clarif/008-SUMMARY.md`
</output>

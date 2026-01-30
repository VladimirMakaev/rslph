//! Unit tests for the fake Claude scenario builder.
//!
//! These tests verify that the scenario builder correctly generates
//! configuration files for the fake Claude binary.

use crate::fake_claude_lib::{FakeClaudeConfig, ScenarioBuilder};
use serde_json::json;

#[test]
fn test_scenario_with_text_response() {
    let handle = ScenarioBuilder::new()
        .respond_with_text("Hello, world!")
        .build();

    assert!(handle.config_path.exists());
    let config: FakeClaudeConfig =
        serde_json::from_str(&std::fs::read_to_string(&handle.config_path).unwrap()).unwrap();

    assert_eq!(config.invocations.len(), 1);
    // respond_with_text adds system_init, assistant_text, and result events
    assert_eq!(config.invocations[0].events.len(), 3);
}

#[test]
fn test_scenario_with_tool_calls() {
    let handle = ScenarioBuilder::new()
        .uses_read("/path/to/file")
        .uses_write("/output", "content")
        .uses_edit("/edit", "old", "new")
        .uses_bash("echo hello")
        .build();

    let config: FakeClaudeConfig =
        serde_json::from_str(&std::fs::read_to_string(&handle.config_path).unwrap()).unwrap();

    assert_eq!(config.invocations[0].events.len(), 4);
}

#[test]
fn test_multi_invocation() {
    let handle = ScenarioBuilder::new()
        .respond_with_text("First response")
        .next_invocation()
        .respond_with_text("Second response")
        .next_invocation()
        .respond_with_text("Third response")
        .build();

    let config: FakeClaudeConfig =
        serde_json::from_str(&std::fs::read_to_string(&handle.config_path).unwrap()).unwrap();

    assert_eq!(config.invocations.len(), 3);
}

#[test]
fn test_edge_case_delay() {
    let handle = ScenarioBuilder::new()
        .respond_with_text("Slow response")
        .with_delay(100)
        .build();

    let config: FakeClaudeConfig =
        serde_json::from_str(&std::fs::read_to_string(&handle.config_path).unwrap()).unwrap();

    assert_eq!(config.invocations[0].delay_ms, Some(100));
}

#[test]
fn test_edge_case_crash() {
    let handle = ScenarioBuilder::new()
        .respond_with_text("Will crash")
        .crash_after(1)
        .build();

    let config: FakeClaudeConfig =
        serde_json::from_str(&std::fs::read_to_string(&handle.config_path).unwrap()).unwrap();

    assert_eq!(config.invocations[0].crash_after_events, Some(1));
}

#[test]
fn test_raw_output() {
    let handle = ScenarioBuilder::new()
        .send_raw("not valid json at all")
        .build();

    let config: FakeClaudeConfig =
        serde_json::from_str(&std::fs::read_to_string(&handle.config_path).unwrap()).unwrap();

    assert_eq!(config.invocations[0].raw_lines.len(), 1);
    assert_eq!(config.invocations[0].raw_lines[0], "not valid json at all");
}

#[test]
fn test_invocation_counter() {
    let handle = ScenarioBuilder::new()
        .respond_with_text("First")
        .next_invocation()
        .respond_with_text("Second")
        .build();

    // Initially 0
    assert_eq!(handle.invocation_count(), 0);

    // After simulating what binary does (incrementing counter)
    std::fs::write(
        handle
            .config_path
            .parent()
            .unwrap()
            .join("fake_claude_counter"),
        "1",
    )
    .unwrap();
    assert_eq!(handle.invocation_count(), 1);
}

#[test]
fn test_generic_tool() {
    let handle = ScenarioBuilder::new()
        .uses_tool("CustomTool", json!({"custom_field": "value"}))
        .build();

    let config: FakeClaudeConfig =
        serde_json::from_str(&std::fs::read_to_string(&handle.config_path).unwrap()).unwrap();

    assert_eq!(config.invocations[0].events.len(), 1);
}

#[test]
fn test_exit_code() {
    let handle = ScenarioBuilder::new()
        .respond_with_text("Failed response")
        .with_exit_code(1)
        .build();

    let config: FakeClaudeConfig =
        serde_json::from_str(&std::fs::read_to_string(&handle.config_path).unwrap()).unwrap();

    assert_eq!(config.invocations[0].exit_code, Some(1));
}

---
created: 2026-01-18T00:20
title: Use --output-format stream-json for Claude subprocess
area: subprocess
files:
  - src/subprocess/runner.rs
---

## Problem

When interacting with Claude CLI as a subprocess, we need to use `--output-format stream-json` flag to get structured JSON output that can be properly parsed and processed. Without this flag, the output is plain text which is harder to parse reliably for status updates, completion detection, and context usage tracking.

The current Claude CLI invocations in Phase 3 (planning command) may need to be updated, and Phase 4 (build loop) implementation must use this flag from the start.

## Solution

Update Claude CLI invocations to include `--output-format stream-json` flag. Parse the streaming JSON output to extract:
- Content blocks
- Token usage / context metrics
- Completion status
- Any error information

This will provide reliable structured data for the TUI status displays and loop control logic.

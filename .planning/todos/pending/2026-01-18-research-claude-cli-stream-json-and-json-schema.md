---
created: 2026-01-18T04:04
title: Research Claude CLI stream-json and json-schema flags
area: planning
files:
  - src/planning/command.rs
---

## Problem

Currently using `--output-format stream-json` with `--verbose` for Claude CLI subprocess communication (per decision STREAM-JSON-FORMAT). However, there's also a `--json-schema` attribute that was noticed but not investigated.

Questions to research:
1. Is `--output-format stream-json` being used correctly?
2. What does `--json-schema` do and when should it be used?
3. Are there other output format options that might be more appropriate?
4. What's the relationship between these flags?

This research could reveal a better approach for parsing Claude CLI output.

## Solution

TBD - Requires deep research into Claude CLI documentation and behavior.

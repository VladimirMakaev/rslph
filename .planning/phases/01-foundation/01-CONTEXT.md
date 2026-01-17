# Phase 1: Foundation - Context

**Gathered:** 2026-01-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Core infrastructure for rslph: TOML config loading with layered precedence (file < env < CLI), CLI parser with plan/build subcommands, and progress file parsing with atomic writes. No user-facing features yet — just the building blocks.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

This phase is infrastructure with well-established patterns. Claude has flexibility on:

**Config structure:**
- TOML schema and nesting
- Default values
- Environment variable naming (RSLPH_* prefix assumed)
- Validation approach

**CLI design:**
- Short vs long flag conventions
- Help text formatting
- Argument parsing patterns
- Error message style

**Progress file format:**
- Markdown section structure
- Checkbox syntax
- How to handle malformed input

**General:**
- Error messaging verbosity
- Exit code conventions (0 success, 1 error, 2 user error)
- Logging approach

</decisions>

<specifics>
## Specific Ideas

No specific requirements — apply standard Rust CLI conventions:
- Follow clap ecosystem patterns
- Use figment for layered config (per research)
- Atomic writes via temp file + rename (per research)

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-foundation*
*Context gathered: 2026-01-17*

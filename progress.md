# Progress: Calculator CLI

## Status

In Progress

## Analysis

Building a command-line calculator application in Rust that reads mathematical expressions from stdin and outputs the result to stdout. This will require parsing mathematical expressions (handling operator precedence, parentheses, etc.) and evaluating them. Will implement a simple expression parser using either a recursive descent parser or the shunting-yard algorithm.
Assumptions:
- Support basic arithmetic: +, -, *, /
- Support parentheses for grouping
- Handle floating-point numbers
- Provide meaningful error messages for invalid input

## Tasks

### Phase 1: Project Setup and Testing Infrastructure

- [ ] Initialize Rust project with cargo new
- [ ] Configure Cargo.toml with appropriate metadata
- [ ] Verify cargo test runs successfully with placeholder test
- [ ] Add clippy configuration for linting

### Phase 2: Lexer/Tokenizer

- [ ] Define Token enum (Number, Plus, Minus, Star, Slash, LParen, RParen)
- [ ] Write unit tests for tokenizing simple expressions
- [ ] Implement tokenize function to convert input string to tokens
- [ ] Write unit tests for tokenizing edge cases (whitespace, negative numbers, decimals)
- [ ] Add error handling for invalid characters

### Phase 3: Parser

- [ ] Define AST node types (Number, BinaryOp)
- [ ] Write unit tests for parsing simple expressions
- [ ] Implement recursive descent parser with operator precedence
- [ ] Write unit tests for parsing parenthesized expressions
- [ ] Write unit tests for operator precedence (e.g., 2+3
- [ ] Add parser error handling with descriptive messages

### Phase 4: Evaluator

- [ ] Write unit tests for evaluating simple arithmetic
- [ ] Implement AST evaluation function
- [ ] Write unit tests for division by zero handling
- [ ] Add error handling for runtime errors (division by zero)

### Phase 5: CLI Integration

- [ ] Implement main function to read from stdin
- [ ] Connect lexer, parser, and evaluator in pipeline
- [ ] Write integration tests using command-line invocation
- [ ] Format output appropriately (handle integer vs float display)
- [ ] Add helpful error messages to stderr

## Testing Strategy

- Test framework: cargo test (built-in)
- Unit tests: Written in #[cfg(test)] modules after each component
- Integration tests: In tests/ directory for end-to-end CLI testing
- Type checking: rustc (enforced on every cargo build/test)
- Linting: clippy run before commits, warnings as errors
- Tests are integrated after each feature, not batched at end

## Completed This Iteration


## Recent Attempts

## Iteration Log

| Iteration | Started | Duration | Tasks Completed | Notes |
|-----------|---------|----------|-----------------|-------|

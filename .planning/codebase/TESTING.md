# Testing Patterns

**Analysis Date:** 2026-03-26

## Test Framework

**Runner:**
- `cargo test` (built-in Rust test harness)
- No custom test configuration

**Assertion Library:**
- Standard library `assert!`, `assert_eq!`, `assert_ne!` (Rust built-in)

**Run Commands:**
```bash
cargo test                # Run all tests
cargo test -- --nocapture # Run with stdout visible
```

## Current State

**No tests exist in the codebase.** There are zero `#[test]` functions, zero `#[cfg(test)]` modules, and no test files anywhere in `src/`. The entire 1,748-line codebase is untested.

## Test File Organization

**Recommended Location:**
- Use inline test modules (Rust convention) co-located at the bottom of each source file
- Place integration tests in a top-level `tests/` directory if needed

**Recommended Naming:**
- Inline modules: `#[cfg(test)] mod tests { ... }` at the bottom of each `.rs` file
- Integration test files: `tests/{module_name}.rs`

**Recommended Structure:**
```
src/
├── chat.rs          # Add #[cfg(test)] mod tests at bottom
├── provider.rs      # Add #[cfg(test)] mod tests at bottom
├── text_input.rs    # Add #[cfg(test)] mod tests at bottom
├── theme.rs         # Add #[cfg(test)] mod tests at bottom
tests/
└── integration.rs   # For full-app integration tests (if needed)
```

## Recommended Test Structure

**Unit Test Pattern (inline module):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let input = /* ... */;

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

## Testable Components

**High testability (pure logic, no GPUI dependencies):**

- `src/provider.rs` - `parse_event()` function (line 213): takes a `RawEvent` and returns `Vec<ProviderEvent>`. Fully testable with constructed inputs.
- `src/provider.rs` - `summarize_tool_input()` function (line 151): takes a tool name and JSON value, returns a summary string.
- `src/provider.rs` - `truncate()` function (line 189): simple string truncation utility.
- `src/provider.rs` - `extract_tool_result_text()` function (line 197): extracts text from JSON content values.
- `src/theme.rs` - `hex()` function (line 3): converts hex integer to Rgba.
- `src/chat.rs` - `Turn` methods (lines 43-86): `tool_call_count()`, `append_text()`, `add_tool_call()`, `update_tool_result()`, `is_empty_text()`. All pure data operations.
- `src/text_input.rs` - `previous_boundary()` (line 269), `next_boundary()` (line 277): character boundary navigation. Pure string operations.
- `src/text_input.rs` - `offset_from_utf16()` (line 235), `offset_to_utf16()` (line 248): UTF-16/UTF-8 offset conversion. Pure computation.

**Low testability (requires GPUI context):**

- All `Render` implementations require a GPUI `Window` and `Context`
- `ChatPanel::send_turn()` spawns background tasks and requires a running GPUI event loop
- `TextInput` action handlers need a `Window` and `Context`

## Mocking

**Framework:** None configured.

**Recommended Approach for Provider Tests:**
- The `run_turn()` function in `src/provider.rs` spawns a real `claude` subprocess. To test the streaming logic without the CLI, test `parse_event()` directly with crafted `RawEvent` structs.
- For integration testing of `ChatPanel`, consider extracting the provider behind a trait to allow mock implementations.

**Example test for `parse_event`:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_text_delta_event() {
        let event = RawEvent {
            event_type: "content_block_delta".to_string(),
            message: None,
            content: Some("Hello world".to_string()),
            error: None,
        };

        let results = parse_event(event);
        assert_eq!(results.len(), 1);
        match &results[0] {
            ProviderEvent::TextDelta(text) => assert_eq!(text, "Hello world"),
            _ => panic!("Expected TextDelta"),
        }
    }

    #[test]
    fn parse_error_event() {
        let event = RawEvent {
            event_type: "error".to_string(),
            message: None,
            content: None,
            error: Some("Rate limited".to_string()),
        };

        let results = parse_event(event);
        assert_eq!(results.len(), 1);
        match &results[0] {
            ProviderEvent::Error(msg) => assert_eq!(msg, "Rate limited"),
            _ => panic!("Expected Error"),
        }
    }
}
```

**Example test for `Turn` methods:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_text_merges_consecutive() {
        let mut turn = Turn {
            role: MessageRole::Assistant,
            blocks: vec![ContentBlock::Text("Hello".to_string())],
        };
        turn.append_text(" world");
        assert_eq!(turn.blocks.len(), 1);
        match &turn.blocks[0] {
            ContentBlock::Text(t) => assert_eq!(t, "Hello world"),
            _ => panic!("Expected Text block"),
        }
    }

    #[test]
    fn tool_call_count_filters_correctly() {
        let turn = Turn {
            role: MessageRole::Assistant,
            blocks: vec![
                ContentBlock::Text("thinking...".to_string()),
                ContentBlock::ToolCall {
                    id: "1".into(),
                    name: "Bash".into(),
                    input: "ls".into(),
                    output: String::new(),
                    is_error: false,
                },
                ContentBlock::Text("done".to_string()),
            ],
        };
        assert_eq!(turn.tool_call_count(), 1);
    }
}
```

## Fixtures and Factories

**Test Data:**
- No fixtures or factories exist
- For `parse_event` tests, construct `RawEvent` structs directly
- For `Turn` tests, construct `Turn` and `ContentBlock` values inline

**Recommended Location:**
- Keep test data inline within test functions for small values
- If shared test data grows, create a `src/test_helpers.rs` module behind `#[cfg(test)]`

## Coverage

**Requirements:** None enforced.

**Current Coverage:** 0% (no tests exist).

**View Coverage:**
```bash
cargo install cargo-tarpaulin    # Install coverage tool (one-time)
cargo tarpaulin --out html       # Generate coverage report
```

## Test Types

**Unit Tests:**
- Not yet implemented
- Priority targets: `src/provider.rs` (event parsing), `src/chat.rs` (Turn data model), `src/text_input.rs` (cursor/selection logic)

**Integration Tests:**
- Not yet implemented
- Would require GPUI test harness for UI component testing
- The `provider::run_turn()` function would need the `claude` CLI installed for integration tests

**E2E Tests:**
- Not used
- GPUI does not have a standard E2E test framework for desktop apps

## Priority Test Targets

1. **`src/provider.rs` - `parse_event()`**: Most critical pure function. Parses all streaming events from the Claude CLI. Incorrect parsing silently drops messages.

2. **`src/provider.rs` - `summarize_tool_input()`**: Summarizes tool call inputs for display. Easy to test, high value for correctness.

3. **`src/chat.rs` - `Turn` methods**: `append_text`, `add_tool_call`, `update_tool_result`, `is_empty_text`, `tool_call_count`. All pure data manipulation.

4. **`src/text_input.rs` - boundary functions**: `previous_boundary`, `next_boundary`, `offset_from_utf16`, `offset_to_utf16`. Critical for text editing correctness, especially with multi-byte characters.

5. **`src/theme.rs` - `hex()`**: Trivial to test, validates color conversion math.

---

*Testing analysis: 2026-03-26*

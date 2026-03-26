# Coding Conventions

**Analysis Date:** 2026-03-26

## Naming Patterns

**Files:**
- Use `snake_case.rs` for all source files: `chat.rs`, `text_input.rs`, `provider.rs`
- One primary struct/component per file, named after the file in PascalCase
- Module file names match their conceptual purpose: `theme.rs` for colors, `provider.rs` for the Claude CLI integration

**Structs:**
- PascalCase: `ChatPanel`, `HelmWorkspace`, `PlanPanel`, `TextInput`, `TextElement`
- Suffix panels with `Panel`: `ChatPanel`, `PlanPanel`
- Use `Sidebar` (no suffix) for the navigation sidebar

**Enums:**
- PascalCase for type name and variants: `MessageRole::User`, `ContentBlock::Text`, `ProviderEvent::TextDelta`
- Use descriptive variant names that read naturally: `ProviderEvent::ToolUse`, `ProviderEvent::Complete`

**Functions:**
- `snake_case` for all functions and methods: `send_turn`, `push_turn`, `focus_input`, `run_turn`
- Prefix event handlers with `on_`: `on_input_event`, `on_mouse_down`, `on_mouse_up`, `on_mouse_move`
- Prefix render helpers with `render_`: `render_turn`, `render_tool_call`
- Use verb-noun pattern: `bind_text_input_keys`, `toggle_tool_expanded`, `handle_provider_event`

**Constants:**
- `SCREAMING_SNAKE_CASE` for theme constants: `BG_BASE`, `TEXT_PRIMARY`, `ACCENT`
- Group with prefix by category: `BG_*` for backgrounds, `TEXT_*` for text colors, `BORDER_*` for borders

**Variables:**
- `snake_case` for all locals and fields: `list_state`, `is_responding`, `expanded_tools`
- Prefix booleans with `is_`: `is_responding`, `is_selecting`, `is_expanded`, `is_error`
- Use `_` prefix for "held-but-unused" subscriptions: `_subscription`, `_pending_task`

## Code Style

**Formatting:**
- Default `rustfmt` (no `.rustfmt.toml` present -- use standard Rust formatting)
- Rust edition 2024 (`Cargo.toml` line 8)
- No clippy configuration file -- use default clippy rules

**Indentation:**
- 4 spaces (Rust standard)

**Line Length:**
- No explicit limit configured; follows rustfmt default (100 chars)

**Linting:**
- No custom clippy configuration. Use `cargo clippy` with default rules.

## Import Organization

**Order:**
1. `std` library imports (`std::collections`, `std::sync`, `std::io`, `std::ops`)
2. External crate imports (`gpui::*`, `serde::*`, `serde_json`)
3. Internal crate imports (`crate::provider`, `crate::theme`, `crate::text_input`)

**Grouping:**
- Separate each group with a blank line
- Merge multiple items from the same crate into a single `use` with braces: `use gpui::{Context, Entity, Window, div, px};`
- Use glob import for prelude only: `use gpui::prelude::*;`
- Prefer explicit item imports over glob imports for everything else

**Path Aliases:**
- None configured. Use `crate::` for internal references.

**Example** (from `src/workspace.rs`):
```rust
use gpui::prelude::*;
use gpui::{Context, Entity, Window, div, px};

use crate::chat::ChatPanel;
use crate::plan::PlanPanel;
use crate::sidebar::Sidebar;
use crate::theme;
```

**Late Imports:**
- Avoid placing imports after the main code. There is one instance at `src/chat.rs:569` (`use gpui::SharedString;`) placed at the end of the file -- do not follow this pattern in new code. Place all imports at the top.

## Module Structure

**Declaration:**
- All modules declared in `src/main.rs` using `mod` statements
- No `lib.rs` -- this is a binary-only crate with a single `[[bin]]` target
- Flat module structure (no nested directories/sub-modules)

**Visibility:**
- Structs that are used cross-module: `pub struct`
- Struct fields: private by default, public only when needed by other modules (e.g., `pub blocks: Vec<ContentBlock>` on `Turn`)
- Functions callable from other modules: `pub fn`
- Internal helpers: `fn` (private)
- Theme constants: all `pub const`

## GPUI Component Pattern

All UI components follow this structure (see `src/chat.rs`, `src/sidebar.rs`, `src/plan.rs`):

```rust
pub struct MyComponent {
    // fields
}

impl MyComponent {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self { /* ... */ }
    }
}

impl Render for MyComponent {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            // ... fluent builder chain
    }
}
```

**Key patterns:**
- Use `cx.new(|cx| Component::new(cx))` to create child entities (see `src/workspace.rs:17-19`)
- Store child entities as `Entity<T>` fields
- Use `cx.subscribe(&entity, Self::handler)` for event subscriptions (see `src/chat.rs:103`)
- Store subscriptions in `_subscription: Subscription` fields to keep them alive
- Use `cx.notify()` to trigger re-render after state changes
- Use `.when(condition, |this| ...)` for conditional rendering (see `src/chat.rs:305,330`)

**Render method style:**
- Use fluent builder chains (`.flex().flex_col().gap_4()`)
- Extract complex render logic into standalone `fn render_*()` functions (see `src/chat.rs:375,466`)
- Standalone render functions return `AnyElement` (use `.into_any()`)

## Actions Pattern

Actions are declared with the `gpui::actions!` macro (see `src/main.rs:44`, `src/text_input.rs:11-29`):

```rust
actions!(namespace, [ActionName1, ActionName2]);
```

Bind actions in a setup function:
```rust
pub fn bind_text_input_keys(cx: &mut App) {
    cx.bind_keys([
        gpui::KeyBinding::new("backspace", Backspace, Some("TextInput")),
        // ...
    ]);
}
```

Register handlers with `cx.listener()`:
```rust
.on_action(cx.listener(Self::handler_method))
```

Action handler signature:
```rust
fn handler(&mut self, _: &ActionName, _: &mut Window, cx: &mut Context<Self>) {
    // ...
}
```

## Error Handling

**Patterns:**
- Use `anyhow::Result<()>` for fallible async tasks (`src/chat.rs:96`)
- Use `.ok()` to silently discard send errors on `mpsc::Sender` (`src/provider.rs:94,95,106,107`)
- Use `.unwrap()` only at top-level window creation in `main()` (`src/main.rs:31,38`)
- Use `.expect("message")` for invariants that should never fail (`src/text_input.rs:619,628`)
- Use `match` with early `return` for error variants in process spawning (`src/provider.rs:88-98,100-110`)
- Convert errors to user-visible messages via `ProviderEvent::Error(String)` rather than panicking (`src/provider.rs:91,103,118`)
- Use `if let Some(x) = ...` / `let Some(x) = ... else { return; }` for optional value handling (`src/chat.rs:234`)

**Error display:**
- Errors from the provider are surfaced as system messages in the chat UI (`src/chat.rs:263-269`)

## Logging

**Framework:** None. No logging framework is used. No `log`, `tracing`, or `env_logger` dependency.

**Patterns:**
- Errors are communicated through the event channel (`ProviderEvent::Error`) and displayed in the UI
- No `println!`, `eprintln!`, or `dbg!` calls in the codebase

## Comments

**When to Comment:**
- Use section dividers for major code sections: `// -- Section Name --` with unicode box-drawing characters (see `src/chat.rs:15,88,285`)
- Use inline comments for non-obvious logic: `// Force the list to remeasure this item` (`src/chat.rs:137`)
- Use `///` doc comments for public API items in the provider module (`src/provider.rs:6,23,62,73`)

**Section Divider Style:**
```rust
// ── Section Name ──────────────────────────────────────────────────────
```

**Doc Comments:**
- Use `///` for public functions and types that serve as module boundaries (e.g., provider API)
- UI components (panels, sidebar) do not use doc comments

## Theme Usage

**Always reference theme constants** from `crate::theme` for colors. Never use inline color values in component render methods.

```rust
use crate::theme;

// Correct:
.bg(theme::BG_BASE)
.text_color(theme::TEXT_PRIMARY)

// Incorrect:
.bg(rgba(0x1e1e2eff))
```

Exception: `src/text_input.rs` uses inline `hsla()` and `rgba()` for cursor and selection highlight colors that are not yet in the theme module.

## Function Design

**Size:** Functions are generally compact (under 50 lines). The largest is `ChatPanel::render` at ~85 lines and `send_turn` at ~70 lines.

**Parameters:** Follow GPUI conventions -- most methods take `&mut self`, `window: &mut Window`, `cx: &mut Context<Self>`. Action handlers prefix the action with `_` when unused.

**Return Values:** Render methods return `impl IntoElement`. Standalone render helpers return `AnyElement`.

## Module Design

**Exports:**
- Export only what other modules need: primary struct, constructor, and key public methods
- Keep internal types (e.g., `RawEvent`, `RawMessage` in `src/provider.rs`) private
- Export free functions when they serve as module-level API: `provider::run_turn()`, `provider::is_claude_available()`

**Barrel Files:** Not used. Each module is imported individually via `crate::module_name`.

---

*Convention analysis: 2026-03-26*

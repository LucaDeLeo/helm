<!-- GSD:project-start source:PROJECT.md -->
## Project

**Helm**

Helm is a native desktop Agentic Development Environment (ADE) built with Rust and Zed's GPUI framework. It lets you manage multiple projects, each with parallel agent workspaces running on isolated git worktrees, through a GPU-accelerated interface with rich chat rendering — streaming markdown, expandable tool calls, inline diffs, and activity logs. Think Conductor meets T3 Code, but native and fast.

**Core Value:** A high-performance, customizable native ADE — GPU-accelerated UI that's snappier than Electron alternatives, with full control over the agent workflow experience.

### Constraints

- **Tech stack**: Rust + GPUI only — no web technologies, no Electron, no JavaScript
- **Platform**: macOS primary target (GPUI supports Linux/Windows but untested for Helm)
- **Dependency**: Requires local Zed repo clone at `../zed/` for GPUI crates
- **Providers**: Claude integration via Agent SDK (Bun sidecar bridge); Codex via CLI subprocess — Bun runtime and respective SDKs must be available
- **Rendering**: All UI rendering through GPUI's element system — no webviews or HTML
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust (Edition 2024) - All application code in `src/`
- None
## Runtime
- Native desktop binary (compiled via `cargo build`)
- GPU-accelerated rendering via GPUI (Zed's UI framework)
- macOS primary target (via `gpui_platform` with `gpui_macos` backend)
- Cargo 1.93.1
- Lockfile: present (`Cargo.lock`, 6631 lines, ~200+ transitive dependencies)
## Frameworks
- `gpui` 0.2.2 (local path: `../zed/crates/gpui`) - Zed's GPU-accelerated immediate-mode UI framework. Provides the reactive entity model, window management, layout engine, text rendering, key bindings, focus management, and element painting.
- `gpui_platform` 0.1.0 (local path: `../zed/crates/gpui_platform`) - Platform abstraction layer. Used with `font-kit` feature enabled. Provides `application()` entry point.
- Not detected (no test framework configured, no test files present)
- `cargo` - Standard Rust build system
- No custom build scripts (`build.rs`) detected
## Key Dependencies
- `gpui` 0.2.2 - The entire UI framework; every source file imports from it. Provides `Entity`, `Context`, `Window`, `Render` trait, layout primitives (`div`, `px`, `size`), `ListState`, `Focusable`, `IntoElement`, text shaping, and the action/key-binding system.
- `gpui_platform` 0.1.0 - Provides the cross-platform `application()` bootstrap function used in `src/main.rs`.
- `serde` 1.0.228 (with `derive` feature) - JSON deserialization of Claude CLI stream events in `src/provider.rs`
- `serde_json` 1.0.149 - Parsing stream-json output from `claude` CLI in `src/provider.rs`
- `anyhow` 1.0.102 - Error handling (`anyhow::Result`) used in async task returns in `src/chat.rs`
- `smol` 2.0.2 - Async runtime; `smol::Timer` used for polling in `src/chat.rs`
- `async-task` - Pinned to specific git revision (`b4486cd`) from `smol-rs/async-task` via `[patch.crates-io]` in `Cargo.toml`. Required for compatibility with gpui's async runtime.
## Configuration
- No environment variables required by Helm itself
- The `claude` CLI (spawned as subprocess) handles its own authentication and configuration
- No `.env` files present or expected
- `Cargo.toml` - Workspace with single member, Rust edition 2024
- Binary target: `helm` at `src/main.rs`
- `.gitignore` - Contains only `/target`
- `.emdash.json` - Emdash tool config (preservePatterns for env files, empty scripts)
## Platform Requirements
- Rust 1.93+ (edition 2024 requires nightly or recent stable)
- Local clone of Zed repository at `../zed/` (relative to project root) providing `gpui` and `gpui_platform` crates
- `claude` CLI installed globally (`npm install -g @anthropic-ai/claude-code`) for AI chat functionality
- macOS recommended (primary platform; `gpui_platform` also supports Linux/Windows/Web but those are untested for Helm)
- Native desktop binary (no server deployment)
- macOS (primary), potentially Linux/Windows via gpui_platform backends
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- Use `snake_case.rs` for all source files: `chat.rs`, `text_input.rs`, `provider.rs`
- One primary struct/component per file, named after the file in PascalCase
- Module file names match their conceptual purpose: `theme.rs` for colors, `provider.rs` for the Claude CLI integration
- PascalCase: `ChatPanel`, `HelmWorkspace`, `PlanPanel`, `TextInput`, `TextElement`
- Suffix panels with `Panel`: `ChatPanel`, `PlanPanel`
- Use `Sidebar` (no suffix) for the navigation sidebar
- PascalCase for type name and variants: `MessageRole::User`, `ContentBlock::Text`, `ProviderEvent::TextDelta`
- Use descriptive variant names that read naturally: `ProviderEvent::ToolUse`, `ProviderEvent::Complete`
- `snake_case` for all functions and methods: `send_turn`, `push_turn`, `focus_input`, `run_turn`
- Prefix event handlers with `on_`: `on_input_event`, `on_mouse_down`, `on_mouse_up`, `on_mouse_move`
- Prefix render helpers with `render_`: `render_turn`, `render_tool_call`
- Use verb-noun pattern: `bind_text_input_keys`, `toggle_tool_expanded`, `handle_provider_event`
- `SCREAMING_SNAKE_CASE` for theme constants: `BG_BASE`, `TEXT_PRIMARY`, `ACCENT`
- Group with prefix by category: `BG_*` for backgrounds, `TEXT_*` for text colors, `BORDER_*` for borders
- `snake_case` for all locals and fields: `list_state`, `is_responding`, `expanded_tools`
- Prefix booleans with `is_`: `is_responding`, `is_selecting`, `is_expanded`, `is_error`
- Use `_` prefix for "held-but-unused" subscriptions: `_subscription`, `_pending_task`
## Code Style
- Default `rustfmt` (no `.rustfmt.toml` present -- use standard Rust formatting)
- Rust edition 2024 (`Cargo.toml` line 8)
- No clippy configuration file -- use default clippy rules
- 4 spaces (Rust standard)
- No explicit limit configured; follows rustfmt default (100 chars)
- No custom clippy configuration. Use `cargo clippy` with default rules.
## Import Organization
- Separate each group with a blank line
- Merge multiple items from the same crate into a single `use` with braces: `use gpui::{Context, Entity, Window, div, px};`
- Use glob import for prelude only: `use gpui::prelude::*;`
- Prefer explicit item imports over glob imports for everything else
- None configured. Use `crate::` for internal references.
- Avoid placing imports after the main code. There is one instance at `src/chat.rs:569` (`use gpui::SharedString;`) placed at the end of the file -- do not follow this pattern in new code. Place all imports at the top.
## Module Structure
- All modules declared in `src/main.rs` using `mod` statements
- No `lib.rs` -- this is a binary-only crate with a single `[[bin]]` target
- Flat module structure (no nested directories/sub-modules)
- Structs that are used cross-module: `pub struct`
- Struct fields: private by default, public only when needed by other modules (e.g., `pub blocks: Vec<ContentBlock>` on `Turn`)
- Functions callable from other modules: `pub fn`
- Internal helpers: `fn` (private)
- Theme constants: all `pub const`
## GPUI Component Pattern
- Use `cx.new(|cx| Component::new(cx))` to create child entities (see `src/workspace.rs:17-19`)
- Store child entities as `Entity<T>` fields
- Use `cx.subscribe(&entity, Self::handler)` for event subscriptions (see `src/chat.rs:103`)
- Store subscriptions in `_subscription: Subscription` fields to keep them alive
- Use `cx.notify()` to trigger re-render after state changes
- Use `.when(condition, |this| ...)` for conditional rendering (see `src/chat.rs:305,330`)
- Use fluent builder chains (`.flex().flex_col().gap_4()`)
- Extract complex render logic into standalone `fn render_*()` functions (see `src/chat.rs:375,466`)
- Standalone render functions return `AnyElement` (use `.into_any()`)
## Actions Pattern
## Error Handling
- Use `anyhow::Result<()>` for fallible async tasks (`src/chat.rs:96`)
- Use `.ok()` to silently discard send errors on `mpsc::Sender` (`src/provider.rs:94,95,106,107`)
- Use `.unwrap()` only at top-level window creation in `main()` (`src/main.rs:31,38`)
- Use `.expect("message")` for invariants that should never fail (`src/text_input.rs:619,628`)
- Use `match` with early `return` for error variants in process spawning (`src/provider.rs:88-98,100-110`)
- Convert errors to user-visible messages via `ProviderEvent::Error(String)` rather than panicking (`src/provider.rs:91,103,118`)
- Use `if let Some(x) = ...` / `let Some(x) = ... else { return; }` for optional value handling (`src/chat.rs:234`)
- Errors from the provider are surfaced as system messages in the chat UI (`src/chat.rs:263-269`)
## Logging
- Errors are communicated through the event channel (`ProviderEvent::Error`) and displayed in the UI
- No `println!`, `eprintln!`, or `dbg!` calls in the codebase
## Comments
- Use section dividers for major code sections: `// -- Section Name --` with unicode box-drawing characters (see `src/chat.rs:15,88,285`)
- Use inline comments for non-obvious logic: `// Force the list to remeasure this item` (`src/chat.rs:137`)
- Use `///` doc comments for public API items in the provider module (`src/provider.rs:6,23,62,73`)
- Use `///` for public functions and types that serve as module boundaries (e.g., provider API)
- UI components (panels, sidebar) do not use doc comments
## Theme Usage
## Function Design
## Module Design
- Export only what other modules need: primary struct, constructor, and key public methods
- Keep internal types (e.g., `RawEvent`, `RawMessage` in `src/provider.rs`) private
- Export free functions when they serve as module-level API: `provider::run_turn()`, `provider::is_claude_available()`
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Pattern Overview
- Single-binary Rust desktop application
- Entity-based reactive component model (GPUI's `Entity<T>` + `Render` trait)
- Event-driven communication between components via subscriptions and channels
- Three-panel layout: sidebar, chat, plan
- External process integration with Claude CLI via streaming JSON
## Layers
- Purpose: Initialize the GPUI app, bind global keys, open the main window
- Location: `src/main.rs`
- Contains: `main()` function, global action definitions, window configuration
- Depends on: `gpui`, `gpui_platform`, all local modules
- Used by: OS runtime (binary entry point)
- Purpose: Top-level container that owns and composes all panels into a three-column layout
- Location: `src/workspace.rs`
- Contains: `HelmWorkspace` struct holding `Entity<Sidebar>`, `Entity<ChatPanel>`, `Entity<PlanPanel>`
- Depends on: `chat`, `plan`, `sidebar`, `theme`
- Used by: `main.rs` (created as the window's root view)
- Purpose: Self-contained visual components that render and manage their own state
- Location: `src/chat.rs`, `src/sidebar.rs`, `src/plan.rs`
- Contains: Panel structs implementing GPUI's `Render` trait
- Depends on: `theme`, `text_input` (chat only), `provider` (chat only)
- Used by: `workspace.rs`
- Purpose: Spawn and communicate with the `claude` CLI process, parse streaming JSON output
- Location: `src/provider.rs`
- Contains: `ProviderEvent` enum, `run_turn()` blocking function, JSON deserialization structs, event parsing logic
- Depends on: `serde`, `serde_json`, `std::process`
- Used by: `chat.rs`
- Purpose: Single-line editable text field with cursor, selection, IME support, and clipboard
- Location: `src/text_input.rs`
- Contains: `TextInput` struct, `TextElement` custom GPUI element, key binding registration
- Depends on: `gpui` (Element trait, EntityInputHandler trait)
- Used by: `chat.rs`
- Purpose: Centralized color constants for the entire UI
- Location: `src/theme.rs`
- Contains: `const` color values as `Rgba`, helper `hex()` function
- Depends on: `gpui::Rgba`
- Used by: All rendering modules (`workspace.rs`, `chat.rs`, `sidebar.rs`, `plan.rs`, `text_input.rs`)
## Data Flow
- Each panel owns its state directly (no global store)
- `ChatPanel` holds `Vec<Turn>` as the conversation history
- `ListState` from GPUI manages virtual scrolling of turns
- `expanded_tools: HashSet<String>` tracks which tool call blocks are expanded
- Communication between components uses GPUI's `Entity<T>` subscription system (`cx.subscribe()`)
- Communication with external process uses `std::sync::mpsc` channels
## Key Abstractions
- Purpose: Represent a single message turn containing multiple content blocks
- Examples: `src/chat.rs` lines 17-86
- Pattern: A `Turn` has a `MessageRole` (User/Assistant/System) and a `Vec<ContentBlock>`. Each `ContentBlock` is either `Text(String)` or `ToolCall { id, name, input, output, is_error }`. Methods on `Turn` handle appending text, adding tool calls, and updating tool results.
- Purpose: Normalized events from the Claude CLI stream
- Examples: `src/provider.rs` lines 7-21
- Pattern: Enum with variants `TextDelta`, `ToolUse`, `ToolResult`, `Complete`, `Error`. Decouples raw JSON parsing from UI event handling.
- Purpose: Heap-allocated, reference-counted reactive state container provided by GPUI
- Examples: `Entity<ChatPanel>` in `src/workspace.rs`, `Entity<TextInput>` in `src/chat.rs`
- Pattern: Components are created via `cx.new(|cx| T::new(cx))`, read via `.read(cx)`, updated via `.update(cx, |this, cx| ...)`. Calling `cx.notify()` triggers re-render.
- Purpose: Low-level custom rendering element for the text input cursor, selection highlighting, and shaped text
- Examples: `src/text_input.rs` lines 456-641
- Pattern: Implements GPUI's `Element` trait with `request_layout`, `prepaint`, and `paint` phases. Handles text shaping, cursor positioning, and selection rectangle calculation.
## Entry Points
- Location: `src/main.rs`
- Triggers: OS launches the binary
- Responsibilities: Calls `application().run()`, binds global keys (`cmd-q` for Quit), opens a window with `HelmWorkspace` as root view, focuses chat input, activates the app
## Error Handling
- `provider::run_turn()` sends `ProviderEvent::Error(msg)` on failure, which `ChatPanel` renders as a system turn
- `is_claude_available()` check at startup pushes a system message if `claude` CLI is not found
- `.unwrap()` used for window creation in `main.rs` (crash on failure is acceptable at startup)
- `.ok()` used on channel `send()` calls to silently ignore send failures (receiver dropped)
- `anyhow::Result` used as return type for the foreground polling task
## Cross-Cutting Concerns
- Global: `cmd-q` for Quit, bound in `src/main.rs`
- Scoped to `"TextInput"` context: navigation, selection, clipboard, submit -- bound via `bind_text_input_keys()` in `src/text_input.rs`
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->

# Architecture

**Analysis Date:** 2026-03-26

## Pattern Overview

**Overall:** Component-based native GUI application using GPUI framework (Zed's UI framework)

**Key Characteristics:**
- Single-binary Rust desktop application
- Entity-based reactive component model (GPUI's `Entity<T>` + `Render` trait)
- Event-driven communication between components via subscriptions and channels
- Three-panel layout: sidebar, chat, plan
- External process integration with Claude CLI via streaming JSON

## Layers

**Application Bootstrap:**
- Purpose: Initialize the GPUI app, bind global keys, open the main window
- Location: `src/main.rs`
- Contains: `main()` function, global action definitions, window configuration
- Depends on: `gpui`, `gpui_platform`, all local modules
- Used by: OS runtime (binary entry point)

**Workspace (Layout Orchestration):**
- Purpose: Top-level container that owns and composes all panels into a three-column layout
- Location: `src/workspace.rs`
- Contains: `HelmWorkspace` struct holding `Entity<Sidebar>`, `Entity<ChatPanel>`, `Entity<PlanPanel>`
- Depends on: `chat`, `plan`, `sidebar`, `theme`
- Used by: `main.rs` (created as the window's root view)

**UI Panels:**
- Purpose: Self-contained visual components that render and manage their own state
- Location: `src/chat.rs`, `src/sidebar.rs`, `src/plan.rs`
- Contains: Panel structs implementing GPUI's `Render` trait
- Depends on: `theme`, `text_input` (chat only), `provider` (chat only)
- Used by: `workspace.rs`

**Provider (External Process Bridge):**
- Purpose: Spawn and communicate with the `claude` CLI process, parse streaming JSON output
- Location: `src/provider.rs`
- Contains: `ProviderEvent` enum, `run_turn()` blocking function, JSON deserialization structs, event parsing logic
- Depends on: `serde`, `serde_json`, `std::process`
- Used by: `chat.rs`

**Text Input (Custom Widget):**
- Purpose: Single-line editable text field with cursor, selection, IME support, and clipboard
- Location: `src/text_input.rs`
- Contains: `TextInput` struct, `TextElement` custom GPUI element, key binding registration
- Depends on: `gpui` (Element trait, EntityInputHandler trait)
- Used by: `chat.rs`

**Theme (Design Tokens):**
- Purpose: Centralized color constants for the entire UI
- Location: `src/theme.rs`
- Contains: `const` color values as `Rgba`, helper `hex()` function
- Depends on: `gpui::Rgba`
- Used by: All rendering modules (`workspace.rs`, `chat.rs`, `sidebar.rs`, `plan.rs`, `text_input.rs`)

## Data Flow

**User sends a message:**

1. User types in `TextInput` and presses Enter
2. `TextInput` emits `TextInputEvent::Submit(text)` via GPUI event system
3. `ChatPanel::on_input_event()` receives the event via subscription
4. `ChatPanel::send_turn()` is called:
   a. Pushes a `Turn { role: User }` to `self.turns`
   b. Pushes an empty `Turn { role: Assistant }` to stream into
   c. Creates an `mpsc::channel` for `ProviderEvent` messages
   d. Spawns `provider::run_turn()` on a background thread via `cx.background_spawn()`
   e. Spawns a foreground polling task via `cx.spawn()` that reads from the channel at ~60fps (16ms timer)
5. `provider::run_turn()` spawns `claude -p <prompt> --output-format stream-json` as a child process
6. Provider reads stdout line-by-line, parses JSON into `RawEvent`, converts to `ProviderEvent`, sends via channel
7. Foreground task receives `ProviderEvent`s, calls `handle_provider_event()` to mutate the assistant `Turn`
8. `cx.notify()` triggers re-render of `ChatPanel` after each batch of events
9. `ProviderEvent::Complete` ends the loop and sets `is_responding = false`

**State Management:**
- Each panel owns its state directly (no global store)
- `ChatPanel` holds `Vec<Turn>` as the conversation history
- `ListState` from GPUI manages virtual scrolling of turns
- `expanded_tools: HashSet<String>` tracks which tool call blocks are expanded
- Communication between components uses GPUI's `Entity<T>` subscription system (`cx.subscribe()`)
- Communication with external process uses `std::sync::mpsc` channels

## Key Abstractions

**Turn / ContentBlock (Chat Data Model):**
- Purpose: Represent a single message turn containing multiple content blocks
- Examples: `src/chat.rs` lines 17-86
- Pattern: A `Turn` has a `MessageRole` (User/Assistant/System) and a `Vec<ContentBlock>`. Each `ContentBlock` is either `Text(String)` or `ToolCall { id, name, input, output, is_error }`. Methods on `Turn` handle appending text, adding tool calls, and updating tool results.

**ProviderEvent (Streaming Protocol):**
- Purpose: Normalized events from the Claude CLI stream
- Examples: `src/provider.rs` lines 7-21
- Pattern: Enum with variants `TextDelta`, `ToolUse`, `ToolResult`, `Complete`, `Error`. Decouples raw JSON parsing from UI event handling.

**Entity<T> (GPUI Reactive Primitive):**
- Purpose: Heap-allocated, reference-counted reactive state container provided by GPUI
- Examples: `Entity<ChatPanel>` in `src/workspace.rs`, `Entity<TextInput>` in `src/chat.rs`
- Pattern: Components are created via `cx.new(|cx| T::new(cx))`, read via `.read(cx)`, updated via `.update(cx, |this, cx| ...)`. Calling `cx.notify()` triggers re-render.

**TextElement (Custom GPUI Element):**
- Purpose: Low-level custom rendering element for the text input cursor, selection highlighting, and shaped text
- Examples: `src/text_input.rs` lines 456-641
- Pattern: Implements GPUI's `Element` trait with `request_layout`, `prepaint`, and `paint` phases. Handles text shaping, cursor positioning, and selection rectangle calculation.

## Entry Points

**Binary Entry (`main`):**
- Location: `src/main.rs`
- Triggers: OS launches the binary
- Responsibilities: Calls `application().run()`, binds global keys (`cmd-q` for Quit), opens a window with `HelmWorkspace` as root view, focuses chat input, activates the app

## Error Handling

**Strategy:** Minimal -- errors are displayed inline in the chat as System messages

**Patterns:**
- `provider::run_turn()` sends `ProviderEvent::Error(msg)` on failure, which `ChatPanel` renders as a system turn
- `is_claude_available()` check at startup pushes a system message if `claude` CLI is not found
- `.unwrap()` used for window creation in `main.rs` (crash on failure is acceptable at startup)
- `.ok()` used on channel `send()` calls to silently ignore send failures (receiver dropped)
- `anyhow::Result` used as return type for the foreground polling task

## Cross-Cutting Concerns

**Logging:** None implemented. No logging framework in use.

**Validation:** Minimal -- `TextInput` prevents submitting empty strings, `ChatPanel` prevents double-sends via `is_responding` flag.

**Authentication:** Not applicable. The `claude` CLI handles its own authentication.

**Theming:** Centralized in `src/theme.rs` as compile-time constants. Catppuccin Mocha color palette. No runtime theme switching.

**Key Bindings:** Two layers:
- Global: `cmd-q` for Quit, bound in `src/main.rs`
- Scoped to `"TextInput"` context: navigation, selection, clipboard, submit -- bound via `bind_text_input_keys()` in `src/text_input.rs`

---

*Architecture analysis: 2026-03-26*

# Codebase Structure

**Analysis Date:** 2026-03-26

## Directory Layout

```
helm/
├── src/                # All application source code
│   ├── main.rs         # Binary entry point, window setup, global keybindings
│   ├── workspace.rs    # Root view: three-column layout orchestrator
│   ├── chat.rs         # Chat panel: conversation display, message streaming, tool call rendering
│   ├── sidebar.rs      # Left sidebar: app branding, session list (placeholder)
│   ├── plan.rs         # Right panel: plan display (placeholder)
│   ├── provider.rs     # Claude CLI process bridge: spawn, stream JSON, parse events
│   ├── text_input.rs   # Custom single-line text input widget with cursor/selection/IME
│   └── theme.rs        # Color constants (Catppuccin Mocha palette)
├── target/             # Cargo build output (gitignored)
├── Cargo.toml          # Package manifest and dependencies
├── Cargo.lock          # Locked dependency versions
├── .gitignore          # Ignores /target
├── .emdash.json        # Emdash tool config
├── .claude/            # Claude Code tool configuration
│   ├── agents/         # GSD agent definitions
│   ├── commands/       # GSD command definitions
│   ├── get-shit-done/  # GSD framework config
│   ├── hooks/          # Git hooks
│   ├── settings.json   # Claude settings
│   └── settings.local.json  # Local Claude settings
├── .codex/             # Codex tool configuration
│   ├── agents/         # Codex agent definitions
│   ├── get-shit-done/  # GSD framework config for Codex
│   ├── skills/         # Codex skill definitions
│   └── config.toml     # Codex config
└── .planning/          # GSD planning artifacts
    ├── config.json     # Planning config
    └── codebase/       # Codebase analysis documents (this directory)
```

## Directory Purposes

**`src/`:**
- Purpose: All Rust source files for the Helm application
- Contains: Flat module structure -- all `.rs` files at the top level
- Key files: `main.rs` (entry), `workspace.rs` (layout), `chat.rs` (core feature), `provider.rs` (Claude integration)

**`target/`:**
- Purpose: Cargo build artifacts
- Generated: Yes
- Committed: No (gitignored)

**`.claude/`:**
- Purpose: Claude Code / GSD framework configuration
- Contains: Agent definitions, command definitions, hooks, settings
- Generated: Partially (tool-managed)
- Committed: Yes

**`.codex/`:**
- Purpose: Codex tool configuration
- Contains: Agent definitions, skills, config
- Generated: Partially (tool-managed)
- Committed: Yes

**`.planning/`:**
- Purpose: GSD planning artifacts and codebase analysis
- Contains: Planning config, codebase analysis documents
- Generated: Yes (by GSD agents)
- Committed: Yes

## Key File Locations

**Entry Points:**
- `src/main.rs`: Application entry point. Initializes GPUI app, opens window, creates `HelmWorkspace`.

**Configuration:**
- `Cargo.toml`: Package metadata, dependencies, binary target definition. Points to local GPUI at `../zed/crates/gpui`.
- `Cargo.lock`: Pinned dependency versions.

**Core Logic:**
- `src/chat.rs`: Chat panel with conversation history, message streaming, and tool call display. This is the largest and most complex module (~570 lines).
- `src/provider.rs`: Claude CLI integration. Spawns `claude` process, reads streaming JSON, emits normalized `ProviderEvent`s (~280 lines).
- `src/text_input.rs`: Full custom text input widget with IME, clipboard, selection, cursor rendering (~660 lines).

**UI Components:**
- `src/workspace.rs`: Root layout composing sidebar, chat, and plan panels (~65 lines).
- `src/sidebar.rs`: Left sidebar with app title and placeholder session list (~63 lines).
- `src/plan.rs`: Right panel with placeholder plan display (~43 lines).

**Styling:**
- `src/theme.rs`: All color constants as `Rgba` values (~27 lines).

**Testing:**
- No test files exist. No test infrastructure is configured.

## Naming Conventions

**Files:**
- snake_case: `text_input.rs`, `workspace.rs`
- One module per file, no nested module directories

**Modules:**
- Declared in `src/main.rs` via `mod` statements
- All modules are private (`mod`, not `pub mod`)
- Public items within modules are used via `crate::` paths

**Structs:**
- PascalCase: `HelmWorkspace`, `ChatPanel`, `PlanPanel`, `TextInput`, `TextElement`
- Panel components use `Panel` suffix: `ChatPanel`, `PlanPanel`

**Functions:**
- snake_case: `run_turn`, `focus_input`, `send_turn`, `handle_provider_event`
- Private helper functions prefixed with action verbs: `render_turn`, `render_tool_call`, `parse_event`

**Constants:**
- SCREAMING_SNAKE_CASE: `BG_BASE`, `TEXT_PRIMARY`, `ACCENT`, `BORDER_TOOL`
- Theme constants grouped by purpose with comment headers

**Enums:**
- PascalCase variants: `MessageRole::User`, `ContentBlock::Text`, `ProviderEvent::TextDelta`

## Where to Add New Code

**New UI Panel:**
1. Create `src/<panel_name>.rs` with a struct implementing `Render`
2. Add `mod <panel_name>;` to `src/main.rs`
3. Add `Entity<NewPanel>` field to `HelmWorkspace` in `src/workspace.rs`
4. Instantiate in `HelmWorkspace::new()` and add to the layout in `Render::render()`

**New Feature to Chat Panel:**
- Add data model types at the top of `src/chat.rs` (following the `// -- Data Model --` section pattern)
- Add event handling in `ChatPanel` methods
- Add rendering as helper `fn render_*()` functions at the bottom of the file

**New Provider/Backend Integration:**
- Add to `src/provider.rs` for Claude CLI interaction changes
- Add new `ProviderEvent` variants as needed
- Handle new events in `ChatPanel::handle_provider_event()` in `src/chat.rs`

**New Theme Colors:**
- Add `pub const` to `src/theme.rs` using the `hex()` helper
- Group with related constants and use descriptive prefixes (`BG_`, `TEXT_`, `BORDER_`)

**New Custom Widget:**
- Create `src/<widget_name>.rs`
- Implement `Render` trait for simple widgets
- Implement `Element` trait for low-level custom rendering (see `TextElement` in `src/text_input.rs` as reference)
- If the widget needs key bindings, add a `bind_*_keys(cx: &mut App)` function and call it from `main.rs`

**New Actions (Keyboard Shortcuts):**
- Define with `gpui::actions!()` macro in the relevant module
- Bind globally in `src/main.rs` or scoped to a key context in the widget module
- Register handlers with `.on_action(cx.listener(Self::handler))` in the widget's `Render::render()`

**Utilities:**
- Small helpers belong in the module that uses them (no shared utils module exists)
- If a helper is needed by multiple modules, create `src/utils.rs` and add `mod utils;` to `main.rs`

## Special Directories

**`../zed/crates/gpui` and `../zed/crates/gpui_platform`:**
- Purpose: Local path dependencies pointing to the Zed editor's GPUI framework crates
- These are NOT inside the Helm repo -- they live in a sibling `zed` directory
- Referenced in `Cargo.toml` via relative path: `path = "../zed/crates/gpui"`
- Any changes to GPUI APIs may require updating Helm code accordingly

---

*Structure analysis: 2026-03-26*

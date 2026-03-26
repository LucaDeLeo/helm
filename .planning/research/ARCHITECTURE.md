# Architecture Research

**Domain:** Multi-project Agentic Development Environment (native Rust/GPUI desktop app)
**Researched:** 2026-03-26
**Confidence:** HIGH (patterns derived from GPUI source code, Zed codebase, and current codebase analysis)

## System Overview

```
+------------------------------------------------------------------+
|                         Application                              |
|  main.rs: bootstrap, global keys, window creation                |
+------------------------------------------------------------------+
|                       HelmWorkspace                              |
|  Owns the layout and routes between projects/sessions            |
+------+---------------------------+-------------------+-----------+
|      |                           |                   |           |
|  Sidebar              ActiveSession View        ContextPanel     |
|  (project list,       (chat + streaming)        (plan, diffs,    |
|   session list,                                  activity log)   |
|   branch info)                                                   |
|      |                           |                   |           |
+------+---------------------------+-------------------+-----------+
|                     Domain Layer                                 |
|  +-----------+  +-------------+  +-----------+  +------------+  |
|  | ProjectMgr|  | SessionMgr  |  | Provider  |  | Worktree   |  |
|  | (Entity)  |  | (Entity)    |  | Registry  |  | Manager    |  |
|  +-----------+  +-------------+  +-----------+  +------------+  |
|       |               |               |               |          |
+-------+---------------+---------------+---------------+----------+
|                   Process / IO Layer                             |
|  +----------------+  +----------------+  +----------------+     |
|  | Claude CLI     |  | Codex CLI      |  | git CLI /      |     |
|  | subprocess     |  | subprocess     |  | git2 bindings  |     |
|  +----------------+  +----------------+  +----------------+     |
+------------------------------------------------------------------+
```

### Component Responsibilities

| Component | Responsibility | Implementation |
|-----------|----------------|----------------|
| **HelmWorkspace** | Top-level view; owns sidebar, active session view, context panel; routes navigation between projects and sessions | `Entity<HelmWorkspace>` implementing `Render`, holds `Entity<T>` handles for all child panels |
| **Sidebar** | Displays project tree with nested sessions; shows branch name, diff stats, session status per worktree | `Entity<Sidebar>` subscribing to `ProjectManager` and `SessionManager` events |
| **ActiveSessionView** | Renders the chat for the currently-selected session; hosts `TextInput` and the turn list | `Entity<ActiveSessionView>` that swaps which `Entity<Session>` it reads from when selection changes |
| **ContextPanel** | Shows plan, inline diffs, activity log for the active session | `Entity<ContextPanel>` observing the active `Entity<Session>` |
| **ProjectManager** | Owns `Vec<Entity<Project>>`; CRUD for projects; persists project list | `Entity<ProjectManager>` emitting `ProjectEvent` |
| **SessionManager** | Creates/destroys sessions within a project; tracks all active sessions across all projects | `Entity<SessionManager>` emitting `SessionEvent`; holds `HashMap<SessionId, Entity<Session>>` |
| **Session** | Owns one agent conversation: turns, provider handle, worktree path, status | `Entity<Session>` emitting `SessionEvent`; contains the streaming state machine |
| **ProviderRegistry** | Maps provider names to factory functions; returns a `Box<dyn Provider>` | Plain struct (no Entity needed); called by `SessionManager` at session creation |
| **Provider trait** | Abstracts CLI agent lifecycle: spawn, stream events, cancel | Trait with `fn spawn_turn(...)` returning an mpsc receiver of `ProviderEvent` |
| **WorktreeManager** | Creates/lists/removes git worktrees for a project's repo | Pure function module or `Entity<WorktreeManager>` if it needs to track async operations |

## Recommended Project Structure

```
src/
+-- main.rs                  # App bootstrap, global keybindings, window creation
+-- workspace.rs             # HelmWorkspace: top-level layout, panel coordination
+-- ui/                      # All visual components
|   +-- sidebar.rs           # Project tree + session list
|   +-- chat_view.rs         # Active session chat rendering
|   +-- context_panel.rs     # Plan panel, activity log, diff viewer
|   +-- text_input.rs        # Custom text input widget (existing)
|   +-- markdown.rs          # Streaming markdown renderer
|   +-- diff_view.rs         # Inline diff display with syntax highlighting
|   +-- tool_call.rs         # Expandable tool call block rendering
+-- domain/                  # Business logic, no GPUI rendering
|   +-- project.rs           # Project struct, project list management
|   +-- session.rs           # Session entity: turns, status, streaming state
|   +-- turn.rs              # Turn and ContentBlock data model
+-- provider/                # CLI agent abstraction
|   +-- mod.rs               # Provider trait, ProviderEvent enum, ProviderRegistry
|   +-- claude.rs            # Claude Code CLI integration (stream-json parser)
|   +-- codex.rs             # Codex CLI integration (JSONL parser)
+-- git/                     # Git operations
|   +-- worktree.rs          # Worktree creation, listing, cleanup
|   +-- status.rs            # Branch info, diff stats
+-- theme.rs                 # Design tokens (existing)
```

### Structure Rationale

- **`ui/`:** Separates rendering from logic. Every file in `ui/` implements `Render` or produces `AnyElement`. Business logic stays in `domain/` and `provider/`.
- **`domain/`:** Pure Rust data structures and state machines. `Session` is the central domain object -- it owns turns, tracks status, and coordinates with its provider. No GPUI imports here except `Entity` registration.
- **`provider/`:** Each CLI gets its own parser module. The `mod.rs` defines the trait and shared types. Adding a new provider means adding one file and registering it in `ProviderRegistry`.
- **`git/`:** Isolated from UI concerns. Git operations are inherently blocking -- they run on background threads and report results via channels or GPUI tasks.

## Architectural Patterns

### Pattern 1: Entity-per-Session with Centralized Selection

**What:** Each agent session is its own `Entity<Session>`. The workspace tracks which session is "active" via an `Option<Entity<Session>>`. The chat view and context panel observe whichever session is currently active.

**When to use:** Always. This is the fundamental pattern for Helm's multi-session architecture.

**Trade-offs:** Each session has independent state (turns, provider, worktree), so switching sessions is instant (no serialization/deserialization). The cost is that all sessions remain in memory. For a desktop app with a handful of concurrent sessions, this is negligible.

**Example:**

```rust
pub struct HelmWorkspace {
    sidebar: Entity<Sidebar>,
    chat_view: Entity<ActiveSessionView>,
    context_panel: Entity<ContextPanel>,
    project_manager: Entity<ProjectManager>,
    session_manager: Entity<SessionManager>,
    active_session: Option<Entity<Session>>,
    _subscriptions: Vec<Subscription>,
}

impl HelmWorkspace {
    fn select_session(&mut self, session: Entity<Session>, cx: &mut Context<Self>) {
        self.active_session = Some(session.clone());
        // Notify children so they re-bind to the new session
        cx.emit(WorkspaceEvent::ActiveSessionChanged(session));
        cx.notify();
    }
}

impl EventEmitter<WorkspaceEvent> for HelmWorkspace {}
```

### Pattern 2: Provider Trait with Channel-Based Streaming

**What:** A `Provider` trait that spawns a CLI subprocess and returns an `mpsc::Receiver<ProviderEvent>`. The `Session` entity owns the receiver and polls it from a foreground GPUI task. This decouples the streaming protocol (Claude's stream-json vs. Codex's JSONL) from the session logic.

**When to use:** For every provider integration. The trait boundary is the key abstraction.

**Trade-offs:** Using `std::sync::mpsc` (as the codebase already does) is simple and works well with GPUI's `background_spawn` + foreground polling pattern. The 16ms polling interval is appropriate for 60fps rendering. An alternative would be `smol::channel` for async receives, but the polling approach avoids fighting GPUI's main-thread-centric async model.

**Example:**

```rust
pub enum ProviderEvent {
    TextDelta(String),
    ToolUse { id: String, name: String, input: String },
    ToolResult { tool_use_id: String, output: String, is_error: bool },
    Complete,
    Error(String),
}

pub trait Provider: Send + 'static {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
    fn spawn_turn(
        &self,
        prompt: &str,
        working_dir: &Path,
        session_history: &[Turn],
    ) -> mpsc::Receiver<ProviderEvent>;
    fn cancel(&self);
}

pub struct ClaudeProvider;
pub struct CodexProvider;

impl Provider for ClaudeProvider {
    fn spawn_turn(&self, prompt: &str, working_dir: &Path, _history: &[Turn])
        -> mpsc::Receiver<ProviderEvent>
    {
        let (tx, rx) = mpsc::channel();
        let prompt = prompt.to_string();
        let dir = working_dir.to_path_buf();
        std::thread::spawn(move || {
            // Spawn `claude -p <prompt> --output-format stream-json`
            // Parse stream-json lines into ProviderEvent, send via tx
            run_claude_turn(&prompt, &dir, &tx);
        });
        rx
    }
    // ...
}
```

### Pattern 3: EventEmitter Fan-Out for Cross-Component Updates

**What:** Domain entities (`Session`, `ProjectManager`, `SessionManager`) implement `EventEmitter<E>` to broadcast state changes. UI components subscribe to these events at construction time. This replaces direct function calls between components with decoupled event delivery.

**When to use:** Whenever a state change in one entity needs to update multiple UI components. The canonical example: a `Session` emits `SessionEvent::TurnUpdated`, and both the chat view and the context panel react independently.

**Trade-offs:** GPUI's effect queue ensures events fire after the emitting callback completes (run-to-completion semantics), so there are no reentrancy bugs. Subscriptions are automatically cleaned up when the subscriber is dropped. The main risk is event storms -- emit judiciously, especially during streaming.

**Example:**

```rust
pub enum SessionEvent {
    TurnAdded(usize),
    TurnUpdated(usize),
    StatusChanged(SessionStatus),
    StreamingBatch,  // Coalesced notification for streaming updates
}

impl EventEmitter<SessionEvent> for Session {}

// In ActiveSessionView:
impl ActiveSessionView {
    fn bind_to_session(&mut self, session: &Entity<Session>, cx: &mut Context<Self>) {
        self._session_sub = Some(cx.subscribe(session, Self::on_session_event));
    }

    fn on_session_event(
        &mut self, session: Entity<Session>, event: &SessionEvent, cx: &mut Context<Self>
    ) {
        match event {
            SessionEvent::StreamingBatch => {
                // Re-render the turn list
                self.list_state.splice(/* ... */);
                cx.notify();
            }
            SessionEvent::StatusChanged(status) => {
                self.is_responding = matches!(status, SessionStatus::Streaming);
                cx.notify();
            }
            _ => cx.notify(),
        }
    }
}
```

### Pattern 4: Worktree-per-Session Isolation

**What:** Every session operates in its own git worktree, created from the project's repository. The worktree path is set as the `current_dir` for the CLI subprocess. Sessions are isolated at the filesystem level -- no merge conflicts between concurrent agents.

**When to use:** For every session. This is not optional -- concurrent agents modifying the same working directory will create conflicts.

**Trade-offs:** Worktrees use shared `.git` storage so disk overhead is minimal (only modified files are duplicated). Creation is fast (~100ms for typical repos). Cleanup must be explicit -- orphaned worktrees waste disk. The `WorktreeManager` must track session-to-worktree mapping and clean up on session close.

## Data Flow

### Multi-Session Streaming Flow

```
User types prompt in TextInput
    |
    v
ActiveSessionView.on_input_event()
    |
    v
Session.send_turn(prompt)
    |
    +-- Push User turn to self.turns
    +-- Push empty Assistant turn
    +-- Call self.provider.spawn_turn(prompt, self.worktree_path, ...)
    |       |
    |       +-- [Background thread] Spawn CLI subprocess
    |       +-- [Background thread] Parse stdout line-by-line
    |       +-- [Background thread] Send ProviderEvent via mpsc channel
    |       v
    +-- Spawn foreground GPUI task: poll rx at ~60fps
            |
            v
        Receive ProviderEvent batch
            |
            +-- TextDelta  --> append to current Turn's text
            +-- ToolUse    --> add ToolCall block to Turn
            +-- ToolResult --> update ToolCall output
            +-- Complete   --> set status to Idle, drop task
            +-- Error      --> push System turn, set status to Error
            |
            v
        Session.emit(SessionEvent::StreamingBatch)
            |
            +-- ActiveSessionView receives, re-renders turn list
            +-- ContextPanel receives, updates activity log
            +-- Sidebar receives, updates session status indicator
```

### Session Selection Flow

```
User clicks session in Sidebar
    |
    v
Sidebar emits SidebarEvent::SessionSelected(Entity<Session>)
    |
    v
HelmWorkspace.on_sidebar_event()
    |
    +-- self.active_session = Some(session.clone())
    +-- Emit WorkspaceEvent::ActiveSessionChanged(session)
    |
    +-- ActiveSessionView.bind_to_session(session)
    |       +-- Drop old subscription
    |       +-- Subscribe to new session's events
    |       +-- Rebuild list_state from session.turns
    |       +-- cx.notify() -> re-render
    |
    +-- ContextPanel.bind_to_session(session)
            +-- Drop old subscription
            +-- Subscribe to new session's events
            +-- Rebuild plan/log view
            +-- cx.notify() -> re-render
```

### Project Creation Flow

```
User creates new project (via sidebar action)
    |
    v
ProjectManager.create_project(name, repo_path)
    |
    +-- Validate repo_path is a git repository
    +-- Create Project { id, name, repo_path, sessions: vec![] }
    +-- Emit ProjectEvent::ProjectAdded(project_id)
    |
    v
Sidebar receives event, re-renders project list
```

### Session Creation Flow

```
User creates new session within a project
    |
    v
SessionManager.create_session(project, provider_name)
    |
    +-- WorktreeManager.create_worktree(project.repo_path, branch_name)
    |       +-- [Background] `git worktree add .worktrees/<session-id> -b <branch>`
    |       +-- Returns worktree_path
    |
    +-- ProviderRegistry.create(provider_name) -> Box<dyn Provider>
    +-- Create Session { id, project_id, provider, worktree_path, turns, status }
    +-- Register Entity<Session> in sessions map
    +-- Emit SessionEvent::SessionCreated(session_id)
    |
    v
Workspace auto-selects the new session
```

### Key Data Flows

1. **Streaming text from CLI to UI:** CLI subprocess stdout -> line-by-line parsing on background thread -> `mpsc::channel` -> foreground polling task at 16ms intervals -> `Session` mutation -> `cx.emit(StreamingBatch)` -> subscribed UI components re-render.

2. **Session switching:** Sidebar click -> workspace updates `active_session` -> emits `ActiveSessionChanged` -> chat view and context panel drop old subscriptions, subscribe to new session, rebuild their state from the session's current data.

3. **Cross-component coordination:** All coordination flows through GPUI's `EventEmitter` + `cx.subscribe()`. No component directly calls methods on another component's inner state. The workspace acts as mediator for session selection; individual sessions broadcast their own state changes.

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 1-5 concurrent sessions | Current architecture works as-is. All sessions in memory, all streaming in parallel. |
| 5-20 concurrent sessions | Add session suspension: idle sessions serialize turns to disk and drop the `Entity<Session>`. Re-hydrate on selection. Keep only the active session and recently-used sessions live. |
| 20+ concurrent sessions | Unlikely for a desktop ADE. If needed, virtualize the sidebar list and use lazy session loading. |

### Scaling Priorities

1. **First bottleneck: Memory from accumulated turns.** Each session accumulates `Turn` objects with full text content and tool outputs. For long-running sessions with many tool calls, this grows. Mitigation: truncate tool output after a size threshold (keep summary, drop full output). The current `Turn` model already stores tool output as `String` -- add a `truncated: bool` flag.

2. **Second bottleneck: Rendering cost of large turn lists.** GPUI's `ListState` already virtualizes rendering (only visible items are laid out). This is not a bottleneck as long as the list is used. The existing codebase already uses `list()` correctly.

3. **Third bottleneck: File system from worktrees.** Each worktree is a shallow copy of the repo. For large monorepos this can be significant. Mitigation: clean up worktrees when sessions are closed; offer a "clean all worktrees" command.

## Anti-Patterns

### Anti-Pattern 1: Shared Mutable State Between Sessions

**What people do:** Put all sessions' turns in a single `Vec<Turn>` indexed by session ID, or use `Arc<Mutex<>>` to share state between the streaming thread and the UI.
**Why it's wrong:** GPUI's entity model already solves ownership. Using `Arc<Mutex<>>` bypasses GPUI's notification system, causing stale renders. Indexing into a shared vec creates coupling between sessions.
**Do this instead:** One `Entity<Session>` per session, each owning its own `Vec<Turn>`. The GPUI entity system handles the locking internally.

### Anti-Pattern 2: Direct Cross-Component Method Calls

**What people do:** `self.chat_view.update(cx, |chat, cx| chat.add_turn(...))` from inside the session manager.
**Why it's wrong:** Creates tight coupling between domain logic and UI. Makes testing harder. Breaks when UI components are refactored.
**Do this instead:** Session emits events via `cx.emit()`. UI components subscribe and react independently. The session never knows about the chat view.

### Anti-Pattern 3: Polling Multiple Channels in One Task

**What people do:** Create a single foreground task that polls receivers from all active sessions, dispatching events to the right session.
**Why it's wrong:** Complexity explosion. A slow session blocks event delivery for fast sessions. Error in one session's polling affects all sessions.
**Do this instead:** Each `Entity<Session>` spawns its own foreground polling task. Tasks are independent. When a session is dropped, its task is dropped automatically.

### Anti-Pattern 4: Blocking Git Operations on the Main Thread

**What people do:** Call `git worktree add` synchronously during session creation, freezing the UI.
**Why it's wrong:** Git operations on large repos can take seconds. The UI must remain responsive.
**Do this instead:** Use `cx.background_spawn()` for all git operations. Show a "creating worktree..." status in the session. The session starts in a `Preparing` state and transitions to `Ready` when the worktree is available.

### Anti-Pattern 5: Monolithic Provider Module

**What people do:** Add Codex parsing logic alongside Claude parsing logic in a single `provider.rs` with match arms for different providers.
**Why it's wrong:** Two completely different streaming protocols (Claude's stream-json vs. Codex's JSONL) with different event schemas. A single module becomes unmaintainable.
**Do this instead:** Provider trait in `provider/mod.rs`, separate implementation files (`claude.rs`, `codex.rs`). Each parser is self-contained. Registration happens in `ProviderRegistry`.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| Claude Code CLI | Subprocess via `std::process::Command`, stdout parsed as stream-json (NDJSON). Flags: `-p <prompt> --output-format stream-json --verbose` | Already implemented in current codebase. Needs `--continue` / `--resume <session-id>` for multi-turn within the same CLI session. Working directory set to worktree path. |
| Codex CLI | Subprocess via `codex exec --json <prompt>`. Stdout is JSONL with `thread.started`, `item.started`, `item.completed`, `turn.completed` events. | Item types map to ProviderEvent: `agent_message` -> TextDelta, `command_execution` -> ToolUse/ToolResult. Working directory set to worktree path. |
| Git (worktrees) | Shell out to `git worktree add/list/remove`. Prefer `std::process::Command` over `git2` crate -- the git2 Worktree API is limited and does not support all worktree operations cleanly. | Run on background threads. Parse `git worktree list --porcelain` for structured output. `git branch -v` and `git diff --stat` for sidebar info. |
| Git (status/diff) | `git diff --stat`, `git log --oneline -5`, `git branch --show-current` per worktree | Polled periodically or after session completes a turn. Results displayed in sidebar and context panel. |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| HelmWorkspace <-> Sidebar | `EventEmitter<SidebarEvent>` for session/project selection; Sidebar reads `ProjectManager` and `SessionManager` state | Sidebar emits selection events; Workspace mediates by updating `active_session` |
| HelmWorkspace <-> ActiveSessionView | Workspace calls `bind_to_session()` when active session changes | Chat view subscribes to session events, re-renders on streaming updates |
| HelmWorkspace <-> ContextPanel | Same pattern as ActiveSessionView | Context panel shows plan and activity log from the active session |
| Session <-> Provider | `mpsc::channel<ProviderEvent>` | Provider runs on background thread, Session polls on foreground task |
| SessionManager <-> WorktreeManager | Direct function calls (both are domain-layer) | WorktreeManager returns `Result<PathBuf>` for created worktree path |
| Session <-> UI components | `EventEmitter<SessionEvent>` + `cx.subscribe()` | Session never references UI types; UI subscribes to session events |

## Build Order Implications

The architecture has clear dependency layers that determine build order:

1. **Foundation (no internal dependencies):**
   - `domain/turn.rs` -- Turn and ContentBlock data model (extract from current `chat.rs`)
   - `provider/mod.rs` -- ProviderEvent enum, Provider trait definition
   - `theme.rs` -- already exists

2. **Provider implementations (depends on Foundation):**
   - `provider/claude.rs` -- refactor current `provider.rs` into trait impl
   - `provider/codex.rs` -- new, implements Provider trait for Codex JSONL

3. **Domain entities (depends on Foundation + Providers):**
   - `domain/project.rs` -- Project struct, ProjectManager entity
   - `domain/session.rs` -- Session entity with streaming state machine
   - `git/worktree.rs` -- Worktree creation/cleanup

4. **UI components (depends on Domain):**
   - `ui/sidebar.rs` -- project tree, session list (refactor existing)
   - `ui/chat_view.rs` -- active session chat (refactor existing `chat.rs`)
   - `ui/context_panel.rs` -- plan panel, activity log (refactor existing `plan.rs`)
   - `ui/markdown.rs` -- streaming markdown renderer
   - `ui/diff_view.rs` -- inline diff display

5. **Workspace integration (depends on everything):**
   - `workspace.rs` -- wire up all components, handle navigation

**Key dependency insight:** The Provider trait and Session entity are the critical path. Everything else (UI components, git integration) can be built in parallel once these two are stable. The sidebar and chat view are the most complex UI components and will take the most time. Markdown rendering and diff display are isolated -- they can be developed independently and plugged in.

**Build order recommendation for roadmap phases:**
1. Extract data model + define Provider trait (foundation)
2. Refactor existing Claude provider into trait impl + build Session entity
3. Multi-project/session domain layer (ProjectManager, SessionManager, WorktreeManager)
4. Rewire UI to use new domain layer (sidebar, chat view, context panel)
5. Add Codex provider, markdown rendering, diff views (parallel work)

## Sources

- [GPUI ownership and data flow (Zed blog)](https://zed.dev/blog/gpui-ownership) -- Entity model, EventEmitter, effect queue, subscription patterns
- [Zed workspace crate source](https://github.com/zed-industries/zed/tree/main/crates/workspace) -- Multi-workspace pattern, Pane/PaneGroup, Panel trait, Sidebar trait
- [Claude Code CLI headless/programmatic docs](https://code.claude.com/docs/en/headless) -- stream-json output format, -p flag, --continue/--resume for multi-turn
- [Codex CLI reference](https://developers.openai.com/codex/cli/reference) -- exec --json JSONL format, app-server JSONL-over-stdio
- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive) -- JSONL event types (thread.started, item.started, item.completed, turn.completed)
- [git2 Worktree struct docs](https://docs.rs/git2/latest/git2/struct.Worktree.html) -- Limited API, prefer CLI for full worktree operations
- [Git worktree documentation](https://git-scm.com/docs/git-worktree) -- Official git worktree reference

---
*Architecture research for: Helm multi-project ADE*
*Researched: 2026-03-26*

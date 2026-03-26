# Project Research Summary

**Project:** Helm -- Native Desktop Agentic Development Environment
**Domain:** Multi-project ADE (Agentic Development Environment) built with Rust/GPUI
**Researched:** 2026-03-26
**Confidence:** MEDIUM-HIGH

## Executive Summary

Helm is a native, GPU-accelerated desktop application for orchestrating multiple AI coding agents in parallel, each isolated in its own git worktree. The ADE market in 2026 is dominated by Electron apps (Conductor, Emdash, T3 Code, Superset). Helm's GPUI foundation is its singular architectural advantage -- instant startup, zero-lag scrolling, sub-millisecond workspace switching -- and every design decision should reinforce that native performance story rather than chase feature parity with Electron competitors. The existing codebase has a working single-chat UI with a Claude Code CLI integration, a custom text input widget, and a three-panel layout scaffold. The path forward is to refactor this foundation into a multi-session architecture with proper process lifecycle management, then layer on rich rendering and provider breadth.

The recommended approach is to build bottom-up along the architecture's dependency chain: stabilize the data model and provider trait first, wire up git worktree isolation and session management second, then deliver the rendering layer (streaming markdown, inline diffs, tool call blocks) third. This ordering is driven by two forces: (1) the architecture research shows clear dependency layers where the Provider trait and Session entity are the critical path that unblocks all other work, and (2) the pitfall research reveals that process lifecycle management (zombie processes, pipe deadlocks) and conversation context loss are bugs that already exist in the current codebase and will compound with every session added.

The primary risks are: streaming markdown rendering complexity (the hardest single component, requiring incremental parsing with block caching to avoid O(n^2) re-parses and visual flicker), git worktree corruption from concurrent operations (requiring a serialized command queue per repository), and provider abstraction convergence on a lowest-common-denominator that strips Claude and Codex of their unique capabilities. All three are solvable with the patterns identified in research, but all three require correct upfront design -- they cannot be patched onto naive implementations after the fact.

## Key Findings

### Recommended Stack

The existing stack is locked: Rust (Edition 2024), GPUI 0.2.2 (local path dep from Zed), smol 2.0 (async runtime), serde/serde_json, anyhow. All new dependencies must be smol-compatible -- tokio is explicitly forbidden due to dual-runtime conflicts with GPUI.

**Core additions:**
- **pulldown-cmark 0.13** (markdown parsing) -- Zed uses this exact version; pull-based parser is ideal for incremental streaming rendering into GPUI elements
- **tree-sitter 0.26** (syntax highlighting) -- matches Zed's version to avoid linking conflicts; load grammars for the ~8 languages agents commonly output; syntect 5.3 is a viable fallback if tree-sitter integration proves too heavy for the first milestone
- **git2 0.20** (git read operations) -- proven worktree API, used by Zed and cargo; use vendored-libgit2 to avoid system version mismatches; shell out to `git` CLI for write operations (worktree add/remove, push, fetch)
- **similar 2.7** (diff display) -- best high-level Rust diff library for UI-oriented display; supports word-level inline change emphasis
- **smol::process** (subprocess management) -- already available via smol 2.0; replaces current std::process + mpsc polling pattern with proper async subprocess lifecycle
- **futures 0.3** (async combinators) -- stream/future composition for provider output pipelines

**Critical constraint:** Do NOT import Zed's internal crates (markdown: 31 deps, language: 74 deps, git: 37 deps). Build lighter equivalents using the same underlying libraries.

### Expected Features

**Must have (table stakes for 2026 ADE launch):**
- Streaming chat with markdown rendering and syntax-highlighted code blocks
- Expandable tool call blocks (collapsible UI showing agent actions)
- Inline diff display with add/remove coloring
- Multi-project sidebar with workspace status indicators
- Git worktree isolation per workspace (automatic creation/cleanup)
- Claude Code + Codex CLI providers with a shared abstraction trait
- Custom text input with IME, clipboard, multi-line support
- Session persistence (survive app restart)
- Keyboard-driven workflow with command palette
- Three-panel layout (sidebar, chat, context/plan panel)

**Should have (differentiators unique to Helm):**
- Native GPU-accelerated rendering performance (inherent, but must be marketed)
- Collapsible cross-workspace activity log (no competitor has this)
- Context/plan panel as a dedicated third panel (no competitor has this)
- Checkpoint and revert (snapshot git state per agent turn)
- Workspace forking (branch work while preserving history)
- Smart workspace status grouping (by status, project, or branch)

**Defer to v2+:**
- PR creation workflow (GitHub API auth is large scope)
- Issue tracker integration (Linear, Jira, GitHub -- each is its own maintenance burden)
- Built-in code editor (enormous surface area; use "open in Zed/VS Code" instead)
- Background/cloud agents, event-driven automations (different product category)
- Browser/web preview, MCP server hosting, voice input, real-time collaboration

### Architecture Approach

The architecture follows GPUI's entity-based ownership model with clear separation between UI components, domain logic, and process/IO. The central abstraction is Entity-per-Session: each agent conversation is an independent `Entity<Session>` that owns its turns, provider handle, worktree path, and streaming state. The workspace tracks which session is "active" and mediates selection changes via EventEmitter fan-out. All cross-component coordination flows through GPUI's `cx.emit()` + `cx.subscribe()` pattern, never through direct method calls between components.

**Major components:**
1. **HelmWorkspace** -- top-level layout; owns sidebar, chat view, context panel; routes session selection
2. **Session (Entity)** -- central domain object; owns turns, provider, worktree path, streaming state machine; emits SessionEvent for UI subscribers
3. **Provider trait + Registry** -- abstracts CLI agent lifecycle (spawn, stream, cancel, resume); separate implementations for Claude (stream-json) and Codex (JSONL)
4. **ProjectManager / SessionManager** -- CRUD for projects and sessions; SessionManager coordinates with WorktreeManager for worktree creation
5. **WorktreeManager** -- creates/lists/removes git worktrees; serializes git operations per repository to prevent corruption
6. **Streaming Markdown Renderer** -- converts pulldown-cmark events into GPUI elements; incremental parsing with block caching
7. **Sidebar / ChatView / ContextPanel** -- UI components that subscribe to domain entity events and re-render independently

### Critical Pitfalls

1. **Zombie CLI processes** -- current codebase uses `.detach()` on background tasks and never kills child processes on drop. Fix: store Child handles in a process registry, implement Drop with SIGTERM+timeout+SIGKILL, replace `.detach()` with held Task handles. Must fix in foundation phase before scaling to N concurrent agents.

2. **Streaming markdown O(n^2) re-parse and flicker** -- re-parsing the entire response on each token is quadratic; incomplete code fences cause visual flicker. Fix: incremental parsing (only re-parse last incomplete block), batch render updates (every 80 chars or 100ms), cache completed block element trees. Must design correctly upfront in the rendering phase.

3. **Git worktree corruption from concurrent operations** -- multiple agents accessing shared `.git` directory cause index lock collisions and corrupt refs. Fix: serialize all git operations through a single async queue per repository, generate unique branch names per workspace, implement stale lock detection and cleanup.

4. **Conversation context loss between turns** -- each `claude -p` invocation spawns a fresh subprocess with no memory. Fix: capture `session_id` from stream-json output, use `--continue`/`--resume` flags on subsequent turns, store session IDs per workspace.

5. **Provider abstraction too narrow** -- forcing Claude and Codex into identical event types strips provider-specific features (session resume, exec mode, tool approval flows). Fix: define the trait at the session/lifecycle level, use provider-specific event types internally, add a `capabilities()` method, normalize at the display layer not the data layer.

## Implications for Roadmap

Based on combined research, the architecture's dependency layers and the pitfall severity mapping suggest a 6-phase structure.

### Phase 1: Foundation and Process Lifecycle
**Rationale:** The architecture research identifies the data model and Provider trait as the critical path that unblocks everything. The pitfalls research shows zombie processes and conversation context loss are existing bugs that compound with scale. Fix these first.
**Delivers:** Extracted data model (Turn, ContentBlock), Provider trait definition, process registry with proper lifecycle management (kill-on-drop), Claude provider refactored to implement the trait with session continuity via `--continue`/`--resume`.
**Addresses features:** Provider abstraction trait, Claude Code CLI provider (upgraded with session continuity)
**Avoids pitfalls:** Zombie CLI processes (#1), conversation context loss (#7), pipe buffer concerns (#2 -- by establishing dedicated-reader-per-subprocess pattern)
**Stack:** smol::process (replaces std::process), futures 0.3

### Phase 2: Multi-Session Domain Layer
**Rationale:** Architecture shows Session, ProjectManager, SessionManager, and WorktreeManager form the domain layer that all UI depends on. This must be solid before UI work begins.
**Delivers:** Entity<Session> with streaming state machine, ProjectManager, SessionManager, WorktreeManager with serialized git command queue, worktree creation/cleanup lifecycle.
**Addresses features:** Multi-project sidebar (data layer), git worktree isolation, branch display (data layer), session persistence (serialization layer)
**Avoids pitfalls:** Git worktree corruption (#3 -- serialized command queue, unique branch names, stale lock detection), entity lifecycle mismanagement (#6 -- establish ownership documentation and subscription audit from this phase onward)
**Stack:** git2 0.20, uuid 1.0, dirs 5.0

### Phase 3: UI Rewire and Layout
**Rationale:** With the domain layer stable, rewire the existing UI components to use the new entity architecture. The three-panel layout, sidebar, and session switching are structural prerequisites for the rendering work in Phase 4.
**Delivers:** HelmWorkspace with EventEmitter-based coordination, Sidebar with project tree and session list, ActiveSessionView that binds to any Entity<Session>, ContextPanel scaffold, draggable panel dividers, keyboard shortcuts and command palette.
**Addresses features:** Multi-project sidebar (UI), three-panel layout, keyboard-driven workflow, provider switching UI, session selection/switching
**Avoids pitfalls:** Entity subscription loss (#6 -- enforce naming convention for subscription fields, audit all cx.subscribe calls)
**Stack:** No new dependencies; GPUI patterns only

### Phase 4: Rich Chat Rendering
**Rationale:** This is the hardest and highest-value table-stakes feature. It requires the streaming pipeline from Phase 1 and the session architecture from Phase 2. The markdown renderer, diff view, and tool call blocks are the primary UX surface -- users will not adopt without these.
**Delivers:** Streaming markdown renderer with incremental parsing and block caching, syntax-highlighted code blocks, expandable tool call blocks, inline diff display (unified format).
**Addresses features:** Streaming markdown chat rendering, expandable tool call blocks, inline diff display, theme system (syntax highlighting colors, diff coloring)
**Avoids pitfalls:** Markdown O(n^2) re-parse and flicker (#4 -- incremental parsing, batched updates, incomplete-block handling), Vec<Turn> clone per render frame (performance trap -- switch to indexed access)
**Stack:** pulldown-cmark 0.13, tree-sitter 0.26 (or syntect 5.3 fallback), similar 2.7, theme.rs expansion

### Phase 5: Second Provider and Polish
**Rationale:** Adding Codex validates the provider abstraction from Phase 1. This is intentionally after the rendering layer so the same UI can display both providers' output. Polish items (session persistence to disk, text input improvements, error recovery) round out the v1 experience.
**Delivers:** Codex CLI provider implementation, full session persistence (workspace state + chat history to disk), text input with IME and multi-line support, error recovery (retry button on failed turns), cancel/abort with proper subprocess cleanup.
**Addresses features:** Codex CLI provider, session persistence, custom text input (polished), provider switching
**Avoids pitfalls:** Provider abstraction too narrow (#5 -- Codex integration tests the trait; if it requires modifying ProviderEvent, the trait was designed wrong)
**Stack:** No new dependencies; Codex JSONL parser is serde_json (already in stack)

### Phase 6: Differentiators (v1.x)
**Rationale:** After core v1 is validated, add the features that separate Helm from competitors. These are all additive and do not require architectural changes.
**Delivers:** Cross-workspace activity log, context/plan panel with structured agent plan display, checkpoint and revert (git snapshots per turn), workspace status grouping, side-by-side diff mode, chat search.
**Addresses features:** All "should have" differentiators from feature research
**Avoids pitfalls:** None new -- all foundational pitfalls addressed in earlier phases

### Phase Ordering Rationale

- **Foundation before domain before UI:** The architecture research shows clear dependency layers. The Provider trait and Session entity are the critical path. Building UI first would require rework when the domain layer changes.
- **Process lifecycle before multi-session:** The pitfalls research shows zombie processes and context loss exist in the single-session codebase today. Multiplying sessions without fixing these creates exponential problems.
- **Git worktree management alongside domain layer:** Worktree isolation is inseparable from session creation -- the architecture has SessionManager calling WorktreeManager during `create_session()`. These must be designed together.
- **Rendering after session architecture:** The markdown renderer, diff view, and tool call blocks all consume Session data. The data model must be stable before building the rendering pipeline.
- **Second provider after rendering:** Adding Codex validates the abstraction, but testing that abstraction requires visible output. With rendering in place, both providers can be validated end-to-end.
- **Differentiators last:** Activity log, checkpoints, and workspace forking are additive features that build on the multi-session architecture without changing it.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 1 (Foundation):** Claude CLI `--continue`/`--resume` behavior needs hands-on testing -- documentation is clear but edge cases (session expiry, worktree-crossing sessions) are uncertain.
- **Phase 2 (Domain Layer):** Git worktree lifecycle across edge cases (large monorepos, shallow clones, submodules) may surface issues not covered in research. The serialized command queue design needs benchmarking to ensure it does not bottleneck agent throughput.
- **Phase 4 (Rich Rendering):** Streaming markdown rendering is the most complex single component. The incremental parsing strategy needs prototyping before committing to a design. Tree-sitter grammar loading and highlight query integration with GPUI's TextRun system needs spike work. Consider running `/gsd:research-phase` for this phase.

Phases with standard patterns (skip deep research):
- **Phase 3 (UI Rewire):** GPUI entity patterns are well-documented in Zed blog posts and the existing codebase. EventEmitter fan-out and subscription management are established patterns.
- **Phase 5 (Second Provider):** Codex CLI JSONL format is well-documented. The implementation is a straightforward parser mapping to the existing trait.
- **Phase 6 (Differentiators):** Activity log, grouping, and search are standard CRUD/filter patterns with no novel technical challenges.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM-HIGH | Core stack locked and verified against Zed upstream. New additions confirmed on crates.io with compatibility checks. Tree-sitter integration complexity is the main uncertainty -- syntect fallback mitigates. |
| Features | HIGH | Competitive landscape documented across 6 competitors. Table stakes clear. MVP aligns with PROJECT.md. Feature dependency graph complete. |
| Architecture | HIGH | Patterns derived from GPUI source, Zed codebase, and existing Helm code. Entity model and EventEmitter patterns proven at scale in Zed. Build order from dependency analysis. |
| Pitfalls | MEDIUM-HIGH | Critical pitfalls well-documented across multiple sources. GPUI-specific issues inferred from Zed blog posts since external GPUI docs are sparse. |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- **GPUI text rendering performance with streaming markdown:** No benchmarks exist for rendering complex markdown elements (tables, nested lists, code blocks with highlighting) at streaming rates in GPUI. Needs profiling during Phase 4.
- **Tree-sitter grammar loading in GPUI context:** Zed loads grammars through its language crate (74 deps). Helm needs a lightweight alternative. Exact API surface for loading grammars, running highlight queries, and mapping captures to GPUI TextRun styles needs spike work.
- **Codex CLI `--json` edge cases:** JSONL format and event types confirmed, but hands-on testing with real Codex sessions is needed to validate the parser handles all edge cases.
- **Session persistence format:** SQLite vs JSON files not decided. SQLite is better for queryability and crash recovery; JSON is simpler and human-readable. Decide during Phase 2 planning.
- **macOS Keychain integration:** Identified as security best practice but no Rust crate evaluated. Evaluate `security-framework` crate during Phase 2.

## Sources

### Primary (HIGH confidence)
- Zed Cargo.toml and crate source code -- confirmed versions for pulldown-cmark 0.13, git2 0.20, tree-sitter 0.26, smol 2.0
- [GPUI ownership and data flow (Zed blog)](https://zed.dev/blog/gpui-ownership) -- entity model, EventEmitter, subscription lifecycle
- [Claude Code CLI programmatic docs](https://code.claude.com/docs/en/headless) -- stream-json format, --continue/--resume, session management
- [Codex CLI reference](https://developers.openai.com/codex/cli/reference) -- JSONL format, exec mode, event types
- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive) -- JSONL event schema
- crates.io version verification -- pulldown-cmark 0.13.0, git2 0.20.4, tree-sitter 0.26.7, similar 2.7.0, syntect 5.3.0

### Secondary (MEDIUM confidence)
- [Conductor](https://www.conductor.build/), [Emdash](https://docs.emdash.sh/), [T3 Code](https://github.com/pingdotgg/t3code), [Superset](https://superset.sh/) -- competitive feature analysis
- [Chrome LLM response rendering best practices](https://developer.chrome.com/docs/ai/render-llm-responses) -- streaming markdown batching strategy
- [Git worktree concurrent operations](https://www.termdock.com/en/blog/git-worktree-conflicts-ai-agents) -- conflict types with diagnosis
- Helm codebase analysis -- existing patterns, tech debt, CONCERNS.md cross-reference

### Tertiary (LOW confidence)
- Tree-sitter highlight API stability (0.26.3) -- API may change before Helm ships
- gitoxide/gix worktree maturity assessment -- evaluated and deferred, may be worth revisiting late 2026

---
*Research completed: 2026-03-26*
*Ready for roadmap: yes*

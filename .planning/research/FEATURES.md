# Feature Research

**Domain:** Agentic Development Environment (ADE) -- Native Desktop GUI for Parallel Coding Agents
**Researched:** 2026-03-26
**Confidence:** HIGH

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist in any ADE/agent GUI shipping in 2026. Missing these means users immediately bounce to Conductor, Emdash, T3 Code, or Superset.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Streaming chat with markdown rendering | Every ADE renders agent responses as streaming markdown with code blocks and syntax highlighting. Claude Code Desktop, T3 Code, Conductor all do this. Users will not accept raw text. | HIGH | GPUI has no webview -- must implement markdown-to-elements renderer natively. Code blocks need syntax highlighting (tree-sitter). Streaming means incremental parse + render. This is the hardest table-stakes feature. |
| Expandable tool call blocks | Conductor, T3 Code, and Claude Desktop all show agent tool invocations (file reads, edits, shell commands) as collapsible blocks in the chat timeline. Users need to see what the agent did without reading raw JSON. | MEDIUM | Parse tool_use/tool_result events from CLI stream. Render as collapsible UI elements with icon, tool name, summary line, and expandable detail. |
| Inline diff display | Every competitor shows code changes with syntax-highlighted diffs inline in chat. Conductor has turn-by-turn diffs. T3 Code has per-turn diff viewer. Emdash has side-by-side diff view. | HIGH | Unified diff parsing + syntax-highlighted rendering in GPUI. Need both inline (unified) and side-by-side (split) modes. Must handle large diffs with virtualized scrolling. |
| Multi-project sidebar | Conductor's core UX. Emdash has project-level organization. Users managing multiple agent tasks across repos need a project list with workspace counts and status indicators. Without this, Helm is just another single-chat wrapper. | MEDIUM | List of projects with expand/collapse. Each project shows its workspaces. Status indicators (running, waiting, done, error). This is already in PROJECT.md requirements. |
| Git worktree isolation per workspace | Conductor, Emdash, T3 Code, and Superset all isolate each agent task in its own git worktree. This is the defining architectural pattern of the ADE category -- not optional. | MEDIUM | Shell out to `git worktree add/remove`. Track worktree paths per workspace. Clean up on workspace archive. Already in PROJECT.md requirements. |
| Branch display and status per workspace | Conductor shows branch name, PR status (merged, failing CI, conflicts) in the sidebar. T3 Code shows PR icons on threads. Users need to see git state at a glance. | LOW | Read `git branch`, `git status`, `git log` per worktree. Display branch name and basic status in sidebar workspace row. |
| Custom text input with full editing | IME support, clipboard, selection, cursor movement, undo -- standard text input behavior. Conductor has a composer with @-mentions and search. T3 Code has multi-line input with mode selectors. | HIGH | Already in PROJECT.md requirements. GPUI text input is complex (no native text field). Must handle IME for international users, clipboard for code pasting. |
| Provider switching (at least 2 providers) | Emdash supports 22 CLI agents. Conductor supports Claude + Codex. T3 Code supports Codex + Claude. Single-provider lock-in is unacceptable. | MEDIUM | Provider trait abstraction. Parse different CLI output formats. UI selector for provider per workspace. Already planned for Claude Code + Codex. |
| Keyboard-driven workflow | Conductor has extensive shortcuts (Cmd+T new chat, Cmd+N new workspace, Cmd+K command palette, Cmd+Shift+R review). T3 Code has custom actions with keybindings. Power users expect keyboard-first interaction. | MEDIUM | Keymap system with Zed-style key binding configuration. Command palette for discoverability. Already in PROJECT.md as Zed-style key bindings. |
| Session persistence | Users expect to close the app and reopen with all workspaces, chat history, and state intact. Conductor archives workspaces with chat history. T3 Code persists threads. | MEDIUM | Serialize workspace state, chat messages, and provider sessions to disk. Restore on launch. SQLite or JSON files per workspace. |

### Differentiators (Competitive Advantage)

Features that would set Helm apart from Conductor, Emdash, T3 Code, and Superset. These align with Helm's core value of native GPU-accelerated performance.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Native GPU-accelerated rendering | Every competitor is Electron (Conductor, T3 Code, Superset) or web-based (Emdash). Helm is the only native Rust/GPUI ADE. Instant startup, zero-lag scrolling through long chat histories, smooth streaming rendering. This is the core differentiator. | LOW (inherent) | GPUI provides this by default. The differentiator is architectural -- it's why users would choose Helm over Electron alternatives. Marketing-level advantage. |
| Sub-millisecond workspace switching | Electron apps have noticeable lag switching between workspace tabs. GPUI's entity model means switching is a pointer swap with immediate re-render. | LOW (inherent) | Again, inherent to the architecture. Competitors choke on 10+ workspace tabs. |
| Collapsible activity log / work journal | A structured, filterable log of all agent actions across all workspaces -- not just chat messages, but a timeline of file changes, commands run, errors encountered. Conductor has basic workspace history. No competitor has a unified cross-workspace activity view. | MEDIUM | Aggregate tool_use events from all workspaces into a unified timeline. Filter by workspace, event type, time range. Already in PROJECT.md as "collapsible work log / activity feed." |
| Context/plan panel (third panel) | Conductor and T3 Code are two-panel (sidebar + chat). Helm's three-panel layout adds a dedicated context/plan panel for showing the agent's current plan, file context, or reference docs. This reduces the need to scroll through chat to understand what the agent is doing. | MEDIUM | Third panel renders structured plan data, file previews, or reference content. Can show agent's "thinking" separately from the conversation. Already in PROJECT.md as three-panel layout. |
| Best-of-N comparison view | Emdash has this but it's web-based. Running the same task against multiple providers (Claude vs Codex) and comparing results side-by-side is powerful for quality-sensitive work. Native rendering makes diff comparison smoother. | HIGH | Requires: multiple provider spawning for same task, result collection, side-by-side diff rendering. Defer to v1.x -- needs solid multi-provider foundation first. |
| Checkpoint and revert | Conductor has "resume from checkpoints" -- revert to a previous turn in the conversation and branch from there. Critical for agent experimentation. T3 Code has worktree-level git checkpointing. | MEDIUM | Snapshot git state at each agent turn. Allow reverting worktree to any previous snapshot. Store chat history branches. |
| Workspace forking | Conductor supports forking a workspace -- branch the work while preserving chat history. Allows experimentation without losing the original path. | MEDIUM | Clone worktree, duplicate chat state, create new workspace entry pointing to forked branch. |
| Smart workspace status grouping | Conductor groups workspaces by status (backlog, in progress, in review, done) and by repository. Emdash has kanban view. Grouping + filtering reduces cognitive load when managing 5+ parallel agents. | LOW | Group-by selector in sidebar: status, project, branch. Collapsible groups with count badges. |

### Anti-Features (Commonly Requested, Often Problematic)

Features to deliberately NOT build. These create complexity disproportionate to value, or conflict with Helm's design philosophy.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Built-in code editor | Users want to edit agent-generated code without leaving the app. Emdash has an integrated file editor. Superset has a built-in editor. | A code editor is an enormous surface area (syntax highlighting for 30+ languages, LSP integration, keybindings, extensions). Helm's value is orchestrating agents, not replacing Zed/VS Code. Emdash's editor is basic and frustrating compared to a real editor. | "Open in editor" button that launches Zed/VS Code/Cursor at the exact file and line. Conductor does this well. PROJECT.md already excludes this. |
| Full PR workflow (create, CI status, merge) | Conductor has create/merge/CI-check for PRs. T3 Code has commit-push-PR in one action. Users want end-to-end flow. | PR creation requires GitHub API integration, auth management, CI polling, merge conflict resolution UI. Massive scope. For v1, showing diffs and branch status is sufficient. | Show branch name, diff stats, and a "copy branch name" or "open in GitHub" action. Add PR workflow in v2 when core is solid. Already scoped out in PROJECT.md. |
| Issue tracker integration (Linear, Jira, GitHub Issues) | Emdash pulls tasks from Linear/Jira/GitHub. Conductor is adding Linear integration. Users want to assign tickets to agents. | Each integration is its own auth flow, API, data model, and sync logic. Three integrations = triple the maintenance. For a v1 native app, this is scope explosion. | Manual task description in workspace creation. Users copy-paste from their issue tracker. Add integrations in v2. Already scoped out in PROJECT.md. |
| Background/cloud agents | Cursor Automations run agents on cloud sandboxes triggered by events. Users want agents working while they sleep. | Requires cloud infrastructure, sandboxing, billing, security. Completely different product category. Helm is local-first by design. | Show notification when local agent needs attention. Users can leave agents running in background workspaces and switch back when ready. |
| Event-driven automations | Cursor's automations trigger on Slack messages, GitHub events, PagerDuty alerts. Power users want always-on agents. | Requires webhook server, event routing, cloud execution. Not a desktop app feature -- it's a platform feature. | Desktop app can watch for file changes or git events locally, but not remote event sources. Defer entirely. |
| Browser/web preview | Superset has an in-app browser for dev servers. Emdash has browser preview. Useful for frontend work. | Embedding a browser in a native GPUI app means either a webview dependency (breaking the "no web technologies" constraint) or building a rendering engine. Neither is viable. | Port forwarding awareness -- detect when agent starts a dev server and show a "Open in browser" button that launches the system browser. |
| MCP server hosting | Emdash and Cursor support MCP tool integration. Users want to connect external tools via MCP. | MCP server integration adds protocol handling, tool discovery, permission management. For v1, the CLI agents themselves handle MCP -- Helm doesn't need to be in the middle. | CLI agents (Claude Code, Codex) already support MCP. Users configure MCP at the agent level, not the GUI level. Helm passes through. |
| Voice input / push-to-talk | Claude Code Desktop has voice mode with push-to-talk. Novel but niche. | Audio capture, speech-to-text integration, microphone permissions. Significant platform-specific complexity for a feature most developers won't use daily. | Keyboard-first input. If voice is wanted later, can integrate system-level dictation (macOS Dictation is already available system-wide). |
| Real-time collaboration | Multiple users viewing the same workspace simultaneously. | Requires CRDT/OT for state sync, networking, user identity, presence indicators. Completely different product category. | Single-user desktop app. Share results via git branches and PRs. |

## Feature Dependencies

```
[Provider Abstraction Layer]
    |
    +--requires--> [CLI Streaming Parser (Claude)]
    +--requires--> [CLI Streaming Parser (Codex)]
    |
    +--enables--> [Provider Switching UI]
    +--enables--> [Best-of-N Comparison] (v1.x)

[Git Worktree Management]
    |
    +--requires--> [Git CLI Integration]
    |
    +--enables--> [Workspace Isolation]
    +--enables--> [Branch Display in Sidebar]
    +--enables--> [Diff Stats per Workspace]
    +--enables--> [Checkpoint/Revert]
    +--enables--> [Workspace Forking]

[Streaming Markdown Renderer]
    |
    +--requires--> [Incremental Markdown Parser]
    +--requires--> [Syntax Highlighting (tree-sitter)]
    |
    +--enables--> [Rich Chat Rendering]
    +--enables--> [Inline Diff Display]
    +--enables--> [Tool Call Block Rendering]

[Multi-Project Sidebar]
    |
    +--requires--> [Project Configuration/Storage]
    +--requires--> [Workspace State Management]
    |
    +--enables--> [Workspace Status Grouping]
    +--enables--> [Cross-Workspace Activity Log]

[Custom Text Input]
    |
    +--requires--> [IME Handling]
    +--requires--> [Clipboard Integration]
    |
    +--enables--> [Chat Composer]
    +--enables--> [Command Palette]

[Session Persistence]
    |
    +--requires--> [Serialization Layer]
    +--requires--> [Chat Message Storage]
    |
    +--enables--> [Workspace Archival/Restore]
    +--enables--> [Cross-Session History]

[Theme System]
    |
    +--enables--> [Syntax Highlighting Colors]
    +--enables--> [Diff Coloring (add/remove)]
    +--enables--> [UI Consistency]
```

### Dependency Notes

- **Provider Abstraction requires CLI Parsers:** Cannot switch providers without parsing each CLI's output format. Claude Code outputs `stream-json`, Codex uses JSON-RPC. The trait must normalize these into a common event stream.
- **Inline Diff Display requires Streaming Markdown Renderer:** Diffs appear inline in chat messages. The markdown renderer must support diff code blocks with syntax highlighting and add/remove coloring.
- **Checkpoint/Revert requires Git Worktree Management:** Checkpoints are git snapshots. Reverting means resetting the worktree to a prior commit. Must track commit hashes per agent turn.
- **Best-of-N requires Provider Abstraction:** Running the same task on multiple providers requires the abstraction layer to be solid and the result format to be normalized for comparison.
- **Activity Log requires Multi-Project Sidebar:** The log aggregates events across workspaces, which are organized under projects in the sidebar.
- **Command Palette requires Custom Text Input:** The palette is a text input with fuzzy search. Depends on the same input infrastructure as the chat composer.

## MVP Definition

### Launch With (v1)

Minimum viable product -- what makes Helm usable enough that a developer would choose it over opening multiple terminal tabs.

- [ ] **Streaming markdown chat rendering** -- The core interaction. Agent responses render as formatted markdown with code blocks and syntax highlighting as they stream in. Without this, users stay in the terminal.
- [ ] **Expandable tool call blocks** -- Show what the agent is doing (reading files, editing, running commands) without drowning the user in raw output. Collapsed by default, expandable for detail.
- [ ] **Inline diff display** -- When the agent edits files, show the diff inline in chat with syntax highlighting. Unified format minimum, side-by-side as stretch goal.
- [ ] **Multi-project sidebar with workspaces** -- List projects, create workspaces under them, see workspace status (running/waiting/done/error) at a glance.
- [ ] **Git worktree creation per workspace** -- Each workspace gets its own worktree automatically. Agent operates in isolation. Branch name visible in sidebar.
- [ ] **Claude Code CLI provider** -- Parse `stream-json` output, send messages, handle tool approvals. This is the primary provider.
- [ ] **Codex CLI provider** -- Second provider via JSON-RPC. Proves the abstraction layer works.
- [ ] **Provider abstraction trait** -- Common interface that both providers implement. Provider selector per workspace.
- [ ] **Custom text input** -- Full editing support: IME, clipboard, cursor movement, selection, multi-line. The chat composer.
- [ ] **Session persistence** -- Close and reopen without losing workspace state or chat history.
- [ ] **Keyboard shortcuts** -- Cmd+N new workspace, Cmd+T new chat (if multi-chat), Cmd+K command palette, Cmd+D show diff. Keyboard-first.
- [ ] **Theme system** -- Catppuccin Mocha base. Consistent colors for syntax highlighting, diffs, status indicators. Dark mode only for v1.
- [ ] **Three-panel layout** -- Sidebar, chat, context/plan panel. The context panel can start simple (showing agent thinking/plan).

### Add After Validation (v1.x)

Features to add once core is working and users confirm the value proposition.

- [ ] **Collapsible activity log** -- Unified timeline of all agent actions across workspaces. Add when users have 3+ parallel workspaces and need a birds-eye view.
- [ ] **Workspace status grouping** -- Group by status (backlog, active, review, done) or by project. Add when sidebar gets crowded with 5+ workspaces.
- [ ] **Checkpoint and revert** -- Snapshot git state per turn. Revert to previous state. Add when users report wanting to "undo" agent actions beyond git reset.
- [ ] **Workspace forking** -- Branch a workspace into two parallel paths. Add when users want to experiment with agent approaches.
- [ ] **Best-of-N comparison** -- Run same task on Claude vs Codex, compare results side-by-side. Add after multi-provider is solid.
- [ ] **Side-by-side diff mode** -- Toggle between unified and split diff views. Add after inline diffs are stable.
- [ ] **Chat search** -- Cmd+F to search within chat history. Add when chat threads get long (100+ messages).
- [ ] **Workspace archive/restore** -- Archive completed workspaces, restore later. Add when users accumulate 10+ done workspaces.

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] **PR creation workflow** -- Create PRs from within Helm. Defer because GitHub API auth and CI integration is large scope.
- [ ] **Issue tracker integration** -- Pull tasks from Linear/Jira/GitHub Issues. Defer because each integration is a maintenance burden.
- [ ] **Additional provider support** -- Beyond Claude Code and Codex. Architecture supports it, but each provider has its own output format.
- [ ] **Remote/SSH agent execution** -- Run agents on remote machines. Defer because it's a different product category.
- [ ] **CI/CD status display** -- Show GitHub Actions status per workspace. Defer until PR workflow exists.
- [ ] **Kanban view** -- Emdash-style board view as alternative to sidebar list. Defer until sidebar grouping proves insufficient.
- [ ] **Light theme** -- Dark mode only for v1. Add light theme when user demand warrants.
- [ ] **Linux/Windows support** -- GPUI supports them but untested. Defer until macOS version is solid.

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Streaming markdown chat rendering | HIGH | HIGH | P1 |
| Expandable tool call blocks | HIGH | MEDIUM | P1 |
| Inline diff display (unified) | HIGH | HIGH | P1 |
| Multi-project sidebar | HIGH | MEDIUM | P1 |
| Git worktree isolation | HIGH | MEDIUM | P1 |
| Claude Code CLI provider | HIGH | MEDIUM | P1 |
| Custom text input (composer) | HIGH | HIGH | P1 |
| Provider abstraction trait | HIGH | MEDIUM | P1 |
| Codex CLI provider | MEDIUM | MEDIUM | P1 |
| Session persistence | HIGH | MEDIUM | P1 |
| Keyboard shortcuts / command palette | MEDIUM | MEDIUM | P1 |
| Theme system (dark) | MEDIUM | LOW | P1 |
| Three-panel layout | MEDIUM | MEDIUM | P1 |
| Branch display in sidebar | MEDIUM | LOW | P1 |
| Collapsible activity log | MEDIUM | MEDIUM | P2 |
| Workspace status grouping | MEDIUM | LOW | P2 |
| Checkpoint and revert | HIGH | MEDIUM | P2 |
| Workspace forking | MEDIUM | MEDIUM | P2 |
| Best-of-N comparison | MEDIUM | HIGH | P2 |
| Side-by-side diff mode | MEDIUM | MEDIUM | P2 |
| Chat search | LOW | LOW | P2 |
| Workspace archive/restore | LOW | MEDIUM | P2 |
| PR creation workflow | HIGH | HIGH | P3 |
| Issue tracker integration | MEDIUM | HIGH | P3 |
| Additional providers (3+) | LOW | MEDIUM | P3 |
| Remote/SSH execution | LOW | HIGH | P3 |
| CI/CD status display | LOW | MEDIUM | P3 |
| Kanban view | LOW | MEDIUM | P3 |

**Priority key:**
- P1: Must have for launch -- users will not adopt without these
- P2: Should have, add after v1 validates -- enhances the core experience
- P3: Nice to have, future consideration -- only after product-market fit

## Competitor Feature Analysis

| Feature | Conductor | Emdash | T3 Code | Superset | Claude Desktop | Helm (Planned) |
|---------|-----------|--------|---------|----------|----------------|----------------|
| **Platform** | macOS (Electron) | macOS/Win/Linux (Electron) | macOS/Win/Linux (Electron) | macOS/Win/Linux (Terminal+Web) | macOS/Win (Native) | macOS (Native GPUI) |
| **Streaming chat** | Yes | Yes | Yes | Yes | Yes | P1 |
| **Tool call blocks** | Yes (expandable) | Yes | Yes | Basic | Yes | P1 |
| **Inline diffs** | Turn-by-turn diffs | Side-by-side diff view | Per-turn diff viewer | Built-in diff editor | Basic | P1 (unified + split) |
| **Multi-project** | By repo grouping | Per-project config | Project list sidebar | Project-aware | Single project | P1 (core differentiator) |
| **Worktree isolation** | Yes (core feature) | Yes (core feature) | Yes | Yes | No | P1 |
| **Provider support** | Claude + Codex | 22 CLI agents | Codex (Claude coming) | 8+ CLI agents | Claude only | P1 (Claude + Codex) |
| **PR workflow** | Create/merge/CI | Create PR + CI checks | Commit-push-PR | No | No | P3 (v2) |
| **Issue integration** | Linear (upcoming) | Linear/Jira/GitHub | No | No | No | P3 (v2) |
| **Best-of-N** | No | Yes | No | No | No | P2 |
| **Kanban view** | Status grouping | Yes | Thread list | No | No | P3 |
| **Keyboard shortcuts** | Extensive (Cmd+K palette) | Basic | Custom actions | Basic | Standard | P1 (Zed-style) |
| **Activity log** | Workspace history | Task timeline | Thread history | Agent monitoring | Session history | P1 (unified cross-workspace) |
| **Checkpoint/revert** | Yes (resume from turns) | No | Git checkpointing | No | No | P2 |
| **Remote/SSH** | No | Yes | No (planned) | No | No | Out of scope |
| **MCP support** | Yes | Yes | No | Yes | Yes (extensions) | No (agents handle MCP) |
| **Chat search** | Yes (Cmd+F) | No | No | No | No | P2 |
| **Context/plan panel** | No (thinking toggle) | No | No (mode selector) | No | No | P1 (differentiator) |
| **Workspace forking** | Yes | No | No | No | No | P2 |
| **Performance** | Electron (adequate) | Electron (adequate) | Electron (adequate) | Terminal (fast) | Native (fast) | Native GPU (fastest) |

### Key Competitive Observations

1. **The market is Electron-heavy.** Conductor, Emdash, T3 Code are all Electron. Superset is terminal+web. Only Claude Desktop is native. Helm's GPUI architecture is genuinely unique in this space.

2. **Conductor is the feature leader** for workspace management (forking, checkpoints, status grouping, PR workflow, keyboard shortcuts). It sets the bar for what "complete" looks like.

3. **Emdash is the provider breadth leader** with 22 CLI agents and best-of-N. It sets the bar for provider-agnostic design.

4. **T3 Code is the simplicity leader** -- minimal, focused on the chat+diff experience with clean worktree support. It proves you don't need every feature to be useful.

5. **No competitor has a dedicated context/plan panel.** Conductor has a "thinking toggle," but no persistent third panel showing agent reasoning, file context, or task plan. This is a genuine differentiator for Helm.

6. **No competitor has a unified cross-workspace activity log.** Individual workspace history exists, but a birds-eye view across all parallel agents is missing from the market.

## Sources

- [Conductor changelog](https://www.conductor.build/changelog) -- workspace management, PR integration, sidebar features
- [Emdash documentation](https://docs.emdash.sh/) -- provider support, kanban, diff view, best-of-N
- [Emdash GitHub](https://github.com/generalaction/emdash) -- architecture, worktree isolation
- [T3 Code GitHub](https://github.com/pingdotgg/t3code) -- chat interface, terminal drawer, worktree support
- [T3 Code guide (Better Stack)](https://betterstack.com/community/guides/ai/t3-code/) -- feature details, workflow
- [T3 Code UI discussion (#511)](https://github.com/pingdotgg/t3code/issues/511) -- community UX priorities
- [Superset.sh](https://superset.sh/) -- parallel agent execution, editor integration
- [Nimbalyst GUI comparison](https://nimbalyst.com/blog/best-claude-code-gui-tools-2026/) -- competitive landscape
- [Claude Code Desktop docs](https://code.claude.com/docs/en/desktop) -- first-party features
- [Cursor agent features (TechCrunch)](https://techcrunch.com/2026/03/05/cursor-is-rolling-out-a-new-system-for-agentic-coding/) -- automations, background agents
- [Cursor beta features](https://markaicode.com/cursor-beta-features-2026/) -- multi-agent, subagents
- [DataCamp agentic IDEs](https://www.datacamp.com/blog/best-agentic-ide) -- market overview, table stakes
- [Conductor workflow docs](https://docs.conductor.build/workflow) -- workspace lifecycle

---
*Feature research for: Agentic Development Environment (Native Rust/GPUI)*
*Researched: 2026-03-26*

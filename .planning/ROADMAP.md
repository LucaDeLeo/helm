# Roadmap: Helm

## Overview

Helm is built bottom-up along its architectural dependency chain. The provider abstraction and agent bridge form the foundation -- Claude connects via a Bun sidecar running the Anthropic Agent SDK over IPC, while Codex uses a CLI subprocess, both unified behind a single provider trait. Session and worktree management create the domain layer. The UI layout and workspace navigation wire up the multi-project experience. Rich chat rendering -- the highest-value surface -- comes after the data model is stable. The second provider and persistence complete the v1 experience. Each phase delivers a coherent, verifiable capability that unblocks the next.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Provider Architecture & Agent SDK Bridge** - Provider trait supporting both SDK-bridge and CLI subprocess patterns, Bun sidecar for Claude Agent SDK, IPC protocol, process lifecycle management, concurrent agent safety
- [ ] **Phase 2: Session & Worktree Management** - Project/workspace CRUD, git worktree isolation, workspace creation/deletion with cleanup
- [ ] **Phase 3: Layout & Workspace Navigation** - Three-panel layout, project sidebar with workspace tree, workspace switching, keyboard shortcuts, theme system
- [ ] **Phase 4: Rich Chat Rendering** - Streaming markdown, syntax-highlighted code blocks, expandable tool calls, inline diffs, work log, virtualized scrolling
- [ ] **Phase 5: Second Provider & Persistence** - Codex CLI provider, provider selection per workspace, workspace and chat history persistence across restarts

## Phase Details

### Phase 1: Provider Architecture & Agent SDK Bridge
**Goal**: Agents can be started, streamed from, and cleanly stopped -- with a provider trait that abstracts over both SDK-bridge and CLI subprocess patterns, and a Claude implementation using a Bun sidecar communicating with the Anthropic Agent SDK over stdin/stdout JSON IPC
**Depends on**: Nothing (first phase)
**Requirements**: PROV-01, PROV-02, PROV-05, PROV-06
**Success Criteria** (what must be TRUE):
  1. A provider trait defines the full agent lifecycle (start, send, receive stream, stop) with an interface general enough to support both SDK-bridge (Claude) and CLI subprocess (Codex) implementations
  2. A Bun sidecar process runs the Anthropic Agent SDK, and the Rust host communicates with it over stdin/stdout JSON IPC to send messages and receive streaming events as normalized provider events
  3. Stopping a workspace kills its agent process (sidecar or subprocess) cleanly with no zombie processes remaining
  4. Multiple agent processes can run concurrently across workspaces without IPC deadlocks or data corruption
**Plans**: TBD

### Phase 2: Session & Worktree Management
**Goal**: Users can organize work into projects and workspaces, with each worktree-mode workspace getting its own isolated git worktree that is created automatically and cleaned up on deletion
**Depends on**: Phase 1
**Requirements**: WORK-02, WORK-03, WORK-07, WORK-08
**Success Criteria** (what must be TRUE):
  1. User can add a project by selecting a local git repository directory
  2. User can create a new workspace under a project with a name and optional branch
  3. User can choose worktree isolation or same-directory mode when creating a workspace, and worktree-mode workspaces get an automatically created git worktree
  4. Deleting a worktree-mode workspace removes its git worktree from disk
**Plans**: TBD

### Phase 3: Layout & Workspace Navigation
**Goal**: Users can see all their projects and workspaces in a sidebar, switch between them with a click or keyboard shortcut, and work within a consistent three-panel layout with a theme system
**Depends on**: Phase 2
**Requirements**: NAV-01, NAV-02, NAV-03, NAV-04, NAV-05, WORK-01, WORK-04, WORK-05, WORK-06, PERS-03
**Success Criteria** (what must be TRUE):
  1. Sidebar lists projects with expand/collapse, showing workspaces underneath with branch name, diff stats, and status indicator per workspace
  2. Clicking a workspace in the sidebar switches the chat and context panels to that workspace
  3. Three-panel layout displays project sidebar, chat panel, and context/plan panel with consistent theming (Catppuccin Mocha)
  4. Text input supports IME, clipboard, cursor movement, selection, and multi-line editing
  5. Keyboard shortcuts exist for core actions (new workspace, switch workspace, command palette) and the context panel shows agent thinking or file tree with clickable file references
**Plans**: TBD
**UI hint**: yes

### Phase 4: Rich Chat Rendering
**Goal**: Agent responses render as rich, streaming content with markdown formatting, syntax-highlighted code, expandable tool calls, and inline diffs -- performant even in long conversations
**Depends on**: Phase 3
**Requirements**: CHAT-01, CHAT-02, CHAT-03, CHAT-04, CHAT-05, CHAT-06
**Success Criteria** (what must be TRUE):
  1. Agent responses stream in real-time as formatted markdown with headings, lists, bold, italic, and links
  2. Code blocks render with language-aware syntax highlighting
  3. Tool call events appear as expandable/collapsible blocks showing tool name, input summary, and output
  4. File edits display as inline unified diffs with add/remove coloring and syntax highlighting
  5. Agent actions aggregate into a collapsible work log with timestamps, and long conversations (100+ turns) scroll smoothly via virtualized rendering
**Plans**: TBD
**UI hint**: yes

### Phase 5: Second Provider & Persistence
**Goal**: Users can choose between Claude Code and Codex per workspace, and all workspace state and chat history survives app restarts
**Depends on**: Phase 4
**Requirements**: PROV-03, PROV-04, PERS-01, PERS-02
**Success Criteria** (what must be TRUE):
  1. Codex CLI provider parses JSONL output and emits normalized events through the same provider trait as Claude, validating that the trait supports both SDK-bridge and CLI subprocess patterns
  2. User can select which provider (Claude Code or Codex) to use per workspace
  3. Workspace state (name, project, branch, provider, status) persists across app restarts
  4. Chat history (all turns with content blocks) persists across app restarts and loads when the workspace is reopened
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Provider Architecture & Agent SDK Bridge | 0/TBD | Not started | - |
| 2. Session & Worktree Management | 0/TBD | Not started | - |
| 3. Layout & Workspace Navigation | 0/TBD | Not started | - |
| 4. Rich Chat Rendering | 0/TBD | Not started | - |
| 5. Second Provider & Persistence | 0/TBD | Not started | - |

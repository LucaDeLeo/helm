# Requirements: Helm

**Defined:** 2026-03-26
**Core Value:** A high-performance, customizable native ADE — GPU-accelerated UI that's snappier than Electron alternatives, with full control over the agent workflow experience.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Chat Rendering

- [ ] **CHAT-01**: Agent responses render as streaming markdown with headings, lists, bold, italic, and links in real-time
- [ ] **CHAT-02**: Code blocks render with syntax highlighting (language-aware)
- [ ] **CHAT-03**: Tool call events display as expandable/collapsible blocks showing tool name, input summary, and output
- [ ] **CHAT-04**: File edits display as inline unified diffs with add/remove coloring and syntax highlighting
- [ ] **CHAT-05**: Agent actions aggregate into a collapsible work log with timestamps and event types
- [ ] **CHAT-06**: Chat view supports virtualized scrolling for long conversations (100+ turns)

### Workspace Management

- [ ] **WORK-01**: Sidebar lists projects with expand/collapse to show workspaces underneath
- [ ] **WORK-02**: User can create a new workspace under a project with a name and optional branch
- [ ] **WORK-03**: User chooses between worktree isolation or same-directory mode when creating a workspace
- [ ] **WORK-04**: Workspace sidebar row shows branch name and diff stats (+/- lines)
- [ ] **WORK-05**: Workspace sidebar row shows status indicator (running, waiting, done, error)
- [ ] **WORK-06**: Clicking a workspace in the sidebar switches the chat and context panels to that workspace
- [ ] **WORK-07**: User can add a project by selecting a local directory with a git repo
- [ ] **WORK-08**: Worktrees are cleaned up when a worktree-mode workspace is deleted

### Providers

- [ ] **PROV-01**: Provider trait abstracts CLI agent lifecycle (start, send message, receive stream, stop)
- [ ] **PROV-02**: Claude Code provider parses stream-json output and emits normalized events
- [ ] **PROV-03**: Codex CLI provider parses JSONL output and emits normalized events
- [ ] **PROV-04**: User can select which provider to use per workspace
- [ ] **PROV-05**: Provider processes are tracked with proper lifecycle management (no zombie processes)
- [ ] **PROV-06**: Multiple provider processes can run concurrently across workspaces without deadlock

### Input & Navigation

- [ ] **NAV-01**: Text input supports IME, clipboard, cursor movement, selection, and multi-line editing
- [ ] **NAV-02**: Three-panel layout: project sidebar, chat panel, context/plan panel
- [ ] **NAV-03**: Keyboard shortcuts for core actions (new workspace, switch workspace, command palette)
- [ ] **NAV-04**: Context/plan panel displays agent thinking or structured plan data
- [ ] **NAV-05**: Context panel shows a file tree; clicking a file inserts @filename reference into chat input

### Persistence

- [ ] **PERS-01**: Workspace state (name, project, branch, provider, status) persists across app restarts
- [ ] **PERS-02**: Chat history (all turns with content blocks) persists across app restarts
- [ ] **PERS-03**: Theme system provides consistent colors for syntax highlighting, diffs, and status indicators

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Remote

- **REM-01**: User can connect to a remote machine via SSH and run agents there
- **REM-02**: Remote file system browsing in context panel

### Git Integration

- **GIT-01**: User can create a PR from within Helm
- **GIT-02**: Workspace shows CI/CD status from GitHub Actions
- **GIT-03**: User can merge PRs from within Helm

### Integrations

- **INT-01**: Pull tasks from Linear/Jira/GitHub Issues into workspaces
- **INT-02**: Additional provider support beyond Claude Code and Codex

### Polish

- **POL-01**: Checkpoint and revert (git snapshot per agent turn)
- **POL-02**: Workspace forking (branch a workspace into parallel paths)
- **POL-03**: Best-of-N comparison (run same task on multiple providers)
- **POL-04**: Side-by-side diff mode (toggle between unified and split)
- **POL-05**: Chat search (Cmd+F within chat history)
- **POL-06**: Workspace archive/restore
- **POL-07**: Light theme
- **POL-08**: Linux/Windows support

## Out of Scope

| Feature | Reason |
|---------|--------|
| Built-in code editor | Helm orchestrates agents, not editing — use Zed/VS Code for that |
| Web/browser-based interface | Desktop native only — GPUI doesn't support webviews |
| Background/cloud agents | Local-first by design — completely different product category |
| Event-driven automations | Requires cloud infrastructure, not a desktop app feature |
| Browser/web preview | Would require webview, breaking "no web technologies" constraint |
| MCP server hosting | CLI agents handle MCP themselves — Helm passes through |
| Voice input | Niche feature, system-level dictation already available on macOS |
| Real-time collaboration | Requires CRDT/OT networking — different product category |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CHAT-01 | TBD | Pending |
| CHAT-02 | TBD | Pending |
| CHAT-03 | TBD | Pending |
| CHAT-04 | TBD | Pending |
| CHAT-05 | TBD | Pending |
| CHAT-06 | TBD | Pending |
| WORK-01 | TBD | Pending |
| WORK-02 | TBD | Pending |
| WORK-03 | TBD | Pending |
| WORK-04 | TBD | Pending |
| WORK-05 | TBD | Pending |
| WORK-06 | TBD | Pending |
| WORK-07 | TBD | Pending |
| WORK-08 | TBD | Pending |
| PROV-01 | TBD | Pending |
| PROV-02 | TBD | Pending |
| PROV-03 | TBD | Pending |
| PROV-04 | TBD | Pending |
| PROV-05 | TBD | Pending |
| PROV-06 | TBD | Pending |
| NAV-01 | TBD | Pending |
| NAV-02 | TBD | Pending |
| NAV-03 | TBD | Pending |
| NAV-04 | TBD | Pending |
| NAV-05 | TBD | Pending |
| PERS-01 | TBD | Pending |
| PERS-02 | TBD | Pending |
| PERS-03 | TBD | Pending |

**Coverage:**
- v1 requirements: 28 total
- Mapped to phases: 0
- Unmapped: 28

---
*Requirements defined: 2026-03-26*
*Last updated: 2026-03-26 after initial definition*

# Helm

## What This Is

Helm is a native desktop Agentic Development Environment (ADE) built with Rust and Zed's GPUI framework. It lets you manage multiple projects, each with parallel agent workspaces running on isolated git worktrees, through a GPU-accelerated interface with rich chat rendering — streaming markdown, expandable tool calls, inline diffs, and activity logs. Think Conductor meets T3 Code, but native and fast.

## Core Value

A high-performance, customizable native ADE — GPU-accelerated UI that's snappier than Electron alternatives, with full control over the agent workflow experience.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Multi-project sidebar with workspaces per project, each on its own git worktree
- [ ] Rich chat rendering with streaming markdown as agent responds
- [ ] Expandable tool call blocks showing agent actions (file reads, edits, shell commands)
- [ ] Inline diff display with syntax highlighting in chat
- [ ] Collapsible work log / activity feed of all agent actions
- [ ] Claude Code CLI provider integration with streaming JSON parsing
- [ ] Codex CLI provider integration
- [ ] Provider abstraction layer supporting multiple CLI agents
- [ ] Git worktree auto-creation per workspace/task
- [ ] Branch display and diff stats in sidebar per workspace
- [ ] Custom text input with IME, clipboard, cursor, and selection support
- [ ] Three-panel layout: project sidebar, chat panel, context/plan panel
- [ ] Centralized theme system with dark mode (Catppuccin Mocha base)
- [ ] Global and scoped key bindings (Zed-style)

### Out of Scope

- SSH/remote agent execution — complexity too high for v1, local-first
- Full PR workflow (create, CI status, merge) — v1 shows branches and diffs only
- Web/browser-based interface — desktop native only
- Built-in code editor — Helm orchestrates agents, not editing (use Zed for that)
- Ticket/issue tracker integration (Linear, GitHub Issues, Jira) — v2 feature
- Provider support beyond Claude Code and Codex — architecture supports it, but only two providers shipped in v1

## Context

- Built on Zed's GPUI framework (local path dependency at `../zed/crates/gpui`), sharing the same GPU-accelerated rendering, reactive entity model, and element system
- Current codebase has a working three-panel layout, Claude CLI streaming integration, custom text input widget, and Catppuccin Mocha theme
- Reference apps: T3 Code (rich agent chat UX), Emdash/Conductor (multi-project sidebar with worktrees, PR integration, provider-agnostic design)
- The `provider.rs` module already parses Claude's `stream-json` output format — needs to be abstracted into a trait for multi-provider support
- GPUI uses an entity-based reactive model (`Entity<T>` + `Render` trait) with subscription-based communication between components

## Constraints

- **Tech stack**: Rust + GPUI only — no web technologies, no Electron, no JavaScript
- **Platform**: macOS primary target (GPUI supports Linux/Windows but untested for Helm)
- **Dependency**: Requires local Zed repo clone at `../zed/` for GPUI crates
- **Providers**: Claude Code CLI and Codex CLI must be installed on the user's machine — Helm spawns them as subprocesses
- **Rendering**: All UI rendering through GPUI's element system — no webviews or HTML

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| GPUI over Electron/Tauri | Native GPU performance + Rust type safety + Zed ecosystem alignment | — Pending |
| Subprocess CLI providers (not API) | Reuses existing CLI auth/config, provider-agnostic by design | — Pending |
| Local-first, no remote for v1 | Reduces scope significantly while delivering core multi-project value | — Pending |
| Worktrees over same-folder tasks | True isolation between parallel agent workspaces, cleaner git state | — Pending |
| Multi-project sidebar as core UX | Not useful without it — single-project chat doesn't differentiate from existing tools | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-26 after initialization*

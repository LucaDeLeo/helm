# External Integrations

**Analysis Date:** 2026-03-26

## APIs & External Services

**AI/LLM:**
- Claude Code CLI (`claude`) - Core AI integration for chat functionality
  - SDK/Client: Direct subprocess invocation via `std::process::Command` in `src/provider.rs`
  - Auth: Handled by the `claude` CLI itself (not managed by Helm)
  - Invocation: `claude -p <prompt> --output-format stream-json`
  - Communication: Line-buffered stdout streaming of JSON events
  - Event types parsed: `assistant`, `user`, `content_block_delta`, `result`, `error`
  - Content block types: `text`, `tool_use`, `tool_result`
  - Availability check: `claude --version` via `provider::is_claude_available()` at `src/provider.rs:63`
  - Working directory: Optionally configurable per turn (currently passed as `None`)

## Data Storage

**Databases:**
- None

**File Storage:**
- None (no persistence layer; all state is in-memory)

**Caching:**
- None

## Authentication & Identity

**Auth Provider:**
- Not applicable (desktop application, no user accounts)
- Claude CLI authentication is external to Helm

## Monitoring & Observability

**Error Tracking:**
- None (errors displayed inline in chat UI as system messages)

**Logs:**
- None (no logging framework configured)

## CI/CD & Deployment

**Hosting:**
- Local desktop binary (no hosted deployment)

**CI Pipeline:**
- None detected (no `.github/workflows/`, no CI configuration files)

## Environment Configuration

**Required env vars:**
- None required by Helm directly
- The `claude` CLI requires its own API key configuration (managed externally)

**Secrets location:**
- Not applicable (no secrets managed by Helm)

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- None

## Integration Architecture

**Claude CLI Communication Pattern:**

The integration follows a synchronous subprocess pattern with async bridging:

1. `ChatPanel::send_turn()` (`src/chat.rs:157`) creates an `mpsc::channel`
2. A background thread spawns `provider::run_turn()` (`src/provider.rs:74`) which:
   - Spawns `claude -p <prompt> --output-format stream-json` as a child process
   - Reads stdout line-by-line via `BufReader`
   - Parses each JSON line into `RawEvent` structs via serde
   - Converts to `ProviderEvent` enums and sends via `mpsc::Sender`
3. A foreground async task polls the `mpsc::Receiver` at ~60fps (16ms intervals via `smol::Timer`)
4. Events update the assistant turn in-place in the chat panel

**Tool call rendering:**
- Tool uses (Bash, Read, Write, Edit, Glob, Grep) are parsed from the Claude response
- Input is summarized per tool type in `summarize_tool_input()` (`src/provider.rs:151`)
- Tool results are matched back by `tool_use_id` and displayed in expandable UI blocks

---

*Integration audit: 2026-03-26*

# Pitfalls Research

**Domain:** Native Rust/GPUI Agentic Development Environment with multi-project management, concurrent CLI agent processes, git worktree isolation, and streaming chat rendering
**Researched:** 2026-03-26
**Confidence:** MEDIUM-HIGH (domain-specific patterns well-documented across reference apps and official docs; GPUI-specific issues inferred from Zed blog posts and existing codebase patterns since external GPUI documentation is sparse)

## Critical Pitfalls

### Pitfall 1: Zombie CLI Processes and Resource Exhaustion

**What goes wrong:**
Spawned `claude` and `codex` child processes are never reaped after the user closes a workspace, switches projects, or the parent app crashes. On macOS, each orphaned process holds open file descriptors, a pipe pair, and counts against the system process limit. With multi-project parallel agents, a single session could spawn 5-10 concurrent CLI processes. Over hours of use, unreliable cleanup accumulates dozens of zombie processes, eventually hitting macOS's `RLIMIT_NPROC` or exhausting file descriptor limits.

**Why it happens:**
Rust's `std::process::Child` does not automatically wait on children when dropped -- the standard library documentation explicitly warns: "it is up to the application developer to do so." The current codebase already uses `.detach()` on background tasks (`chat.rs` line 185), making the spawned future and its child process fire-and-forget. When a `ChatPanel` entity is dropped (workspace closed, project switched), nothing kills the running `claude` subprocess. The `tx` sender side of the mpsc channel may be dropped, but this only closes the channel -- it does not terminate the child process.

**How to avoid:**
- Store `Child` handles in a process registry (`HashMap<WorkspaceId, Vec<Child>>`) at the application level, not inside individual chat panels.
- Implement `Drop` for the workspace/chat entity that sends `SIGTERM` to all registered children, then `SIGKILL` after a 2-second timeout.
- Replace `.detach()` with a held `Task` handle (the `_pending_task` field already exists but is not used for cleanup). On cancel, kill the child and await its exit.
- Register a SIGINT/SIGTERM handler at the application level that iterates all tracked children and kills them on abnormal exit.

**Warning signs:**
- `ps aux | grep claude` shows processes from previous sessions still running.
- macOS Activity Monitor shows Helm's child process count growing over time without shrinking.
- "Too many open files" or "Resource temporarily unavailable" errors after extended use.
- System fan ramps up when Helm is "idle" because zombie CLI processes are still consuming CPU in tool loops.

**Phase to address:**
Foundation phase -- before multi-project support multiplies the problem. The current single-ChatPanel architecture already has this bug. Must be fixed before scaling to N concurrent agents.

---

### Pitfall 2: Pipe Buffer Deadlock with Concurrent Subprocesses

**What goes wrong:**
When multiple CLI agents produce output simultaneously, the parent process must read from each child's stdout pipe promptly. If the app's event loop stalls (during a heavy render, markdown parse, or synchronous git operation), the OS pipe buffer fills (64KB on macOS). The child process blocks on its next `write()` call, waiting for the parent to drain the pipe. If the parent is waiting for something that depends on the child, both processes deadlock. The child hangs mid-response, the UI shows "Claude is thinking..." forever, and the only recovery is force-quit.

**Why it happens:**
The current implementation reads stdout line-by-line with `BufReader` on a blocking background thread (`provider.rs` lines 112-133). This works for a single subprocess but does not scale. When 5 agents stream simultaneously, each needs its own dedicated reader. The 16ms busy-polling loop (`chat.rs` lines 199-205) adds latency -- if output arrives faster than the poll rate, the `mpsc` channel buffers it, but the stdout pipe buffer can still fill if the background reader blocks for any reason (e.g., the thread pool is saturated).

**How to avoid:**
- Dedicate one async task per subprocess that continuously reads stdout. Never share a reader across subprocesses.
- Replace `std::sync::mpsc` with an async channel (`smol::channel` or `flume`) so the consumer can `.await` instead of busy-polling. This is already flagged in CONCERNS.md as a known issue.
- Use larger BufReader buffer sizes (64KB-256KB instead of the default 8KB) to reduce syscall frequency and pipe pressure.
- Never perform synchronous I/O on the main/render thread. All git operations, file reads, and subprocess management must be async or on dedicated threads.
- Read stdout and stderr on separate tasks/threads to prevent stderr output from backing up independently.

**Warning signs:**
- Agent responses freeze mid-sentence during heavy multi-agent workloads.
- One agent's response stalls until another agent completes.
- CPU usage drops to near-zero while agents appear to be working (all threads blocked, not busy).
- The 16ms polling timer fires but `try_recv` always returns `Empty` even though the agent is visibly doing work in the terminal.

**Phase to address:**
Provider abstraction phase -- when building the multi-provider subprocess manager. The single-process architecture masks this but it surfaces immediately when running 2+ agents concurrently.

---

### Pitfall 3: Git Worktree Corruption from Concurrent Operations

**What goes wrong:**
Git's internal locking is designed for single-process access. When multiple agents in separate worktrees run git commands concurrently, they all access the shared `.git` directory and object database. This causes index lock collisions ("Unable to create index.lock: File exists"), potentially corrupt pack files, or invalid object references. Git enforces a one-branch-per-worktree rule -- attempting to create two worktrees for the same branch fails with "fatal: 'branch' is already checked out." If agents crash mid-operation, stale `index.lock` files block all subsequent git operations in that worktree until manually cleaned.

**Why it happens:**
Git was designed as a single-user CLI tool. Its documentation explicitly states it is "meant to be run by a single process, not multiple processes at the same time." Worktrees share the object database (`.git/objects/`), refs, and hooks. Concurrent commits from different worktrees may both try to update refs or pack objects, leading to races. The lock file mechanism (`index.lock`) serializes operations within a single worktree but is fragile -- a killed agent or app crash leaves the lock file behind permanently.

**How to avoid:**
- Serialize all git operations through a single async task queue per repository (not per worktree). Use a mutex or ordered channel to ensure only one git command runs against the shared `.git` at a time.
- Generate unique branch names per workspace (e.g., `helm/task-{uuid}`) to avoid the "already checked out" error. Never reuse branch names across worktrees.
- Implement stale lock detection: before any git operation, check for `index.lock` files older than 30 seconds and remove them with a warning log.
- Run `git worktree prune` on app startup and when workspaces are closed to clean orphaned worktree metadata.
- Place worktrees as siblings of the main repo (e.g., `../project-worktrees/task-123/`), never nested inside the main checkout. Nested worktrees cause metadata confusion.
- Implement worktree creation as a transaction: track state at each step (branch created, worktree added, directory populated) and roll back on failure.

**Warning signs:**
- "fatal: Unable to create '.git/worktrees/X/index.lock': File exists" errors in agent logs.
- `git worktree list` shows entries for directories that no longer exist on disk ("prunable" entries).
- Branches show as "locked" or "already checked out" when attempting to create new worktrees.
- Disk usage in `.git/objects/` grows unexpectedly from orphaned pack files after interrupted operations.
- Users reporting "weird git state" or inability to delete branches after using the app.

**Phase to address:**
Git worktree management phase -- this must be solved with the initial worktree lifecycle implementation, not bolted on later. The serialized command queue and lock cleanup are foundational. Design the `WorktreeManager` to own all git interactions for a repository.

---

### Pitfall 4: Streaming Markdown Rendering Causes Cascading Re-parses and Flicker

**What goes wrong:**
As the agent streams text token-by-token, naive markdown rendering re-parses the entire response on every token. A 500-token response triggers 500 full re-parses. Worse, an incomplete markdown block (e.g., a code fence opened but not yet closed: ` ```rust\nfn main()` without the closing ` ``` `) causes the parser to treat everything after the opening fence as code, producing wildly wrong rendering that flickers between states as tokens arrive. Users see the entire response flash between "code block" and "normal text" multiple times per second. With GPUI's GPU-accelerated rendering, this produces visible frame drops because each re-parse invalidates the entire element tree for that turn.

**Why it happens:**
CommonMark/GFM markdown is context-sensitive -- you cannot know if you are inside a code block, table, or blockquote until you see the closing delimiter. Parsers like pulldown-cmark operate on complete documents. Feeding partial documents produces structurally valid but semantically wrong parse trees. The current codebase renders all text as plain strings (no markdown parsing at all), so this pitfall is latent -- it will surface immediately when markdown rendering is implemented.

**How to avoid:**
- Use incremental parsing: only re-parse the last (incomplete) block. Track the byte offset where the last completed block ended. Feed only the suffix to the parser. This reduces parsing from O(n^2) total to O(n) over the stream.
- Buffer incoming tokens and batch render updates. Do not render on every single `TextDelta`. Use a dual trigger: render after 80+ characters accumulated OR every 100ms, whichever comes first. This matches patterns used by Chrome's LLM response rendering guidance.
- For incomplete code blocks, detect the unterminated fence and render the content as preformatted text with a visual "streaming..." indicator, rather than letting the parser flip the entire suffix between states.
- Cache completed blocks. Once a code block, heading, or paragraph is fully closed by the parser, cache its rendered GPUI element tree and never re-parse it.
- Consider pulldown-cmark's `offset_iter()` which provides byte ranges, enabling selective re-rendering of only changed regions.
- Treat the markdown renderer as a standalone, testable module from day one. It is the most complex single component and the primary UX surface.

**Warning signs:**
- Response text visually flickers between different formatting during streaming.
- Frame rate drops below 30fps during agent responses (check GPUI's debug frame timing).
- CPU spikes proportional to response length (quadratic parsing: O(n) parse on each of n tokens = O(n^2) total work).
- Code blocks render with wrong language or no highlighting until the closing fence arrives.

**Phase to address:**
Chat rendering phase -- when implementing rich markdown rendering. The incremental/batched strategy must be designed upfront, not patched onto a naive "re-render everything" implementation.

---

### Pitfall 5: Provider Abstraction Converges on Lowest Common Denominator

**What goes wrong:**
To support both Claude Code CLI and Codex CLI, developers create an abstraction trait that reduces all providers to a shared event model. But Claude and Codex have meaningfully different streaming formats, session management capabilities, tool approval flows, and output structures. Over-abstraction strips provider-specific features: Claude's `--continue`/`--resume` session management, Codex's `exec` mode with PTY streaming, Claude's `system/api_retry` retry events, Codex's `app-server` JSON-RPC mode. Users of a specific provider get a degraded experience compared to using that CLI directly, eliminating the value proposition of the wrapper app.

**Why it happens:**
The natural instinct is to define a clean `trait Provider { fn run_turn(...) -> Stream<ProviderEvent> }` and force all providers into it. The existing `ProviderEvent` enum (`provider.rs` lines 7-21) already has this shape: `TextDelta`, `ToolUse`, `ToolResult`, `Complete`, `Error`. When Codex support is added, the temptation is to map Codex events to the same enum. But Codex's events have different semantics (exec sessions, multi-modal UserInput blocks), and Claude's streaming format includes events the current enum ignores (retry events, partial messages, structured output).

**How to avoid:**
- Design the provider trait at the session/lifecycle level, not the event level. The trait should define `start_session`, `send_turn`, `cancel`, `resume`, `get_capabilities` -- not the format of individual events.
- Use provider-specific event types internally. Each provider module owns its own event enum. The chat rendering layer handles `ClaudeEvent` and `CodexEvent` variants separately, with shared rendering for common patterns (text, tool calls) and provider-specific rendering for unique features.
- Add a `capabilities()` method to the provider trait that returns what this provider supports (session resume, structured output, tool approval, exec mode). The UI adapts based on capabilities rather than assuming all providers are identical.
- Normalize at the display layer, not the data layer. Store raw provider events; transform to UI elements at render time.
- Study both Claude Code `stream-json` AND Codex `--json` JSONL formats before designing the trait. Never design around the first provider alone.

**Warning signs:**
- Adding a new provider requires modifying the core `ProviderEvent` enum.
- Provider-specific CLI flags (e.g., `--continue`, `--bare`) are being dropped or hardcoded because they do not fit the abstraction.
- The `ProviderEvent` enum grows an "Other" or "Raw(serde_json::Value)" variant to smuggle provider-specific data.
- Users ask "why can't I resume sessions?" when using Claude through Helm, even though the CLI supports it natively.

**Phase to address:**
Provider abstraction phase -- the most consequential architectural decision. The abstraction boundary must be drawn correctly from the start because changing it later requires rewriting both the provider modules and the chat rendering layer.

---

### Pitfall 6: GPUI Entity Lifecycle Mismanagement and Silent Subscription Loss

**What goes wrong:**
GPUI's entity model has unique ownership semantics: all entities are owned by the top-level `App`, handles are inert identifiers, and subscriptions are silently dropped if not stored. In a multi-project app with dozens of entities (projects, workspaces, chat panels, plan panels, sidebars, text inputs per workspace), it is easy to: (a) drop a subscription by not storing the return value of `cx.subscribe()`, silently severing the event connection; (b) hold stale `Entity<T>` handles to closed workspaces, preventing deallocation; (c) trigger reentrancy panics by attempting to update an entity that is currently being leased during a callback.

**Why it happens:**
GPUI's ownership model is unfamiliar even to experienced Rust developers. The "lease" pattern (entity state is temporarily moved from App to the stack during a callback) means that nested updates to the same entity panic at runtime, not compile time. Subscriptions return a `Subscription` guard that must be stored -- if assigned to `_` or a temporary, the subscription is immediately cancelled. The current codebase stores one subscription (`_subscription` in `ChatPanel`), but as the entity graph grows (project list observing workspace changes, workspace observing chat events, chat observing provider events), missed subscriptions become likely. GPUI uses a queued effect model: `emit` and `notify` push to a queue rather than invoking listeners immediately. This prevents reentrancy but introduces non-obvious ordering dependencies.

**How to avoid:**
- Establish a naming convention: all subscription fields begin with `_sub_` and are never named with just `_`. Audit every `cx.subscribe()` and `cx.observe()` call to verify the return value is stored in a struct field.
- Never nest `entity.update(cx, |this, cx| { same_entity.update(cx, ...) })` calls. GPUI will panic. Instead, use `cx.notify()` to defer updates to the effect queue.
- Use `WeakEntity<T>` for cross-component references (already done correctly in `chat.rs` line 293). `WeakEntity` does not prevent deallocation and returns `Err` on upgrade if the entity was dropped.
- Document the entity ownership graph: which entities own which, which observe which, and which subscribe to which. Review this graph whenever new entities are added.
- Test with CJK input methods (Japanese IME, Chinese Pinyin) early -- the `TextInput` widget's `Element` implementation (`text_input.rs` lines 456-641) has bidirectional state coupling between prepaint and paint phases that is especially fragile with IME marked ranges.

**Warning signs:**
- UI components stop responding to events silently (subscription was dropped without being stored).
- Panic with message about entity already being leased/borrowed (reentrant update).
- Memory usage grows when switching between projects (old project entities not deallocated because strong handles remain).
- Adding a new observer/subscription in one component breaks an unrelated component (effect queue ordering dependency).

**Phase to address:**
Every phase that introduces new entities. Establish the ownership documentation and subscription audit practice in the foundation phase, then enforce it as each new component (workspace, project sidebar, plan panel) is added.

---

### Pitfall 7: Conversation Context Loss Between Turns

**What goes wrong:**
Each call to `claude -p` spawns a fresh subprocess with no memory of previous turns. The agent cannot follow up on its own work, reference earlier tool results, or maintain a coherent multi-step workflow. Users type "now fix the tests" and Claude responds "What tests? I don't see any context about previous work." This makes the app feel broken compared to the interactive `claude` CLI, which maintains full conversation state.

**Why it happens:**
The current implementation (`provider.rs` line 80-81) passes only the current prompt to `claude -p`. This is already documented in CONCERNS.md. The naive fix of concatenating all previous turns as a mega-prompt hits `ARG_MAX` limits (~256KB on macOS) and wastes tokens. Using `--continue` requires capturing the `session_id` from the previous turn's JSON output and passing `--resume <session_id>` on the next invocation, which means the provider must maintain session state across subprocess lifetimes.

**How to avoid:**
- Use Claude CLI's `--continue` flag with `--output-format json` or `stream-json` to capture the `session_id` from each response. Store it in the workspace state. Pass `--resume <session_id>` on subsequent turns. The CLI documentation confirms: "Claude --continue resumes your last session" and sessions show across worktrees from the same git repository.
- For Codex, use its equivalent session continuation mechanism.
- Store session IDs per workspace, not per chat panel. When a workspace is closed and reopened, the session can be resumed.
- Implement a fallback for providers without native session management: pipe conversation history via stdin using `--input-format stream-json` rather than CLI arguments, avoiding `ARG_MAX`.
- Display the active session ID in the workspace status area so users can verify continuity is working.

**Warning signs:**
- Agent responses say "I don't have context about..." or re-ask questions answered in previous turns.
- Follow-up prompts like "fix that" or "now do the same for the other file" produce confused responses.
- Response quality degrades compared to using `claude` interactively in the terminal.

**Phase to address:**
Provider integration phase -- immediately after the basic provider abstraction is defined. Session continuity is table-stakes UX; without it, the app is strictly worse than a terminal.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Clone entire `Vec<Turn>` on every render frame (`chat.rs` line 289) | Satisfies GPUI `'static` closure requirement quickly | O(n) allocation per frame; jank at 100+ tool calls with large outputs | Only in MVP with <20 turns; refactor to `Rc<Vec<Turn>>` or indexed access before rich markdown rendering |
| Use `std::sync::mpsc` with 16ms busy-polling (`chat.rs` lines 199-205) | Avoids async channel dependency, simple to implement | Wastes CPU at 60Hz polling when idle, prevents proper backpressure, blocks scaling to multi-agent | Never acceptable long-term; replace with `smol::channel` or `flume` in provider abstraction phase |
| Hardcode panel widths (sidebar 250px, plan 300px) | Ship UI faster, avoid resize drag-handle complexity | Cannot adapt to different screen sizes; feels rigid and amateurish | Acceptable in first milestone; add draggable dividers before multi-project support |
| Silence JSON parse errors (`if let Ok(event) = ...` at `provider.rs` line 128) | Prevents crashes on unexpected CLI output | Hides provider format changes, makes debugging impossible, silently drops events | Never acceptable; log every parse failure at minimum, even in MVP |
| Single `.detach()` for background spawn (`chat.rs` line 185) | Simpler task lifecycle, no cleanup code needed | No cancellation, no error propagation, no subprocess cleanup on drop | Only for true fire-and-forget tasks (telemetry); never for subprocess management |
| Path-dependency on local Zed checkout (`../zed/crates/gpui`) | Gets GPUI working immediately with latest source | Cannot build on CI, cannot distribute, breaks silently when upstream Zed API changes | Acceptable for local dev; pin to specific Zed git commit before any CI/CD or distribution |
| In-memory-only conversation storage (`Vec<Turn>`) | No serialization complexity | All context lost on restart, no session resumption, unbounded memory growth | Only until chat rendering is stable; add SQLite/JSON persistence as soon as the data model stabilizes |

## Integration Gotchas

Common mistakes when connecting to CLI providers and git.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Claude CLI `stream-json` | Assuming each line is a complete, valid JSON event. Lines can be empty, contain error output from the CLI itself, or arrive as partial UTF-8 at chunk boundaries. | Trim whitespace, skip empty lines, log and continue on parse failure. Read stdout and stderr on separate threads/tasks. Never assume clean NDJSON. |
| Claude CLI `--continue` | Calling `--continue` without `--output-format json` or `stream-json` -- the `session_id` is only available in JSON output, not plain text. | Always use structured output format when you need to capture `session_id` for continuation. Parse it from the final `result` event. |
| Claude CLI `--bare` mode | Not realizing `--bare` skips project CLAUDE.md, hooks, MCP servers, and plugins. Agents lose project context. | Only use `--bare` for CI/scripts. For interactive Helm use, omit `--bare` so Claude reads the project's CLAUDE.md and MCP config. Pass `--append-system-prompt` for Helm-specific instructions. |
| Codex CLI `--json` | Assuming Codex JSONL events share the same schema as Claude's stream-json. They use completely different event types and data structures. | Parse each provider's output with a provider-specific deserializer. Never reuse one provider's serde types for another. |
| Git worktree creation | Calling `git worktree add` with an existing branch name that is checked out elsewhere, or using a branch name that already exists. | Always create fresh branches with unique names: `git worktree add -b helm/task-{uuid} ../worktrees/task-{uuid} HEAD`. Check `git worktree list` before creation. |
| Git worktree cleanup | Deleting the worktree directory with `rm -rf` without calling `git worktree remove`. Leaves stale metadata in `.git/worktrees/`. | Always use `git worktree remove <path>` which handles both directory and metadata. Fall back to `git worktree prune` if the directory was already deleted externally. |
| GPUI subscriptions | Calling `cx.subscribe(&entity, handler)` without storing the returned `Subscription`. The subscription is immediately dropped and the handler never fires. | Store in a named struct field: `self._sub_chat = cx.subscribe(&entity, handler);`. Never let the subscription go out of scope or use `_` binding. |
| GPUI entity updates | Emitting events or updating entities synchronously inside a callback, expecting listeners to fire immediately. | GPUI queues effects -- `emit` and `notify` push to a queue flushed after the callback completes. Design for deferred, not immediate, propagation. |

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Full `Vec<Turn>` clone per render frame | Frame drops during scrolling, increasing allocation per frame | Use `Rc<Vec<Turn>>` with COW, or pass indices into a shared turn store | 50+ turns with tool outputs; ~100KB+ cloned per frame at 60fps |
| Re-parsing entire markdown document on each streaming token | CPU spikes proportional to response length squared | Incremental parsing: only re-parse last incomplete block; cache completed blocks | Responses >200 tokens; code-heavy responses with syntax highlighting |
| `ListState::splice` on every streaming token | List remeasures/relayouts every token instead of every frame | Batch token accumulation; splice once per render frame (16ms window) | Fast-streaming responses at 50+ tokens/second; visible stutter |
| Synchronous git commands on main thread | UI freezes during git operations | Run all git operations on background threads; update UI via `cx.notify()` | Repositories with 1000+ commits; `git worktree add` on large repos takes 1-5s |
| Synchronous CLI availability check on startup (`provider.rs` line 63-71) | App hangs on launch if `claude --version` is slow or times out | Move to background task; show "checking..." state; update async | When CLI is not installed and spawn fails with a timeout |
| Unbounded in-memory conversation storage | Memory grows linearly forever; no upper bound | Paginate old turns to disk (SQLite); lazy-load tool outputs on expand; cap stored output length | Conversations with 100+ tool calls; tool outputs averaging 5KB+ each |
| One `BufReader` per CLI process with default 8KB buffer | Excessive syscalls on high-throughput streaming; pipe buffer pressure | Use 64KB-256KB BufReader buffer; read in larger chunks | Agents streaming large tool outputs (file contents, grep results >64KB) |

## Security Mistakes

Domain-specific security issues for an ADE that spawns CLI agents with filesystem access.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Passing prompts as CLI arguments without length limits | Exceeding `ARG_MAX` (~256KB on macOS) crashes the spawn; long arguments may be truncated silently by some shells | Pipe prompts via stdin using `--input-format stream-json` for any prompt over 4KB. Enforce a max prompt length in the text input. |
| Inheriting the full parent environment in spawned CLIs | Provider CLIs inherit all env vars, potentially exposing secrets via tool calls that echo environment | Explicitly set the subprocess environment. Pass only required vars (HOME, PATH, provider-specific API keys). Scrub sensitive vars from child env. |
| Displaying raw tool call output without sanitization | Agent tool outputs may contain ANSI escape sequences, terminal control codes, or multi-megabyte strings that crash the renderer | Strip ANSI escapes from tool output before display. Truncate to a max display length (50KB). Render in a bounded container with overflow hidden. |
| Storing session tokens or API keys in plaintext config files | Persistence files readable by any process running as the user | Use macOS Keychain for API key storage. Never serialize API keys into JSON session files. Set restrictive file permissions (0600) on session data. |
| Auto-approving all tool calls without user visibility | Agent could execute destructive commands (`rm -rf`, `git push --force`, `git reset --hard`) silently | Default to requiring confirmation for write/execute operations. Show tool input in the UI before execution. Allow configurable auto-approve rules per workspace with explicit opt-in. |
| No rate limiting on subprocess spawning | Malicious or buggy interaction could spawn hundreds of CLI processes rapidly | Enforce a per-repository max concurrent agent count (e.g., 10). Queue excess requests with a visible "waiting for agent slot" indicator. |

## UX Pitfalls

Common user experience mistakes in ADE/agent GUI applications.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No indication of which agent is doing what | Users lose track of 3+ parallel agents; cannot tell if an agent is working, stuck, or finished | Per-workspace status indicator showing current activity (reading file, running command, thinking). Aggregate status bar: "3 agents active, 1 idle." |
| Streaming text with no progress on long tool calls | Tool executions (test suites, builds) show nothing for minutes, then dump a wall of text | Show live "running: npm test" indicator with elapsed time. Stream tool output incrementally, not as a single blob on completion. |
| No way to cancel a running agent | Users must wait for completion or force-quit the app | Cancel button per workspace that sends SIGTERM to the child process. Confirm before cancelling to prevent accidental loss. |
| Chat input disabled during agent response | Users cannot queue the next prompt or prepare follow-ups | Allow typing while the agent responds. Queue the message for sending after completion. Show a "pending" indicator for queued messages. |
| Worktree management exposed as raw git concepts | Non-git-experts do not understand worktrees, branches, or detached HEAD | Abstract worktrees as "isolated workspaces." Show branch name and diff summary, not git internals. Auto-create and auto-name worktrees from task descriptions. |
| Sidebar information density overload | Wall of text per workspace (branch, status, PR, diff stats, provider, last activity) is unreadable | Show minimal info by default (name + status icon). Reveal details on hover or expand. Conductor went through multiple sidebar iterations to find the right density. |
| Forcing serial workflow (one agent at a time) | Wastes time when tasks are independent | Allow starting new workspace agents while others run. Make it visually clear which workspaces have active agents vs idle. |
| No error recovery / retry | Transient failures require re-typing the entire prompt | Offer "retry" button on failed turns. Preserve the failed prompt for editing and resending. |

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **Streaming chat:** Often missing handling for incomplete markdown blocks mid-stream -- verify that partial code fences, tables, and blockquotes render gracefully without flickering between states
- [ ] **Provider integration:** Often missing session continuity (`--continue`/`--resume`) -- verify that follow-up questions reference previous context correctly
- [ ] **Worktree management:** Often missing orphan cleanup -- verify that closing a workspace removes the worktree and prunes stale metadata; verify correct behavior after app crash (stale locks, orphaned directories)
- [ ] **Multi-project sidebar:** Often missing project-specific working directory -- verify each workspace's agent runs with `current_dir` set to the worktree path, not the app's launch directory
- [ ] **Tool call display:** Often missing error state rendering -- verify that failed tool calls (`is_error=true`) are visually distinct and show the error output, not just the tool name
- [ ] **Cancel/abort:** Often missing child process cleanup -- verify that cancelling a turn kills the subprocess AND does not leave a zombie; check with `ps aux` after cancellation
- [ ] **Text input:** Often missing multiline support -- verify that Shift+Enter inserts a newline and that pasted text with newlines preserves them (currently stripped: `text_input.rs` line 138)
- [ ] **Panel layout:** Often missing resize persistence -- verify that dragged panel widths survive app restart
- [ ] **Provider abstraction:** Often missing capability detection -- verify that provider-specific features (session resume, structured output) are enabled only when the provider supports them, and gracefully unavailable otherwise
- [ ] **Conversation persistence:** Often missing crash recovery -- verify that an in-progress conversation is recoverable after a crash or force-quit, not just after clean shutdown

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Zombie processes accumulated | LOW | Run `pkill -f "claude -p"` / `pkill -f "codex"`. Add process tracking to prevent recurrence. No data loss. |
| Pipe buffer deadlock (agent hangs) | LOW | Kill the stuck child process (`kill -9 <pid>`). User re-sends the prompt. Add dedicated reader threads to prevent recurrence. |
| Git worktree corruption (stale locks) | LOW-MEDIUM | Remove stale `index.lock` files. Run `git worktree prune`. Run `git fsck` to verify repository integrity. May need to re-create affected worktrees. |
| Provider abstraction too narrow | HIGH | Requires redesigning the provider trait, provider-specific event types, and updating all rendering code. Worse if session management was omitted -- historical sessions are not resumable. Design correctly upfront. |
| Markdown rendering is O(n^2) | MEDIUM | Refactor to incremental parsing with block caching and batched updates. Isolated to the markdown module if rendering was properly separated from data storage. |
| Entity subscription silently dropped | LOW | Add the missing `Subscription` storage field. No data loss, just missed events. The difficulty is *finding* the bug -- symptoms are non-obvious (UI stops updating). Add debug logging for subscription lifecycle. |
| Conversation context lost (no --continue) | MEDIUM | Cannot retroactively add context to past turns. Implement `--continue`/`--resume` going forward. Users must re-establish context manually for existing conversations. |
| Full turn vec clone causing jank | LOW-MEDIUM | Refactor render path to use `Rc<Vec<Turn>>` or indexed access. The fix is straightforward but touches the core render loop, so test thoroughly. |

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Zombie CLI processes | Foundation / Process management | `ps aux` shows zero orphaned claude/codex processes after closing all workspaces and after simulated app crash |
| Pipe buffer deadlock | Provider abstraction / Multi-agent | Stress test: 5 agents streaming simultaneously with verbose output for 10 minutes; no hangs, no stalls |
| Git worktree corruption | Git worktree management | Create 10 worktrees, run concurrent git operations in each, close half abruptly; `git fsck` clean; `git worktree list` shows only active entries |
| Streaming markdown re-parse/flicker | Chat rendering / Markdown | Profile frame time during 1000-token code-heavy response: average <8ms per frame; no visible flicker on incomplete code blocks |
| Provider lowest-common-denominator | Provider abstraction | Claude session resume works through Helm; Codex exec mode works; adding a third provider does not require modifying the core trait |
| Entity subscription loss | Every phase adding entities | Audit: every `cx.subscribe()` / `cx.observe()` return value stored in a named struct field; no `_` bindings for subscriptions |
| Conversation context loss | Provider integration | Send 3 sequential referencing prompts ("create X", "test X", "fix failing tests for X"); agent correctly references previous context in each |
| Vec<Turn> clone per render | Chat rendering optimization | Profile memory allocation during scroll of 100-turn conversation: no per-frame allocation proportional to turn count |
| Unbounded chat storage | Session persistence | App startup time <2s with 500-turn conversation history; memory usage <200MB with lazy-loaded tool outputs |
| Text input edge cases | Input widget phase | IME composition with Japanese input method works; paste of 10KB code block with newlines preserves formatting |

## Sources

- [Rust std::process::Child documentation](https://doc.rust-lang.org/std/process/struct.Child.html) -- zombie process warnings, drop semantics
- [Zed blog: Ownership and data flow in GPUI](https://zed.dev/blog/gpui-ownership) -- entity model, subscription lifecycle, lease pattern, reentrancy prevention
- [Leveraging Rust and the GPU to render user interfaces at 120 FPS](https://zed.dev/blog/videogame) -- GPUI rendering architecture and frame budget
- [Claude Code CLI documentation: Run Claude Code programmatically](https://code.claude.com/docs/en/headless) -- stream-json format, --continue/--resume, --bare mode, session management
- [Git worktree documentation](https://git-scm.com/docs/git-worktree) -- one-branch-per-worktree rule, prune, lock semantics
- [Git Worktree Conflicts with Multiple AI Agents](https://www.termdock.com/en/blog/git-worktree-conflicts-ai-agents) -- six conflict types with diagnosis and fixes
- [Git worktree concurrent operations warning](https://github.com/kaeawc/auto-worktree/issues/176) -- single-process limitation documentation
- [Best practices to render streamed LLM responses](https://developer.chrome.com/docs/ai/render-llm-responses) -- batching strategies, incremental rendering triggers
- [Streaming AI responses and the incomplete JSON problem](https://www.aha.io/engineering/articles/streaming-ai-responses-incomplete-json) -- NDJSON edge cases, buffer splitting
- [Be careful when redirecting both stdin and stdout to pipes](https://devblogs.microsoft.com/oldnewthing/20110707-00/?p=10223) -- pipe buffer deadlock mechanics
- [Dealing with long-lived child processes in Rust](https://www.nikbrendler.com/rust-process-communication-part-2/) -- async patterns for subprocess management
- [Conductor: Run a team of coding agents](https://www.conductor.build/) -- reference architecture for multi-agent worktree isolation
- [Codex CLI reference](https://developers.openai.com/codex/cli/reference) -- JSONL format, exec mode differences from Claude
- [10 Things Developers Want from their Agentic IDEs](https://redmonk.com/kholterhoff/2025/12/22/10-things-developers-want-from-their-agentic-ides-in-2025/) -- UX expectations and common frustrations
- Helm codebase analysis: `src/provider.rs`, `src/chat.rs`, `src/text_input.rs`, `src/workspace.rs` -- existing patterns and tech debt (cross-referenced with `.planning/codebase/CONCERNS.md`)

---
*Pitfalls research for: Helm -- Native Rust/GPUI Agentic Development Environment*
*Researched: 2026-03-26*

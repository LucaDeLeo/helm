# Stack Research

**Domain:** Native Desktop Agentic Development Environment (Rust/GPUI)
**Researched:** 2026-03-26
**Confidence:** MEDIUM-HIGH (existing stack is locked; new additions verified against Zed upstream and crates.io)

## Existing Stack (Locked)

These are already in use and should not change. Listed for completeness since new dependencies must be compatible.

| Technology | Version | Purpose | Status |
|------------|---------|---------|--------|
| Rust | Edition 2024 | Application language | Locked |
| GPUI | 0.2.2 (local path) | GPU-accelerated UI framework | Locked |
| gpui_platform | 0.1.0 (local path) | Platform bootstrap | Locked |
| serde | 1.0 | Serialization/deserialization | Locked |
| serde_json | 1.0 | JSON parsing for CLI stream events | Locked |
| anyhow | 1.0 | Error handling | Locked |
| smol | 2.0 | Async runtime (matches Zed's choice) | Locked |
| async-task | git pin | Compatibility shim for GPUI async | Locked |

**Constraint:** Helm uses `smol` as its async runtime because GPUI is built on `smol`. Do NOT introduce `tokio` -- it would create a dual-runtime situation. All async crates must be `smol`-compatible (most `futures`-based crates work fine).

## Recommended Stack

### Markdown Parsing and Rendering

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| pulldown-cmark | 0.13.0 | Parse streaming markdown from agent responses | Zed uses this exact version for its markdown crate. Pull-based parser is ideal for incremental rendering -- parse events as text arrives, render them into GPUI elements progressively. CommonMark + GFM support covers all agent output (tables, code blocks, task lists, strikethrough). | HIGH |

**How it fits:** The current `chat.rs` renders agent text as raw strings via `div().child(text.clone())`. Replace this with a pulldown-cmark parser that converts markdown events into styled GPUI elements (headings, bold, code spans, code blocks, links, lists). Zed's `markdown` crate does exactly this but has 31 internal workspace dependencies -- too coupled to reuse directly. Build a lighter version using the same approach: `pulldown_cmark::Parser` -> iterate events -> emit GPUI `div`/`StyledText` elements.

**Key options to enable:**
```rust
use pulldown_cmark::{Options, Parser};

const PARSE_OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_STRIKETHROUGH)
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_GFM);
```

### Syntax Highlighting (for code blocks in chat)

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| tree-sitter | 0.26 | Parse code blocks for syntax highlighting | Zed uses tree-sitter 0.26 for all syntax highlighting. Since Helm already depends on the Zed repo at `../zed/`, using the same tree-sitter version avoids version conflicts. Tree-sitter produces a full parse tree, enabling accurate token-level highlighting that maps cleanly to GPUI's `TextRun` styling. | MEDIUM-HIGH |

**Why not syntect:** Syntect (5.3.0) is simpler to integrate and would work for Helm's read-only code block display. However, tree-sitter is what Zed uses, the grammars are already available in the Zed repo checkout, and using tree-sitter means Helm's highlighting matches Zed's exactly. This matters because users will view the same code in both tools. If tree-sitter integration proves too heavy for the first milestone, syntect is a valid fallback -- but plan for tree-sitter.

**Integration approach:** Do NOT depend on Zed's `language` crate (74 workspace deps). Instead:
1. Depend on `tree-sitter = "0.26"` directly
2. Load grammar `.so` files or compile grammars inline for the ~8 languages agents commonly output (Rust, TypeScript, Python, JSON, Bash, TOML, Markdown, HTML)
3. Use tree-sitter highlight queries (`.scm` files from Zed's `languages/` directory) to map captures to `HighlightId`s
4. Map `HighlightId` -> theme colors -> `TextRun` styles for GPUI's `StyledText`

**Fallback if tree-sitter is too heavy for milestone scope:**

| Technology | Version | Purpose | When to Use | Confidence |
|------------|---------|---------|-------------|------------|
| syntect | 5.3.0 | Simpler syntax highlighting | If tree-sitter integration takes >2 days and blocks other work. Syntect ships with bundled Sublime syntax definitions, requires no external grammar files, and outputs highlight spans directly. Trade-off: highlighting won't match Zed's exactly. | MEDIUM |

### Git and Worktree Management

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| git2 | 0.20.4 | Git operations (branch info, diff stats, status) | Zed uses git2 0.20.1 for its git crate. git2 provides full worktree API (`Repository::worktrees()`, `Worktree` struct, `WorktreeAddOptions`). Mature, well-documented, used in production by Zed, cargo, and many others. Bindings to libgit2 with vendored option. | HIGH |

**Why not gitoxide/gix:** Gitoxide is a pure-Rust git implementation that's gaining adoption, but its worktree API is less mature than git2's. Zed itself still uses git2 for all git operations despite being a Rust-first project. For Helm's needs (list worktrees, create worktrees, read branch/status/diff), git2 is proven and stable.

**Why also shell out to `git` CLI:** Zed's git crate actually does BOTH -- uses git2 for fast read operations (branch, status, blame) and shells out to the `git` CLI for write operations (worktree add/remove, push, fetch). This is the correct pattern because:
1. `git worktree add` via CLI handles hook execution, config inheritance, and edge cases that git2 may not
2. The git CLI respects user's git config (aliases, hooks, credential helpers)
3. Use `smol::process::Command` for async subprocess execution

**Recommended pattern:**
```rust
// Read operations: git2 (fast, no subprocess overhead)
let repo = git2::Repository::open(path)?;
let worktrees = repo.worktrees()?;
let head = repo.head()?;
let diff = repo.diff_index_to_workdir(None, None)?;

// Write operations: shell out to git CLI
let output = smol::process::Command::new("git")
    .args(["worktree", "add", "-b", branch_name, worktree_path])
    .current_dir(repo_path)
    .output()
    .await?;
```

**Cargo.toml addition:**
```toml
git2 = { version = "0.20", default-features = false, features = ["vendored-libgit2"] }
```

Use `vendored-libgit2` to avoid system libgit2 version mismatches (matches Zed's approach).

### Process Management (CLI Agent Subprocess Control)

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| async-process | 2.5.0 (via smol) | Async subprocess spawning and I/O | Part of the smol ecosystem. Replaces the current `std::process::Command` + `std::sync::mpsc` pattern with fully async subprocess management. Enables non-blocking stdout/stderr reading, proper process lifecycle management, and graceful termination. | HIGH |
| smol::process | (re-export) | Convenience re-export of async-process | Use `smol::process::Command` directly -- no additional dependency needed since Helm already depends on smol 2.0. | HIGH |

**Current problem:** The existing `provider.rs` uses `std::process::Command` synchronously on a background thread, with `std::sync::mpsc` for cross-thread event passing and a 16ms polling loop in `chat.rs`. This works but is inefficient and makes it hard to manage process lifecycle (no clean cancellation, no kill-on-drop).

**Recommended migration:**
```rust
use smol::io::{AsyncBufReadExt, BufReader};
use smol::process::{Command, Stdio};

async fn run_turn(prompt: &str, working_dir: Option<&str>) -> impl Stream<Item = ProviderEvent> {
    let mut child = Command::new("claude")
        .args(["-p", prompt, "--output-format", "stream-json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = BufReader::new(child.stdout.take().unwrap());
    let mut lines = stdout.lines();

    while let Some(line) = lines.next().await {
        let line = line?;
        // Parse and yield ProviderEvent
    }
}
```

**Graceful termination:** Use `child.kill()` on drop or explicit cancel. GPUI's `Task` handles cancellation automatically when the entity is dropped.

### Streaming JSON Parsing

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| serde_json (already in stack) | 1.0 | Line-delimited JSON parsing for both Claude and Codex CLI output | Both Claude (`--output-format stream-json`) and Codex (`--json`) emit newline-delimited JSON (one JSON object per line). The existing `serde_json::from_str` approach of parsing each line independently is correct and sufficient. No streaming JSON parser needed -- the stream boundary is the newline, not partial JSON. | HIGH |

**Why NOT a streaming JSON parser:** Libraries like `json-stream-parser` or `struson` handle partial/incomplete JSON objects. Neither Claude nor Codex emit partial JSON -- each line is a complete, self-contained JSON object. The current pattern of `BufRead::lines()` + `serde_json::from_str()` per line is the right approach. Just make it async with `AsyncBufReadExt::lines()`.

**Provider abstraction for Claude + Codex:**

Claude stream-json events: `assistant`, `content_block_delta`, `result`, `error`
Codex JSONL events: `thread.started`, `turn.started`, `item.started`, `item.completed`, `turn.completed`, `error`

Both are newline-delimited JSON but with different schemas. The provider trait should normalize both into a common `ProviderEvent` enum (which already exists).

### Diff Display

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| similar | 2.7.0 | Generate inline diffs for display in chat | Best Rust diff library. Supports Myers and Patience algorithms. The `inline` feature provides word-level diff highlighting within changed lines -- essential for showing what an agent edited. Used widely in production (insta test framework, etc). | HIGH |

**Why not imara-diff:** Zed uses imara-diff (0.1.8) internally, but `similar` has a better high-level API for Helm's use case (displaying diffs in a chat UI, not computing diffs for a buffer engine). `similar::TextDiff` provides unified diff output and inline change emphasis directly.

**Why not diffy:** Diffy (0.4.2) is for parsing and applying patch files (unified diff format). Helm needs to generate diffs from two strings and display them, not apply patches. `similar` is the right tool.

**Cargo.toml addition:**
```toml
similar = { version = "2.7", features = ["inline"] }
```

### Supporting Libraries

| Library | Version | Purpose | When to Use | Confidence |
|---------|---------|---------|-------------|------------|
| futures | 0.3 | Stream/Future combinators for async pipelines | When composing async streams from provider output. Needed for `Stream` trait, `StreamExt`, channel-based patterns. | HIGH |
| linkify | latest | Auto-detect URLs in plain text | When rendering agent text that contains URLs not in markdown link format. Zed's markdown crate uses this. | MEDIUM |
| regex | 1.0 | Pattern matching for provider output parsing | For extracting file paths, diff headers, and other structured text from agent output that isn't JSON. | HIGH |
| log | 0.4 | Structured logging | For debug logging of provider events, git operations, process lifecycle. | HIGH |
| dirs | 5.0 | Platform-standard config/data directories | For storing Helm config, project state, and worktree mappings in `~/.config/helm/` and `~/.local/share/helm/`. | MEDIUM |
| uuid | 1.0 | Unique identifiers for workspaces and turns | For generating workspace IDs, correlating tool calls across provider events. | MEDIUM |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| cargo clippy | Lint checking | Run with `-- -W clippy::pedantic` for strict checks |
| cargo fmt | Code formatting | Use Rust 2024 edition formatting rules |
| cargo test | Unit/integration tests | Add tests alongside modules as the codebase grows |

## Installation

```toml
# Add to [dependencies] in Cargo.toml

# Markdown rendering
pulldown-cmark = { version = "0.13.0", default-features = false }

# Syntax highlighting (choose one approach)
# Option A: tree-sitter (recommended, matches Zed)
tree-sitter = "0.26"
# Plus individual grammar crates as needed:
# tree-sitter-rust, tree-sitter-typescript, tree-sitter-python, etc.

# Option B: syntect (simpler fallback)
# syntect = { version = "5.3", default-features = false, features = ["default-syntaxes", "default-themes", "regex-fancy"] }

# Git operations
git2 = { version = "0.20", default-features = false, features = ["vendored-libgit2"] }

# Diff display
similar = { version = "2.7", features = ["inline"] }

# Async support (smol already provides process management)
futures = "0.3"

# Utilities
log = "0.4"
regex = "1"
dirs = "5"
uuid = { version = "1", features = ["v4"] }
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| pulldown-cmark 0.13 | comrak | If you need full GitHub-Flavored Markdown rendering including raw HTML sanitization. Comrak is a GFM-compliant parser/renderer but is heavier and Helm doesn't need HTML output. |
| tree-sitter 0.26 | syntect 5.3 | If tree-sitter grammar management proves too complex for the first milestone. Syntect bundles grammars and themes, requires zero configuration, but won't match Zed's highlighting. |
| git2 0.20 | gitoxide/gix | When gitoxide's worktree API matures (possibly 2026-2027). Pure Rust, no C dependency, potentially faster. But today its high-level API is less complete than git2's. |
| similar 2.7 | imara-diff 0.2 | If you need maximum diff performance on very large files. imara-diff is lower-level and faster, but similar's API is better for UI-oriented diff display. |
| smol (process) | tokio::process | Never for Helm. GPUI is built on smol. Mixing runtimes causes thread pool conflicts and subtle bugs. |
| serde_json line-by-line | json-stream-parser | If a future provider emits partial/fragmented JSON within a single line. No current provider does this. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| tokio | GPUI uses smol. Dual async runtimes cause thread pool conflicts, executor contention, and subtle deadlocks. | smol 2.0 (already in stack) |
| Zed's `markdown` crate | 31 internal workspace dependencies. Pulls in `ui`, `theme`, `settings`, `language`, and many more Zed-internal crates. Would couple Helm to Zed's full component system. | pulldown-cmark directly + custom GPUI renderer |
| Zed's `language` crate | 74 internal workspace dependencies. Designed for Zed's full editor with LSP, buffer management, file system integration. Massive overkill. | tree-sitter directly + highlight queries |
| Zed's `git` crate | 37 internal workspace dependencies. Tightly coupled to Zed's project/worktree model and UI abstractions. | git2 directly + git CLI for write operations |
| reqwest/hyper | Helm doesn't make HTTP requests. It spawns CLI subprocesses. No HTTP client needed. | smol::process for subprocess management |
| crossterm/ratatui | Terminal UI frameworks. Helm uses GPUI for GPU-accelerated native rendering. | GPUI (already in stack) |
| egui/iced | Alternative Rust GUI frameworks. Would require abandoning the existing GPUI codebase entirely. | GPUI (already in stack) |

## Stack Patterns by Context

**If adding a new CLI provider (beyond Claude and Codex):**
- Parse its `--help` to find JSON/JSONL output flags
- Implement the `Provider` trait (to be defined) with the same `async fn run_turn() -> Stream<ProviderEvent>` signature
- Both Claude and Codex use newline-delimited JSON; assume new providers will too
- If a provider uses SSE instead, add `eventsource-client` crate (smol-compatible)

**If syntax highlighting scope grows beyond code blocks:**
- Move from loading individual grammars to a `LanguageRegistry` pattern (inspired by but not importing Zed's)
- Cache compiled grammars in `~/.local/share/helm/grammars/`

**If git operations need to go beyond read + worktree management:**
- Consider shelling out to `git` for ALL operations (Zed's hybrid approach is an optimization)
- For push/fetch: definitely use `git` CLI (handles SSH auth, credential helpers)

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| pulldown-cmark 0.13 | serde_json 1.0 | No direct interaction; markdown events are not serialized |
| tree-sitter 0.26 | Zed's tree-sitter 0.26 | Must match Zed's version to avoid linking conflicts since GPUI is a path dependency |
| git2 0.20 | smol 2.0 | git2 is sync; wrap in `cx.background_spawn()` for async |
| similar 2.7 | No dependencies | Standalone, no compatibility concerns |
| smol 2.0 | async-task (pinned) | The `async-task` git pin in Cargo.toml is required for GPUI compatibility. Do not remove. |
| futures 0.3 | smol 2.0 | smol is built on top of futures traits. Full compatibility. |

## Sources

- [pulldown-cmark on crates.io](https://crates.io/crates/pulldown-cmark) -- version 0.13.0 confirmed (2025-02-12 release)
- [Zed's Cargo.toml](../zed/Cargo.toml) -- confirmed pulldown-cmark 0.13.0, git2 0.20.1, tree-sitter 0.26, smol 2.0
- [Zed's markdown crate](../zed/crates/markdown/) -- confirmed pulldown-cmark integration pattern with GPUI, 31 workspace deps
- [Zed's git crate](../zed/crates/git/) -- confirmed git2 + git CLI hybrid pattern, worktree support, 37 workspace deps
- [Zed's language crate](../zed/crates/language/) -- confirmed tree-sitter integration, 74 workspace deps
- [git2 on crates.io](https://crates.io/crates/git2) -- version 0.20.4 confirmed (HIGH confidence)
- [git2 Worktree API](https://docs.rs/git2/latest/git2/struct.Worktree.html) -- worktree list/add/prune API confirmed (HIGH confidence)
- [tree-sitter on crates.io](https://crates.io/crates/tree-sitter) -- version 0.26.7 confirmed (HIGH confidence)
- [tree-sitter-highlight on crates.io](https://crates.io/crates/tree-sitter-highlight) -- version 0.26.3 available (MEDIUM confidence on API stability)
- [similar on crates.io](https://crates.io/crates/similar) -- version 2.7.0 confirmed (HIGH confidence)
- [syntect on crates.io](https://crates.io/crates/syntect) -- version 5.3.0 confirmed as fallback option (HIGH confidence)
- [smol process docs](https://docs.rs/smol/latest/smol/process/) -- async subprocess API confirmed (HIGH confidence)
- [async-process on crates.io](https://crates.io/crates/async-process) -- version 2.5.0 confirmed (HIGH confidence)
- [Codex CLI reference](https://developers.openai.com/codex/cli/reference) -- `--json` flag for JSONL output confirmed (HIGH confidence)
- [Codex CLI non-interactive mode](https://developers.openai.com/codex/noninteractive) -- JSONL event types confirmed: thread.started, turn.started/completed, item.started/completed, error (HIGH confidence)
- [gitoxide/gix](https://github.com/GitoxideLabs/gitoxide) -- evaluated and deferred; worktree API less mature than git2 (MEDIUM confidence)
- [imara-diff on crates.io](https://crates.io/crates/imara-diff) -- version 0.2.0, deferred in favor of similar's higher-level API (HIGH confidence)

---
*Stack research for: Helm -- Native Desktop ADE (Rust/GPUI)*
*Researched: 2026-03-26*

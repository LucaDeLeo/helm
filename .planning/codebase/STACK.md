# Technology Stack

**Analysis Date:** 2026-03-26

## Languages

**Primary:**
- Rust (Edition 2024) - All application code in `src/`

**Secondary:**
- None

## Runtime

**Environment:**
- Native desktop binary (compiled via `cargo build`)
- GPU-accelerated rendering via GPUI (Zed's UI framework)
- macOS primary target (via `gpui_platform` with `gpui_macos` backend)

**Package Manager:**
- Cargo 1.93.1
- Lockfile: present (`Cargo.lock`, 6631 lines, ~200+ transitive dependencies)

## Frameworks

**Core:**
- `gpui` 0.2.2 (local path: `../zed/crates/gpui`) - Zed's GPU-accelerated immediate-mode UI framework. Provides the reactive entity model, window management, layout engine, text rendering, key bindings, focus management, and element painting.
- `gpui_platform` 0.1.0 (local path: `../zed/crates/gpui_platform`) - Platform abstraction layer. Used with `font-kit` feature enabled. Provides `application()` entry point.

**Testing:**
- Not detected (no test framework configured, no test files present)

**Build/Dev:**
- `cargo` - Standard Rust build system
- No custom build scripts (`build.rs`) detected

## Key Dependencies

**Critical:**
- `gpui` 0.2.2 - The entire UI framework; every source file imports from it. Provides `Entity`, `Context`, `Window`, `Render` trait, layout primitives (`div`, `px`, `size`), `ListState`, `Focusable`, `IntoElement`, text shaping, and the action/key-binding system.
- `gpui_platform` 0.1.0 - Provides the cross-platform `application()` bootstrap function used in `src/main.rs`.

**Infrastructure:**
- `serde` 1.0.228 (with `derive` feature) - JSON deserialization of Claude CLI stream events in `src/provider.rs`
- `serde_json` 1.0.149 - Parsing stream-json output from `claude` CLI in `src/provider.rs`
- `anyhow` 1.0.102 - Error handling (`anyhow::Result`) used in async task returns in `src/chat.rs`
- `smol` 2.0.2 - Async runtime; `smol::Timer` used for polling in `src/chat.rs`

**Patched:**
- `async-task` - Pinned to specific git revision (`b4486cd`) from `smol-rs/async-task` via `[patch.crates-io]` in `Cargo.toml`. Required for compatibility with gpui's async runtime.

## Configuration

**Environment:**
- No environment variables required by Helm itself
- The `claude` CLI (spawned as subprocess) handles its own authentication and configuration
- No `.env` files present or expected

**Build:**
- `Cargo.toml` - Workspace with single member, Rust edition 2024
- Binary target: `helm` at `src/main.rs`
- `.gitignore` - Contains only `/target`
- `.emdash.json` - Emdash tool config (preservePatterns for env files, empty scripts)

## Platform Requirements

**Development:**
- Rust 1.93+ (edition 2024 requires nightly or recent stable)
- Local clone of Zed repository at `../zed/` (relative to project root) providing `gpui` and `gpui_platform` crates
- `claude` CLI installed globally (`npm install -g @anthropic-ai/claude-code`) for AI chat functionality
- macOS recommended (primary platform; `gpui_platform` also supports Linux/Windows/Web but those are untested for Helm)

**Production:**
- Native desktop binary (no server deployment)
- macOS (primary), potentially Linux/Windows via gpui_platform backends

---

*Stack analysis: 2026-03-26*

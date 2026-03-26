# Codebase Concerns

**Analysis Date:** 2026-03-26

## Tech Debt

**Single-line text input only:**
- Issue: `TextInput` in `src/text_input.rs` is a single-line input widget. Paste strips newlines (line 138: `text.replace("\n", " ")`). There is no multiline editor for composing longer prompts.
- Files: `src/text_input.rs`
- Impact: Users cannot compose multi-paragraph prompts or paste code blocks. All newlines are flattened to spaces.
- Fix approach: Build a multiline `TextArea` variant or integrate GPUI's built-in editor if one exists. At minimum, support shift-enter for newlines.

**No conversation history or session persistence:**
- Issue: `ChatPanel` stores turns in a `Vec<Turn>` in memory only. The sidebar shows "No sessions yet" as a static string (line 59). There is no save/load, no session management, no conversation resumption.
- Files: `src/chat.rs`, `src/sidebar.rs`
- Impact: All conversation context is lost on application restart. Users cannot switch between conversations or review past sessions.
- Fix approach: Serialize `Vec<Turn>` to disk (JSON or SQLite). Add session listing to `Sidebar`. Wire up session selection to load/save turns.

**Placeholder plan panel:**
- Issue: `PlanPanel` is a static placeholder that renders "No active plan" and nothing else. It has no data model, no interactivity, no connection to the chat or provider.
- Files: `src/plan.rs`
- Impact: The right panel occupies 300px of screen space but provides no value. The three-panel layout implies planning functionality that does not exist.
- Fix approach: Either implement plan tracking (parse structured output from Claude, display task lists) or remove the panel until ready.

**No conversation context passed to Claude CLI:**
- Issue: `provider::run_turn` (line 74-149 in `src/provider.rs`) spawns a fresh `claude -p` process per turn with only the current prompt. Previous conversation turns are not passed as context. Each turn is stateless from Claude's perspective.
- Files: `src/provider.rs` (line 83: `cmd.args(["-p", prompt, ...])`)
- Impact: Claude has no memory of previous messages in the conversation. Follow-up questions and iterative work are broken because the model sees each message in isolation.
- Fix approach: Use the `--resume` or `--conversation-id` flag of the Claude CLI, or pass the full conversation history via `--continue`. Alternatively, accumulate messages and pass them as a structured prompt.

**Fixed panel widths, no resizing:**
- Issue: Sidebar is hardcoded to 250px (`src/workspace.rs` line 44), plan panel to 300px (line 57). There are no drag handles or resize logic.
- Files: `src/workspace.rs`
- Impact: Users cannot adjust the layout. On smaller screens the chat area may be too narrow. On larger screens the sidebar/plan panels may be too small.
- Fix approach: Add draggable dividers with min/max width constraints. Store widths in layout state.

**Heavy cloning in render path:**
- Issue: `ChatPanel::render` clones the entire `self.turns` vector and `self.expanded_tools` HashSet on every frame (lines 289, 292 in `src/chat.rs`).
- Files: `src/chat.rs` (lines 289-293)
- Impact: As conversation grows, each render allocates and copies all turns including all text content and tool call data. This will cause increasing GC pressure and frame drops during long conversations.
- Fix approach: Use references or indices into turns rather than cloning. Alternatively, use `Rc<Vec<Turn>>` with copy-on-write semantics, or restructure to pass turn data through the list callback without full cloning.

## Known Bugs

**`truncate` panics on multi-byte UTF-8:**
- Symptoms: Calling `truncate(s, max)` where `max` falls inside a multi-byte character causes a panic at `&s[..max]` because Rust string slicing requires char boundaries.
- Files: `src/provider.rs` (lines 189-195)
- Trigger: Any tool call input containing non-ASCII characters (CJK, emoji, accented characters) where the truncation point lands mid-character.
- Workaround: None currently.
- Fix: Use `s.char_indices()` to find the last valid boundary at or before `max`, or use `s.floor_char_boundary(max)` (nightly) / a manual equivalent.

**Detached background task has no error propagation:**
- Symptoms: If `provider::run_turn` panics or the background task fails, the error is silently swallowed because the spawned future is `.detach()`-ed (line 185 in `src/chat.rs`).
- Files: `src/chat.rs` (line 182-185)
- Trigger: Any panic inside `run_turn` or if the `tx` sender is dropped before sending `Complete`.
- Workaround: The polling loop will eventually exit on `Disconnected`, but the user sees no error message.
- Fix: Remove `.detach()` and hold the task handle. Propagate errors through a `JoinHandle` or by sending an error event before the task exits.

**Polling loop uses busy-wait with timer workaround:**
- Symptoms: The event polling loop in `send_turn` (lines 187-222 in `src/chat.rs`) uses `try_recv` in a loop, sleeping 16ms via `smol::Timer` when empty. This is not a true async wait -- it polls at ~60Hz even when idle.
- Files: `src/chat.rs` (lines 187-222)
- Trigger: Every active turn. The loop runs continuously until `Complete` is received.
- Workaround: The 16ms sleep prevents pure busy-spinning but still wastes CPU.
- Fix: Use an async channel (`smol::channel` or `futures::channel::mpsc`) instead of `std::sync::mpsc` so the receiver can `.await` without polling. This would also eliminate the `smol` dependency used solely for the timer.

## Security Considerations

**User input passed directly to shell command:**
- Risk: The user's prompt string is passed directly as a CLI argument to `claude -p` (line 83 in `src/provider.rs`). While `Command::new` with `.args()` does not invoke a shell and should handle this safely, the prompt is not sanitized or length-limited.
- Files: `src/provider.rs` (lines 79-83)
- Current mitigation: Rust's `std::process::Command` passes arguments directly to the OS without shell interpretation, so shell injection is not possible via this vector.
- Recommendations: Add a maximum prompt length to prevent OS argument length limits from being exceeded (`ARG_MAX`). Consider using stdin piping instead of CLI arguments for long prompts.

**No authentication or access control:**
- Risk: The application inherits whatever API key and permissions the `claude` CLI has configured. There is no scoping, no permission prompts for tool use, no confirmation before executing tool calls.
- Files: `src/provider.rs` (entire file)
- Current mitigation: The Claude CLI itself handles auth and may prompt for tool permissions.
- Recommendations: Add a confirmation step in the UI before allowing tool execution results to be acted upon. Display tool inputs to the user before execution.

## Performance Bottlenecks

**Full turn vector cloned on every render:**
- Problem: `self.turns.clone()` in `ChatPanel::render` (line 289 of `src/chat.rs`) performs a deep clone of all conversation data on every frame.
- Files: `src/chat.rs` (line 289)
- Cause: The `list()` closure requires `'static` data, so the turns vector is cloned to move into the closure.
- Improvement path: Store turns in an `Rc<Vec<Turn>>` or use indices with a shared reference. Alternatively, extract only the data needed for visible items.

**Synchronous CLI availability check on startup:**
- Problem: `provider::is_claude_available()` (line 63-71 in `src/provider.rs`) spawns a blocking `claude --version` process on the main thread during `ChatPanel::new`.
- Files: `src/provider.rs` (lines 63-71), `src/chat.rs` (line 115)
- Cause: Called synchronously in the `ChatPanel` constructor.
- Improvement path: Move this check to a background task and update the UI asynchronously. Show a "checking..." state initially.

**Line-by-line JSON parsing without buffering strategy:**
- Problem: `provider::run_turn` reads stdout line by line with `BufReader` and deserializes each line individually. For high-throughput streaming with many small events, this creates many small allocations.
- Files: `src/provider.rs` (lines 112-133)
- Cause: Each line is allocated as a new `String`, parsed into `RawEvent`, then converted to `ProviderEvent`.
- Improvement path: Use a pre-allocated buffer for reading lines. Consider batch deserialization or a streaming JSON parser.

## Fragile Areas

**TextInput custom Element implementation:**
- Files: `src/text_input.rs` (lines 456-641)
- Why fragile: `TextElement` implements the raw GPUI `Element` trait with manual layout, prepaint, and paint phases. It caches `last_layout` and `last_bounds` between frames, creating implicit state coupling. The `prepaint` phase reads from the `TextInput` entity and the `paint` phase writes back to it, creating a bidirectional dependency within a single render cycle.
- Safe modification: Changes to text rendering must preserve the layout-prepaint-paint ordering. Any change to `last_layout` or `last_bounds` usage requires testing with IME input (marked ranges) and mouse selection simultaneously.
- Test coverage: Zero. No tests exist anywhere in the codebase.

**Provider event parsing with loose deserialization:**
- Files: `src/provider.rs` (lines 24-60, 213-279)
- Why fragile: `RawEvent` uses `#[serde(default)]` on every field, making nearly any JSON object deserialize successfully. Unrecognized `event_type` values are silently ignored (line 275: `_ => {}`). Silently dropped JSON parse errors at line 128 (`if let Ok(event) = ...`) mean malformed output from the Claude CLI is invisible.
- Safe modification: Add logging for unrecognized event types and parse failures. Consider using tagged enums (`#[serde(tag = "type")]`) for stricter parsing.
- Test coverage: Zero.

**ListState splice coordination:**
- Files: `src/chat.rs` (lines 139-141, 257, 274, 281)
- Why fragile: `ListState::splice` must be called with correct ranges every time the `turns` vector changes. The `push_turn`, `handle_provider_event`, and `toggle_tool_expanded` methods all manipulate both `self.turns` and `self.list_state` independently. An off-by-one error in any splice call would cause the list to render incorrectly or panic.
- Safe modification: Extract turn management into a dedicated struct that keeps `Vec<Turn>` and `ListState` in sync atomically. Never modify one without the other.
- Test coverage: Zero.

## Scaling Limits

**In-memory conversation storage:**
- Current capacity: All turns stored in `Vec<Turn>` in memory.
- Limit: Long conversations with many tool calls (each storing full input/output strings) will grow unboundedly. Hundreds of tool calls with large outputs (file contents, grep results) could consume significant memory.
- Scaling path: Implement pagination or lazy loading of turn content. Store tool outputs on disk and load on demand. Cap stored output length.

**Single-process Claude CLI spawning:**
- Current capacity: One `claude -p` process at a time per `ChatPanel`.
- Limit: The `is_responding` flag (line 158) blocks new turns while one is in progress. Long-running tool executions block the entire conversation.
- Scaling path: Allow cancellation of in-progress turns. Consider a request queue or concurrent turn handling.

## Dependencies at Risk

**Local path dependencies on Zed crates:**
- Risk: `gpui` and `gpui_platform` are referenced via relative paths (`../zed/crates/gpui`, `../zed/crates/gpui_platform`). These are not versioned. Any change in the upstream Zed repository can break the build without warning. The GPUI API is unstable and evolving.
- Impact: Build failures after Zed upstream changes. API breakage requiring code changes in `helm`. Cannot build without a specific Zed checkout at the sibling path.
- Migration plan: Pin to a specific Zed commit via git dependency. Or vendor the needed GPUI crates. Long-term, depend on published crate versions if GPUI stabilizes.

**Patched `async-task` dependency:**
- Risk: `Cargo.toml` patches `async-task` to a specific git revision (`b4486cd`). This is fragile -- if the upstream rev is removed or the patch becomes incompatible with newer dependency versions, the build breaks.
- Impact: Build failures if the git rev becomes unavailable or if other dependencies update `async-task` requirements.
- Migration plan: Track why this patch is needed. Move to an official release of `async-task` when the fix is merged upstream.

## Missing Critical Features

**No cancellation of in-progress turns:**
- Problem: Once a turn is sent, there is no way to cancel it. The `is_responding` flag prevents new sends but provides no abort mechanism. The spawned `claude` child process runs to completion.
- Blocks: Users cannot stop runaway tool executions or long responses. Must wait or restart the application.

**No error recovery UI:**
- Problem: Errors are displayed as system messages but the user has no way to retry a failed turn, edit and resend, or recover from errors other than typing a new message.
- Blocks: Any transient failure (network, CLI crash) requires the user to re-type their prompt.

**No working directory selection:**
- Problem: `provider::run_turn` accepts an optional `working_dir` parameter but it is always called with `None` (line 183 in `src/chat.rs`). Claude's tool calls operate in whatever directory the helm process was started from.
- Blocks: Users cannot point Claude at a specific project directory without launching Helm from that directory.

**No markdown rendering:**
- Problem: All text content is rendered as plain text. Claude's responses typically contain markdown (headings, code blocks, lists, bold/italic) which appears as raw markup characters.
- Blocks: Code blocks are unformatted and hard to read. Structured responses lose their visual hierarchy.

## Test Coverage Gaps

**Entire codebase is untested:**
- What's not tested: There are zero tests in the entire project. No unit tests, no integration tests, no UI tests.
- Files: All files in `src/` -- `src/chat.rs`, `src/provider.rs`, `src/text_input.rs`, `src/workspace.rs`, `src/sidebar.rs`, `src/plan.rs`, `src/theme.rs`, `src/main.rs`
- Risk: Any refactoring, GPUI API update, or feature addition could introduce regressions with no safety net. The most critical untested areas are:
  - `provider.rs` event parsing -- malformed JSON, unexpected event types, missing fields
  - `text_input.rs` UTF-16/UTF-8 offset conversion -- incorrect boundaries cause panics
  - `chat.rs` turn/list synchronization -- off-by-one errors cause rendering bugs or panics
  - `provider.rs` `truncate` function -- already has a known UTF-8 boundary bug
- Priority: High. At minimum, add unit tests for `parse_event`, `truncate`, `summarize_tool_input`, and the UTF-16 offset conversion functions. These are pure functions with well-defined inputs and outputs.

---

*Concerns audit: 2026-03-26*

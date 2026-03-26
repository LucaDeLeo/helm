use std::collections::HashSet;
use std::sync::mpsc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    AnyElement, Context, CursorStyle, Entity, Focusable, FontWeight, ListAlignment, ListState,
    Subscription, Task, WeakEntity, Window, div, list, px,
};

use crate::provider::{self, ProviderEvent};
use crate::text_input::{TextInput, TextInputEvent};
use crate::theme;

// ── Data Model ──────────────────────────────────────────────────────

#[derive(Clone)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Clone)]
pub enum ContentBlock {
    Text(String),
    ToolCall {
        id: String,
        name: String,
        input: String,
        output: String,
        is_error: bool,
    },
}

#[derive(Clone)]
pub struct Turn {
    pub role: MessageRole,
    pub blocks: Vec<ContentBlock>,
}

impl Turn {
    fn tool_call_count(&self) -> usize {
        self.blocks
            .iter()
            .filter(|b| matches!(b, ContentBlock::ToolCall { .. }))
            .count()
    }

    fn append_text(&mut self, text: &str) {
        if let Some(ContentBlock::Text(existing)) = self.blocks.last_mut() {
            existing.push_str(text);
        } else {
            self.blocks.push(ContentBlock::Text(text.to_string()));
        }
    }

    fn add_tool_call(&mut self, id: String, name: String, input: String) {
        self.blocks.push(ContentBlock::ToolCall {
            id,
            name,
            input,
            output: String::new(),
            is_error: false,
        });
    }

    fn update_tool_result(&mut self, tool_use_id: &str, output: String, is_error: bool) {
        for block in &mut self.blocks {
            if let ContentBlock::ToolCall { id, output: out, is_error: err, .. } = block {
                if id == tool_use_id {
                    *out = output;
                    *err = is_error;
                    return;
                }
            }
        }
    }

    fn is_empty_text(&self) -> bool {
        self.blocks.iter().all(|b| match b {
            ContentBlock::Text(t) => t.is_empty(),
            _ => false,
        })
    }
}

// ── Chat Panel ──────────────────────────────────────────────────────

pub struct ChatPanel {
    turns: Vec<Turn>,
    list_state: ListState,
    input: Entity<TextInput>,
    _subscription: Subscription,
    is_responding: bool,
    _pending_task: Option<Task<anyhow::Result<()>>>,
    expanded_tools: HashSet<String>,
}

impl ChatPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextInput::new("Send a message to Claude...", cx));
        let subscription = cx.subscribe(&input, Self::on_input_event);

        let mut this = Self {
            turns: Vec::new(),
            list_state: ListState::new(0, ListAlignment::Bottom, px(200.)),
            input,
            _subscription: subscription,
            is_responding: false,
            _pending_task: None,
            expanded_tools: HashSet::new(),
        };

        if !provider::is_claude_available() {
            this.push_turn(Turn {
                role: MessageRole::System,
                blocks: vec![ContentBlock::Text(
                    "Claude CLI not found. Install: npm install -g @anthropic-ai/claude-code"
                        .to_string(),
                )],
            });
        }

        this
    }

    pub fn focus_input(&self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = self.input.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
    }

    fn toggle_tool_expanded(&mut self, id: &str, turn_idx: usize) {
        if !self.expanded_tools.remove(id) {
            self.expanded_tools.insert(id.to_string());
        }
        // Force the list to remeasure this item (height changed)
        if turn_idx < self.turns.len() {
            self.list_state
                .splice(turn_idx..turn_idx + 1, 1);
        }
    }

    fn on_input_event(
        &mut self,
        _input: Entity<TextInput>,
        event: &TextInputEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            TextInputEvent::Submit(text) => {
                self.send_turn(text.clone(), cx);
            }
        }
    }

    fn send_turn(&mut self, prompt: String, cx: &mut Context<Self>) {
        if self.is_responding {
            return;
        }

        // User turn
        self.push_turn(Turn {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text(prompt.clone())],
        });

        self.input.update(cx, |input, cx| input.clear(cx));

        // Empty assistant turn to stream into
        let assistant_idx = self.turns.len();
        self.push_turn(Turn {
            role: MessageRole::Assistant,
            blocks: Vec::new(),
        });

        self.is_responding = true;
        cx.notify();

        let (tx, rx) = mpsc::channel::<ProviderEvent>();

        cx.background_spawn(async move {
            provider::run_turn(&prompt, None, &tx);
        })
        .detach();

        let task = cx.spawn(async move |this, cx| {
            loop {
                let mut events = Vec::new();
                match rx.try_recv() {
                    Ok(event) => events.push(event),
                    Err(mpsc::TryRecvError::Disconnected) => break,
                    Err(mpsc::TryRecvError::Empty) => {}
                }
                while let Ok(event) = rx.try_recv() {
                    events.push(event);
                }

                if events.is_empty() {
                    cx.background_spawn(async {
                        smol::Timer::after(Duration::from_millis(16)).await;
                    })
                    .await;
                    continue;
                }

                let mut done = false;
                for event in events {
                    if matches!(event, ProviderEvent::Complete) {
                        done = true;
                    }
                    this.update(cx, |this, cx| {
                        this.handle_provider_event(event, assistant_idx, cx);
                    })?;
                }

                if done {
                    break;
                }
            }
            Ok(())
        });

        self._pending_task = Some(task);
        cx.notify();
    }

    fn handle_provider_event(
        &mut self,
        event: ProviderEvent,
        assistant_idx: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(turn) = self.turns.get_mut(assistant_idx) else {
            return;
        };

        match event {
            ProviderEvent::TextDelta(text) => {
                turn.append_text(&text);
            }
            ProviderEvent::ToolUse { id, name, input } => {
                turn.add_tool_call(id, name, input);
            }
            ProviderEvent::ToolResult {
                tool_use_id,
                output,
                is_error,
            } => {
                turn.update_tool_result(&tool_use_id, output, is_error);
            }
            ProviderEvent::Complete => {
                self.is_responding = false;
                if let Some(turn) = self.turns.get(assistant_idx) {
                    if turn.blocks.is_empty() || turn.is_empty_text() {
                        self.turns.remove(assistant_idx);
                        self.list_state.splice(assistant_idx..assistant_idx + 1, 0);
                        cx.notify();
                        return;
                    }
                }
            }
            ProviderEvent::Error(err) => {
                self.is_responding = false;
                self.push_turn(Turn {
                    role: MessageRole::System,
                    blocks: vec![ContentBlock::Text(format!("Error: {err}"))],
                });
            }
        }

        // Remeasure the assistant turn in the list
        self.list_state
            .splice(assistant_idx..assistant_idx + 1, 1);
        cx.notify();
    }

    fn push_turn(&mut self, turn: Turn) {
        let index = self.turns.len();
        self.turns.push(turn);
        self.list_state.splice(index..index, 1);
    }
}

// ── Render ───────────────────────────────────────────────────────────

impl Render for ChatPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let turns = self.turns.clone();
        let has_turns = !turns.is_empty();
        let is_responding = self.is_responding;
        let expanded_tools = self.expanded_tools.clone();
        let entity = cx.entity().downgrade();

        div()
            .size_full()
            .bg(theme::BG_BASE)
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .when(!has_turns, |this| {
                        this.flex()
                            .justify_center()
                            .items_center()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_color(theme::TEXT_PRIMARY)
                                            .text_lg()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("Start a conversation"),
                                    )
                                    .child(
                                        div()
                                            .text_color(theme::TEXT_MUTED)
                                            .text_sm()
                                            .child("Type a message below to talk to Claude"),
                                    ),
                            )
                    })
                    .when(has_turns, |this| {
                        this.child(
                            list(self.list_state.clone(), move |ix, _window, _cx| {
                                render_turn(
                                    &turns[ix],
                                    &expanded_tools,
                                    entity.clone(),
                                    ix,
                                )
                            })
                            .size_full(),
                        )
                    }),
            )
            .child(div().h(px(1.)).w_full().bg(theme::BG_OVERLAY))
            .child(
                div()
                    .flex_shrink_0()
                    .w_full()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .when(is_responding, |this| {
                        this.child(
                            div()
                                .text_color(theme::TEXT_MUTED)
                                .text_xs()
                                .child("Claude is thinking..."),
                        )
                    })
                    .child(
                        div()
                            .w_full()
                            .bg(theme::BG_SURFACE)
                            .rounded_lg()
                            .border_1()
                            .border_color(theme::BG_OVERLAY)
                            .text_color(theme::TEXT_PRIMARY)
                            .child(self.input.clone()),
                    ),
            )
    }
}

fn render_turn(
    turn: &Turn,
    expanded_tools: &HashSet<String>,
    entity: WeakEntity<ChatPanel>,
    turn_idx: usize,
) -> AnyElement {
    let (label, label_color) = match turn.role {
        MessageRole::User => ("You", theme::ACCENT),
        MessageRole::Assistant => ("Claude", theme::TEXT_MUTED),
        MessageRole::System => ("System", theme::TEXT_MUTED),
    };

    let is_system = matches!(turn.role, MessageRole::System);
    let tool_count = turn.tool_call_count();

    let mut content = div().flex().flex_col().gap_1();

    // Role label with optional tool count badge
    content = content.child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(label_color)
                    .child(label),
            )
            .when(tool_count > 0, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(theme::TEXT_MUTED)
                        .child(format!(
                            "{tool_count} tool call{}",
                            if tool_count == 1 { "" } else { "s" }
                        )),
                )
            }),
    );

    // Content blocks
    for (block_idx, block) in turn.blocks.iter().enumerate() {
        match block {
            ContentBlock::Text(text) => {
                if !text.is_empty() {
                    content = content.child(
                        div()
                            .text_sm()
                            .text_color(if is_system {
                                theme::TEXT_MUTED
                            } else {
                                theme::TEXT_PRIMARY
                            })
                            .child(text.clone()),
                    );
                }
            }
            ContentBlock::ToolCall {
                id,
                name,
                input,
                output,
                is_error,
            } => {
                let is_expanded = expanded_tools.contains(id);
                content = content.child(render_tool_call(
                    id,
                    name,
                    input,
                    output,
                    *is_error,
                    is_expanded,
                    entity.clone(),
                    turn_idx,
                    block_idx,
                ));
            }
        }
    }

    div()
        .w_full()
        .px_4()
        .py_2()
        .child(content)
        .into_any()
}

fn render_tool_call(
    id: &str,
    name: &str,
    input: &str,
    output: &str,
    is_error: bool,
    is_expanded: bool,
    entity: WeakEntity<ChatPanel>,
    turn_idx: usize,
    _block_idx: usize,
) -> AnyElement {
    let chevron = if is_expanded { "▼" } else { "▶" };
    let id_owned = id.to_string();
    let element_id = SharedString::from(format!("tool-{id}"));

    let mut container = div()
        .id(element_id)
        .w_full()
        .rounded_md()
        .border_1()
        .border_color(theme::BORDER_TOOL)
        .bg(theme::BG_TOOL)
        .overflow_hidden()
        .cursor(CursorStyle::PointingHand)
        .on_click({
            let entity = entity.clone();
            let id = id_owned.clone();
            move |_, _, cx| {
                entity
                    .update(cx, |this, cx| {
                        this.toggle_tool_expanded(&id, turn_idx);
                        cx.notify();
                    })
                    .ok();
            }
        });

    // Summary line (always visible)
    container = container.child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py(px(6.))
            .hover(|style| style.bg(theme::BG_OVERLAY))
            .child(
                div()
                    .text_xs()
                    .text_color(theme::TEXT_MUTED)
                    .child(chevron),
            )
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if is_error {
                        theme::TEXT_ERROR
                    } else {
                        theme::TEXT_TOOL_NAME
                    })
                    .child(name.to_string()),
            )
            .when(!input.is_empty(), |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(theme::TEXT_TOOL_INPUT)
                        .overflow_hidden()
                        .flex_1()
                        .child(input.to_string()),
                )
            }),
    );

    // Expanded output
    if is_expanded && !output.is_empty() {
        container = container.child(
            div()
                .w_full()
                .border_t_1()
                .border_color(theme::BORDER_TOOL)
                .bg(theme::BG_TOOL_OUTPUT)
                .px_3()
                .py_2()
                .max_h(px(300.))
                .overflow_hidden()
                .child(
                    div()
                        .text_xs()
                        .text_color(if is_error {
                            theme::TEXT_ERROR
                        } else {
                            theme::TEXT_MUTED
                        })
                        .child(output.to_string()),
                ),
        );
    }

    container.into_any()
}

use gpui::SharedString;

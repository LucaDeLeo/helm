use gpui::prelude::*;
use gpui::{Context, Entity, Window, div, px};

use crate::chat::ChatPanel;
use crate::plan::PlanPanel;
use crate::sidebar::Sidebar;
use crate::theme;

pub struct HelmWorkspace {
    sidebar: Entity<Sidebar>,
    chat: Entity<ChatPanel>,
    plan: Entity<PlanPanel>,
}

impl HelmWorkspace {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|_| Sidebar::new());
        let chat = cx.new(|cx| ChatPanel::new(cx));
        let plan = cx.new(|_| PlanPanel::new());
        Self {
            sidebar,
            chat,
            plan,
        }
    }

    pub fn focus_chat_input(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.chat.update(cx, |chat, cx| {
            chat.focus_input(window, cx);
        });
    }
}

impl Render for HelmWorkspace {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .size_full()
            .bg(theme::BG_BASE)
            // Left sidebar
            .child(
                div()
                    .w(px(250.))
                    .h_full()
                    .flex_none()
                    .child(self.sidebar.clone()),
            )
            // Divider
            .child(div().w(px(1.)).h_full().bg(theme::BG_OVERLAY))
            // Center chat
            .child(div().flex_1().h_full().child(self.chat.clone()))
            // Divider
            .child(div().w(px(1.)).h_full().bg(theme::BG_OVERLAY))
            // Right plan panel
            .child(
                div()
                    .w(px(300.))
                    .h_full()
                    .flex_none()
                    .child(self.plan.clone()),
            )
    }
}

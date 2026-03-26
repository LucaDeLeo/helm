use gpui::prelude::*;
use gpui::{Context, FontWeight, Window, div, px};

use crate::theme;

pub struct Sidebar;

impl Sidebar {
    pub fn new() -> Self {
        Self
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(theme::BG_SURFACE)
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_color(theme::ACCENT)
                            .text_base()
                            .font_weight(FontWeight::BOLD)
                            .child("Helm"),
                    ),
            )
            .child(
                div()
                    .h(px(1.))
                    .w_full()
                    .bg(theme::BG_OVERLAY),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_color(theme::TEXT_PRIMARY)
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Sessions"),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_color(theme::TEXT_MUTED)
                            .text_xs()
                            .child("No sessions yet"),
                    ),
            )
    }
}

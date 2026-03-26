use gpui::prelude::*;
use gpui::{Context, FontWeight, Window, div, px};

use crate::theme;

pub struct PlanPanel;

impl PlanPanel {
    pub fn new() -> Self {
        Self
    }
}

impl Render for PlanPanel {
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
                    .text_color(theme::TEXT_PRIMARY)
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Plan"),
            )
            .child(
                div()
                    .h(px(1.))
                    .w_full()
                    .bg(theme::BG_OVERLAY),
            )
            .child(
                div()
                    .text_color(theme::TEXT_MUTED)
                    .text_xs()
                    .child("No active plan"),
            )
    }
}

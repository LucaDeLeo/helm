use gpui::{App, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size};
use gpui_platform::application;

mod chat;
mod plan;
mod provider;
mod sidebar;
mod text_input;
mod theme;
mod workspace;

fn main() {
    application().run(|cx: &mut App| {
        text_input::bind_text_input_keys(cx);
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.on_action(|_: &Quit, cx| cx.quit());

        let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Helm".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(|cx| workspace::HelmWorkspace::new(cx)),
            )
            .unwrap();

        // Focus the chat input on startup
        window
            .update(cx, |workspace, window, cx| {
                workspace.focus_chat_input(window, cx);
            })
            .unwrap();

        cx.activate(true);
    });
}

gpui::actions!(helm, [Quit]);

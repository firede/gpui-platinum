use gpui::{prelude::*, *};
use gpui_platinum::text_input;

#[path = "support/catalog.rs"]
mod catalog;
use catalog::{Controls, NextFocus, PreviousFocus, Quit};

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        text_input::init(cx);
        cx.bind_keys([
            KeyBinding::new("tab", NextFocus, None),
            KeyBinding::new("shift-tab", PreviousFocus, None),
            KeyBinding::new("cmd-q", Quit, None),
        ]);
        let bounds = Bounds::centered(None, size(px(820.), px(540.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Platinum Controls".into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(660.), px(420.))),
                ..Default::default()
            },
            |_, cx| cx.new(Controls::new),
        )
        .expect("open Controls window");
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}

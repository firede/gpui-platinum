use gpui::{prelude::*, *};

#[path = "support/capture.rs"]
mod capture;
#[path = "support/catalog.rs"]
mod catalog;
use catalog::{Controls, NextFocus, PreviousFocus, Quit};

fn main() {
    let arguments: Vec<_> = std::env::args().collect();
    if let Some(path) = capture::argument(&arguments, "--png") {
        let scale = capture::argument(&arguments, "--scale")
            .map(|value| value.parse().expect("numeric image scale"))
            .unwrap_or(2.);
        capture::render_png(path, 820., 540., scale, |_, cx| cx.new(Controls::new))
            .expect("render Controls image");
        return;
    }
    gpui_platform::application().run(|cx: &mut App| {
        gpui_platinum::init(cx);
        cx.bind_keys([
            KeyBinding::new("tab", NextFocus, Some("Controls")),
            KeyBinding::new("shift-tab", PreviousFocus, Some("Controls")),
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

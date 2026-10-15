use gpui::{prelude::*, *};

#[path = "support/capture.rs"]
mod capture;
#[path = "support/gallery.rs"]
mod gallery;

fn value<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    capture::argument(arguments, &format!("--{name}"))
        .or_else(|| capture::argument(arguments, &format!("-{name}")))
}

fn main() {
    let arguments: Vec<_> = std::env::args().collect();
    let page = value(&arguments, "page")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, 5)
        - 1;
    if let Some(path) = value(&arguments, "png") {
        let scale = value(&arguments, "scale")
            .and_then(|value| value.parse().ok())
            .unwrap_or(1.);
        capture::render_png(path, 756., 476., scale, |_, cx| {
            cx.new(|cx| gallery::Gallery::for_capture(page, cx))
        })
        .expect("render Platinum gallery PNG");
        return;
    }
    gpui_platform::application().run(move |cx: &mut App| {
        gpui_platinum::init(cx);
        cx.bind_keys([
            KeyBinding::new("tab", gallery::NextFocus, Some("Gallery")),
            KeyBinding::new("shift-tab", gallery::PreviousFocus, Some("Gallery")),
        ]);
        let bounds = Bounds::centered(None, size(px(756.), px(476.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Platinum Gallery".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| gallery::Gallery::new(page, cx)),
        )
        .expect("open Platinum gallery");
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}

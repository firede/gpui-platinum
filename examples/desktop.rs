use gpui::{
    App, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size,
};
use gpui_platinum::Language;

#[path = "support/accessories.rs"]
mod accessories;
#[path = "support/capture.rs"]
mod capture;
#[path = "support/catalog.rs"]
mod catalog;
#[path = "support/desktop.rs"]
mod desktop;
#[path = "support/documents.rs"]
mod documents;
#[path = "support/file_apps.rs"]
mod file_apps;
#[path = "support/menu_icons.rs"]
mod menu_icons;

fn argument_value<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    capture::argument(arguments, &format!("--{name}"))
        .or_else(|| capture::argument(arguments, &format!("-{name}")))
}

fn main() {
    let arguments: Vec<_> = std::env::args().collect();
    let language = argument_value(&arguments, "lang")
        .map(|tag| match tag {
            "zh-Hans" => Language::SimplifiedChinese,
            "zh-Hant" => Language::TraditionalChinese,
            "ja" => Language::Japanese,
            "ko" => Language::Korean,
            _ => Language::English,
        })
        .unwrap_or_else(accessories::system_language);
    if let Some(path) = argument_value(&arguments, "png") {
        let scale = argument_value(&arguments, "scale")
            .and_then(|value| value.parse().ok())
            .unwrap_or(1.);
        capture::render_png(path, 900., 620., scale, move |_, cx| {
            cx.new(|cx| desktop::Desktop::new(language, true, cx))
        })
        .expect("render Platinum desktop PNG");
        return;
    }
    let scene = arguments.iter().any(|argument| argument == "--scene");
    gpui_platform::application().run(move |cx: &mut App| {
        gpui_platinum::init(cx);
        cx.bind_keys([
            KeyBinding::new("tab", desktop::NextFocus, Some("Desktop")),
            KeyBinding::new("shift-tab", desktop::PreviousFocus, Some("Desktop")),
        ]);
        let bounds = Bounds::centered(None, size(px(900.), px(620.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(gpui_platinum::locale::desk(language, "platinumDesk").into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(700.), px(480.))),
                ..Default::default()
            },
            move |_, cx| cx.new(|cx| desktop::Desktop::new(language, scene, cx)),
        )
        .expect("open Platinum desktop");
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}

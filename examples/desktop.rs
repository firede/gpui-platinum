use gpui::{
    App, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size,
};
use gpui_platinum::{Language, text_input};

#[path = "support/accessories.rs"]
mod accessories;
#[path = "support/catalog.rs"]
mod catalog;
#[path = "support/desktop.rs"]
mod desktop;
#[path = "support/documents.rs"]
mod documents;
#[path = "support/file_apps.rs"]
mod file_apps;

fn main() {
    let arguments: Vec<_> = std::env::args().collect();
    let language = arguments
        .iter()
        .position(|argument| argument == "--lang")
        .and_then(|index| arguments.get(index + 1))
        .map(|tag| match tag.as_str() {
            "zh-Hans" => Language::SimplifiedChinese,
            "zh-Hant" => Language::TraditionalChinese,
            "ja" => Language::Japanese,
            "ko" => Language::Korean,
            _ => Language::English,
        })
        .unwrap_or(Language::English);
    let scene = arguments.iter().any(|argument| argument == "--scene");
    gpui_platform::application().run(move |cx: &mut App| {
        text_input::init(cx);
        gpui_platinum::dialogs::init(cx);
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

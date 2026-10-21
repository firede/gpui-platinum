//! The gallery's pages and the desktop, as the original's examples draw them.

use crate::{Scene, desktop::Desktop, gallery::Gallery};
use gpui_platinum::Language;

pub fn scenes() -> Vec<Scene> {
    let mut scenes: Vec<Scene> = [
        ("gallery-page-1", false),
        ("gallery-page-2", true),
        ("gallery-page-3", false),
        ("gallery-page-4", false),
        ("gallery-page-5", false),
    ]
    .into_iter()
    .enumerate()
    .map(|(page, (name, pending))| {
        let scene = Scene::new(name, 756., 476., move |_, cx| {
            Gallery::for_capture(page, cx)
        });
        if pending { scene.pending() } else { scene }
    })
    .collect();
    for (name, language) in [
        ("desktop-en", Language::English),
        ("desktop-ja", Language::Japanese),
        ("desktop-ko", Language::Korean),
        ("desktop-zh-hans", Language::SimplifiedChinese),
        ("desktop-zh-hant", Language::TraditionalChinese),
    ] {
        scenes.push(
            Scene::new(name, 900., 620., move |_, cx| {
                Desktop::new(language, true, cx)
            })
            .east_asian()
            .pending(),
        );
    }
    scenes
}

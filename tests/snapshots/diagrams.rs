//! Diagrams laid out by limu and drawn in the look, on white paper with a
//! margin of 8, against the original's: its diagram package's flowcharts,
//! and the first example of each type its diagrams example shows.

use crate::Scene;
use crate::diagram_samples::samples;
use gpui::{div, prelude::*, px, rgb};
use gpui_platinum::Language;
use gpui_platinum::diagram::{self, limu};
use gpui_platinum::theme::WHITE;
use std::sync::Arc;

pub fn scenes() -> Vec<Scene> {
    let mut scenes = vec![
        drawn("diagram-order", include_str!("diagram/order.mmd")),
        drawn("diagram-desk", include_str!("diagram/desk.mmd")),
        drawn("diagram-fills", include_str!("diagram/fills.mmd")),
    ];
    for t in samples() {
        let name = format!("diagrams-{}", t.title.to_lowercase().replace(' ', "-"));
        let scene = drawn(name.leak(), &t.examples[0].src);
        // limu-rs snaps each point of a mark of lines across and down, as a
        // chart's grid, to the pixels, where the original's limu moves the
        // mark as a whole by what snaps its first point; so the lines of a
        // gantt chart's grid, and an XY chart's grid and axis, fall a pixel
        // from the original's.
        scenes.push(match t.title.as_str() {
            "Gantt" | "XY chart" => scene.pending(),
            _ => scene,
        });
    }
    scenes
}

/// A diagram laid out and drawn at scale 1, as the original's tests draw
/// one, but for the labels turned by other than a quarter turn. The
/// original resamples such a label, and this port turns its pixels whole,
/// so those pixels are left out.
fn drawn(name: &'static str, src: &str) -> Scene {
    let scene = limu::layout(src, &diagram::layout_options(Language::English))
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    let turned: Vec<[u32; 4]> = scene
        .items
        .iter()
        .filter(|it| it.label.as_ref().is_some_and(|l| l.rotate % 90. != 0.))
        .map(|it| {
            let b = it.label_bounds();
            let at = |v: f64| (v + 8.).max(0.) as u32;
            [
                at(b.x.floor() - 2.),
                at(b.y.floor() - 2.),
                at((b.x + b.w).ceil() + 2.),
                at((b.y + b.h).ceil() + 2.),
            ]
        })
        .collect();
    let (width, height) = (scene.width as f32 + 16., scene.height as f32 + 16.);
    let scene = Arc::new(scene);
    Scene::elements(name, width, height, move |_, _| {
        div()
            .size_full()
            .bg(rgb(WHITE))
            .p(px(8.))
            .child(diagram::view("diagram", scene.clone()))
    })
    .ignore(turned)
}

//! The diagrams example's samples: one type for each that limu reads, and
//! every example laid out in the look.
#![cfg(feature = "diagram")]

use gpui::{Context, IntoElement, Render, TestAppContext, Window};
use gpui_platinum::Language;
use gpui_platinum::diagram::{self, limu, limu::scene::Scene};
use std::sync::Arc;

#[path = "../examples/support/diagram_samples.rs"]
mod diagram_samples;

#[test]
fn every_example_of_every_type_lays_out() {
    let types = diagram_samples::samples();
    // limu's gallery has every type limu reads but info, which shows only
    // limu's version.
    assert_eq!(types.len(), limu::types().len() - 1);
    let o = diagram::layout_options(Language::English);
    for t in &types {
        assert!(!t.examples.is_empty(), "{} has no examples", t.title);
        for e in &t.examples {
            let scene = limu::layout(&e.src, &o)
                .unwrap_or_else(|error| panic!("{}, {:?}: {error}", t.title, e.title));
            assert!(
                !scene.items.is_empty() && scene.width > 0. && scene.height > 0.,
                "{}, {:?}: an empty scene",
                t.title,
                e.title
            );
        }
    }
}

struct Drawing(Arc<Scene>);

impl Render for Drawing {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        diagram::view("diagram", self.0.clone())
    }
}

// Every example draws, in pixels of its own.
#[gpui::test]
fn every_example_draws(cx: &mut TestAppContext) {
    cx.update(gpui_platinum::init);
    let o = diagram::layout_options(Language::English);
    let scenes: Vec<(String, Arc<Scene>)> = diagram_samples::samples()
        .into_iter()
        .flat_map(|t| {
            let o = &o;
            t.examples.into_iter().map(move |e| {
                let scene = limu::layout(&e.src, o).expect("an example that lays out");
                (format!("{}, {:?}", t.title, e.title), Arc::new(scene))
            })
        })
        .collect();
    let (view, cx) = cx.add_window_view(|_, _| Drawing(scenes[0].1.clone()));
    for (name, scene) in scenes {
        view.update(cx, |view, cx| {
            view.0 = scene;
            cx.notify();
        });
        cx.run_until_parked();
        let quads = cx.update(|window, _| window.painted_quads().len());
        assert!(quads > 0, "{name} drew nothing");
    }
}

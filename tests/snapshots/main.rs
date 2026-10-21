//! Scenes drawn by GPUI's offscreen renderer and compared, pixel for pixel,
//! with the original's pictures in tests/reference/snapshots: the same
//! scenes as mygo-platinum's golden tests, the gallery's pages and the
//! desktop.
//!
//! GPUI's platform must be made on the main thread, so this runs without
//! the test harness, one scene after another:
//!
//!     cargo test --test snapshots              # every scene
//!     cargo test --test snapshots -- tabs      # the scenes named with "tabs"
//!
//! A scene that differs leaves what it drew, and a map of the pixels that
//! differ in red, in target/snapshots. They are checked on macOS, where
//! Metal draws them; elsewhere they are skipped.

use gpui::{
    AnyView, AnyWindowHandle, App, AppContext as _, Context, HeadlessAppContext, IntoElement,
    Render, Window, div, prelude::*, px, rgb, size,
};
use image::RgbaImage;
use std::{path::PathBuf, sync::Arc};

#[allow(dead_code, unused_imports)]
#[path = "../../examples/support/accessories.rs"]
mod accessories;
#[allow(dead_code, unused_imports)]
#[path = "../../examples/support/catalog.rs"]
mod catalog;
#[allow(dead_code, unused_imports)]
#[path = "../../examples/support/desktop.rs"]
mod desktop;
#[allow(dead_code, unused_imports)]
#[path = "../../examples/support/documents.rs"]
mod documents;
#[allow(dead_code, unused_imports)]
#[path = "../../examples/support/file_apps.rs"]
mod file_apps;
#[allow(dead_code, unused_imports)]
#[path = "../../examples/support/gallery.rs"]
mod gallery;
#[allow(dead_code, unused_imports)]
#[path = "../../examples/support/menu_icons.rs"]
mod menu_icons;

mod controls;
mod examples;
mod forms;
mod lists;

type MakeView = Box<dyn FnOnce(&mut Window, &mut App) -> AnyView>;
type Prepare = Box<dyn FnOnce(&mut HeadlessAppContext, AnyWindowHandle)>;

/// A scene: what to draw, how big, and the picture it must match.
pub struct Scene {
    pub name: &'static str,
    pub width: f32,
    pub height: f32,
    pub view: MakeView,
    /// Not yet the original's: its differences are reported but do not fail.
    pub pending: bool,
    /// Run in the open window before it is drawn: focus, clicks, keys.
    pub prepare: Option<Prepare>,
}

impl Scene {
    /// A scene of a view made anew, in the window the scene opens.
    pub fn new<V: Render>(
        name: &'static str,
        width: f32,
        height: f32,
        view: impl FnOnce(&mut Window, &mut Context<V>) -> V + 'static,
    ) -> Self {
        Self {
            name,
            width,
            height,
            view: Box::new(move |window, cx| cx.new(|cx| view(window, cx)).into()),
            pending: false,
            prepare: None,
        }
    }

    /// A scene of elements built anew each frame, on the window's Gray2.
    pub fn elements<E: IntoElement>(
        name: &'static str,
        width: f32,
        height: f32,
        build: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        Self::new(name, width, height, move |_, _| {
            Elements(Box::new(move |window, cx| {
                div()
                    .size_full()
                    .bg(rgb(gpui_platinum::theme::GRAY_2))
                    .child(build(window, cx))
                    .into_any_element()
            }))
        })
    }

    pub fn pending(mut self) -> Self {
        self.pending = true;
        self
    }

    pub fn prepare(
        mut self,
        prepare: impl FnOnce(&mut HeadlessAppContext, AnyWindowHandle) + 'static,
    ) -> Self {
        self.prepare = Some(Box::new(prepare));
        self
    }
}

type Build = Box<dyn Fn(&mut Window, &mut App) -> gpui::AnyElement>;

struct Elements(Build);

impl Render for Elements {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        (self.0)(window, cx)
    }
}

fn main() {
    let filters: Vec<String> = std::env::args()
        .skip(1)
        .filter(|argument| !argument.starts_with('-'))
        .collect();
    let scenes: Vec<Scene> = [
        controls::scenes(),
        lists::scenes(),
        forms::scenes(),
        examples::scenes(),
    ]
    .into_iter()
    .flatten()
    .filter(|scene| filters.is_empty() || filters.iter().any(|f| scene.name.contains(f.as_str())))
    .collect();
    // Metal draws the pixels the references hold; elsewhere the scenes are
    // not checked yet.
    if !cfg!(target_os = "macos")
        || !matches!(gpui_platform::current_headless_renderer(), Ok(Some(_)))
    {
        println!("no Metal renderer: {} scenes skipped", scenes.len());
        return;
    }
    let platform = gpui_platform::current_platform(true);
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .parent()
        .unwrap()
        .join("snapshots");
    std::fs::create_dir_all(&out).unwrap();
    let mut failed = Vec::new();
    for scene in scenes {
        let name = scene.name;
        let pending = scene.pending;
        let reference = reference(name);
        let image = draw(&platform, scene);
        let differ = compare(&image, &reference);
        if differ == 0 {
            if pending {
                println!("{name}: matches, so it is no longer pending");
                failed.push(name);
            } else {
                println!("{name}: ok");
            }
            continue;
        }
        image.save(out.join(format!("{name}.png"))).unwrap();
        diff_map(&image, &reference)
            .save(out.join(format!("{name}-diff.png")))
            .unwrap();
        if pending {
            println!("{name}: {differ} pixels differ (pending)");
        } else {
            println!("{name}: {differ} pixels differ");
            failed.push(name);
        }
    }
    if !failed.is_empty() {
        println!("\n{} scenes failed; see {}", failed.len(), out.display());
        std::process::exit(1);
    }
}

fn reference(name: &str) -> RgbaImage {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/reference/snapshots")
        .join(format!("{name}.png"));
    image::open(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        .to_rgba8()
}

fn draw(platform: &std::rc::Rc<dyn gpui::Platform>, scene: Scene) -> RgbaImage {
    let mut app = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(()),
        gpui_platform::current_headless_renderer,
    );
    app.update(gpui_platinum::init);
    let view = scene.view;
    let window = app
        .open_window(
            size(px(scene.width), px(scene.height)),
            move |window, cx| {
                let view = view(window, cx);
                cx.new(|_| Root(view))
            },
        )
        .expect("open the scene's window");
    let window: AnyWindowHandle = window.into();
    app.run_until_parked();
    if let Some(prepare) = scene.prepare {
        prepare(&mut app, window);
        app.run_until_parked();
    }
    app.update_window(window, |_, window, cx| {
        let _ = window.draw(cx);
    })
    .unwrap();
    let image = app.capture_screenshot(window).expect("capture the scene");
    // Drawn at the display's scale; every pixel of the look is whole, so
    // one in each block is the picture at 1×.
    let (width, height) = (scene.width as u32, scene.height as u32);
    let scale = image.width() / width;
    RgbaImage::from_fn(width, height, |x, y| {
        let mut pixel = *image.get_pixel(x * scale, y * scale);
        pixel[3] = 255;
        pixel
    })
}

struct Root(AnyView);

impl Render for Root {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.0.clone())
    }
}

fn compare(image: &RgbaImage, reference: &RgbaImage) -> usize {
    if image.dimensions() != reference.dimensions() {
        return (image.width() * image.height()).max(1) as usize;
    }
    image
        .pixels()
        .zip(reference.pixels())
        .filter(|(a, b)| a.0[0..3] != b.0[0..3])
        .count()
}

fn diff_map(image: &RgbaImage, reference: &RgbaImage) -> RgbaImage {
    RgbaImage::from_fn(image.width(), image.height(), |x, y| {
        let a = image.get_pixel(x, y);
        match reference.get_pixel_checked(x, y) {
            Some(b) if a.0[0..3] == b.0[0..3] => {
                let gray = ((a[0] as u32 + a[1] as u32 + a[2] as u32) / 6 + 128) as u8;
                image::Rgba([gray, gray, gray, 255])
            }
            _ => image::Rgba([255, 0, 0, 255]),
        }
    })
}

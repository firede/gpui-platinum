//! The pictures of the web page (site/), as `desktop --site <dir>` writes
//! them for scripts/site.sh: the pictures a link to the page is shown with;
//! the desk in each of its languages, which the page shows in place of the
//! desk itself; and, drawn once at a scale of 1 for the page to scale
//! without smoothing, the pattern of the desktop, the icons of its menu bar
//! and the frames of a window, buttons, a pop-up menu, a text field and a
//! progress bar, which the page's style sheet stretches around its own
//! content (border-image).

use std::{error::Error, path::Path};

use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, Render, Window, div, prelude::*, px,
};
use gpui_platinum::{
    Font, Language, PlatinumWindow, Theme, draw_bitmap_icon, draw_button, draw_default_ring,
    draw_progress, draw_window_frame,
    menu::draw_popup_button,
    paint::{Pen, drawing},
    text_in,
    theme::*,
    window::{PlatinumWindowOptions, WindowPart},
};
use image::{Rgba, RgbaImage};

use super::{
    accessories::PATTERNS,
    capture,
    desktop::Desktop,
    menu_icons::{DESK_LOGO, application_icon},
};

/// The languages of the desk, by the tags the page knows them by.
const LANGUAGES: [(&str, Language); 5] = [
    ("en", Language::English),
    ("zh-Hans", Language::SimplifiedChinese),
    ("zh-Hant", Language::TraditionalChinese),
    ("ja", Language::Japanese),
    ("ko", Language::Korean),
];

/// A default button's ring, as far as it reaches outside the button.
const RING: i32 = 4;

/// Writes the pictures of the web page to dir.
pub fn write(dir: &Path) -> Result<(), Box<dyn Error>> {
    for (name, image) in pictures() {
        capture::save_png(&dir.join(name), &image)?;
    }
    // The pictures a link to the page is shown with: the card of a link
    // (1200×630) and the preview of a repository (1280×640).
    for (name, width, height) in [("card.png", 400., 210.), ("preview.png", 426., 213.)] {
        let image = capture::render(width, height, |_, cx| cx.new(Share::new))?;
        capture::save_png(&dir.join(name), &at_scale(&image, width, height, 3))?;
    }
    for (tag, language) in LANGUAGES {
        let image = capture::render(900., 620., move |_, cx| {
            cx.new(|cx| Desktop::new(language, true, cx))
        })?;
        let name = format!("scene-{tag}.png");
        capture::save_png(&dir.join(name), &at_scale(&image, 900., 620., 2))?;
    }
    Ok(())
}

/// The pictures drawn by a pen, by the names of their files.
fn pictures() -> Vec<(&'static str, RgbaImage)> {
    let accent = Theme::default().accent;
    let pattern = PATTERNS[0];
    let mut pictures = vec![
        (
            "pattern.png",
            drawn(8, 8, |pen| {
                for y in 0..8 {
                    for x in 0..8 {
                        let on = pattern.bits[y as usize] & (0x80 >> x) != 0;
                        let color = if on {
                            pattern.foreground
                        } else {
                            pattern.background
                        };
                        pen.dot(x, y, color);
                    }
                }
            }),
        ),
        // The icons of the menu bar, dimmed: the page's menus do nothing,
        // as under a modal dialog.
        (
            "logo.png",
            drawn(10, 10, |pen| {
                draw_bitmap_icon(pen, DESK_LOGO, 0, 0, accent, false, true)
            }),
        ),
        (
            "desk.png",
            drawn(12, 12, |pen| {
                draw_bitmap_icon(pen, application_icon("desk"), 0, 0, accent, false, true)
            }),
        ),
        (
            "icon-32.png",
            drawn(32, 32, |pen| {
                let rows = scale_rows(DESK_LOGO, 3);
                let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
                draw_bitmap_icon(pen, &rows, 1, 1, accent, false, false);
            }),
        ),
        (
            "icon-180.png",
            drawn(180, 180, |pen| {
                pen.rect(0, 0, 180, 180, GRAY_2);
                let rows = scale_rows(DESK_LOGO, 12);
                let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
                draw_bitmap_icon(pen, &rows, 30, 30, accent, false, false);
            }),
        ),
        (
            "window.png",
            drawn(64, 64, |pen| {
                let options = PlatinumWindowOptions {
                    no_close: true,
                    no_zoom: true,
                    no_collapse: true,
                    no_grow: true,
                    ..Default::default()
                };
                draw_window_frame(pen, "", options, false, WindowPart::None);
            }),
        ),
        ("field.png", drawn(12, 12, Pen::sunken)),
        (
            "progress.png",
            drawn(24, 12, |pen| draw_progress(pen, 0., accent)),
        ),
        (
            "progress-full.png",
            drawn(24, 12, |pen| draw_progress(pen, 1., accent)),
        ),
    ];
    for (name, pressed) in [("button.png", false), ("button-pressed.png", true)] {
        pictures.push((
            name,
            drawn(24, 20, |pen| draw_button(pen, "", false, pressed)),
        ));
    }
    for (name, disabled, pressed) in [
        ("button-default.png", false, false),
        ("button-default-pressed.png", false, true),
        ("button-default-disabled.png", true, false),
    ] {
        let image = drawn(24 + 2 * RING, 20 + 2 * RING, |pen| {
            draw_default_ring(pen, disabled);
            let mut button = Pen::new(24, 20);
            draw_button(&mut button, "", disabled, pressed);
            pen.append(button, RING, RING);
        });
        pictures.push((name, image));
    }
    for (name, pressed) in [("popup.png", false), ("popup-pressed.png", true)] {
        pictures.push((
            name,
            drawn(40, 20, |pen| draw_popup_button(pen, "", false, pressed)),
        ));
    }
    pictures
}

/// Returns a width×height picture of what draw draws, on nothing.
fn drawn(width: i32, height: i32, draw: impl FnOnce(&mut Pen)) -> RgbaImage {
    let mut pen = Pen::new(width, height);
    draw(&mut pen);
    let mut image = RgbaImage::new(width as u32, height as u32);
    for fill in &pen.fills {
        let [_, r, g, b] = fill.color.to_be_bytes();
        let rect = fill.rect;
        for y in rect.y.max(0)..(rect.y + rect.height).min(height) {
            for x in rect.x.max(0)..(rect.x + rect.width).min(width) {
                image.put_pixel(x as u32, y as u32, Rgba([r, g, b, 255]));
            }
        }
    }
    image
}

/// Returns the rows of an icon with each of its pixels k×k.
fn scale_rows(rows: &[&str], k: usize) -> Vec<String> {
    rows.iter()
        .flat_map(|row| {
            let wide: String = row
                .chars()
                .flat_map(|c| std::iter::repeat_n(c, k))
                .collect();
            std::iter::repeat_n(wide, k)
        })
        .collect()
}

/// Returns a picture of width×height DIPs, drawn at the display's scale, at
/// a scale of k: as drawn if it is k, else from one pixel of each DIP, as
/// every pixel of the look is whole.
fn at_scale(image: &RgbaImage, width: f32, height: f32, k: u32) -> RgbaImage {
    let (width, height) = (width as u32, height as u32);
    let mut out = if image.dimensions() == (width * k, height * k) {
        image.clone()
    } else {
        let scale = (image.width() / width).max(1);
        RgbaImage::from_fn(width * k, height * k, |x, y| {
            *image.get_pixel(x / k * scale, y / k * scale)
        })
    };
    for pixel in out.pixels_mut() {
        pixel[3] = 255;
    }
    out
}

/// The view of the pictures a link to the page is shown with: the
/// calculator open, at 42, behind a window of the library's name, four
/// times the size of the system font, and what it is.
struct Share {
    desktop: Entity<Desktop>,
    welcome: Entity<PlatinumWindow>,
}

const WORD: &str = "Platinum";
const BIG: i32 = 4;

impl Share {
    fn new(cx: &mut Context<Self>) -> Self {
        let desktop = cx.new(Desktop::for_share);
        let content = cx.new(|_| Welcome);
        let font = Font::Bold;
        let height = 14
            + (font.ascent() + 1) * BIG
            + 9
            + font.line_height()
            + 9
            + Font::Small.line_height()
            + 14;
        let welcome = cx.new(|cx| {
            PlatinumWindow::new("", cx)
                .position(18., 40.)
                .size(282., (height + 19 + 6) as f32)
                .options(PlatinumWindowOptions {
                    no_grow: true,
                    no_zoom: true,
                    no_collapse: true,
                    ..Default::default()
                })
                .content(content)
        });
        Self { desktop, welcome }
    }
}

impl Render for Share {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .child(self.desktop.clone())
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .size_full()
                    .child(self.welcome.clone()),
            )
    }
}

struct Welcome;

impl Render for Welcome {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let font = Font::Bold;
        div()
            .flex()
            .flex_col()
            .p(px(14.))
            .gap(px(9.))
            .child(drawing(
                (font.measure(WORD) * BIG) as f32,
                ((font.ascent() + 1) * BIG) as f32,
                move |pen, _: &Window, _: &App| {
                    let mut word = Pen::new(pen.width, pen.height);
                    word.text(0, font.ascent(), WORD, font, BLACK);
                    for fill in word.fills {
                        let r = fill.rect;
                        pen.rect(
                            r.x * BIG,
                            r.y * BIG,
                            r.width * BIG,
                            r.height * BIG,
                            fill.color,
                        );
                    }
                },
            ))
            .child(text_in("Classic desktop controls for GPUI", font, BLACK))
            .child(text_in(
                "English · 简体中文 · 繁體中文 · 日本語 · 한국어",
                Font::Small,
                GRAY_9,
            ))
    }
}

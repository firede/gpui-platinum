//! Mermaid diagrams in the Platinum look: laid out by
//! [limu](https://limu.firede.com) on whole pixels, with the library's bitmap
//! faces, and drawn with its [`Pen`](crate::paint::Pen) — boxes as its
//! controls are drawn, lines and arrowheads a pixel at a time.
//!
//! ```no_run
//! use gpui_platinum::diagram::{self, limu};
//!
//! let scene = limu::layout("flowchart LR; a --> b", &diagram::layout_options(Default::default()))
//!     .expect("a diagram limu reads");
//! let view = diagram::view("diagram", scene);
//! ```
//!
//! It is the third level of limu's styles at work: the look's theme and its
//! overrides set the colors, as for any backend, and a drawing of its own,
//! [`draw`], replaces how each kind of part is drawn.

mod draw;
mod pixels;
mod view;

pub use draw::draw;
pub use limu;
pub use limu_gpui;
pub use view::{DiagramView, view};

use gpui::{App, Pixels, Point, Window};
use limu::scene::Scene;
use limu::scene::style::{Color, Look};
use limu::scene::text::{Face, Measurer};
use limu::scene::{Grid, Icons};

use crate::{Font, Language, Theme, theme::*};

/// The size layout is told the text is: the bitmap faces have one size, and
/// this one gives their lines room.
pub const FONT_SIZE: f64 = 12.0;

/// How far from a line the pointer may be and still be over it, in pixels.
pub const SLOP: f64 = 4.0;

/// Measures text in [`Font::Small`], as [`draw`] sets it: bold and italic as
/// [`Pen::text_styled`](crate::paint::Pen::text_styled) draws them, code in
/// the same face, as it has no monospaced one, and East Asian characters in
/// the language's letterforms. Sub and sup spans are set at the faces' one
/// size too, so their width does not depend on the size they are measured
/// at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextMeasurer {
    pub language: Language,
}

impl Measurer for TextMeasurer {
    fn width(&self, s: &str, _: f64, f: Face) -> f64 {
        f64::from(Font::Small.measure_styled_in(s, draw::text_style(f), self.language))
    }
}

/// The layout options of the look, for text in a language: text measured
/// in the bitmap faces, edges in runs across and down, and everything on
/// whole pixels.
pub fn layout_options(language: Language) -> limu::layout::Options<'static> {
    static MEASURERS: [TextMeasurer; 5] = [
        TextMeasurer {
            language: Language::English,
        },
        TextMeasurer {
            language: Language::SimplifiedChinese,
        },
        TextMeasurer {
            language: Language::TraditionalChinese,
        },
        TextMeasurer {
            language: Language::Japanese,
        },
        TextMeasurer {
            language: Language::Korean,
        },
    ];
    let measurer = MEASURERS
        .iter()
        .find(|m| m.language == language)
        .expect("a measurer for every language");
    limu::layout::Options {
        measurer,
        font_size: FONT_SIZE,
        snap: Some(Grid::square(1.0)),
        routing: limu::layout::Routing::Orthogonal,
        ..Default::default()
    }
}

/// The limu theme of a Platinum theme: white paper, black lines and text,
/// faces of the gray ramp, and the accent's colors.
pub fn theme(theme: &Theme) -> limu::scene::style::Theme {
    let c = |rgb: u32| Color::rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8);
    let mut series = [Color::default(); 8];
    for (i, color) in series.iter_mut().enumerate() {
        *color = c(match ACCENTS.get(i) {
            Some(accent) => accent.mid,
            None => [GRAY_5, GRAY_8][i - ACCENTS.len()],
        });
    }
    limu::scene::style::Theme {
        bg: c(WHITE),
        fg: c(BLACK),
        muted: c(GRAY_8),
        line: c(BLACK),
        surface: c(GRAY_2),
        surface_alt: c(GRAY_3),
        border: c(BLACK),
        accent: c(theme.accent.dark),
        critical: c(ACCENTS[4].dark),
        series,
        radius: 2.0,
        stroke_width: 1.0,
    }
}

/// How a scene is drawn in the look.
pub struct Options<'a> {
    /// The colors: a Platinum theme's, from [`theme`], to which overrides
    /// of roles may be added.
    pub look: Look,
    /// The language the scene's text was measured in.
    pub language: Language,
    /// The icons, as limu-gpui draws them. Without them, icons are left out.
    pub icons: Option<Icons<'a>>,
    /// The images, as limu-gpui draws them. Without them, images are left
    /// out.
    pub images: Option<limu_gpui::Images<'a>>,
}

impl Options<'_> {
    /// The options of a Platinum theme, for text in English.
    pub fn new(platinum: &Theme) -> Self {
        Self {
            look: Look::new(theme(platinum)),
            language: Language::English,
            icons: None,
            images: None,
        }
    }
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self::new(&Theme::default())
    }
}

/// Paints a scene laid out with [`layout_options`] in a window, its top left
/// corner at a point, each item as [`draw`] draws it. The scene's background
/// is left to the host.
pub fn paint(scene: &Scene, origin: Point<Pixels>, o: &Options, window: &mut Window, cx: &mut App) {
    let gpui_options = limu_gpui::Options {
        look: o.look.clone(),
        fonts: limu_gpui::Fonts::default(),
        icons: o.icons.as_ref().map(|icons| {
            let icons: Icons<'_> = Box::new(move |name| icons(name));
            icons
        }),
        images: o.images.as_ref().map(|images| {
            let images: limu_gpui::Images<'_> = Box::new(move |src| images(src));
            images
        }),
    };
    let language = o.language;
    limu_gpui::paint_with(scene, &gpui_options, origin, 1.0, window, cx, |p, it, s| {
        draw(p, it, s, origin, language)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // Text is measured as the small face draws it, in its styles.
    #[test]
    fn measurer() {
        let m = TextMeasurer::default();
        for f in [
            Face::REGULAR,
            Face::BOLD,
            Face::ITALIC,
            Face::BOLD | Face::ITALIC,
            Face::MONO,
        ] {
            for s in ["", "Notes", "Quality check", "a b"] {
                assert_eq!(
                    m.width(s, 99., f),
                    f64::from(Font::Small.measure_styled(s, draw::text_style(f))),
                    "{s:?} in {f:?}"
                );
            }
        }
    }

    // The small face's capitals are as tall as the original's, which places
    // a label's lines by them.
    #[test]
    fn cap_height() {
        assert_eq!(Font::Small.cap_height(), 8);
    }
}

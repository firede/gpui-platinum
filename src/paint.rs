//! Drawing on whole pixels of the Platinum grid.

use crate::rich_text::TextStyle;
use crate::theme::*;
use crate::{Font, Language};
use gpui::{
    App, Bounds, ContentMask, IntoElement, Pixels, Styled, Window, canvas, fill, point, px, rgb,
    size,
};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Fill {
    pub rect: Rect,
    pub color: u32,
}

struct SystemText {
    after_fill: usize,
    x: i32,
    baseline: i32,
    color: u32,
    line: Arc<gpui::LineLayout>,
    clip: Option<Rect>,
}

/// A recording pen. Its fills can be painted by GPUI or inspected in tests.
pub struct Pen {
    pub width: i32,
    pub height: i32,
    pub language: Language,
    pub fills: Vec<Fill>,
    system_text: Vec<SystemText>,
    dimmed: bool,
}

impl Pen {
    pub fn append(&mut self, other: Pen, x: i32, y: i32) {
        let fill_offset = self.fills.len();
        self.system_text
            .extend(other.system_text.into_iter().map(|mut text| {
                text.after_fill += fill_offset;
                text.x += x;
                text.baseline += y;
                if let Some(clip) = &mut text.clip {
                    clip.x += x;
                    clip.y += y;
                }
                if self.dimmed {
                    text.color = dim_color(text.color);
                }
                text
            }));
        self.fills.extend(other.fills.into_iter().map(|mut fill| {
            fill.rect.x += x;
            fill.rect.y += y;
            if self.dimmed {
                fill.color = dim_color(fill.color);
            }
            fill
        }));
    }

    /// Clips recorded drawing, including native fallback glyphs, to a rectangle.
    pub fn clip(&mut self, bounds: Rect) {
        let mut kept_before = Vec::with_capacity(self.fills.len() + 1);
        let mut kept = 0;
        self.fills.retain_mut(|fill| {
            kept_before.push(kept);
            fill.rect = intersect(fill.rect, bounds);
            let visible = fill.rect.width > 0 && fill.rect.height > 0;
            kept += usize::from(visible);
            visible
        });
        kept_before.push(kept);
        for text in &mut self.system_text {
            text.after_fill = kept_before[text.after_fill];
            text.clip = Some(text.clip.map_or(bounds, |clip| intersect(clip, bounds)));
        }
    }
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            language: Language::English,
            fills: Vec::new(),
            system_text: Vec::new(),
            dimmed: false,
        }
    }

    /// Dims control ink while preserving the geometry and light gray surfaces.
    pub fn set_dimmed(&mut self, dimmed: bool) {
        self.dimmed = dimmed;
    }

    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        if width > 0 && height > 0 {
            self.fills.push(Fill {
                rect: Rect {
                    x,
                    y,
                    width,
                    height,
                },
                color: if self.dimmed { dim_color(color) } else { color },
            });
        }
    }

    pub fn dot(&mut self, x: i32, y: i32, color: u32) {
        self.rect(x, y, 1, 1, color);
    }
    pub fn hline(&mut self, x: i32, y: i32, width: i32, color: u32) {
        self.rect(x, y, width, 1, color);
    }
    pub fn vline(&mut self, x: i32, y: i32, height: i32, color: u32) {
        self.rect(x, y, 1, height, color);
    }

    pub fn frame(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        self.hline(x, y, width, color);
        self.hline(x, y + height - 1, width, color);
        self.vline(x, y + 1, height - 2, color);
        self.vline(x + width - 1, y + 1, height - 2, color);
    }

    pub fn bevel(&mut self, x: i32, y: i32, width: i32, height: i32, top: u32, bottom: u32) {
        self.hline(x, y, width - 1, top);
        self.vline(x, y + 1, height - 2, top);
        self.hline(x + 1, y + height - 1, width - 1, bottom);
        self.vline(x + width - 1, y + 1, height - 2, bottom);
    }

    pub fn round_frame(&mut self, rect: Rect, radius: usize, line: u32, face: Option<u32>) {
        let Rect {
            x,
            y,
            width,
            height,
        } = rect;
        for row in 0..height {
            let inset = corner_inset(row, height, radius);
            if row == 0 || row == height - 1 {
                self.hline(x + inset, y + row, width - 2 * inset, line);
            } else {
                self.dot(x + inset, y + row, line);
                self.dot(x + width - inset - 1, y + row, line);
                if let Some(color) = face {
                    self.hline(x + inset + 1, y + row, width - inset * 2 - 2, color);
                }
            }
        }
        for row in 1..(radius as i32).min(height / 2) {
            for col in
                (corner_inset(row, height, radius) + 1)..corner_inset(row - 1, height, radius)
            {
                for (dx, dy) in [
                    (col, row),
                    (width - 1 - col, row),
                    (col, height - 1 - row),
                    (width - 1 - col, height - 1 - row),
                ] {
                    self.dot(x + dx, y + dy, line);
                }
            }
        }
    }

    pub fn raised(&mut self, face: u32) {
        self.rect(0, 0, self.width, self.height, face);
        self.frame(0, 0, self.width, self.height, BLACK);
        self.bevel(1, 1, self.width - 2, self.height - 2, WHITE, GRAY_7);
    }

    pub fn sunken(&mut self) {
        self.bevel(0, 0, self.width, self.height, GRAY_7, WHITE);
        self.frame(1, 1, self.width - 2, self.height - 2, BLACK);
        self.rect(2, 2, self.width - 4, self.height - 4, WHITE);
    }

    pub fn text(&mut self, mut x: i32, baseline: i32, text: &str, font: Font, color: u32) {
        for grapheme in text.graphemes(true) {
            if !font.has_bitmap_grapheme(grapheme, self.language)
                && let Some(advance) =
                    self.system_text((x, baseline), grapheme, font, color, 1, false)
            {
                x += advance;
                continue;
            }
            for ch in grapheme.chars() {
                let glyph = font.glyph_or_replacement(ch, self.language);
                glyph.for_each_run(|dx, dy, width| self.hline(x + dx, baseline + dy, width, color));
                x += glyph.advance;
            }
        }
    }

    /// Draws a line in a style and returns the x after it: in the bold face;
    /// italic slanted right, each pair of rows a pixel right of the pair
    /// below, with the slant's room added after the run and East Asian
    /// characters upright; underlined a pixel under the baseline; and each
    /// pixel of all that at the style's scale. [`Font::measure_styled`]
    /// measures what it draws.
    pub fn text_styled(
        &mut self,
        x: i32,
        baseline: i32,
        text: &str,
        font: Font,
        color: u32,
        style: TextStyle,
    ) -> i32 {
        let language = self.language;
        self.styled_text_in(x, baseline, text, font, color, style, language)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn styled_text_in(
        &mut self,
        start: i32,
        baseline: i32,
        text: &str,
        font: Font,
        color: u32,
        style: TextStyle,
        language: Language,
    ) -> i32 {
        let font = font.styled(style);
        let scale = i32::from(style.scale.clamp(1, 4));
        let mut x = start;
        for grapheme in text.graphemes(true) {
            if grapheme == "\t" {
                x += font.styled_advance(grapheme, scale, style.italic, language);
                continue;
            }
            if !font.has_bitmap_grapheme(grapheme, language)
                && let Some(advance) = self.system_text(
                    (x, baseline),
                    grapheme,
                    font,
                    color,
                    scale as u8,
                    style.italic,
                )
            {
                x += advance;
                continue;
            }
            for ch in grapheme.chars() {
                let glyph = font.glyph_or_replacement(ch, language);
                let east_asian = matches!(ch as u32, 0x1100..=0x11ff | 0x2e80..=0xa4cf | 0xac00..=0xd7af | 0xf900..=0xfaff | 0xfe30..=0xffef | 0x20000..=0x3134f);
                glyph.for_each_run(|dx, dy, width| {
                    let slant = if style.italic && !east_asian {
                        (-dy + 1).div_euclid(2)
                    } else {
                        0
                    };
                    self.rect(
                        x + (dx + slant) * scale,
                        baseline + dy * scale,
                        width * scale,
                        scale,
                        color,
                    );
                });
                x += glyph.advance * scale;
            }
        }
        if style.italic {
            x += font.italic_slant() * scale;
        }
        if style.underline {
            self.rect(start, baseline + scale, x - start - scale, scale, color);
        }
        x
    }

    /// Records a shaped platform fallback at the same baseline as bitmap text.
    pub(crate) fn system_text(
        &mut self,
        origin: (i32, i32),
        text: &str,
        font: Font,
        color: u32,
        scale: u8,
        italic: bool,
    ) -> Option<i32> {
        let (x, baseline) = origin;
        let line = font.system_line(text, scale, italic)?;
        let advance = f32::from(line.width).ceil() as i32 + scale as i32;
        let color = if self.dimmed { dim_color(color) } else { color };
        self.system_text.push(SystemText {
            after_fill: self.fills.len(),
            x,
            baseline,
            color,
            line,
            clip: None,
        });
        Some(advance)
    }

    pub fn text_centered(&mut self, text: &str, font: Font, color: u32) {
        let width = font.measure_in(text, self.language);
        let cap = if matches!(font, Font::Small | Font::SmallBold) {
            8
        } else {
            9
        };
        self.text(
            (self.width - width) / 2,
            (self.height - cap) / 2 + cap,
            text,
            font,
            color,
        );
    }

    pub fn paint(&self, origin: gpui::Point<Pixels>, window: &mut Window) {
        let mut text = self.system_text.iter().peekable();
        for index in 0..=self.fills.len() {
            while text.peek().is_some_and(|text| text.after_fill == index) {
                let text = text.next().expect("pending system text");
                let mask = text.clip.map(|clip| ContentMask {
                    bounds: Bounds::new(
                        origin + point(px(clip.x as f32), px(clip.y as f32)),
                        size(px(clip.width as f32), px(clip.height as f32)),
                    ),
                });
                window.with_content_mask(mask, |window| {
                    for run in &text.line.runs {
                        for glyph in &run.glyphs {
                            let position = origin
                                + point(px(text.x as f32), px(text.baseline as f32))
                                + glyph.position;
                            let result = if glyph.is_emoji {
                                window.paint_emoji(
                                    position,
                                    run.font_id,
                                    glyph.id,
                                    text.line.font_size,
                                )
                            } else {
                                window.paint_glyph(
                                    position,
                                    run.font_id,
                                    glyph.id,
                                    text.line.font_size,
                                    rgb(text.color).into(),
                                )
                            };
                            if let Err(error) = result {
                                eprintln!("Unable to paint fallback glyph: {error}");
                            }
                        }
                    }
                });
            }
            let Some(Fill { rect, color }) = self.fills.get(index) else {
                break;
            };
            window.paint_quad(fill(
                Bounds::new(
                    origin + point(px(rect.x as f32), px(rect.y as f32)),
                    size(px(rect.width as f32), px(rect.height as f32)),
                ),
                rgb(*color),
            ));
        }
    }
}

fn intersect(left: Rect, right: Rect) -> Rect {
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    Rect {
        x,
        y,
        width: ((left.x + left.width).min(right.x + right.width) - x).max(0),
        height: ((left.y + left.height).min(right.y + right.height) - y).max(0),
    }
}

pub fn corner_inset(row: i32, height: i32, radius: usize) -> i32 {
    let corners: &[i32] = match radius {
        1 => &[1],
        2 => &[2, 1],
        3 => &[3, 1, 1],
        4 => &[4, 2, 1, 1],
        _ => &[],
    };
    corners
        .get(row.min(height - row - 1).max(0) as usize)
        .copied()
        .unwrap_or(0)
}

pub fn drawing(
    width: f32,
    height: f32,
    draw: impl FnOnce(&mut Pen, &Window, &App) + 'static,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            let mut pen = Pen::new(
                f32::from(bounds.size.width).round() as i32,
                f32::from(bounds.size.height).round() as i32,
            );
            draw(&mut pen, window, cx);
            pen.paint(
                point(bounds.origin.x.round(), bounds.origin.y.round()),
                window,
            );
        },
    )
    .w(px(width))
    .h(px(height))
    .flex_shrink_0()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rounded_corners_have_the_specified_steps() {
        assert_eq!(
            (0..5)
                .map(|row| corner_inset(row, 20, 4))
                .collect::<Vec<_>>(),
            [4, 2, 1, 1, 0]
        );
        assert_eq!(corner_inset(19, 20, 4), 4);
    }
    #[test]
    fn empty_rectangles_do_not_reach_the_renderer() {
        let mut pen = Pen::new(10, 10);
        pen.rect(0, 0, -2, 1, BLACK);
        pen.rect(0, 0, 1, 0, BLACK);
        assert!(pen.fills.is_empty());
    }

    #[gpui::test]
    fn fallback_text_preserves_graphemes_and_paint_order(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::font::init(cx));
        let grapheme = "👩‍💻";
        let mut fragment = Pen::new(40, 20);
        fragment.rect(0, 0, 40, 20, WHITE);
        fragment.text(3, 12, grapheme, Font::Small, BLACK);
        fragment.vline(24, 1, 16, BLACK);
        assert_eq!(fragment.system_text.len(), 1);
        assert_eq!(fragment.system_text[0].line.len, grapheme.len());
        assert_eq!(fragment.system_text[0].after_fill, 1);
        assert_eq!(
            Font::Small.measure(grapheme),
            f32::from(fragment.system_text[0].line.width).ceil() as i32
        );

        let mut parent = Pen::new(80, 40);
        parent.rect(0, 0, 80, 40, GRAY_2);
        parent.append(fragment, 10, 8);
        assert_eq!(parent.system_text[0].after_fill, 2);
        assert_eq!(
            (parent.system_text[0].x, parent.system_text[0].baseline),
            (13, 20)
        );

        parent.clip(Rect {
            x: 12,
            y: 10,
            width: 20,
            height: 15,
        });
        assert_eq!(
            parent.system_text[0].clip,
            Some(Rect {
                x: 12,
                y: 10,
                width: 20,
                height: 15,
            })
        );
        assert!(parent.system_text[0].after_fill <= parent.fills.len());
        assert!(parent.fills.iter().all(|fill| {
            fill.rect.x >= 12
                && fill.rect.y >= 10
                && fill.rect.x + fill.rect.width <= 32
                && fill.rect.y + fill.rect.height <= 25
        }));

        parent.clip(Rect {
            x: 200,
            y: 200,
            width: 10,
            height: 10,
        });
        assert!(parent.fills.is_empty());
        assert_eq!(parent.system_text[0].after_fill, 0);
        assert_eq!(parent.system_text[0].clip.unwrap().width, 0);
    }
}

//! Drawing on whole pixels of the Platinum grid.

use crate::theme::*;
use crate::{Font, Language};
use gpui::{App, Bounds, IntoElement, Pixels, Styled, Window, canvas, fill, point, px, rgb, size};

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

/// A recording pen. Its fills can be painted by GPUI or inspected in tests.
pub struct Pen {
    pub width: i32,
    pub height: i32,
    pub language: Language,
    pub fills: Vec<Fill>,
}

impl Pen {
    pub fn append(&mut self, other: Pen, x: i32, y: i32) {
        self.fills.extend(other.fills.into_iter().map(|mut fill| {
            fill.rect.x += x;
            fill.rect.y += y;
            fill
        }));
    }
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            language: Language::English,
            fills: Vec::new(),
        }
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
                color,
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
        for ch in text.chars() {
            let glyph = font.glyph_or_replacement(ch, self.language);
            glyph.for_each_run(|dx, dy, width| self.hline(x + dx, baseline + dy, width, color));
            x += glyph.advance;
        }
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
        for Fill { rect, color } in &self.fills {
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
}

//! Labels, status displays and other controls without a bound value.

use crate::{
    Font, Theme,
    paint::{Pen, Rect, drawing},
    theme::*,
    widgets::text_in,
};
use gpui::{prelude::*, *};
use std::{sync::OnceLock, time::Instant};

pub fn paragraph(text: impl Into<SharedString>, font: Font, width: i32) -> impl IntoElement {
    let text = text.into();
    let lines = wrap(&text, font, width);
    div().flex().flex_col().w(px(width as f32)).children(
        lines
            .into_iter()
            .map(move |line| text_in(line, font, BLACK)),
    )
}

pub fn wrap(text: &str, font: Font, width: i32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_inclusive(' ') {
            if !line.is_empty() && font.measure(&(line.clone() + word)) > width {
                lines.push(line.trim_end().to_owned());
                line.clear();
            }
            for ch in word.chars() {
                if !line.is_empty() && font.measure(&(line.clone() + &ch.to_string())) > width {
                    lines.push(std::mem::take(&mut line));
                }
                line.push(ch);
            }
        }
        lines.push(line.trim_end().to_owned());
    }
    lines
}

pub fn placard(text: impl Into<SharedString>, width: i32) -> impl IntoElement {
    let text = text.into();
    drawing(width as f32, 20., move |pen, _, _| {
        pen.raised(GRAY_2);
        pen.text_centered(&text, Font::Small, BLACK);
    })
}

pub fn window_header(text: impl Into<SharedString>, width: i32) -> impl IntoElement {
    let text = text.into();
    drawing(width as f32, 24., move |pen, _, _| {
        pen.rect(0, 0, width, 24, GRAY_2);
        pen.hline(0, 0, width, WHITE);
        pen.hline(0, 23, width, GRAY_7);
        pen.text(8, 16, &text, Font::Bold, BLACK);
    })
}

pub fn badge(text: impl Into<SharedString>, theme: Theme) -> impl IntoElement {
    let text = text.into();
    let width = Font::Bold.measure(&text) + 10;
    drawing(width as f32, 16., move |pen, _, _| {
        pen.round_frame(
            Rect {
                x: 0,
                y: 0,
                width,
                height: 16,
            },
            4,
            theme.accent.dark,
            Some(theme.accent.light),
        );
        pen.text_centered(&text, Font::Bold, BLACK);
    })
}

pub fn tag(text: impl Into<SharedString>) -> impl IntoElement {
    let text = text.into();
    let width = Font::Small.measure(&text) + 14;
    drawing(width as f32, 16., move |pen, _, _| {
        pen.round_frame(
            Rect {
                x: 0,
                y: 0,
                width,
                height: 16,
            },
            2,
            GRAY_7,
            Some(GRAY_1),
        );
        pen.text_centered(&text, Font::Small, BLACK);
    })
}

pub fn image_well(width: i32, height: i32) -> impl IntoElement {
    drawing(width as f32, height as f32, move |pen, _, _| {
        pen.sunken();
        pen.rect(2, 2, width - 4, height - 4, GRAY_3);
        for y in (3..height - 3).step_by(4) {
            for x in (3..width - 3).step_by(4) {
                pen.dot(x, y, GRAY_5);
            }
        }
    })
}

pub fn balloon(text: impl Into<SharedString>, width: i32) -> impl IntoElement {
    let text = text.into();
    let lines = wrap(&text, Font::Small, width - 20);
    let height = lines.len() as i32 * 14 + 28;
    drawing(width as f32, height as f32, move |pen, _, _| {
        pen.round_frame(
            Rect {
                x: 0,
                y: 0,
                width: width - 2,
                height: height - 10,
            },
            4,
            BLACK,
            Some(WHITE),
        );
        for i in 0..9 {
            pen.hline(12, height - 11 + i, 10 - i, WHITE);
            pen.dot(12, height - 11 + i, BLACK);
            pen.dot(22 - i, height - 11 + i, BLACK);
        }
        for (index, line) in lines.iter().enumerate() {
            pen.text(10, 17 + index as i32 * 14, line, Font::Small, BLACK);
        }
    })
}

pub fn card(title: impl Into<SharedString>, width: i32) -> Div {
    div()
        .w(px(width as f32))
        .p(px(12.))
        .bg(rgb(WHITE))
        .border_1()
        .border_color(rgb(GRAY_7))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(crate::label(title))
}

pub fn empty_state(
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
    width: i32,
) -> impl IntoElement {
    let title = title.into();
    div()
        .w(px(width as f32))
        .py(px(20.))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(10.))
        .child(crate::label(title))
        .child(paragraph(body, Font::Small, width - 32))
}

pub fn status_bar(
    parts: impl IntoIterator<Item = impl Into<SharedString>>,
    width: i32,
) -> impl IntoElement {
    let parts: Vec<SharedString> = parts.into_iter().map(Into::into).collect();
    drawing(width as f32, 20., move |pen, _, _| {
        pen.rect(0, 0, width, 20, GRAY_2);
        pen.hline(0, 0, width, WHITE);
        let section = width / parts.len().max(1) as i32;
        for (index, part) in parts.iter().enumerate() {
            let x = index as i32 * section;
            if index > 0 {
                pen.vline(x, 3, 14, GRAY_7);
                pen.vline(x + 1, 3, 14, WHITE);
            }
            pen.text(x + 6, 14, part, Font::Small, BLACK);
        }
    })
}

fn frame_number() -> usize {
    static START: OnceLock<Instant> = OnceLock::new();
    (START.get_or_init(Instant::now).elapsed().as_millis() / 100) as usize
}

pub fn indeterminate_progress(width: i32, theme: Theme) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let mut pen = Pen::new(width, 14);
            pen.sunken();
            let phase = frame_number() as i32 % 12;
            for x in 2..width - 2 {
                for y in 2..12 {
                    pen.dot(
                        x,
                        y,
                        if (x + y + phase) % 12 < 6 {
                            theme.accent.mid
                        } else {
                            theme.accent.light
                        },
                    );
                }
            }
            pen.paint(bounds.origin, window);
            window.request_animation_frame();
        },
    )
    .w(px(width as f32))
    .h(px(14.))
    .flex_shrink_0()
}

pub fn chasing_arrows() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let mut pen = Pen::new(16, 16);
            let points = [
                (7, 1),
                (11, 3),
                (13, 7),
                (11, 11),
                (7, 13),
                (3, 11),
                (1, 7),
                (3, 3),
            ];
            let phase = frame_number() % 8;
            for (index, (x, y)) in points.into_iter().enumerate() {
                pen.rect(
                    x,
                    y,
                    3,
                    3,
                    if (index + phase) % 8 < 4 {
                        BLACK
                    } else {
                        GRAY_7
                    },
                );
            }
            pen.paint(bounds.origin, window);
            window.request_animation_frame();
        },
    )
    .w(px(16.))
    .h(px(16.))
    .flex_shrink_0()
}

#[cfg(test)]
mod tests {
    use super::{Font, wrap};
    #[test]
    fn paragraphs_preserve_empty_lines_and_wrap_long_unicode_words() {
        let lines = wrap("One two\n\n世界世界世界", Font::Small, 30);
        assert!(lines.iter().any(String::is_empty));
        assert!(lines.iter().all(|line| Font::Small.measure(line) <= 30));
        assert_eq!(lines.join("").replace(' ', ""), "Onetwo世界世界世界");
    }
}

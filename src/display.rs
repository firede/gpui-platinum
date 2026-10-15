//! Labels, status displays and other controls without a bound value.

use crate::{
    Font, Language, Theme,
    paint::{Pen, Rect, drawing},
    theme::*,
    widgets::text_in_language,
};
use gpui::{prelude::*, *};
use std::{sync::OnceLock, time::Instant};

#[track_caller]
pub fn paragraph(text: impl Into<SharedString>, font: Font, width: i32) -> impl IntoElement {
    paragraph_in(text, font, width, Language::English)
}

#[track_caller]
pub fn paragraph_in(
    text: impl Into<SharedString>,
    font: Font,
    width: i32,
    language: Language,
) -> impl IntoElement {
    let text = text.into();
    let lines = wrap_in(&text, font, width, language);
    div()
        .id(ElementId::from(std::panic::Location::caller()))
        .flex()
        .flex_col()
        .w(px(width as f32))
        .children(lines.into_iter().enumerate().map(move |(index, line)| {
            div()
                .id(("line", index))
                .child(text_in_language(line, font, BLACK, language))
        }))
}

pub fn wrap(text: &str, font: Font, width: i32) -> Vec<String> {
    wrap_in(text, font, width, Language::English)
}

pub fn wrap_in(text: &str, font: Font, width: i32, language: Language) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_inclusive(' ') {
            if !line.is_empty() && font.measure_in(&(line.clone() + word), language) > width {
                lines.push(line.trim_end().to_owned());
                line.clear();
            }
            for ch in word.chars() {
                if !line.is_empty()
                    && font.measure_in(&(line.clone() + &ch.to_string()), language) > width
                {
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
    placard_in(text, width, Language::English)
}

pub fn placard_in(
    text: impl Into<SharedString>,
    width: i32,
    language: Language,
) -> impl IntoElement {
    let text = text.into();
    drawing(width as f32, 16., move |pen, _, _| {
        pen.language = language;
        pen.rect(0, 0, width, 16, GRAY_2);
        pen.bevel(0, 0, width, 16, WHITE, GRAY_7);
        pen.text(6, 12, &text, Font::Small, BLACK);
    })
}

pub fn window_header(text: impl Into<SharedString>, width: i32) -> impl IntoElement {
    window_header_in(text, width, Language::English)
}

pub fn window_header_in(
    text: impl Into<SharedString>,
    width: i32,
    language: Language,
) -> impl IntoElement {
    window_header_parts(text, "", width, language)
}

pub fn window_header_parts(
    left: impl Into<SharedString>,
    right: impl Into<SharedString>,
    width: i32,
    language: Language,
) -> impl IntoElement {
    let (left, right) = (left.into(), right.into());
    drawing(width as f32, 20., move |pen, _, _| {
        pen.language = language;
        pen.rect(0, 0, width, 20, GRAY_2);
        pen.hline(0, 18, width, GRAY_7);
        pen.hline(0, 19, width, BLACK);
        pen.text(8, 13, &left, Font::Small, BLACK);
        pen.text(
            width - Font::Small.measure_in(&right, language) - 8,
            13,
            &right,
            Font::Small,
            BLACK,
        );
    })
}

pub fn badge(text: impl Into<SharedString>, theme: Theme) -> impl IntoElement {
    badge_in(text, theme, Language::English)
}

pub fn badge_in(
    text: impl Into<SharedString>,
    _theme: Theme,
    language: Language,
) -> impl IntoElement {
    label_box(text, BLACK, WHITE, language)
}

pub fn tag(text: impl Into<SharedString>) -> impl IntoElement {
    tag_in(text, Language::English)
}

pub fn tag_in(text: impl Into<SharedString>, language: Language) -> impl IntoElement {
    label_box(text, GRAY_1, BLACK, language)
}

fn label_box(
    text: impl Into<SharedString>,
    face: u32,
    ink: u32,
    language: Language,
) -> impl IntoElement {
    let text = text.into();
    let width = (Font::Small.measure_in(&text, language) + 8).max(14);
    drawing(width as f32, 14., move |pen, _, _| {
        pen.language = language;
        pen.rect(0, 0, width, 14, face);
        pen.frame(0, 0, width, 14, BLACK);
        pen.text_centered(&text, Font::Small, ink);
    })
}

pub fn image_well(width: i32, height: i32) -> impl IntoElement {
    drawing(width as f32, height as f32, |pen, _, _| pen.sunken())
}

pub fn balloon(text: impl Into<SharedString>, width: i32) -> impl IntoElement {
    balloon_in(text, width, Language::English)
}

pub fn balloon_in(
    text: impl Into<SharedString>,
    width: i32,
    language: Language,
) -> impl IntoElement {
    let text = text.into();
    let lines = wrap_in(&text, Font::Small, width - 20, language);
    let height = lines.len() as i32 * 14 + 20;
    drawing(width as f32, height as f32, move |pen, _, _| {
        pen.language = language;
        pen.round_frame(
            Rect {
                x: 1,
                y: 9,
                width: width - 1,
                height: height - 9,
            },
            4,
            GRAY_6,
            Some(GRAY_6),
        );
        pen.round_frame(
            Rect {
                x: 0,
                y: 8,
                width: width - 1,
                height: height - 9,
            },
            4,
            BLACK,
            Some(WHITE),
        );
        for y in 0..8 {
            pen.hline(13, y, y, WHITE);
            pen.dot(12, y, BLACK);
            pen.dot(13 + y, y, BLACK);
        }
        pen.hline(13, 8, 8, WHITE);
        for (index, line) in lines.iter().enumerate() {
            pen.text(10, 23 + index as i32 * 14, line, Font::Small, BLACK);
        }
    })
}

#[track_caller]
pub fn card(title: impl Into<SharedString>, width: i32) -> Div {
    card_in(title, width, Language::English)
}

#[track_caller]
pub fn card_in(title: impl Into<SharedString>, width: i32, language: Language) -> Div {
    div()
        .w(px(width as f32))
        .p(px(12.))
        .bg(rgb(WHITE))
        .border_1()
        .border_color(rgb(GRAY_7))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(crate::label_in(title, language))
}

#[track_caller]
pub fn empty_state(
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
    width: i32,
) -> impl IntoElement {
    empty_state_in(title, body, width, Language::English)
}

#[track_caller]
pub fn empty_state_in(
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
    width: i32,
    language: Language,
) -> impl IntoElement {
    let title = title.into();
    div()
        .w(px(width as f32))
        .py(px(20.))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(10.))
        .child(crate::label_in(title, language))
        .child(paragraph_in(body, Font::Small, width - 32, language))
}

pub fn status_bar(
    parts: impl IntoIterator<Item = impl Into<SharedString>>,
    width: i32,
) -> impl IntoElement {
    status_bar_in(parts, width, Language::English)
}

pub fn status_bar_in(
    parts: impl IntoIterator<Item = impl Into<SharedString>>,
    width: i32,
    language: Language,
) -> impl IntoElement {
    let parts: Vec<SharedString> = parts.into_iter().map(Into::into).collect();
    drawing(width as f32, 16., move |pen, _, _| {
        pen.language = language;
        let mut x = 0;
        for (index, part) in parts.iter().enumerate() {
            let section = if index + 1 == parts.len() {
                width - x
            } else {
                Font::Small.measure_in(part, language) + 12
            };
            pen.rect(x, 0, section, 16, GRAY_2);
            pen.bevel(x, 0, section, 16, WHITE, GRAY_7);
            pen.text(x + 6, 12, part, Font::Small, BLACK);
            x += section;
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
            let phase = frame_number() as i32;
            crate::draw_progress_indeterminate(&mut pen, phase, theme.accent);
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
            draw_chasing_arrows(&mut pen, frame_number() % 8);
            pen.paint(bounds.origin, window);
            window.request_animation_frame();
        },
    )
    .w(px(16.))
    .h(px(16.))
    .flex_shrink_0()
}

pub fn draw_chasing_arrows(pen: &mut Pen, step: usize) {
    let turn = step as f64 / 8. * std::f64::consts::TAU;
    for y in 0..16 {
        for x in 0..16 {
            let (dx, dy) = (x as f64 + 0.5 - 8., y as f64 + 0.5 - 8.);
            let distance = dx.hypot(dy);
            let angle = (dy.atan2(dx) - turn + 4. * std::f64::consts::PI)
                .rem_euclid(std::f64::consts::TAU)
                .rem_euclid(std::f64::consts::PI);
            let ink = if angle < 0.55 {
                false
            } else if angle < 1.15 {
                (distance - 5.5).abs() <= (angle - 0.55) / 0.6 * 3.2
            } else {
                (4.3..=6.7).contains(&distance)
            };
            if ink {
                pen.dot(x, y, BLACK);
            }
        }
    }
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

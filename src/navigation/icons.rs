use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, KeyDownEvent, MouseButton, Role,
    SharedString, Window, div, prelude::*, px,
};

use crate::{
    Font, Language, ScrollBar, Theme,
    paint::{Pen, drawing},
    theme::*,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconKind {
    Folder,
    Document,
    Disk,
    Calculator,
    Jotter,
    Scribe,
    Puzzle,
    Clock,
    Appearance,
    Ledger,
    Controls,
    Trash,
    TrashFull,
    DateTime,
    Sound,
    Mouse,
    Language,
}

#[derive(Clone, Debug)]
pub struct IconItem {
    pub title: SharedString,
    pub kind: IconKind,
}
impl IconItem {
    pub fn new(title: impl Into<SharedString>, kind: IconKind) -> Self {
        Self {
            title: title.into(),
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconEvent {
    Selected(usize),
    Activated(usize),
}

pub struct IconView {
    focus: FocusHandle,
    items: Vec<IconItem>,
    selected: Option<usize>,
    width: i32,
    height: i32,
    offset: i32,
    label: SharedString,
    theme: Theme,
    language: Language,
}

impl EventEmitter<IconEvent> for IconView {}
impl Focusable for IconView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl IconView {
    pub fn new(items: Vec<IconItem>, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            items,
            selected: None,
            width: 280,
            height: 180,
            offset: 0,
            label: "Files".into(),
            theme: Theme::default(),
            language: Language::English,
        }
    }
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width.max(100);
        self.height = height.max(64);
        self
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = (index < self.items.len()).then_some(index);
        self
    }
    pub fn selection(&self) -> Option<usize> {
        self.selected
    }
    pub fn set_size(&mut self, width: i32, height: i32, cx: &mut Context<Self>) {
        self.width = width.max(100);
        self.height = height.max(64);
        self.offset = self.offset.min(self.max_offset());
        cx.notify();
    }
    pub fn items(&self) -> &[IconItem] {
        &self.items
    }
    pub fn set_items(&mut self, items: Vec<IconItem>, cx: &mut Context<Self>) {
        self.items = items;
        self.selected = self.selected.filter(|index| *index < self.items.len());
        self.offset = self.offset.min(self.max_offset());
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        cx.notify();
    }
    fn columns(&self) -> usize {
        ((self.width - 16) / 84).max(1) as usize
    }
    fn content_height(&self) -> i32 {
        self.items.len().div_ceil(self.columns()) as i32 * 64
    }
    fn max_offset(&self) -> i32 {
        (self.content_height() - self.height).max(0)
    }
    pub fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.items.len() {
            return;
        }
        if self.selected != Some(index) {
            self.selected = Some(index);
            cx.emit(IconEvent::Selected(index));
        }
        let top = (index / self.columns()) as i32 * 64;
        if top < self.offset {
            self.offset = top;
        }
        if top + 64 > self.offset + self.height {
            self.offset = top + 64 - self.height;
        }
        self.offset = self.offset.clamp(0, self.max_offset());
        cx.notify();
    }
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        let at = self.selected.unwrap_or(0);
        let next = match event.keystroke.key.as_str() {
            "left" => at.saturating_sub(1),
            "right" => (at + 1).min(self.items.len() - 1),
            "up" => at.saturating_sub(self.columns()),
            "down" => (at + self.columns()).min(self.items.len() - 1),
            "home" => 0,
            "end" => self.items.len() - 1,
            "enter" => {
                if let Some(index) = self.selected {
                    cx.emit(IconEvent::Activated(index));
                }
                cx.stop_propagation();
                return;
            }
            _ => return,
        };
        self.select(next, cx);
        cx.stop_propagation();
    }
}

impl Render for IconView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let columns = self.columns();
        let offset = self.offset;
        let theme = self.theme;
        let language = self.language;
        let focused = self.focus.is_focused(window);
        let scroll_entity = cx.entity();
        div()
            .id("icons")
            .role(Role::ListBox)
            .aria_label(self.label.clone())
            .track_focus(&self.focus)
            .tab_stop(true)
            .w(px(self.width as f32))
            .h(px(self.height as f32))
            .flex_shrink_0()
            .flex()
            .bg(gpui::rgb(WHITE))
            .on_key_down(cx.listener(Self::key_down))
            .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _, cx| {
                this.offset = (this.offset
                    - f32::from(event.delta.pixel_delta(px(16.)).y).round() as i32)
                    .clamp(0, this.max_offset());
                cx.notify();
                cx.stop_propagation();
            }))
            .child(
                div()
                    .relative()
                    .w(px((self.width - 16) as f32))
                    .h_full()
                    .overflow_hidden()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window, cx| {
                            window.focus(&this.focus, cx);
                        }),
                    )
                    .children(
                        self.items
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| {
                                let top = (*index / columns) as i32 * 64 - offset;
                                top + 64 > 0 && top < self.height
                            })
                            .map(|(index, item)| {
                                let title = item.title.clone();
                                let kind = item.kind;
                                let selected = self.selected == Some(index);
                                div()
                                    .id(index)
                                    .role(Role::ListBoxOption)
                                    .aria_label(title.clone())
                                    .aria_selected(selected)
                                    .absolute()
                                    .left(px((index % columns * 84) as f32))
                                    .top(px((index / columns) as f32 * 64. - offset as f32))
                                    .w(px(84.))
                                    .h(px(64.))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            move |this,
                                                  event: &gpui::MouseDownEvent,
                                                  window,
                                                  cx| {
                                                window.focus(&this.focus, cx);
                                                this.select(index, cx);
                                                if event.click_count == 2 {
                                                    cx.emit(IconEvent::Activated(index));
                                                }
                                                cx.stop_propagation();
                                            },
                                        ),
                                    )
                                    .child(drawing(84., 64., move |pen, _, _| {
                                        pen.language = language;
                                        draw_icon(pen, 26, 4, kind, selected);
                                        let name = fit_title(&title, 80, language);
                                        let width = Font::Small.measure_in(&name, language);
                                        pen.rect(
                                            (84 - width) / 2 - 2,
                                            38,
                                            width + 4,
                                            13,
                                            if selected {
                                                if focused { theme.accent.dark } else { BLACK }
                                            } else {
                                                WHITE
                                            },
                                        );
                                        pen.text(
                                            (84 - width) / 2,
                                            48,
                                            &name,
                                            Font::Small,
                                            if selected { WHITE } else { BLACK },
                                        );
                                    }))
                            }),
                    ),
            )
            .child(
                ScrollBar::new(
                    "icon-scroll",
                    self.offset as f64,
                    self.height as f64,
                    self.content_height() as f64,
                )
                .length(self.height)
                .step(64.)
                .theme(theme)
                .on_change(move |value, _, cx| {
                    scroll_entity.update(cx, |this, cx| {
                        this.offset = value.round() as i32;
                        cx.notify();
                    });
                }),
            )
    }
}

pub(super) fn fit_title(title: &str, width: i32, language: Language) -> String {
    if Font::Small.measure_in(title, language) <= width {
        return title.to_owned();
    }
    let mut result = String::new();
    for ch in title.chars() {
        if Font::Small.measure_in(&format!("{result}{ch}…"), language) > width {
            break;
        }
        result.push(ch);
    }
    result.push('…');
    result
}

/// Draws the original desk's 32-pixel icons without bitmap scaling.
pub fn draw_icon(pen: &mut Pen, x: i32, y: i32, kind: IconKind, selected: bool) {
    let mut icon = Pen::new(32, 32);
    let p = &mut icon;
    match kind {
        IconKind::Folder => {
            box_at(p, 2, 7, 12, 4, 0x9988DD);
            p.hline(3, 8, 10, 0xCCC4F4);
            box_at(p, 2, 10, 28, 18, 0x9988DD);
            p.hline(3, 11, 26, 0xCCC4F4);
            p.vline(3, 11, 16, 0xCCC4F4);
            p.vline(28, 11, 16, 0x665CAA);
            p.hline(3, 26, 26, 0x665CAA);
            p.hline(3, 13, 25, 0x665CAA);
            p.hline(4, 14, 24, 0xCCC4F4);
            p.hline(3, 28, 28, GRAY_7);
        }
        IconKind::Disk => {
            box_at(p, 2, 9, 28, 16, GRAY_2);
            p.bevel(3, 10, 26, 14, WHITE, GRAY_7);
            p.rect(5, 18, 14, 2, BLACK);
            p.rect(24, 18, 2, 2, 0x559955);
            p.hline(3, 25, 27, GRAY_7);
        }
        IconKind::Document => {
            for row in 2..30 {
                let right = if row < 8 { 19 + row - 2 } else { 24 };
                p.hline(7, row, right - 6, WHITE);
                p.dot(7, row, BLACK);
                p.dot(right, row, BLACK);
            }
            p.hline(7, 2, 13, BLACK);
            p.hline(7, 29, 18, BLACK);
            p.vline(19, 2, 6, BLACK);
            p.hline(19, 7, 6, BLACK);
            for row in (11..27).step_by(3) {
                p.hline(10, row, if row % 2 == 0 { 9 } else { 12 }, GRAY_7);
            }
            p.hline(8, 30, 18, GRAY_7);
        }
        IconKind::Calculator => {
            box_at(p, 5, 2, 22, 28, GRAY_4);
            box_at(p, 8, 5, 16, 6, 0xBBCCAA);
            p.rect(17, 7, 5, 2, BLACK);
            for row in 0..4 {
                for col in 0..3 {
                    box_at(p, 8 + col * 6, 13 + row * 4, 4, 3, GRAY_1);
                }
            }
            box_at(p, 8, 29, 16, 2, 0xCC3333);
        }
        IconKind::Jotter => {
            box_at(p, 5, 4, 22, 26, 0xFFE066);
            for col in (8..26).step_by(4) {
                p.rect(col, 2, 2, 5, GRAY_7);
                p.vline(col, 2, 5, BLACK);
            }
            for row in (11..28).step_by(3) {
                p.hline(7, row, 18, 0xCC9933);
            }
            for i in 0..3 {
                p.dot(24 + i, 26 + i, BLACK);
            }
        }
        IconKind::Scribe => {
            box_at(p, 4, 6, 18, 24, WHITE);
            for row in (10..26).step_by(3) {
                p.hline(7, row, 11, GRAY_7);
            }
            for i in 0..16 {
                p.rect(12 + i, 25 - i, 3, 2, 0x334488);
            }
            p.hline(12, 27, 3, BLACK);
            p.rect(27, 8, 3, 2, 0xCC3333);
        }
        IconKind::Puzzle => {
            for (i, color) in [0x6688CC, 0xFFE066, 0x559955].into_iter().enumerate() {
                let col = 3 + i as i32 % 2 * 14;
                let row = 3 + i as i32 / 2 * 14;
                box_at(p, col, row, 13, 13, color);
                p.hline(col + 1, row + 1, 11, WHITE);
                p.vline(col + 1, row + 1, 11, WHITE);
            }
            p.frame(17, 17, 13, 13, GRAY_7);
        }
        IconKind::Appearance => {
            box_at(p, 3, 4, 26, 20, GRAY_2);
            box_at(p, 6, 7, 20, 14, 0x334488);
            p.rect(7, 8, 18, 4, 0x9988DD);
            p.rect(7, 12, 9, 8, 0x6688CC);
            p.rect(16, 12, 9, 8, 0x559955);
            box_at(p, 11, 24, 10, 3, GRAY_4);
            box_at(p, 7, 27, 18, 3, GRAY_4);
        }
        IconKind::Ledger => {
            box_at(p, 4, 3, 24, 26, WHITE);
            p.rect(5, 4, 22, 4, 0x99BB99);
            p.hline(5, 8, 22, BLACK);
            for row in (12..28).step_by(4) {
                p.hline(5, row, 22, GRAY_4);
            }
            p.vline(12, 4, 24, GRAY_7);
            p.vline(20, 4, 24, GRAY_7);
            p.rect(22, 21, 4, 2, 0x334488);
            p.rect(14, 13, 4, 2, 0x334488);
        }
        IconKind::Controls => {
            box_at(p, 3, 4, 26, 24, GRAY_2);
            p.rect(4, 5, 24, 4, GRAY_4);
            p.hline(3, 9, 26, BLACK);
            box_at(p, 7, 12, 6, 6, WHITE);
            p.rect(9, 14, 2, 2, BLACK);
            box_at(p, 16, 12, 6, 6, WHITE);
            p.rect(18, 14, 2, 2, 0x334488);
            p.hline(7, 21, 18, GRAY_7);
            box_at(p, 15, 19, 3, 5, 0x9988DD);
            box_at(p, 9, 25, 14, 1, BLACK);
        }
        IconKind::Trash => {
            box_at(p, 9, 3, 14, 3, GRAY_4);
            box_at(p, 5, 5, 22, 4, GRAY_4);
            box_at(p, 7, 9, 18, 21, GRAY_2);
            for col in (11..23).step_by(4) {
                p.rect(col, 11, 2, 17, GRAY_5);
                p.vline(col, 11, 17, GRAY_7);
            }
        }
        IconKind::Clock => {
            p.rect(5, 27, 4, 3, BLACK);
            p.rect(23, 27, 4, 3, BLACK);
            disc(p, 32, 33, 26, WHITE);
            disc(p, 9, 7, 7, 0xCC3333);
            disc(p, 55, 7, 7, 0xCC3333);
            line(p, 16, 16, 16, 9, BLACK);
            line(p, 16, 16, 21, 19, BLACK);
            p.rect(15, 15, 2, 2, BLACK);
        }
        IconKind::TrashFull => {
            box_at(p, 7, 9, 18, 21, GRAY_2);
            for col in (11..23).step_by(4) {
                p.rect(col, 11, 2, 17, GRAY_5);
                p.vline(col, 11, 17, GRAY_7);
            }
            box_at(p, 8, 5, 8, 6, WHITE);
            p.hline(10, 7, 4, GRAY_5);
            box_at(p, 15, 6, 9, 5, WHITE);
            p.hline(17, 8, 5, GRAY_5);
            box_at(p, 9, 0, 14, 3, GRAY_4);
            box_at(p, 5, 2, 22, 4, GRAY_4);
        }
        IconKind::DateTime => {
            box_at(p, 4, 6, 24, 22, WHITE);
            p.rect(5, 7, 22, 5, 0xCC3333);
            p.rect(9, 3, 2, 6, BLACK);
            p.rect(21, 3, 2, 6, BLACK);
            for row in (14..26).step_by(4) {
                for col in (7..25).step_by(4) {
                    p.rect(col, row, 2, 2, GRAY_7);
                }
            }
        }
        IconKind::Sound => {
            box_at(p, 6, 11, 6, 10, GRAY_4);
            for i in 0..8 {
                p.vline(11 + i, 11 - i, 10 + 2 * i, GRAY_4);
                p.dot(11 + i, 11 - i, BLACK);
                p.dot(11 + i, 20 + i, BLACK);
            }
            p.vline(19, 3, 26, BLACK);
            line(p, 23, 10, 25, 8, BLACK);
            line(p, 23, 16, 27, 16, BLACK);
            line(p, 23, 22, 25, 24, BLACK);
        }
        IconKind::Mouse => {
            box_at(p, 9, 8, 14, 21, GRAY_2);
            p.rect(10, 9, 12, 6, GRAY_4);
            p.hline(10, 15, 12, BLACK);
            line(p, 16, 8, 16, 3, BLACK);
            line(p, 16, 3, 22, 1, BLACK);
        }
        IconKind::Language => {
            disc(p, 32, 32, 24, 0x6688CC);
            for (x0, y0, x1, y1) in [
                (16, 5, 16, 27),
                (5, 16, 27, 16),
                (7, 10, 25, 10),
                (7, 22, 25, 22),
                (12, 6, 10, 16),
                (10, 16, 12, 26),
                (20, 6, 22, 16),
                (22, 16, 20, 26),
            ] {
                line(p, x0, y0, x1, y1, BLACK);
            }
        }
    }
    for fill in icon.fills {
        let color = if selected {
            (fill.color & 0xFEFEFE) >> 1
        } else {
            fill.color
        };
        pen.rect(
            x + fill.rect.x,
            y + fill.rect.y,
            fill.rect.width,
            fill.rect.height,
            color,
        );
    }
}

fn box_at(pen: &mut Pen, x: i32, y: i32, width: i32, height: i32, color: u32) {
    pen.rect(x, y, width, height, color);
    pen.frame(x, y, width, height, BLACK);
}

fn disc(pen: &mut Pen, center_x: i32, center_y: i32, radius: i32, face: u32) {
    for y in 0..32 {
        for x in 0..32 {
            let distance = (2 * x + 1 - center_x).pow(2) + (2 * y + 1 - center_y).pow(2);
            if distance <= radius * radius {
                pen.dot(
                    x,
                    y,
                    if distance > (radius - 3).pow(2) {
                        BLACK
                    } else {
                        face
                    },
                );
            }
        }
    }
}

fn line(pen: &mut Pen, mut x: i32, mut y: i32, end_x: i32, end_y: i32, color: u32) {
    let dx = (end_x - x).abs();
    let dy = -(end_y - y).abs();
    let step_x = (end_x - x).signum();
    let step_y = (end_y - y).signum();
    let mut error = dx + dy;
    loop {
        pen.dot(x, y, color);
        if x == end_x && y == end_y {
            break;
        }
        let twice_error = 2 * error;
        if twice_error >= dy {
            error += dy;
            x += step_x;
        }
        if twice_error <= dx {
            error += dx;
            y += step_y;
        }
    }
}

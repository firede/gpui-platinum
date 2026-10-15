use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, KeyDownEvent, MouseButton, Role, Window,
    div, prelude::*, px,
};

use super::icons::fit_title;
use crate::{
    Font, Language, ScrollBar, Theme,
    models::{FileTree, TreeNode},
    paint::drawing,
    theme::*,
};

const COLUMN_WIDTH: i32 = 140;
const ROW_HEIGHT: i32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnEvent {
    Selected(usize),
    Activated(usize),
}

pub struct ColumnView {
    focus: FocusHandle,
    model: FileTree,
    path: Vec<usize>,
    active_column: usize,
    offsets: Vec<i32>,
    horizontal: i32,
    width: i32,
    height: i32,
    theme: Theme,
    language: Language,
}
impl EventEmitter<ColumnEvent> for ColumnView {}
impl Focusable for ColumnView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl ColumnView {
    pub fn new(model: FileTree, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            model,
            path: Vec::new(),
            active_column: 0,
            offsets: vec![0],
            horizontal: 0,
            width: 420,
            height: 160,
            theme: Theme::default(),
            language: Language::English,
        }
    }
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width.max(140);
        self.height = height.max(64);
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
    pub fn path(&self) -> &[usize] {
        &self.path
    }
    pub fn model(&self) -> &FileTree {
        &self.model
    }
    pub fn model_mut(&mut self) -> &mut FileTree {
        &mut self.model
    }
    pub fn selection(&self) -> Option<usize> {
        self.path.last().copied()
    }
    pub fn set_size(&mut self, width: i32, height: i32, cx: &mut Context<Self>) {
        self.width = width.max(140);
        self.height = height.max(64);
        self.horizontal = self.horizontal.min(self.max_horizontal());
        for column in 0..self.offsets.len() {
            self.offsets[column] = self.offsets[column].min(self.max_vertical(column));
        }
        cx.notify();
    }
    pub fn set_model(&mut self, model: FileTree, cx: &mut Context<Self>) {
        self.model = model;
        let valid = self
            .path
            .iter()
            .take_while(|id| self.model.find(**id).is_some())
            .count();
        self.path.truncate(valid);
        self.active_column = self.active_column.min(self.columns().saturating_sub(1));
        self.offsets.resize(self.columns(), 0);
        self.horizontal = self.horizontal.min(self.max_horizontal());
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
    fn rows(&self, column: usize) -> &[TreeNode] {
        if column == 0 {
            &self.model.roots
        } else {
            self.path
                .get(column - 1)
                .and_then(|id| self.model.find(*id))
                .map(|node| node.children.as_slice())
                .unwrap_or_default()
        }
    }
    fn columns(&self) -> usize {
        self.path.len()
            + usize::from(
                self.path
                    .last()
                    .and_then(|id| self.model.find(*id))
                    .is_none_or(|node| node.kind.is_container()),
            )
    }
    fn max_horizontal(&self) -> i32 {
        (self.columns() as i32 * COLUMN_WIDTH - self.width).max(0)
    }
    fn max_vertical(&self, column: usize) -> i32 {
        (self.rows(column).len() as i32 * ROW_HEIGHT - (self.height - 16)).max(0)
    }
    fn select(&mut self, column: usize, id: usize, cx: &mut Context<Self>) {
        if !self.rows(column).iter().any(|node| node.id == id) {
            return;
        }
        self.path.truncate(column);
        self.path.push(id);
        self.active_column = column;
        self.model.selected = Some(id);
        self.offsets.resize(self.columns(), 0);
        self.horizontal = self.max_horizontal();
        if let Some(index) = self.rows(column).iter().position(|node| node.id == id) {
            let top = index as i32 * ROW_HEIGHT;
            if top < self.offsets[column] {
                self.offsets[column] = top;
            }
            if top + ROW_HEIGHT > self.offsets[column] + self.height - 16 {
                self.offsets[column] = top + ROW_HEIGHT - (self.height - 16);
            }
        }
        cx.emit(ColumnEvent::Selected(id));
        cx.notify();
    }
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let column = self.active_column.min(self.columns().saturating_sub(1));
        let rows = self.rows(column);
        let current = self
            .path
            .get(column)
            .and_then(|id| rows.iter().position(|node| node.id == *id));
        match event.keystroke.key.as_str() {
            "up" | "down" | "home" | "end" | "pageup" | "pagedown" => {
                if rows.is_empty() {
                    return;
                }
                let page = ((self.height - 16) / ROW_HEIGHT).max(1) as usize;
                let at = current.unwrap_or(0);
                let next = match event.keystroke.key.as_str() {
                    "up" => at.saturating_sub(1),
                    "down" => current.map_or(0, |i| i + 1).min(rows.len() - 1),
                    "home" => 0,
                    "end" => rows.len() - 1,
                    "pageup" => at.saturating_sub(page),
                    _ => (at + page).min(rows.len() - 1),
                };
                let id = rows[next].id;
                self.select(column, id, cx);
            }
            "right" => {
                if let Some(id) = self.rows(column + 1).first().map(|node| node.id) {
                    self.select(column + 1, id, cx);
                }
            }
            "left" => {
                if column > 0 {
                    let id = self.path[column - 1];
                    self.select(column - 1, id, cx);
                }
            }
            "enter" => {
                if let Some(id) = self.path.get(column) {
                    cx.emit(ColumnEvent::Activated(*id));
                }
            }
            _ => return,
        }
        cx.stop_propagation();
    }
}
impl Render for ColumnView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let height = self.height - 16;
        let theme = self.theme;
        let language = self.language;
        let focused = self.focus.is_focused(window);
        let horizontal_entity = cx.entity();
        let mut columns = Vec::new();
        for column in 0..self.columns() {
            let rows = self.rows(column);
            let content = rows.len() as i32 * ROW_HEIGHT;
            let offset = *self.offsets.get(column).unwrap_or(&0);
            let selected = self.path.get(column).copied();
            let scroll_entity = cx.entity();
            columns.push(div().id(column).w(px(COLUMN_WIDTH as f32)).h(px(height as f32)).flex_shrink_0().flex()
                .border_r_1().border_color(gpui::rgb(GRAY_7))
                .on_scroll_wheel(cx.listener(move |this, event: &gpui::ScrollWheelEvent, _, cx| {
                    let max = this.max_vertical(column); this.offsets[column] = (this.offsets[column] - f32::from(event.delta.pixel_delta(px(16.)).y).round() as i32).clamp(0, max); cx.notify(); cx.stop_propagation();
                }))
                .child(div().relative().w(px((COLUMN_WIDTH - 17) as f32)).h_full().overflow_hidden()
                    .children(rows.iter().enumerate().filter(|(row, _)| *row as i32 * ROW_HEIGHT - offset + ROW_HEIGHT > 0 && *row as i32 * ROW_HEIGHT - offset < height).map(|(row, node)| {
                        let id = node.id; let title = node.title.clone(); let has_children = node.kind.is_container(); let is_selected = selected == Some(id);
                        div().id(id).role(Role::ListBoxOption).aria_label(title.clone()).aria_selected(is_selected)
                            .absolute().left_0().top(px((row as i32 * ROW_HEIGHT - offset) as f32)).w_full().h(px(ROW_HEIGHT as f32))
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                                window.focus(&this.focus, cx); this.select(column, id, cx); if event.click_count == 2 { cx.emit(ColumnEvent::Activated(id)); } cx.stop_propagation();
                            }))
                            .child(drawing((COLUMN_WIDTH - 17) as f32, ROW_HEIGHT as f32, move |pen, _, _| {
                                pen.language = language; let ink = if is_selected { WHITE } else { BLACK };
                                pen.rect(0, 0, pen.width, ROW_HEIGHT, if is_selected { if focused { theme.accent.dark } else { GRAY_7 } } else { WHITE });
                                pen.text(4, 12, &fit_title(&title, pen.width - 16, language), Font::Small, ink);
                                if has_children { for row in 0..5 { let width = 3 - (row - 2i32).abs(); pen.hline(pen.width - 7, 5 + row, width, ink); } }
                            }))
                    })))
                .child(ScrollBar::new(("column-scroll", column), offset as f64, height as f64, content as f64).length(height).step(16.).theme(theme).on_change(move |value, _, cx| { scroll_entity.update(cx, |this, cx| { if let Some(offset) = this.offsets.get_mut(column) { *offset = value.round() as i32; cx.notify(); } }); })));
        }
        div()
            .id("columns")
            .role(Role::ListBox)
            .aria_label(crate::tr(language, "files"))
            .track_focus(&self.focus)
            .tab_stop(true)
            .w(px(self.width as f32))
            .h(px(self.height as f32))
            .flex_shrink_0()
            .bg(gpui::rgb(WHITE))
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| window.focus(&this.focus, cx)),
            )
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(height as f32))
                    .overflow_hidden()
                    .child(
                        div()
                            .absolute()
                            .left(px(-self.horizontal as f32))
                            .top_0()
                            .flex()
                            .children(columns),
                    ),
            )
            .child(
                ScrollBar::new(
                    "columns-horizontal",
                    self.horizontal as f64,
                    self.width as f64,
                    (self.columns() as i32 * COLUMN_WIDTH) as f64,
                )
                .horizontal()
                .length(self.width)
                .step(COLUMN_WIDTH as f64)
                .theme(theme)
                .on_change(move |value, _, cx| {
                    horizontal_entity.update(cx, |this, cx| {
                        this.horizontal = value.round() as i32;
                        cx.notify();
                    });
                }),
            )
    }
}

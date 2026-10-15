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
    always_scrollbar: bool,
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
    pub fn from_nodes(roots: Vec<TreeNode>, cx: &mut Context<Self>) -> Self {
        Self::new(FileTree::new(roots), cx)
    }
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
            always_scrollbar: false,
            theme: Theme::default(),
            language: Language::English,
        }
    }
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width.max(140);
        self.height = height.max(64);
        self.clamp_scroll_offsets();
        self
    }
    /// Keeps vertical scroll bars visible even when a column's rows fit.
    pub fn always_scrollbar(mut self, always: bool) -> Self {
        self.always_scrollbar = always;
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
    /// Updates the path and reveals the selected item without emitting an event.
    pub fn set_selection(&mut self, selected: Option<usize>, cx: &mut Context<Self>) {
        self.path = selected.map(|id| self.model.path(id)).unwrap_or_default();
        self.model.selected = self.path.last().copied();
        self.active_column = self.path.len().saturating_sub(1);
        self.clamp_scroll_offsets();
        for column in 0..self.path.len() {
            if let Some(row) = self
                .rows(column)
                .iter()
                .position(|node| node.id == self.path[column])
            {
                let top = row as i32 * ROW_HEIGHT;
                let bottom = top + ROW_HEIGHT - (self.height - 16);
                self.offsets[column] = self.offsets[column].min(top).max(bottom.max(0));
            }
        }
        self.horizontal = self.max_horizontal();
        cx.notify();
    }
    pub fn set_size(&mut self, width: i32, height: i32, cx: &mut Context<Self>) {
        self.width = width.max(140);
        self.height = height.max(64);
        self.clamp_scroll_offsets();
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
        self.clamp_scroll_offsets();
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
                    .is_none_or(|node| !node.children.is_empty()),
            )
    }
    fn max_horizontal(&self) -> i32 {
        (self.content_width() - self.width).max(0)
    }
    fn content_width(&self) -> i32 {
        self.columns() as i32 * (COLUMN_WIDTH - 1) + 1
    }
    fn max_vertical(&self, column: usize) -> i32 {
        (self.rows(column).len() as i32 * ROW_HEIGHT - (self.height - 16)).max(0)
    }
    fn clamp_scroll_offsets(&mut self) {
        self.offsets.resize(self.columns(), 0);
        for column in 0..self.offsets.len() {
            self.offsets[column] = self.offsets[column].clamp(0, self.max_vertical(column));
        }
        self.horizontal = self.horizontal.clamp(0, self.max_horizontal());
    }
    fn select(&mut self, column: usize, id: usize, cx: &mut Context<Self>) {
        if !self.rows(column).iter().any(|node| node.id == id) {
            return;
        }
        self.path.truncate(column);
        self.path.push(id);
        self.active_column = column;
        self.model.selected = Some(id);
        self.clamp_scroll_offsets();
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
        self.clamp_scroll_offsets();
        let height = self.height - 16;
        let theme = self.theme;
        let language = self.language;
        let focused = self.focus.is_focused(window);
        let horizontal_entity = cx.entity();
        let mut columns = Vec::new();
        for column in 0..self.columns() {
            let rows = self.rows(column);
            let content = rows.len() as i32 * ROW_HEIGHT;
            let scrolls = self.always_scrollbar || content > height;
            let content_width = COLUMN_WIDTH - 1 - if scrolls { 16 } else { 0 };
            let offset = *self.offsets.get(column).unwrap_or(&0);
            let selected = self.path.get(column).copied();
            let scroll_entity = cx.entity();
            let row_elements = rows.iter().enumerate().filter_map(|(row, node)| {
                let top = row as i32 * ROW_HEIGHT - offset;
                if top + ROW_HEIGHT <= 0 || top >= height {
                    return None;
                }
                let id = node.id;
                let title = node.title.clone();
                let has_children = !node.children.is_empty();
                let is_selected = selected == Some(id);
                Some(
                    div()
                        .id(id)
                        .role(Role::ListBoxOption)
                        .aria_label(title.clone())
                        .aria_selected(is_selected)
                        .absolute()
                        .left_0()
                        .top(px(top as f32))
                        .w_full()
                        .h(px(ROW_HEIGHT as f32))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                                window.focus(&this.focus, cx);
                                this.select(column, id, cx);
                                if event.click_count == 2 {
                                    cx.emit(ColumnEvent::Activated(id));
                                }
                                cx.stop_propagation();
                            }),
                        )
                        .child(drawing(
                            content_width as f32,
                            ROW_HEIGHT as f32,
                            move |pen, _, _| {
                                pen.language = language;
                                let focused = focused && !crate::window::content_is_inactive();
                                let ink = if is_selected && focused { WHITE } else { BLACK };
                                let face = if is_selected {
                                    if focused { theme.accent.dark } else { GRAY_4 }
                                } else {
                                    WHITE
                                };
                                pen.rect(0, 0, pen.width, ROW_HEIGHT, face);
                                pen.text(
                                    4,
                                    12,
                                    &fit_title(&title, pen.width - 16, language),
                                    Font::Small,
                                    ink,
                                );
                                if has_children {
                                    for row in 0..5 {
                                        let width = 3 - (row - 2i32).abs();
                                        pen.hline(pen.width - 7, 5 + row, width, ink);
                                    }
                                }
                            },
                        )),
                )
            });
            columns.push(
                div()
                    .id(column)
                    .w(px(COLUMN_WIDTH as f32))
                    .h(px(height as f32))
                    .flex_shrink_0()
                    .mr(px(-1.))
                    .flex()
                    .border_r_1()
                    .border_color(gpui::rgb(GRAY_7))
                    .on_scroll_wheel(cx.listener(
                        move |this, event: &gpui::ScrollWheelEvent, _, cx| {
                            let max = this.max_vertical(column);
                            let delta =
                                f32::from(event.delta.pixel_delta(px(16.)).y).round() as i32;
                            this.offsets[column] = (this.offsets[column] - delta).clamp(0, max);
                            cx.notify();
                            cx.stop_propagation();
                        },
                    ))
                    .child(
                        div()
                            .relative()
                            .w(px(content_width as f32))
                            .h_full()
                            .overflow_hidden()
                            .children(row_elements),
                    )
                    .when(scrolls, |element| {
                        element.child(
                            ScrollBar::new(
                                ("column-scroll", column),
                                offset as f64,
                                height as f64,
                                content as f64,
                            )
                            .tab_stop(false)
                            .length(height)
                            .step(16.)
                            .theme(theme)
                            .on_change(move |value, _, cx| {
                                scroll_entity.update(cx, |this, cx| {
                                    if let Some(offset) = this.offsets.get_mut(column) {
                                        *offset = value.round() as i32;
                                        cx.notify();
                                    }
                                });
                            }),
                        )
                    }),
            );
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
                    self.content_width() as f64,
                )
                .tab_stop(false)
                .horizontal()
                .length(self.width)
                .step(COLUMN_WIDTH as f64 / 2.)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn branch(id: usize, child_count: usize) -> TreeNode {
        TreeNode::item(id, format!("Folder {id}")).with_children(
            (0..child_count)
                .map(|index| TreeNode::item(id * 100 + index, format!("Item {index}")))
                .collect(),
        )
    }

    #[gpui::test]
    fn changing_to_a_shorter_directory_keeps_its_children_visible(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let view = cx.new(|cx| {
                ColumnView::from_nodes(vec![branch(1, 20), branch(2, 2)], cx).size(420, 160)
            });
            view.update(cx, |view, cx| {
                for silent in [false, true] {
                    view.select(0, 1, cx);
                    view.offsets[1] = view.max_vertical(1);
                    assert!(view.offsets[1] > 0);

                    if silent {
                        view.set_selection(Some(2), cx);
                    } else {
                        view.select(0, 2, cx);
                    }

                    assert_eq!(view.selection(), Some(2));
                    assert_eq!(view.rows(1).len(), 2);
                    assert_eq!(view.offsets[1], 0);
                }
            });
        });
    }

    #[gpui::test]
    fn replacing_the_model_clamps_root_and_child_scroll_positions(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let view = cx.new(|cx| {
                ColumnView::from_nodes((1..=20).map(|id| branch(id, 20)).collect(), cx)
                    .size(420, 160)
            });
            view.update(cx, |view, cx| {
                view.set_selection(Some(20), cx);
                view.offsets[1] = view.max_vertical(1);
                assert!(view.offsets.iter().all(|offset| *offset > 0));

                view.set_model(FileTree::new(vec![branch(20, 2), branch(21, 2)]), cx);

                assert_eq!(view.path(), &[20]);
                assert_eq!(view.offsets, vec![0, 0]);
                view.set_selection(None, cx);
                assert_eq!(view.offsets, vec![0]);
            });
        });
    }
}

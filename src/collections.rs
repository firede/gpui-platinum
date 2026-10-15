//! Trees and tables with pixel-drawn rows and scroll bars.

use std::{
    cmp::Ordering,
    collections::BTreeSet,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, ContentMask, Context, EventEmitter, FocusHandle, Focusable, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Point, Role, ScrollWheelEvent,
    SharedString, Window, canvas, div, point, prelude::*, px,
};

use crate::{
    Font, Language, Theme,
    models::{FileTree, NodeKind},
    paint::Pen,
    theme::*,
};

const ROW_HEIGHT: i32 = 16;
const HEADER_HEIGHT: i32 = 18;
const SCROLL_WIDTH: i32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeEvent {
    Selected(usize),
    Activated(usize),
}

pub struct TreeView {
    focus: FocusHandle,
    model: FileTree,
    viewport: Viewport,
    label: SharedString,
    theme: Theme,
    language: Language,
    prefix: String,
    typed_at: Instant,
}

impl EventEmitter<TreeEvent> for TreeView {}

impl TreeView {
    pub fn new(model: FileTree, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            model,
            viewport: Viewport::new(240, 120, 0),
            label: "Tree".into(),
            theme: Theme::default(),
            language: Language::English,
            prefix: String::new(),
            typed_at: Instant::now(),
        }
    }

    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.viewport.width = width.max(80);
        self.viewport.height = height.max(40);
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
    pub fn selection(&self) -> Option<usize> {
        self.model.selected
    }
    pub fn model(&self) -> &FileTree {
        &self.model
    }
    pub fn model_mut(&mut self) -> &mut FileTree {
        &mut self.model
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        cx.notify();
    }

    pub fn select(&mut self, id: usize, cx: &mut Context<Self>) {
        let previous = self.model.selected;
        if self.model.select(id) {
            self.changed(previous, cx);
        }
    }

    fn changed(&mut self, previous: Option<usize>, cx: &mut Context<Self>) {
        let rows = self.model.visible_rows();
        if let Some(index) = rows
            .iter()
            .position(|row| Some(row.node.id) == self.model.selected)
        {
            self.viewport.reveal(index, rows.len());
        }
        self.viewport.clamp(rows.len());
        if previous != self.model.selected
            && let Some(id) = self.model.selected
        {
            cx.emit(TreeEvent::Selected(id));
        }
        cx.notify();
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let previous = self.model.selected;
        let page = self.viewport.visible() as i32;
        match event.keystroke.key.as_str() {
            "up" => self.model.move_selection(-1),
            "down" => self.model.move_selection(1),
            "left" => self.model.left(),
            "right" => self.model.right(),
            "pageup" => self.model.move_selection(-page),
            "pagedown" => self.model.move_selection(page),
            "home" => {
                if let Some(row) = self.model.visible_rows().first() {
                    let id = row.node.id;
                    self.model.select(id);
                }
            }
            "end" => {
                if let Some(row) = self.model.visible_rows().last() {
                    let id = row.node.id;
                    self.model.select(id);
                }
            }
            "enter" => {
                if let Some(id) = self.model.selected {
                    cx.emit(TreeEvent::Activated(id));
                }
            }
            _ => {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                let Some(text) = &event.keystroke.key_char else {
                    return;
                };
                if text.chars().any(char::is_control) {
                    return;
                }
                if self.typed_at.elapsed() > Duration::from_secs(1) {
                    self.prefix.clear();
                }
                self.prefix.push_str(text);
                self.typed_at = Instant::now();
                self.model.select_prefix(&self.prefix);
            }
        }
        self.changed(previous, cx);
        cx.stop_propagation();
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        let count = self.model.visible_rows().len();
        if self.viewport.press_scroll(event.position, count) {
            cx.notify();
            return;
        }
        let Some((x, y)) = self.viewport.local(event.position) else {
            return;
        };
        let Some(index) = self.viewport.row_at(y, count) else {
            return;
        };
        let (id, depth, children) = {
            let rows = self.model.visible_rows();
            let row = &rows[index];
            (row.node.id, row.depth, !row.node.children.is_empty())
        };
        let previous = self.model.selected;
        self.model.select(id);
        if children && x >= 2 + depth as i32 * 16 && x < 18 + depth as i32 * 16 {
            self.model.toggle(id);
        } else if event.click_count == 2 {
            cx.emit(TreeEvent::Activated(id));
        }
        self.changed(previous, cx);
    }

    fn paint(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        let focused = self.focus.is_focused(window);
        let mut pen = Pen::new(self.viewport.width, self.viewport.height);
        pen.language = self.language;
        self.viewport.draw_frame(&mut pen, focused, self.theme);
        let rows = self.model.visible_rows();
        for (visible, row) in rows
            .iter()
            .skip(self.viewport.offset)
            .take(self.viewport.visible())
            .enumerate()
        {
            let y = 2 + visible as i32 * ROW_HEIGHT;
            let x = 2 + row.depth as i32 * 16;
            let selected = self.model.selected == Some(row.node.id);
            let ink = if selected && focused { WHITE } else { BLACK };
            if selected {
                pen.rect(
                    x + 16,
                    y,
                    (self.viewport.content_width() - x - 16).max(0),
                    ROW_HEIGHT,
                    if focused {
                        self.theme.accent.dark
                    } else {
                        GRAY_4
                    },
                );
            }
            if !row.node.children.is_empty() {
                draw_disclosure(&mut pen, x + 3, y + 3, row.node.open);
            }
            draw_file_icon(&mut pen, x + 18, y + 2, row.node.kind, ink);
            let title = elide(
                &row.node.title,
                self.viewport.content_width() - x - 38,
                self.language,
            );
            pen.text(x + 35, y + 12, &title, Font::Small, ink);
        }
        self.viewport
            .draw_scrollbar(&mut pen, rows.len(), self.theme);
        paint_inside(&pen, bounds, window);
    }
}

impl Focusable for TreeView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for TreeView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let layout = cx.entity();
        let paint = cx.entity();
        let rows = self.model.visible_rows();
        self.viewport.clamp(rows.len());
        let accessible = rows
            .iter()
            .skip(self.viewport.offset)
            .take(self.viewport.visible())
            .enumerate()
            .map(|(index, row)| {
                div()
                    .id(row.node.id)
                    .role(Role::TreeItem)
                    .aria_label(row.node.title.clone())
                    .aria_selected(self.model.selected == Some(row.node.id))
                    .aria_expanded(row.node.open)
                    .aria_level(row.depth + 1)
                    .absolute()
                    .left(px(2.))
                    .top(px((2 + index as i32 * ROW_HEIGHT) as f32))
                    .w(px(self.viewport.content_width() as f32))
                    .h(px(ROW_HEIGHT as f32))
            })
            .collect::<Vec<_>>();
        div()
            .id("platinum-tree")
            .relative()
            .role(Role::Tree)
            .aria_label(self.label.clone())
            .track_focus(&self.focus)
            .tab_stop(true)
            .w(px(self.viewport.width as f32))
            .h(px(self.viewport.height as f32))
            .flex_shrink_0()
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if event.dragging()
                    && this
                        .viewport
                        .drag_scroll(event.position, this.model.visible_rows().len())
                {
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.viewport.drag = None),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.viewport.drag = None),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                this.viewport.scroll(event, this.model.visible_rows().len());
                cx.stop_propagation();
                cx.notify();
            }))
            .child(
                canvas(
                    move |bounds, _, cx| {
                        layout.update(cx, |this, _| this.viewport.bounds = Some(bounds));
                    },
                    move |bounds, _, window, cx| paint.read(cx).paint(bounds, window),
                )
                .size_full(),
            )
            .children(accessible)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableEvent {
    Selected(usize),
    Activated(usize),
}

pub struct Table {
    focus: FocusHandle,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    order: Vec<usize>,
    widths: Vec<i32>,
    selected: Option<usize>,
    selection: BTreeSet<usize>,
    anchor: Option<usize>,
    sort: Option<(usize, bool)>,
    resizing: Option<(usize, i32, i32)>,
    viewport: Viewport,
    theme: Theme,
    language: Language,
    label: SharedString,
}

impl EventEmitter<TableEvent> for Table {}

impl Table {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>, cx: &mut Context<Self>) -> Self {
        let width = (headers.len().max(1) as i32 * 120 + 20).max(200);
        Self {
            focus: cx.focus_handle().tab_stop(true),
            widths: vec![120; headers.len()],
            headers,
            order: (0..rows.len()).collect(),
            rows,
            selected: None,
            selection: BTreeSet::new(),
            anchor: None,
            sort: None,
            resizing: None,
            viewport: Viewport::new(width, 120, HEADER_HEIGHT),
            theme: Theme::default(),
            language: Language::English,
            label: "Table".into(),
        }
    }

    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.viewport.width = width.max(80);
        self.viewport.height = height.max(58);
        if !self.widths.is_empty() {
            let columns = self.widths.len() as i32;
            self.widths
                .fill(((self.viewport.content_width() - 2) / columns).max(30));
        }
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
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    pub fn selection(&self) -> Option<usize> {
        self.selected
    }
    pub fn selected_rows(&self) -> &BTreeSet<usize> {
        &self.selection
    }
    pub fn rows(&self) -> &[Vec<String>] {
        &self.rows
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        cx.notify();
    }
    pub fn set_rows(&mut self, rows: Vec<Vec<String>>, cx: &mut Context<Self>) {
        self.rows = rows;
        self.order = (0..self.rows.len()).collect();
        self.selection.retain(|index| *index < self.rows.len());
        if self.selected.is_some_and(|index| index >= self.rows.len()) {
            self.selected = None;
        }
        if self.anchor.is_some_and(|index| index >= self.rows.len()) {
            self.anchor = None;
        }
        self.apply_sort();
        self.viewport.clamp(self.rows.len());
        cx.notify();
    }

    pub fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        self.select_row(index, false, false, cx);
    }

    fn select_row(&mut self, index: usize, extend: bool, toggle: bool, cx: &mut Context<Self>) {
        if index >= self.rows.len() {
            return;
        }
        if extend && let Some(anchor) = self.anchor {
            let from = self
                .order
                .iter()
                .position(|row| *row == anchor)
                .unwrap_or(0);
            let to = self.order.iter().position(|row| *row == index).unwrap_or(0);
            self.selection = self.order[from.min(to)..=from.max(to)]
                .iter()
                .copied()
                .collect();
        } else if toggle {
            if !self.selection.remove(&index) {
                self.selection.insert(index);
            }
            self.anchor = Some(index);
        } else {
            self.selection.clear();
            self.selection.insert(index);
            self.anchor = Some(index);
        }
        if self.selected != Some(index) {
            self.selected = Some(index);
            cx.emit(TableEvent::Selected(index));
        }
        if let Some(row) = self.order.iter().position(|row| *row == index) {
            self.viewport.reveal(row, self.rows.len());
        }
        cx.notify();
    }

    pub fn sort_by(&mut self, column: usize, descending: bool, cx: &mut Context<Self>) {
        if column >= self.headers.len() {
            return;
        }
        self.sort = Some((column, descending));
        self.apply_sort();
        cx.notify();
    }

    fn apply_sort(&mut self) {
        if let Some((column, descending)) = self.sort {
            self.order.sort_by(|left, right| {
                let left = self.rows[*left]
                    .get(column)
                    .map(String::as_str)
                    .unwrap_or("");
                let right = self.rows[*right]
                    .get(column)
                    .map(String::as_str)
                    .unwrap_or("");
                let order = compare_cells(left, right);
                if descending { order.reverse() } else { order }
            });
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.rows.is_empty() {
            return;
        }
        let modifiers = event.keystroke.modifiers;
        if (modifiers.platform || modifiers.control) && event.keystroke.key == "a" {
            self.selection = (0..self.rows.len()).collect();
            cx.notify();
            cx.stop_propagation();
            return;
        }
        let current = self
            .order
            .iter()
            .position(|row| Some(*row) == self.selected)
            .unwrap_or(0);
        let page = self.viewport.visible();
        let target = match event.keystroke.key.as_str() {
            "up" => current.saturating_sub(1),
            "down" if self.selected.is_none() => 0,
            "down" => (current + 1).min(self.rows.len() - 1),
            "pageup" => current.saturating_sub(page),
            "pagedown" => (current + page).min(self.rows.len() - 1),
            "home" => 0,
            "end" => self.rows.len() - 1,
            "enter" => {
                if let Some(index) = self.selected {
                    cx.emit(TableEvent::Activated(index));
                }
                cx.stop_propagation();
                return;
            }
            _ => return,
        };
        self.select_row(self.order[target], modifiers.shift, false, cx);
        cx.stop_propagation();
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        if self.viewport.press_scroll(event.position, self.rows.len()) {
            cx.notify();
            return;
        }
        let Some((x, y)) = self.viewport.local(event.position) else {
            return;
        };
        if y < HEADER_HEIGHT + 2 {
            let mut left = 2;
            for (column, width) in self.widths.iter().enumerate() {
                if (x - left - width).abs() <= 3 {
                    self.resizing = Some((column, x, *width));
                    return;
                }
                if x >= left && x < left + width {
                    let descending = self
                        .sort
                        .is_some_and(|(sorted, descending)| sorted == column && !descending);
                    self.sort_by(column, descending, cx);
                    return;
                }
                left += width;
            }
        } else if let Some(row) = self.viewport.row_at(y, self.rows.len()) {
            let index = self.order[row];
            self.select_row(
                index,
                event.modifiers.shift,
                event.modifiers.platform || event.modifiers.control,
                cx,
            );
            if event.click_count == 2 {
                cx.emit(TableEvent::Activated(index));
            }
        }
    }

    fn paint(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        let focused = self.focus.is_focused(window);
        let mut pen = Pen::new(self.viewport.width, self.viewport.height);
        pen.language = self.language;
        self.viewport.draw_frame(&mut pen, focused, self.theme);
        for (visible, row) in self
            .order
            .iter()
            .skip(self.viewport.offset)
            .take(self.viewport.visible())
            .enumerate()
        {
            let y = 2 + HEADER_HEIGHT + visible as i32 * ROW_HEIGHT;
            let selected = self.selection.contains(row);
            let mut x = 2;
            for (column, width) in self.widths.iter().enumerate() {
                let width = (*width).min(self.viewport.content_width() - x).max(0);
                if width == 0 {
                    break;
                }
                let sorted = self.sort.is_some_and(|(sorted, _)| sorted == column);
                let face = if selected {
                    if focused {
                        self.theme.accent.dark
                    } else {
                        GRAY_4
                    }
                } else if sorted {
                    GRAY_1
                } else {
                    WHITE
                };
                pen.rect(x, y, width, ROW_HEIGHT, face);
                pen.vline(x + width - 1, y, ROW_HEIGHT, GRAY_3);
                let raw = self.rows[*row]
                    .get(column)
                    .map(String::as_str)
                    .unwrap_or("");
                let text = elide(raw, width - 10, self.language);
                let numeric = raw.trim().parse::<f64>().is_ok();
                let text_x = if numeric {
                    x + width - 5 - Font::Small.measure_in(&text, self.language)
                } else {
                    x + 5
                };
                pen.text(
                    text_x,
                    y + 12,
                    &text,
                    Font::Small,
                    if selected && focused { WHITE } else { BLACK },
                );
                x += width;
            }
        }
        let mut x = 2;
        for (column, width) in self.widths.iter().enumerate() {
            let width = (*width).min(self.viewport.content_width() - x).max(0);
            if width == 0 {
                break;
            }
            let sorted = self.sort.filter(|(sorted, _)| *sorted == column);
            pen.rect(
                x,
                2,
                width,
                HEADER_HEIGHT,
                if sorted.is_some() { GRAY_4 } else { GRAY_2 },
            );
            pen.bevel(x, 2, width, HEADER_HEIGHT - 1, WHITE, GRAY_7);
            pen.hline(x, HEADER_HEIGHT + 1, width, BLACK);
            let title = elide(
                &self.headers[column],
                width - if sorted.is_some() { 22 } else { 10 },
                self.language,
            );
            pen.text(x + 5, 14, &title, Font::Small, BLACK);
            if let Some((_, descending)) = sorted {
                for row in 0..3 {
                    pen.hline(
                        x + width - 10 + row,
                        if descending { 8 + row } else { 10 - row },
                        5 - row * 2,
                        BLACK,
                    );
                }
            }
            x += width;
        }
        self.viewport
            .draw_scrollbar(&mut pen, self.rows.len(), self.theme);
        paint_inside(&pen, bounds, window);
    }
}

impl Focusable for Table {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Table {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let layout = cx.entity();
        let paint = cx.entity();
        let mut x = 2;
        let headers = self
            .headers
            .iter()
            .enumerate()
            .map(|(column, title)| {
                let left = x;
                x += self.widths[column];
                div()
                    .id(("header", column))
                    .role(Role::ColumnHeader)
                    .aria_label(title.clone())
                    .absolute()
                    .left(px(left as f32))
                    .top(px(2.))
                    .w(px(self.widths[column] as f32))
                    .h(px(HEADER_HEIGHT as f32))
            })
            .collect::<Vec<_>>();
        let rows = self
            .order
            .iter()
            .skip(self.viewport.offset)
            .take(self.viewport.visible())
            .enumerate()
            .map(|(visible, index)| {
                div()
                    .id(*index)
                    .role(Role::Row)
                    .aria_label(self.rows[*index].join(", "))
                    .aria_selected(self.selection.contains(index))
                    .absolute()
                    .left(px(2.))
                    .top(px((2 + HEADER_HEIGHT + visible as i32 * ROW_HEIGHT) as f32))
                    .w(px(self.viewport.content_width() as f32))
                    .h(px(ROW_HEIGHT as f32))
            })
            .collect::<Vec<_>>();
        div()
            .id("platinum-table")
            .relative()
            .role(Role::Table)
            .aria_label(self.label.clone())
            .track_focus(&self.focus)
            .tab_stop(true)
            .overflow_hidden()
            .w(px(self.viewport.width as f32))
            .h(px(self.viewport.height as f32))
            .flex_shrink_0()
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if !event.dragging() {
                    return;
                }
                if let Some((column, start, width)) = this.resizing {
                    if let Some((x, _)) = this.viewport.local(event.position) {
                        this.widths[column] = (width + x - start).max(30);
                        cx.notify();
                    }
                } else if this.viewport.drag_scroll(event.position, this.rows.len()) {
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.resizing = None;
                    this.viewport.drag = None;
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.resizing = None;
                    this.viewport.drag = None;
                }),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                this.viewport.scroll(event, this.rows.len());
                cx.stop_propagation();
                cx.notify();
            }))
            .child(
                canvas(
                    move |bounds, _, cx| {
                        layout.update(cx, |this, _| this.viewport.bounds = Some(bounds));
                    },
                    move |bounds, _, window, cx| paint.read(cx).paint(bounds, window),
                )
                .size_full(),
            )
            .children(headers)
            .children(rows)
    }
}

struct Viewport {
    width: i32,
    height: i32,
    header: i32,
    offset: usize,
    bounds: Option<Bounds<Pixels>>,
    drag: Option<i32>,
    wheel: f32,
}

impl Viewport {
    fn new(width: i32, height: i32, header: i32) -> Self {
        Self {
            width,
            height,
            header,
            offset: 0,
            bounds: None,
            drag: None,
            wheel: 0.,
        }
    }
    fn content_width(&self) -> i32 {
        self.width - SCROLL_WIDTH - 2
    }
    fn visible(&self) -> usize {
        ((self.height - self.header - 4) / ROW_HEIGHT).max(1) as usize
    }
    fn max_offset(&self, count: usize) -> usize {
        count.saturating_sub(self.visible())
    }
    fn clamp(&mut self, count: usize) {
        self.offset = self.offset.min(self.max_offset(count));
    }
    fn reveal(&mut self, row: usize, count: usize) {
        if row < self.offset {
            self.offset = row;
        }
        if row >= self.offset + self.visible() {
            self.offset = row + 1 - self.visible();
        }
        self.clamp(count);
    }
    fn local(&self, point: Point<Pixels>) -> Option<(i32, i32)> {
        self.bounds.map(|bounds| {
            (
                f32::from(point.x - bounds.left()).round() as i32,
                f32::from(point.y - bounds.top()).round() as i32,
            )
        })
    }
    fn row_at(&self, y: i32, count: usize) -> Option<usize> {
        if y < self.header + 2 || y >= self.header + 2 + self.visible() as i32 * ROW_HEIGHT {
            return None;
        }
        let row = ((y - self.header - 2) / ROW_HEIGHT) as usize + self.offset;
        (row < count).then_some(row)
    }
    fn thumb(&self, count: usize) -> (i32, i32, i32) {
        let track = (self.height - self.header - 36).max(1);
        let height = ((track as f32 * self.visible() as f32 / count.max(1) as f32).round() as i32)
            .clamp(16.min(track), track);
        let top = if self.max_offset(count) == 0 {
            0
        } else {
            ((track - height) as f32 * self.offset as f32 / self.max_offset(count) as f32).round()
                as i32
        };
        (self.header + 18 + top, height, track)
    }
    fn press_scroll(&mut self, point: Point<Pixels>, count: usize) -> bool {
        let Some((x, y)) = self.local(point) else {
            return false;
        };
        if x < self.content_width() || y < self.header + 2 {
            return false;
        }
        let (top, height, _) = self.thumb(count);
        if y < self.header + 18 {
            self.offset = self.offset.saturating_sub(1);
        } else if y >= self.height - 18 {
            self.offset = (self.offset + 1).min(self.max_offset(count));
        } else if y >= top && y < top + height {
            self.drag = Some(y - top);
        } else if y < top {
            self.offset = self.offset.saturating_sub(self.visible());
        } else {
            self.offset = (self.offset + self.visible()).min(self.max_offset(count));
        }
        true
    }
    fn drag_scroll(&mut self, point: Point<Pixels>, count: usize) -> bool {
        let (Some(grab), Some((_, y))) = (self.drag, self.local(point)) else {
            return false;
        };
        let (_, height, track) = self.thumb(count);
        let travel = track - height;
        if travel > 0 {
            self.offset = (((y - self.header - 18 - grab) as f32 / travel as f32).clamp(0., 1.)
                * self.max_offset(count) as f32)
                .round() as usize;
        }
        true
    }
    fn scroll(&mut self, event: &ScrollWheelEvent, count: usize) {
        self.wheel += f32::from(event.delta.pixel_delta(px(ROW_HEIGHT as f32)).y);
        let rows = (self.wheel / ROW_HEIGHT as f32).trunc() as isize;
        self.wheel -= rows as f32 * ROW_HEIGHT as f32;
        self.offset = self
            .offset
            .saturating_add_signed(-rows)
            .min(self.max_offset(count));
    }
    fn draw_frame(&self, pen: &mut Pen, focused: bool, theme: Theme) {
        pen.rect(0, 0, self.width, self.height, WHITE);
        pen.bevel(0, 0, self.width, self.height, GRAY_7, WHITE);
        pen.frame(1, 1, self.width - 2, self.height - 2, BLACK);
        if focused {
            pen.frame(0, 0, self.width, self.height, theme.accent.dark);
        }
    }
    fn draw_scrollbar(&self, pen: &mut Pen, count: usize, theme: Theme) {
        let x = self.content_width();
        let y = self.header + 2;
        let height = self.height - y - 2;
        let scrollable = self.max_offset(count) > 0;
        pen.rect(
            x,
            y,
            SCROLL_WIDTH,
            height,
            if scrollable { GRAY_4 } else { GRAY_2 },
        );
        pen.vline(x, y, height, BLACK);
        for (top, down) in [(y, false), (self.height - 18, true)] {
            pen.rect(x + 1, top, 14, 16, GRAY_2);
            pen.frame(x, top, 16, 16, BLACK);
            pen.bevel(x + 1, top + 1, 14, 14, WHITE, GRAY_7);
            for row in 0..4 {
                pen.hline(
                    x + 4 + row,
                    top + if down { 6 + row } else { 9 - row },
                    7 - row * 2,
                    if scrollable { BLACK } else { GRAY_6 },
                );
            }
        }
        if scrollable {
            let (top, height, _) = self.thumb(count);
            let face = blend(theme.accent.light, theme.accent.mid);
            pen.rect(x, top, 16, height, face);
            pen.frame(x, top, 16, height, BLACK);
            pen.bevel(x + 1, top + 1, 14, height - 2, WHITE, theme.accent.dark);
            for dy in [-3, -1, 1, 3] {
                pen.hline(x + 4, top + height / 2 + dy, 8, theme.accent.dark);
                pen.hline(x + 4, top + height / 2 + dy + 1, 8, WHITE);
            }
        }
    }
}

fn paint_inside(pen: &Pen, bounds: Bounds<Pixels>, window: &mut Window) {
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        pen.paint(
            point(bounds.origin.x.round(), bounds.origin.y.round()),
            window,
        )
    });
}

fn blend(a: u32, b: u32) -> u32 {
    let channel = |shift: u32| (((a >> shift) & 255u32) + ((b >> shift) & 255u32)) / 2u32;
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

fn draw_disclosure(pen: &mut Pen, x: i32, y: i32, open: bool) {
    for row in 0..5 {
        if open {
            pen.hline(x + row, y + row + 2, 9 - row * 2, BLACK);
        } else {
            pen.vline(x + row + 2, y + row, 9 - row * 2, BLACK);
        }
    }
}

fn draw_file_icon(pen: &mut Pen, x: i32, y: i32, kind: NodeKind, ink: u32) {
    if kind.is_container() {
        pen.rect(x + 1, y, 5, 2, GRAY_4);
        pen.frame(x + 1, y, 5, 3, ink);
        pen.rect(x, y + 2, 13, 9, GRAY_2);
        pen.frame(x, y + 2, 13, 9, ink);
        pen.hline(x + 1, y + 3, 11, WHITE);
    } else {
        pen.rect(x + 2, y, 9, 12, WHITE);
        pen.frame(x + 2, y, 9, 12, ink);
        for line in [4, 6, 8] {
            pen.hline(x + 4, y + line, 5, GRAY_7);
        }
    }
}

fn elide(text: &str, width: i32, language: Language) -> String {
    if Font::Small.measure_in(text, language) <= width {
        return text.into();
    }
    let mut text = text.to_string();
    while !text.is_empty()
        && Font::Small.measure_in(&text, language) + Font::Small.measure("…") > width
    {
        text.pop();
    }
    if width >= Font::Small.measure("…") {
        text.push('…');
    }
    text
}

fn compare_cells(left: &str, right: &str) -> Ordering {
    match (left.trim().parse::<f64>(), right.trim().parse::<f64>()) {
        (Ok(left), Ok(right)) if left.is_finite() && right.is_finite() => left.total_cmp(&right),
        _ => left.to_lowercase().cmp(&right.to_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_columns_sort_by_value_and_other_cells_by_text() {
        assert_eq!(compare_cells("10", "2"), Ordering::Greater);
        assert_eq!(compare_cells("-3.5", "2"), Ordering::Less);
        assert_eq!(compare_cells("apple", "Books"), Ordering::Less);
    }
    #[test]
    fn scrolling_reveals_selection_and_stays_valid_when_rows_shrink() {
        let mut viewport = Viewport::new(200, 84, 0);
        viewport.reveal(9, 20);
        assert_eq!(viewport.offset, 5);
        viewport.reveal(2, 20);
        assert_eq!(viewport.offset, 2);
        viewport.clamp(3);
        assert_eq!(viewport.offset, 0);
    }
}

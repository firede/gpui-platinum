//! Trees and tables with pixel-drawn rows and scroll bars.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, ContentMask, Context, EventEmitter, FocusHandle, Focusable, ImageSource,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Point, Role,
    ScrollWheelEvent, SharedString, Window, canvas, div, img, point, prelude::*, px,
};

use crate::{
    Font, Language, ScrollBar, Theme,
    models::{FileTree, NodeKind, TreeNode},
    paint::{Pen, Rect},
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
    default_icons: bool,
}

impl EventEmitter<TreeEvent> for TreeView {}

impl TreeView {
    pub fn from_nodes(roots: Vec<TreeNode>, cx: &mut Context<Self>) -> Self {
        Self::new(FileTree::new(roots), cx)
    }
    pub fn new(model: FileTree, cx: &mut Context<Self>) -> Self {
        let mut viewport = Viewport::new(240, 120, 0);
        viewport.clamp(model.visible_rows().len());
        Self {
            focus: cx.focus_handle().tab_stop(true),
            model,
            viewport,
            label: "Tree".into(),
            theme: Theme::default(),
            language: Language::English,
            prefix: String::new(),
            typed_at: Instant::now(),
            default_icons: true,
        }
    }

    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.viewport.width = width.max(80);
        self.viewport.height = height.max(40);
        self
    }
    pub fn always_scrollbar(mut self, always: bool) -> Self {
        self.viewport.always_scrollbar = always;
        self
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    /// Shows built-in file and folder pictures for nodes without an explicit icon.
    pub fn default_icons(mut self, show: bool) -> Self {
        self.default_icons = show;
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
    /// Updates the selection without emitting a user selection event.
    pub fn set_selection(&mut self, selected: Option<usize>, cx: &mut Context<Self>) {
        self.model.selected = selected.filter(|id| self.model.find(*id).is_some());
        if let Some(id) = self.model.selected {
            let ancestors = self.model.path(id);
            for ancestor in ancestors.into_iter().take_while(|ancestor| *ancestor != id) {
                if let Some(node) = self.model.find_mut(ancestor) {
                    node.open = true;
                }
            }
            let rows = self.model.visible_rows();
            if let Some(index) = rows.iter().position(|row| row.node.id == id) {
                self.viewport.reveal(index, rows.len());
            }
        }
        cx.notify();
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
        if self.viewport.is_scrollbar(event.position) {
            return;
        }
        let Some((x, y)) = self.viewport.local(event.position) else {
            return;
        };
        let count = self.model.visible_rows().len();
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
        let focused = self.focus.is_focused(window) && !crate::window::content_is_inactive();
        let mut frame = Pen::new(self.viewport.width, self.viewport.height);
        self.viewport.draw_frame(&mut frame, focused, self.theme);
        let mut pen = Pen::new(self.viewport.width, self.viewport.height);
        pen.language = self.language;
        let rows = self.model.visible_rows();
        for (visible, row) in rows
            .iter()
            .skip(self.viewport.offset)
            .take(self.viewport.painted_rows())
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
            let icon_width = row
                .node
                .icon
                .as_ref()
                .map_or_else(|| if self.default_icons { 12 } else { 0 }, tree_icon_width);
            if row.node.icon.is_none() && self.default_icons {
                draw_file_icon(&mut pen, x + 18, y + 2, row.node.kind, ink);
            }
            let title_x = x + 18 + if icon_width > 0 { icon_width + 4 } else { 0 };
            let title = elide(
                &row.node.title,
                self.viewport.content_width() - title_x - 3,
                self.language,
            );
            pen.text(title_x, y + 12, &title, Font::Small, ink);
        }
        pen.clip(Rect {
            x: 2,
            y: 2,
            width: self.viewport.content_width() - 2,
            height: self.viewport.height - 4,
        });
        frame.append(pen, 0, 0);
        paint_inside(&frame, bounds, window);
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
        let scroll_entity = cx.entity();
        let rows = self.model.visible_rows();
        self.viewport.clamp(rows.len());
        let accessible = rows
            .iter()
            .skip(self.viewport.offset)
            .take(self.viewport.painted_rows())
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
                    .when_some(row.node.icon.clone(), |row_element, icon| {
                        row_element.child(
                            div()
                                .absolute()
                                .left(px((row.depth as i32 * 16 + 18) as f32))
                                .top(px(2.))
                                .child(
                                    img(icon.clone())
                                        .w(px(tree_icon_width(&icon) as f32))
                                        .h(px(12.))
                                        .object_fit(gpui::ObjectFit::None),
                                ),
                        )
                    })
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
            .overflow_hidden()
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
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
            .when(self.viewport.shows_scrollbar(), |root| {
                root.child(self.viewport.scrollbar(
                    "tree-scroll",
                    rows.len(),
                    self.theme,
                    move |value, _, cx| {
                        scroll_entity.update(cx, |this, cx| {
                            this.viewport.offset = value.round() as usize;
                            cx.notify();
                        });
                    },
                ))
            })
    }
}

fn tree_icon_width(icon: &ImageSource) -> i32 {
    match icon {
        ImageSource::Render(image) => u32::from(image.size(0).width).min(12) as i32,
        _ => 12,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableEvent {
    Selected(usize),
    SelectionChanged(BTreeSet<usize>),
    Activated(usize),
}

/// The title, initial geometry and behavior of one table column.
#[derive(Clone)]
pub struct TableColumn {
    pub title: String,
    pub width: i32,
    pub right: bool,
    pub fixed: bool,
    icon: Option<Rc<dyn Fn(usize) -> Option<ImageSource>>>,
}

impl TableColumn {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 120,
            right: false,
            fixed: false,
            icon: None,
        }
    }
    pub fn width(mut self, width: i32) -> Self {
        self.width = width.max(30);
        self
    }
    pub fn right(mut self, right: bool) -> Self {
        self.right = right;
        self
    }
    /// Keeps the user from sorting this column; resizing remains available.
    pub fn fixed(mut self, fixed: bool) -> Self {
        self.fixed = fixed;
        self
    }
    /// Supplies an optional 16-pixel image by the row's model index.
    pub fn icon(mut self, icon: impl Fn(usize) -> Option<ImageSource> + 'static) -> Self {
        self.icon = Some(Rc::new(icon));
        self
    }
}

struct TableColumns {
    headers: Vec<String>,
    widths: Vec<i32>,
    options: Vec<TableColumn>,
    automatic_alignment: bool,
    sort: Option<(usize, bool)>,
    resizing: Option<(usize, i32, i32)>,
}

impl TableColumns {
    fn new(headers: Vec<String>) -> Self {
        Self {
            widths: vec![120; headers.len()],
            options: headers.iter().cloned().map(TableColumn::new).collect(),
            automatic_alignment: true,
            headers,
            sort: None,
            resizing: None,
        }
    }

    fn configured(columns: Vec<TableColumn>) -> Self {
        Self {
            headers: columns.iter().map(|column| column.title.clone()).collect(),
            widths: columns.iter().map(|column| column.width.max(30)).collect(),
            options: columns,
            automatic_alignment: false,
            sort: None,
            resizing: None,
        }
    }

    /// Keeps every header and its interactive column geometry in step.
    /// Returns true when a removed sort column requires restoring row order.
    fn set_headers(&mut self, headers: Vec<String>, content_width: i32) -> bool {
        let mut reset_order = false;
        if headers.len() != self.headers.len() {
            self.options
                .resize_with(headers.len(), || TableColumn::new(""));
            self.widths.resize(headers.len(), 120);
            if self.automatic_alignment && !self.widths.is_empty() {
                let width = (content_width / self.widths.len() as i32).max(30);
                self.widths.fill(width);
            }
            self.resizing = None;
            if self.sort.is_some_and(|(column, _)| column >= headers.len()) {
                self.sort = None;
                reset_order = true;
            }
        }
        for (option, title) in self.options.iter_mut().zip(&headers) {
            option.title.clone_from(title);
        }
        self.headers = headers;
        reset_order
    }
}

pub struct Table {
    focus: FocusHandle,
    columns: TableColumns,
    rows: Vec<Vec<String>>,
    order: Vec<usize>,
    comparators: BTreeMap<usize, RowComparator>,
    selected: Option<usize>,
    selection: BTreeSet<usize>,
    anchor: Option<usize>,
    viewport: Viewport,
    theme: Theme,
    language: Language,
    label: SharedString,
}

impl EventEmitter<TableEvent> for Table {}

type RowComparator = Rc<dyn Fn(usize, usize) -> Ordering>;

impl Table {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>, cx: &mut Context<Self>) -> Self {
        let width = (headers.len().max(1) as i32 * 120 + 20).max(200);
        let mut viewport = Viewport::new(width, 120, HEADER_HEIGHT);
        viewport.clamp(rows.len());
        Self {
            focus: cx.focus_handle().tab_stop(true),
            columns: TableColumns::new(headers),
            order: (0..rows.len()).collect(),
            comparators: BTreeMap::new(),
            rows,
            selected: None,
            selection: BTreeSet::new(),
            anchor: None,
            viewport,
            theme: Theme::default(),
            language: Language::English,
            label: "Table".into(),
        }
    }

    pub fn with_columns(
        columns: Vec<TableColumn>,
        rows: Vec<Vec<String>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let headers = columns.iter().map(|column| column.title.clone()).collect();
        let mut table = Self::new(headers, rows, cx);
        table.columns = TableColumns::configured(columns);
        table.viewport.width =
            (table.columns.widths.iter().sum::<i32>() + SCROLL_WIDTH + 4).max(80);
        table
    }

    pub fn set_columns(&mut self, columns: Vec<TableColumn>, cx: &mut Context<Self>) {
        self.columns = TableColumns::configured(columns);
        self.comparators
            .retain(|column, _| *column < self.columns.headers.len());
        self.order = (0..self.rows.len()).collect();
        cx.notify();
    }

    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.viewport.width = width.max(80);
        self.viewport.height = height.max(58);
        if self.columns.automatic_alignment && !self.columns.widths.is_empty() {
            let columns = self.columns.widths.len() as i32;
            self.columns
                .widths
                .fill(((self.viewport.content_width() - 2) / columns).max(30));
        }
        self
    }
    pub fn always_scrollbar(mut self, always: bool) -> Self {
        self.viewport.always_scrollbar = always;
        if self.columns.automatic_alignment && !self.columns.widths.is_empty() {
            let width =
                ((self.viewport.content_width() - 2) / self.columns.widths.len() as i32).max(30);
            self.columns.widths.fill(width);
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
    /// Updates one selected row without emitting a user selection event.
    pub fn set_selection(&mut self, selected: Option<usize>, cx: &mut Context<Self>) {
        self.selected = selected.filter(|index| *index < self.rows.len());
        self.selection = self.selected.into_iter().collect();
        self.anchor = self.selected;
        if let Some(index) = self.selected
            && let Some(row) = self.order.iter().position(|row| *row == index)
        {
            self.viewport.reveal(row, self.rows.len());
        }
        cx.notify();
    }
    /// Updates a multi-selection without emitting user selection events.
    pub fn set_selected_rows(
        &mut self,
        rows: impl IntoIterator<Item = usize>,
        cx: &mut Context<Self>,
    ) {
        self.selection = rows
            .into_iter()
            .filter(|index| *index < self.rows.len())
            .collect();
        self.selected = activation_row(self.selected, &self.selection);
        self.anchor = self.selected;
        if let Some(index) = self.selected
            && let Some(row) = self.order.iter().position(|row| *row == index)
        {
            self.viewport.reveal(row, self.rows.len());
        }
        cx.notify();
    }
    pub fn set_size(&mut self, width: i32, height: i32, cx: &mut Context<Self>) {
        self.viewport.width = width.max(80);
        self.viewport.height = height.max(40);
        self.viewport.clamp(self.rows.len());
        cx.notify();
    }
    pub fn set_headers(&mut self, headers: Vec<String>, cx: &mut Context<Self>) {
        self.comparators.retain(|column, _| *column < headers.len());
        if self
            .columns
            .set_headers(headers, self.viewport.content_width() - 2)
        {
            self.order = (0..self.rows.len()).collect();
        }
        cx.notify();
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
        self.comparators.clear();
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
        let selection = row_selection(
            &self.order,
            &self.selection,
            self.anchor,
            index,
            extend,
            toggle,
        );
        let events = selection_events(self.selected, &self.selection, Some(index), &selection);
        self.selection = selection;
        if !extend || self.anchor.is_none() {
            self.anchor = Some(index);
        }
        self.selected = Some(index);
        for event in events {
            cx.emit(event);
        }
        if let Some(row) = self.order.iter().position(|row| *row == index) {
            self.viewport.reveal(row, self.rows.len());
        }
        cx.notify();
    }

    pub fn sort_by(&mut self, column: usize, descending: bool, cx: &mut Context<Self>) {
        if self
            .columns
            .options
            .get(column)
            .is_none_or(|column| column.fixed)
        {
            return;
        }
        self.columns.sort = Some((column, descending));
        self.apply_sort();
        cx.notify();
    }

    /// Sorts a display column using its underlying row values. Set this after
    /// replacing rows, which clears comparators for the previous data set.
    pub fn set_column_comparator(
        &mut self,
        column: usize,
        compare: impl Fn(usize, usize) -> Ordering + 'static,
        cx: &mut Context<Self>,
    ) {
        if column < self.columns.headers.len() {
            self.comparators.insert(column, Rc::new(compare));
            self.apply_sort();
            cx.notify();
        }
    }

    fn apply_sort(&mut self) {
        if let Some((column, descending)) = self.columns.sort {
            sort_rows(
                &mut self.order,
                &self.rows,
                column,
                descending,
                self.comparators
                    .get(&column)
                    .map(|compare| compare.as_ref()),
            );
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.rows.is_empty() {
            return;
        }
        let modifiers = event.keystroke.modifiers;
        if (modifiers.platform || modifiers.control) && event.keystroke.key == "a" {
            let selection = (0..self.rows.len()).collect();
            let selected = self.selected.or_else(|| self.order.first().copied());
            for event in selection_events(self.selected, &self.selection, selected, &selection) {
                cx.emit(event);
            }
            self.selection = selection;
            if self.selected.is_none() {
                self.selected = selected;
                self.anchor = self.selected;
            }
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
                if let Some(index) = activation_row(self.selected, &self.selection) {
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
        if self.viewport.is_scrollbar(event.position) {
            return;
        }
        let Some((x, y)) = self.viewport.local(event.position) else {
            return;
        };
        if y < HEADER_HEIGHT + 2 {
            let mut left = 2;
            for (column, width) in self.columns.widths.iter().enumerate() {
                if (x - left - width).abs() <= 3 {
                    self.columns.resizing = Some((column, x, *width));
                    return;
                }
                if x >= left && x < left + width && !self.columns.options[column].fixed {
                    let descending = self
                        .columns
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
            if event.click_count == 2
                && let Some(index) = activation_row(self.selected, &self.selection)
            {
                cx.emit(TableEvent::Activated(index));
            }
        }
    }

    fn paint(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        let inactive = crate::window::content_is_inactive();
        let focused = self.focus.is_focused(window) && !inactive;
        let mut frame = Pen::new(self.viewport.width, self.viewport.height);
        self.viewport.draw_frame(&mut frame, focused, self.theme);
        let mut pen = Pen::new(self.viewport.width, self.viewport.height);
        pen.language = self.language;
        for (visible, row) in self
            .order
            .iter()
            .skip(self.viewport.offset)
            .take(self.viewport.painted_rows())
            .enumerate()
        {
            let y = 2 + HEADER_HEIGHT + visible as i32 * ROW_HEIGHT;
            let selected = self.selection.contains(row);
            let mut x = 2;
            for (column, width) in self.columns.widths.iter().enumerate() {
                let width = (*width).min(self.viewport.content_width() - x).max(0);
                if width == 0 {
                    break;
                }
                let sorted = self
                    .columns
                    .sort
                    .is_some_and(|(sorted, _)| sorted == column);
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
                let option = &self.columns.options[column];
                let has_icon = option.icon.as_ref().and_then(|icon| icon(*row)).is_some();
                let icon_width = if has_icon { 20 } else { 0 };
                let text = elide(raw, width - 10 - icon_width, self.language);
                let right = option.right
                    || (self.columns.automatic_alignment && raw.trim().parse::<f64>().is_ok());
                let text_x = if right {
                    x + width - 5 - Font::Small.measure_in(&text, self.language)
                } else {
                    x + 5 + icon_width
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
        pen.set_dimmed(inactive);
        let mut x = 2;
        for (column, width) in self.columns.widths.iter().enumerate() {
            let width = (*width).min(self.viewport.content_width() - x).max(0);
            if width == 0 {
                break;
            }
            let sorted = self.columns.sort.filter(|(sorted, _)| *sorted == column);
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
                &self.columns.headers[column],
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
        pen.clip(Rect {
            x: 2,
            y: 2,
            width: self.viewport.content_width() - 2,
            height: self.viewport.height - 4,
        });
        frame.append(pen, 0, 0);
        if self.viewport.shows_scrollbar() {
            let x = self.viewport.content_width();
            frame.set_dimmed(inactive);
            frame.rect(x, 2, SCROLL_WIDTH, HEADER_HEIGHT, GRAY_2);
            frame.bevel(x, 2, SCROLL_WIDTH, HEADER_HEIGHT - 1, WHITE, GRAY_7);
            frame.hline(x, HEADER_HEIGHT + 1, SCROLL_WIDTH, BLACK);
        }
        paint_inside(&frame, bounds, window);
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
        let scroll_entity = cx.entity();
        let mut x = 2;
        let headers = self
            .columns
            .headers
            .iter()
            .enumerate()
            .map(|(column, title)| {
                let left = x;
                x += self.columns.widths[column];
                div()
                    .id(("header", column))
                    .role(Role::ColumnHeader)
                    .aria_column_index(column + 1)
                    .aria_label(title.clone())
                    .when_some(
                        self.columns.sort.filter(|(sorted, _)| *sorted == column),
                        |header, (_, descending)| {
                            header.aria_value(if descending {
                                "descending"
                            } else {
                                "ascending"
                            })
                        },
                    )
                    .absolute()
                    .left(px(left as f32))
                    .top(px(2.))
                    .w(px(self.columns.widths[column] as f32))
                    .h(px(HEADER_HEIGHT as f32))
            })
            .collect::<Vec<_>>();
        let rows = self
            .order
            .iter()
            .skip(self.viewport.offset)
            .take(self.viewport.painted_rows())
            .enumerate()
            .map(|(visible, index)| {
                div()
                    .id(*index)
                    .role(Role::Row)
                    .aria_selected(self.selection.contains(index))
                    .aria_row_index(visible + self.viewport.offset + 1)
                    .absolute()
                    .left(px(2.))
                    .top(px((2 + HEADER_HEIGHT + visible as i32 * ROW_HEIGHT) as f32))
                    .w(px(self.viewport.content_width() as f32))
                    .h(px(ROW_HEIGHT as f32))
                    .flex()
                    .children(self.columns.headers.iter().enumerate().map(|(column, _)| {
                        let image = self.columns.options[column]
                            .icon
                            .as_ref()
                            .and_then(|icon| icon(*index));
                        div()
                            .id(column)
                            .role(Role::Cell)
                            .aria_column_index(column + 1)
                            .aria_label(self.rows[*index].get(column).cloned().unwrap_or_default())
                            .w(px(self.columns.widths[column] as f32))
                            .h(px(ROW_HEIGHT as f32))
                            .flex_shrink_0()
                            .relative()
                            .when_some(image, |cell, image| {
                                cell.child(
                                    div()
                                        .absolute()
                                        .left(px(3.))
                                        .top_0()
                                        .child(img(image).size(px(16.))),
                                )
                            })
                    }))
            })
            .collect::<Vec<_>>();
        div()
            .id("platinum-table")
            .relative()
            .role(Role::Table)
            .aria_label(self.label.clone())
            .aria_row_count(self.rows.len())
            .aria_column_count(self.columns.headers.len())
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
                if let Some((column, start, width)) = this.columns.resizing
                    && let Some((x, _)) = this.viewport.local(event.position)
                {
                    this.columns.widths[column] = (width + x - start).max(30);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.columns.resizing = None;
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.columns.resizing = None;
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
            .when(self.viewport.shows_scrollbar(), |root| {
                root.child(self.viewport.scrollbar(
                    "table-scroll",
                    self.rows.len(),
                    self.theme,
                    move |value, _, cx| {
                        scroll_entity.update(cx, |this, cx| {
                            this.viewport.offset = value.round() as usize;
                            cx.notify();
                        });
                    },
                ))
            })
    }
}

struct Viewport {
    width: i32,
    height: i32,
    header: i32,
    offset: usize,
    bounds: Option<Bounds<Pixels>>,
    wheel: f32,
    row_count: usize,
    always_scrollbar: bool,
}

impl Viewport {
    fn new(width: i32, height: i32, header: i32) -> Self {
        Self {
            width,
            height,
            header,
            offset: 0,
            bounds: None,
            wheel: 0.,
            row_count: 0,
            always_scrollbar: false,
        }
    }
    fn content_width(&self) -> i32 {
        self.width
            - 2
            - if self.shows_scrollbar() {
                SCROLL_WIDTH
            } else {
                0
            }
    }
    fn shows_scrollbar(&self) -> bool {
        self.always_scrollbar || self.row_count > self.visible()
    }
    fn painted_rows(&self) -> usize {
        let height = (self.height - self.header - 4).max(1);
        ((height + ROW_HEIGHT - 1) / ROW_HEIGHT) as usize
    }
    fn visible(&self) -> usize {
        ((self.height - self.header - 4) / ROW_HEIGHT).max(1) as usize
    }
    fn max_offset(&self, count: usize) -> usize {
        count.saturating_sub(self.visible())
    }
    fn clamp(&mut self, count: usize) {
        self.row_count = count;
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
        if y < self.header + 2 || y >= self.height - 2 {
            return None;
        }
        let row = ((y - self.header - 2) / ROW_HEIGHT) as usize + self.offset;
        (row < count).then_some(row)
    }
    fn is_scrollbar(&self, point: Point<Pixels>) -> bool {
        self.shows_scrollbar()
            && self
                .local(point)
                .is_some_and(|(x, _)| x >= self.content_width())
    }
    fn scrollbar(
        &self,
        id: &'static str,
        count: usize,
        theme: Theme,
        on_change: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        let height = self.height - self.header - 4;
        div()
            .absolute()
            .left(px(self.content_width() as f32))
            .top(px((self.header + 2) as f32))
            .child(
                ScrollBar::new(id, self.offset as f64, self.visible() as f64, count as f64)
                    .tab_stop(false)
                    .length(height)
                    .step(1.)
                    .theme(theme)
                    .on_change(on_change),
            )
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
}

fn paint_inside(pen: &Pen, bounds: Bounds<Pixels>, window: &mut Window) {
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        pen.paint(
            point(bounds.origin.x.round(), bounds.origin.y.round()),
            window,
        )
    });
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

fn selection_events(
    previous_cursor: Option<usize>,
    previous: &BTreeSet<usize>,
    cursor: Option<usize>,
    selection: &BTreeSet<usize>,
) -> Vec<TableEvent> {
    let mut events = Vec::new();
    if previous_cursor != cursor
        && let Some(cursor) = cursor
    {
        events.push(TableEvent::Selected(cursor));
    }
    if previous != selection {
        events.push(TableEvent::SelectionChanged(selection.clone()));
    }
    events
}

fn activation_row(cursor: Option<usize>, selection: &BTreeSet<usize>) -> Option<usize> {
    cursor
        .filter(|index| selection.contains(index))
        .or_else(|| selection.first().copied())
}

fn row_selection(
    order: &[usize],
    previous: &BTreeSet<usize>,
    anchor: Option<usize>,
    index: usize,
    extend: bool,
    toggle: bool,
) -> BTreeSet<usize> {
    if extend && let Some(anchor) = anchor {
        let from = order.iter().position(|row| *row == anchor).unwrap_or(0);
        let to = order.iter().position(|row| *row == index).unwrap_or(0);
        order[from.min(to)..=from.max(to)].iter().copied().collect()
    } else if toggle {
        let mut selection = previous.clone();
        if !selection.remove(&index) {
            selection.insert(index);
        }
        selection
    } else {
        BTreeSet::from([index])
    }
}

fn sort_rows(
    order: &mut [usize],
    rows: &[Vec<String>],
    column: usize,
    descending: bool,
    compare: Option<&dyn Fn(usize, usize) -> Ordering>,
) {
    order.sort_by(|left, right| {
        let order = if let Some(compare) = compare {
            compare(*left, *right)
        } else {
            let left = rows[*left].get(column).map(String::as_str).unwrap_or("");
            let right = rows[*right].get(column).map(String::as_str).unwrap_or("");
            compare_cells(left, right)
        };
        if descending { order.reverse() } else { order }
    });
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
    fn scrollbars_only_reserve_space_when_rows_overflow_or_the_owner_requests_them() {
        let mut viewport = Viewport::new(200, 116, HEADER_HEIGHT);
        viewport.clamp(5);
        assert!(!viewport.shows_scrollbar());
        assert_eq!(viewport.content_width(), 198);
        viewport.clamp(6);
        assert!(viewport.shows_scrollbar());
        assert_eq!(viewport.content_width(), 182);
        viewport.clamp(0);
        viewport.always_scrollbar = true;
        assert!(viewport.shows_scrollbar());
        assert_eq!(viewport.content_width(), 182);
    }

    #[test]
    fn the_last_partial_row_is_painted_and_hit_tested_without_changing_page_navigation() {
        let mut viewport = Viewport::new(200, 116, HEADER_HEIGHT);
        viewport.clamp(10);
        assert_eq!(viewport.visible(), 5);
        assert_eq!(viewport.painted_rows(), 6);
        assert_eq!(viewport.row_at(113, 10), Some(5));
        assert_eq!(viewport.row_at(114, 10), None);
        viewport.reveal(5, 10);
        assert_eq!(viewport.offset, 1);
    }
    #[test]
    fn toggling_the_cursor_row_reports_the_changed_selection_and_prevents_activation() {
        let previous = BTreeSet::from([2]);
        let selection = row_selection(&[0, 1, 2], &previous, Some(2), 2, false, true);
        assert!(selection.is_empty());
        assert_eq!(
            selection_events(Some(2), &previous, Some(2), &selection),
            vec![TableEvent::SelectionChanged(BTreeSet::new())]
        );
        assert_eq!(activation_row(Some(2), &selection), None);
    }

    #[test]
    fn selecting_every_row_reports_the_collection_and_allows_return_without_a_previous_cursor() {
        let selection = BTreeSet::from([0, 1, 2]);
        assert_eq!(
            selection_events(None, &BTreeSet::new(), Some(0), &selection),
            vec![
                TableEvent::Selected(0),
                TableEvent::SelectionChanged(selection.clone())
            ]
        );
        assert_eq!(activation_row(None, &selection), Some(0));
        assert_eq!(activation_row(Some(2), &selection), Some(2));
        assert!(selection_events(Some(0), &selection, Some(0), &selection).is_empty());
    }

    #[test]
    fn shift_selection_uses_the_display_order_after_sorting() {
        let selected = row_selection(&[2, 0, 3, 1], &BTreeSet::from([0]), Some(0), 1, true, false);
        assert_eq!(selected, BTreeSet::from([0, 1, 3]));
    }

    #[test]
    fn explicit_column_configuration_preserves_width_alignment_and_sort_restrictions() {
        let mut columns = TableColumns::configured(vec![
            TableColumn::new("Code").width(80).fixed(true),
            TableColumn::new("Size").width(72).right(true),
        ]);
        assert_eq!(columns.widths, [80, 72]);
        assert!(columns.options[0].fixed);
        assert!(!columns.options[0].right);
        assert!(columns.options[1].right);
        assert!(!columns.automatic_alignment);
        columns.set_headers(vec!["编号".into(), "大小".into()], 400);
        assert_eq!(columns.widths, [80, 72]);
        assert!(columns.options[0].fixed);
        assert!(columns.options[1].right);
        columns.set_headers(vec!["编号".into(), "大小".into(), "类型".into()], 400);
        assert_eq!(columns.widths, [80, 72, 120]);
    }
    #[test]
    fn display_size_columns_sort_by_bytes_with_folders_after_documents() {
        let rows = vec![
            vec!["1 MB".into()],
            vec!["900K".into()],
            vec!["—".into()],
            vec!["84 bytes".into()],
        ];
        let values = [(false, 1 << 20), (false, 900 << 10), (true, 0), (false, 84)];
        let compare = |left: usize, right: usize| values[left].cmp(&values[right]);
        let mut order = vec![0, 1, 2, 3];
        sort_rows(&mut order, &rows, 0, false, Some(&compare));
        assert_eq!(order, [3, 1, 0, 2]);
        sort_rows(&mut order, &rows, 0, true, Some(&compare));
        assert_eq!(order, [2, 0, 1, 3]);
    }
    #[test]
    fn replacing_headers_keeps_column_geometry_and_interaction_valid() {
        let mut columns = TableColumns::new(vec!["Name".into(), "Size".into(), "Kind".into()]);
        columns.sort = Some((2, true));
        columns.resizing = Some((2, 30, 120));
        assert!(columns.set_headers(vec!["Name".into(), "Size".into()], 300));
        assert_eq!(columns.headers.len(), columns.widths.len());
        assert_eq!(columns.widths, [150, 150]);
        assert_eq!(columns.sort, None);
        assert_eq!(columns.resizing, None);
        for count in [0, 4, 1, 3] {
            columns.set_headers((0..count).map(|index| index.to_string()).collect(), 300);
            assert_eq!(columns.headers.len(), columns.widths.len());
            for index in 0..columns.headers.len() {
                assert!(columns.widths[index] >= 30);
            }
        }
        columns.widths[0] = 77;
        columns.sort = Some((0, false));
        assert!(!columns.set_headers(vec!["名称".into(), "大小".into(), "类型".into()], 300));
        assert_eq!(columns.widths[0], 77);
        assert_eq!(columns.sort, Some((0, false)));
    }
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

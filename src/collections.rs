//! Trees and tables with pixel-drawn rows and scroll bars.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, Context, EventEmitter, FocusHandle, Focusable, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, Pixels, Point, Role, ScrollEvent, SharedString, Window, canvas,
    div, point, prelude::*, px,
};

use crate::{
    Accent, Font, Language, ScrollBar, Theme,
    list::text_middle,
    models::{FileTree, NodeKind, TreeNode},
    paint::{Pen, Rect},
    theme::*,
};

const ROW_HEIGHT: i32 = 16;
const HEADER_HEIGHT: i32 = 18;
const SCROLL_WIDTH: i32 = 16;
/// The room at either side of a table cell's text.
const CELL_PAD: i32 = 5;
/// How near a column's edge the pointer resizes it.
const RESIZE_GRIP: i32 = 3;
/// How far each level of a tree view is indented.
const TREE_INDENT: i32 = 16;
const TREE_TRIANGLE_LEFT: i32 = 3;
const DISCLOSURE_SIZE: i32 = 12;
const SORT_UP: [&str; 3] = ["..#..", ".###.", "#####"];
const SORT_DOWN: [&str; 3] = ["#####", ".###.", "..#.."];

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
            typed_at: cx.background_executor().now(),
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
                let now = cx.background_executor().now();
                if now.duration_since(self.typed_at) > Duration::from_secs(1) {
                    self.prefix.clear();
                }
                self.prefix.push_str(text);
                self.typed_at = now;
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
        // A click on a triangle opens or closes its item, and chooses none.
        let triangle = 2 + TREE_TRIANGLE_LEFT + depth as i32 * TREE_INDENT;
        if children && x >= triangle - 2 && x < triangle + DISCLOSURE_SIZE + 2 {
            self.model.toggle(id);
            self.viewport.clamp(self.model.visible_rows().len());
            cx.notify();
            return;
        }
        let previous = self.model.selected;
        self.model.select(id);
        if event.click_count == 2 {
            cx.emit(TreeEvent::Activated(id));
        }
        self.changed(previous, cx);
    }

    fn paint(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        let inactive = crate::window::content_is_inactive();
        let focused = self.focus.is_focused(window) && !inactive;
        let (width, height) = (self.viewport.width, self.viewport.height);
        let accent = self.theme.accent;
        let mut frame = Pen::new(width, height);
        frame.bevel(0, 0, width, height, GRAY_7, WHITE);
        let list_width = self.viewport.content_width();
        let mut list = Pen::new(list_width, height - 2);
        list.language = self.language;
        list.rect(1, 1, list_width - 2, height - 4, WHITE);
        list.frame(0, 0, list_width, height - 2, BLACK);
        let mut pen = Pen::new(list_width, height - 2);
        pen.language = self.language;
        let rows = self.model.visible_rows();
        for (visible, row) in rows.iter().skip(self.viewport.offset).enumerate() {
            let y = 1 + visible as i32 * ROW_HEIGHT;
            if y >= height - 3 {
                break;
            }
            let x = 1 + TREE_TRIANGLE_LEFT + row.depth as i32 * TREE_INDENT;
            let selected = self.model.selected == Some(row.node.id);
            let mut ink = BLACK;
            if selected {
                let mut face = GRAY_4;
                if focused {
                    (face, ink) = (accent.dark, WHITE);
                }
                pen.rect(
                    x + DISCLOSURE_SIZE + 2,
                    y,
                    list_width - x - DISCLOSURE_SIZE - 3,
                    ROW_HEIGHT,
                    face,
                );
            }
            if !row.node.children.is_empty() {
                let mut triangle = Pen::new(DISCLOSURE_SIZE, DISCLOSURE_SIZE);
                crate::widgets::draw_disclosure(&mut triangle, row.node.open, false, false);
                pen.append(triangle, x, y + (ROW_HEIGHT - DISCLOSURE_SIZE) / 2);
            }
            let mut title_x = x + DISCLOSURE_SIZE + 5;
            if let Some(icon) = &row.node.icon {
                draw_icon(
                    &mut pen,
                    title_x,
                    y + (ROW_HEIGHT - icon.len() as i32) / 2,
                    icon,
                    accent,
                    selected && focused,
                );
                title_x += icon_width(icon) + 4;
            } else if self.default_icons {
                draw_file_icon(&mut pen, title_x, y + 2, row.node.kind, ink);
                title_x += 12 + 4;
            }
            let title =
                Font::Small.truncate_in(&row.node.title, list_width - title_x - 4, self.language);
            text_middle(&mut pen, title_x, y, ROW_HEIGHT, &title, Font::Small, ink);
        }
        pen.clip(Rect {
            x: 1,
            y: 1,
            width: list_width - 2,
            height: height - 4,
        });
        list.append(pen, 0, 0);
        frame.set_dimmed(inactive);
        frame.append(list, 1, 1);
        if focused {
            crate::widgets::draw_focus_ring(&mut frame, 0, accent.dark);
        }
        paint_at_origin(&frame, bounds, window);
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
                    .w(px((self.viewport.content_width() - 2) as f32))
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
            .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| {
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
    icon: Option<RowIcon>,
}

type RowIcon = Rc<dyn Fn(usize) -> Option<Vec<String>>>;

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
    /// Gives each row, by its model index, a small picture at the left of
    /// its cell: at most 16×16, as rows of '#' (black), 'o' (the accent's
    /// dark color), '+' (its mid color), ':' (gray) and 'w' (white).
    pub fn icon<I, S>(mut self, icon: impl Fn(usize) -> Option<I> + 'static) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.icon = Some(Rc::new(move |row| {
            icon(row).map(|rows| {
                rows.into_iter()
                    .map(|row| row.as_ref().to_owned())
                    .collect()
            })
        }));
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
    focus_ring: bool,
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
            focus_ring: true,
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
    /// Rings the table while it has the keyboard, as it does unless turned
    /// off: classic list views showed none, their window telling where
    /// typing goes.
    pub fn focus_ring(mut self, ring: bool) -> Self {
        self.focus_ring = ring;
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
        if y < HEADER_HEIGHT + 1 {
            // An edge resizes its column; elsewhere a header sorts by its.
            let mut right = 2;
            for (column, width) in self.columns.widths.iter().enumerate() {
                right += width;
                if (x - right).abs() <= RESIZE_GRIP {
                    self.columns.resizing = Some((column, x, *width));
                    return;
                }
            }
            let mut left = 2;
            for (column, width) in self.columns.widths.iter().enumerate() {
                if x >= left && x < left + width {
                    if !self.columns.options[column].fixed {
                        let descending = self
                            .columns
                            .sort
                            .is_some_and(|(sorted, descending)| sorted == column && !descending);
                        self.sort_by(column, descending, cx);
                    }
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
        let (width, height) = (self.viewport.width, self.viewport.height);
        let mut frame = Pen::new(width, height);
        frame.bevel(0, 0, width, height, GRAY_7, WHITE);
        let list_width = self.viewport.content_width();
        let mut table = Pen::new(list_width, height - 2);
        table.language = self.language;
        self.draw_table(&mut table, focused);
        if inactive {
            // In the background, its headers gray, the cells as they are.
            let mut headers = Pen::new(list_width, height - 2);
            headers.language = self.language;
            headers.set_dimmed(true);
            self.draw_table(&mut headers, focused);
            headers.clip(Rect {
                x: 1,
                y: 1,
                width: list_width - 2,
                height: HEADER_HEIGHT - 1,
            });
            table.append(headers, 0, 0);
        }
        frame.append(table, 1, 1);
        if self.viewport.shows_scrollbar() {
            // The corner over the scroll bar, a header of its own.
            let mut corner = Pen::new(SCROLL_WIDTH, HEADER_HEIGHT);
            corner.set_dimmed(inactive);
            corner.rect(0, 0, SCROLL_WIDTH, HEADER_HEIGHT, GRAY_2);
            corner.frame(0, 0, SCROLL_WIDTH, HEADER_HEIGHT, BLACK);
            corner.bevel(1, 1, SCROLL_WIDTH - 2, HEADER_HEIGHT - 2, WHITE, GRAY_7);
            frame.append(corner, list_width, 1);
        }
        if focused && self.focus_ring {
            crate::widgets::draw_focus_ring(&mut frame, 0, self.theme.accent.dark);
        }
        paint_at_origin(&frame, bounds, window);
    }

    /// Draws the table in the pen's box: its headers, and its rows from the
    /// first in view, as the original's DrawTable does.
    fn draw_table(&self, pen: &mut Pen, focused: bool) {
        let (width, height) = (pen.width, pen.height);
        let language = self.language;
        let accent = self.theme.accent;
        let widths = &self.columns.widths;
        let sorted_column = self
            .columns
            .sort
            .map(|(column, _)| column)
            .filter(|column| self.columns.options.get(*column).is_some_and(|c| !c.fixed));
        pen.rect(1, 1, width - 2, height - 2, WHITE);
        pen.frame(0, 0, width, height, BLACK);
        let mut inner = Pen::new(width, height);
        inner.language = language;
        // The column the rows are sorted by, shaded all the way down.
        let mut x = 1;
        for (column, column_width) in widths.iter().enumerate() {
            if sorted_column == Some(column) {
                inner.rect(
                    x,
                    HEADER_HEIGHT,
                    *column_width,
                    height - 1 - HEADER_HEIGHT,
                    GRAY_1,
                );
            }
            x += column_width;
        }
        for (visible, row) in self.order.iter().skip(self.viewport.offset).enumerate() {
            let y = HEADER_HEIGHT + visible as i32 * ROW_HEIGHT;
            if y >= height - 1 {
                break;
            }
            let selected = self.selection.contains(row);
            let mut ink = BLACK;
            if selected && focused {
                inner.rect(1, y, width - 2, ROW_HEIGHT, accent.dark);
                ink = WHITE;
            } else if selected {
                inner.rect(1, y, width - 2, ROW_HEIGHT, GRAY_4);
            }
            let mut x = 1;
            for (column, &column_width) in widths.iter().enumerate() {
                if !selected && sorted_column == Some(column) {
                    inner.rect(x, y, column_width, ROW_HEIGHT, GRAY_1);
                }
                let option = &self.columns.options[column];
                let raw = self.rows[*row]
                    .get(column)
                    .map(String::as_str)
                    .unwrap_or("");
                let mut text_x = x + CELL_PAD;
                if let Some(icon) = &option.icon {
                    if let Some(rows) = icon(*row) {
                        draw_icon(
                            &mut inner,
                            x + 4,
                            y + (ROW_HEIGHT - rows.len() as i32) / 2,
                            &rows,
                            accent,
                            selected && focused,
                        );
                    }
                    text_x = x + 4 + 16 + 4;
                }
                let text =
                    Font::Small.truncate_in(raw, column_width - (text_x - x) - CELL_PAD, language);
                let right = option.right
                    || (self.columns.automatic_alignment && raw.trim().parse::<f64>().is_ok());
                if right {
                    text_x = x + column_width - CELL_PAD - Font::Small.measure_in(&text, language);
                }
                let mut cell = Pen::new(width, height);
                cell.language = language;
                text_middle(&mut cell, text_x, y, ROW_HEIGHT, &text, Font::Small, ink);
                cell.clip(Rect {
                    x,
                    y,
                    width: column_width,
                    height: ROW_HEIGHT,
                });
                inner.append(cell, 0, 0);
                x += column_width;
                if !selected {
                    inner.vline(x - 1, y, ROW_HEIGHT, GRAY_3);
                }
            }
        }
        // The headers.
        let mut x = 1;
        for (column, &column_width) in widths.iter().enumerate() {
            let sorted = sorted_column == Some(column);
            let header_height = HEADER_HEIGHT - 1;
            let mut header = Pen::new(column_width, header_height);
            header.language = language;
            header.rect(
                0,
                0,
                column_width,
                header_height,
                if sorted { GRAY_4 } else { GRAY_2 },
            );
            header.bevel(0, 0, column_width, header_height - 1, WHITE, GRAY_7);
            header.hline(0, header_height - 1, column_width, BLACK);
            header.vline(column_width - 1, 0, header_height, GRAY_7);
            let title = Font::Small.truncate_in(
                &self.columns.headers[column],
                column_width - 2 * CELL_PAD - 8,
                language,
            );
            text_middle(
                &mut header,
                CELL_PAD,
                0,
                header_height - 1,
                &title,
                Font::Small,
                BLACK,
            );
            if sorted {
                let descending = self.columns.sort.is_some_and(|(_, descending)| descending);
                draw_bitmap(
                    &mut header,
                    column_width - CELL_PAD - 5,
                    (header_height - 4) / 2,
                    if descending { &SORT_DOWN } else { &SORT_UP },
                    BLACK,
                );
            }
            inner.append(header, x, 1);
            x += column_width;
        }
        if x < width - 1 {
            inner.rect(x, 1, width - 1 - x, HEADER_HEIGHT - 1, GRAY_2);
            inner.hline(x, HEADER_HEIGHT - 1, width - 1 - x, BLACK);
        }
        inner.clip(Rect {
            x: 1,
            y: 1,
            width: width - 2,
            height: height - 2,
        });
        pen.append(inner, 0, 0);
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
        let content_width = self.viewport.content_width();
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
                    .w(px(
                        self.columns.widths[column].min(content_width - left).max(0) as f32,
                    ))
                    .h(px((HEADER_HEIGHT - 1) as f32))
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
                    .top(px((1 + HEADER_HEIGHT + visible as i32 * ROW_HEIGHT) as f32))
                    .w(px((content_width - 2) as f32))
                    .h(px(ROW_HEIGHT as f32))
                    .overflow_hidden()
                    .flex()
                    .children(self.columns.headers.iter().enumerate().map(|(column, _)| {
                        div()
                            .id(column)
                            .role(Role::Cell)
                            .aria_column_index(column + 1)
                            .aria_label(self.rows[*index].get(column).cloned().unwrap_or_default())
                            .w(px(self.columns.widths[column] as f32))
                            .h(px(ROW_HEIGHT as f32))
                            .flex_shrink_0()
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
            .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| {
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
    /// The width of the framed rows, inside the box's ring, which shares
    /// its right line with the scroll bar's left one.
    fn content_width(&self) -> i32 {
        self.width
            - 2
            - if self.shows_scrollbar() {
                SCROLL_WIDTH - 1
            } else {
                0
            }
    }
    /// Whether the rows do not all show, so the scroll bar is active.
    fn overflows(&self) -> bool {
        self.row_count > self.visible()
    }
    fn shows_scrollbar(&self) -> bool {
        self.always_scrollbar || self.overflows()
    }
    /// The rows that may show, the last perhaps in part.
    fn painted_rows(&self) -> usize {
        let height = (self.height - 4 - self.rows_top()).max(1);
        ((height + ROW_HEIGHT - 1) / ROW_HEIGHT) as usize
    }
    /// The rows that show whole.
    fn visible(&self) -> usize {
        if self.header > 0 {
            ((self.height - self.header - 3) / ROW_HEIGHT).max(1) as usize
        } else {
            ((self.height - 4) / ROW_HEIGHT).max(1) as usize
        }
    }
    /// Where the first row starts, below the headers, from the box's top.
    fn rows_top(&self) -> i32 {
        if self.header > 0 { self.header + 1 } else { 2 }
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
        if y < self.rows_top() || y >= self.height - 2 {
            return None;
        }
        let row = ((y - self.rows_top()) / ROW_HEIGHT) as usize + self.offset;
        (row < count).then_some(row)
    }
    fn is_scrollbar(&self, point: Point<Pixels>) -> bool {
        self.shows_scrollbar()
            && self
                .local(point)
                .is_some_and(|(x, _)| x >= self.width - SCROLL_WIDTH)
    }
    fn scrollbar(
        &self,
        id: &'static str,
        count: usize,
        theme: Theme,
        on_change: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        // Below the headers' corner, which overlaps its top line.
        let top = 1 + (self.header - 1).max(0);
        let visible = self.visible();
        div()
            .absolute()
            .left(px((self.width - 1 - SCROLL_WIDTH) as f32))
            .top(px(top as f32))
            .child(
                ScrollBar::new(
                    id,
                    self.offset as f64,
                    visible as f64,
                    count.max(visible) as f64,
                )
                .tab_stop(false)
                .length(self.height - 1 - top)
                .step(1.)
                .inactive(!self.overflows())
                .theme(theme)
                .on_change(on_change),
            )
    }
    fn scroll(&mut self, event: &ScrollEvent, count: usize) {
        self.wheel += f32::from(event.delta.pixel_delta(px(ROW_HEIGHT as f32)).y);
        let rows = (self.wheel / ROW_HEIGHT as f32).trunc() as isize;
        self.wheel -= rows as f32 * ROW_HEIGHT as f32;
        self.offset = self
            .offset
            .saturating_add_signed(-rows)
            .min(self.max_offset(count));
    }
}

fn paint_at_origin(pen: &Pen, bounds: Bounds<Pixels>, window: &mut Window) {
    pen.paint(
        point(bounds.origin.x.round(), bounds.origin.y.round()),
        window,
    );
}

/// Draws a picture of '#' rows in one color, as the original's bitmaps.
pub(crate) fn draw_bitmap(pen: &mut Pen, x: i32, y: i32, rows: &[&str], color: u32) {
    for (dy, row) in rows.iter().enumerate() {
        for (dx, pixel) in row.bytes().enumerate() {
            if pixel == b'#' {
                pen.dot(x + dx as i32, y + dy as i32, color);
            }
        }
    }
}

fn icon_width(rows: &[String]) -> i32 {
    rows.iter().map(|row| row.len()).max().unwrap_or(0) as i32
}

fn draw_icon(pen: &mut Pen, x: i32, y: i32, rows: &[String], accent: Accent, inverted: bool) {
    let rows = rows.iter().map(String::as_str).collect::<Vec<_>>();
    crate::bitmap_icons::draw_bitmap_icon(pen, &rows, x, y, accent, inverted, false);
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
        assert_eq!(viewport.content_width(), 183);
        viewport.clamp(0);
        viewport.always_scrollbar = true;
        assert!(viewport.shows_scrollbar());
        assert_eq!(viewport.content_width(), 183);
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

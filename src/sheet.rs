//! An editable sheet with a formula bar and a rectangular selection.

use gpui::{
    AccessibleAction, App, Bounds, ClipboardItem, ContentMask, Context, Entity, EventEmitter,
    FocusHandle, Focusable, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels,
    Point, Role, ScrollWheelEvent, SharedString, Subscription, Window, canvas, div, point,
    prelude::*, px,
};

use crate::{
    Font, InputEvent, Language, ScrollBar, TextField, Theme,
    models::{Cell, Sheet},
    paint::{Pen, drawing},
    theme::*,
    window::GROW_BOX_SIZE,
};

const CELL_WIDTH: i32 = 72;
const CELL_HEIGHT: i32 = 18;
const ROW_HEADER: i32 = 32;
const FORMULA_HEIGHT: i32 = 30;
const SCROLLBAR: i32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetEvent {
    Changed(Cell),
    Selected(Cell),
}

/// The formula bar and grid share one model and native text-editing fields.
pub struct SheetView {
    focus: FocusHandle,
    model: Sheet,
    formula: Entity<TextField>,
    editor: Entity<TextField>,
    _subscriptions: Vec<Subscription>,
    blur_subscription: Option<Subscription>,
    width: i32,
    height: i32,
    rows: usize,
    cols: usize,
    top: usize,
    left: usize,
    formula_bar: bool,
    fills_window: bool,
    cell_editing: bool,
    dragging: bool,
    focus_after_edit: bool,
    grid_bounds: Option<Bounds<Pixels>>,
    theme: Theme,
    language: Language,
    label: SharedString,
    configured: Option<(i32, Theme, Language)>,
}

impl EventEmitter<SheetEvent> for SheetView {}

impl SheetView {
    pub fn new(model: Sheet, cx: &mut Context<Self>) -> Self {
        let initial = model.raw(model.cursor).to_string();
        let formula = cx.new(|cx| {
            TextField::new(cx)
                .with_text(initial)
                .label("Formula")
                .size(292., 22.)
        });
        let editor = cx.new(|cx| {
            TextField::new(cx)
                .label("Cell")
                .size(CELL_WIDTH as f32, 22.)
        });
        let formula_subscription = cx.subscribe(&formula, |this, field, event, cx| {
            this.input_event(field, *event, false, cx);
        });
        let editor_subscription = cx.subscribe(&editor, |this, field, event, cx| {
            this.input_event(field, *event, true, cx);
        });
        Self {
            focus: cx.focus_handle().tab_stop(true),
            model,
            formula,
            editor,
            _subscriptions: vec![formula_subscription, editor_subscription],
            blur_subscription: None,
            width: 360,
            height: 240,
            rows: 99,
            cols: 26,
            top: 0,
            left: 0,
            formula_bar: true,
            fills_window: false,
            cell_editing: false,
            dragging: false,
            focus_after_edit: false,
            grid_bounds: None,
            theme: Theme::default(),
            language: Language::English,
            label: "Sheet".into(),
            configured: None,
        }
    }

    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width.max(150);
        self.height = height.max(100);
        self
    }
    pub fn set_size(&mut self, width: i32, height: i32, cx: &mut Context<Self>) {
        self.width = width.max(150);
        self.height = height.max(100);
        self.reveal();
        cx.notify();
    }
    pub fn command(&mut self, command: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.formula.focus_handle(cx).is_focused(window)
            || self.editor.focus_handle(cx).is_focused(window)
        {
            match command {
                "copy" => window.dispatch_action(Box::new(crate::text_input::Copy), cx),
                "cut" => window.dispatch_action(Box::new(crate::text_input::Cut), cx),
                "paste" => window.dispatch_action(Box::new(crate::text_input::Paste), cx),
                "selectAll" | "select-all" => {
                    window.dispatch_action(Box::new(crate::text_input::SelectAll), cx)
                }
                "clear" => window.dispatch_action(Box::new(crate::text_input::Delete), cx),
                "undo" => window.dispatch_action(Box::new(crate::text_input::Undo), cx),
                "redo" => window.dispatch_action(Box::new(crate::text_input::Redo), cx),
                _ => {}
            }
            return;
        }
        self.finish_edit(cx);
        match command {
            "selectAll" | "select-all" => {
                self.model.anchor = Cell::new(0, 0);
                self.model.cursor = Cell::new(self.cols - 1, self.rows - 1);
                cx.notify();
            }
            "copy" | "cut" => {
                cx.write_to_clipboard(ClipboardItem::new_string(self.model.copy_tsv()));
                if command == "cut" {
                    self.clear_selection(cx);
                }
            }
            "paste" => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    let changed = self.model.paste_tsv(&text, self.rows, self.cols);
                    for cell in changed {
                        cx.emit(SheetEvent::Changed(cell));
                    }
                    self.reveal();
                    self.sync_formula(cx);
                    cx.emit(SheetEvent::Selected(self.model.cursor));
                    cx.notify();
                }
            }
            "clear" => self.clear_selection(cx),
            _ => return,
        }
        window.focus(&self.focus, cx);
    }
    pub fn dimensions(mut self, rows: usize, cols: usize) -> Self {
        self.rows = rows.max(1);
        self.cols = cols.max(1);
        self.model.cursor.col = self.model.cursor.col.min(self.cols - 1);
        self.model.cursor.row = self.model.cursor.row.min(self.rows - 1);
        self.model.anchor.col = self.model.anchor.col.min(self.cols - 1);
        self.model.anchor.row = self.model.anchor.row.min(self.rows - 1);
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
    pub fn formula_bar(mut self, show: bool) -> Self {
        self.formula_bar = show;
        self
    }
    /// Lays a sheet that fills its window on the window's lines: the grid's
    /// outline on them and on the resize corner's top edge, its vertical
    /// scroll bar sharing that outline down to the corner, and its
    /// horizontal one over the window's lines at the left and bottom, its
    /// end at the corner's edge.
    pub fn fill_window(mut self, fill: bool) -> Self {
        self.fills_window = fill;
        self
    }
    pub fn model(&self) -> &Sheet {
        &self.model
    }
    pub fn model_mut(&mut self) -> &mut Sheet {
        &mut self.model
    }
    pub fn selection(&self) -> Cell {
        self.model.cursor
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        self.formula
            .update(cx, |field, cx| field.set_theme(theme, cx));
        self.editor
            .update(cx, |field, cx| field.set_theme(theme, cx));
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        self.formula
            .update(cx, |field, cx| field.set_language(language, cx));
        self.editor
            .update(cx, |field, cx| field.set_language(language, cx));
        cx.notify();
    }

    pub fn select(&mut self, cell: Cell, extend: bool, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        let cell = Cell::new(cell.col.min(self.cols - 1), cell.row.min(self.rows - 1));
        let previous = self.model.cursor;
        self.model.select(cell, extend);
        self.reveal();
        self.sync_formula(cx);
        if previous != cell {
            cx.emit(SheetEvent::Selected(cell));
        }
        cx.notify();
    }

    fn grid_width(&self) -> i32 {
        if self.fills_window {
            // From under the window's line at the left to the scroll bar's
            // outline, the bar's right one on the window's line.
            self.width + 3 - SCROLLBAR + 2
        } else {
            self.width - SCROLLBAR
        }
    }
    fn grid_height(&self) -> i32 {
        let formula = if self.formula_bar { FORMULA_HEIGHT } else { 0 };
        if self.fills_window {
            // The outline on the resize corner's top edge.
            self.height - formula - GROW_BOX_SIZE + 2
        } else {
            self.height - SCROLLBAR - formula
        }
    }
    fn visible_cols(&self) -> usize {
        ((self.grid_width() - ROW_HEADER - 4) / CELL_WIDTH).max(1) as usize
    }
    fn visible_rows(&self) -> usize {
        ((self.grid_height() - CELL_HEIGHT - 4) / CELL_HEIGHT).max(1) as usize
    }
    fn reveal(&mut self) {
        if self.model.cursor.col < self.left {
            self.left = self.model.cursor.col;
        }
        if self.model.cursor.col >= self.left + self.visible_cols() {
            self.left = self.model.cursor.col + 1 - self.visible_cols();
        }
        if self.model.cursor.row < self.top {
            self.top = self.model.cursor.row;
        }
        if self.model.cursor.row >= self.top + self.visible_rows() {
            self.top = self.model.cursor.row + 1 - self.visible_rows();
        }
        self.left = self.left.min(self.cols.saturating_sub(self.visible_cols()));
        self.top = self.top.min(self.rows.saturating_sub(self.visible_rows()));
    }

    fn input_event(
        &mut self,
        field: Entity<TextField>,
        event: InputEvent,
        in_cell: bool,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Changed => {
                let value = field.read(cx).value().to_string();
                self.model.begin_edit(value.clone());
                if in_cell {
                    self.formula
                        .update(cx, |field, cx| field.set_value(value, cx));
                } else if self.cell_editing {
                    self.editor
                        .update(cx, |field, cx| field.set_value(value, cx));
                }
                cx.notify();
            }
            InputEvent::Submitted => {
                self.finish_edit(cx);
                self.move_cursor(0, 1, false, cx);
                self.focus_after_edit = true;
                cx.notify();
            }
        }
    }

    fn sync_formula(&mut self, cx: &mut Context<Self>) {
        let value = self
            .model
            .editing
            .as_deref()
            .unwrap_or_else(|| self.model.raw(self.model.cursor))
            .to_string();
        self.formula
            .update(cx, |field, cx| field.set_value(value, cx));
    }

    fn start_edit(
        &mut self,
        text: String,
        select_all: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.model.begin_edit(text.clone());
        self.cell_editing = true;
        self.dragging = false;
        self.editor.update(cx, |field, cx| {
            field.set_value(text.clone(), cx);
            field.select(if select_all { 0 } else { text.len() }, text.len(), cx);
        });
        self.sync_formula(cx);
        let focus = self.editor.focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    fn finish_edit(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.model.editing.take() {
            let cell = self.model.cursor;
            if self.model.raw(cell) != text {
                self.model.set(cell, text);
                cx.emit(SheetEvent::Changed(cell));
            }
        }
        self.cell_editing = false;
    }

    fn cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.model.cancel_edit();
        self.cell_editing = false;
        self.sync_formula(cx);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn move_cursor(&mut self, col: i32, row: i32, extend: bool, cx: &mut Context<Self>) {
        let cell = Cell::new(
            self.model.cursor.col.saturating_add_signed(col as isize),
            self.model.cursor.row.saturating_add_signed(row as isize),
        );
        self.select(cell, extend, cx);
    }

    fn capture_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let formula_focused = self.formula.focus_handle(cx).is_focused(window);
        let editor_focused = self.editor.focus_handle(cx).is_focused(window);
        if !formula_focused && !editor_focused {
            return;
        }
        match event.keystroke.key.as_str() {
            "escape" => self.cancel_edit(window, cx),
            "tab" if editor_focused => self.tab(event.keystroke.modifiers.shift, window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    fn tab(&mut self, backwards: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        window.focus(&self.focus, cx);
        let at_edge = if backwards {
            self.model.cursor.col == 0
        } else {
            self.model.cursor.col + 1 == self.cols
        };
        if at_edge {
            // Wait until the cell editor has left the tab order.
            cx.on_next_frame(window, move |this, window, cx| {
                window.focus(&this.focus, cx);
                if backwards {
                    window.focus_prev(cx);
                } else {
                    window.focus_next(cx);
                }
            });
            cx.notify();
        } else {
            self.move_cursor(if backwards { -1 } else { 1 }, 0, false, cx);
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(window) {
            return;
        }
        let modifiers = event.keystroke.modifiers;
        let command = modifiers.platform || modifiers.control;
        if command {
            let command = match event.keystroke.key.as_str() {
                "a" => "selectAll",
                "c" => "copy",
                "x" => "cut",
                "v" => "paste",
                _ => return,
            };
            self.command(command, window, cx);
            cx.stop_propagation();
            return;
        }
        match event.keystroke.key.as_str() {
            "left" => self.move_cursor(-1, 0, modifiers.shift, cx),
            "right" => self.move_cursor(1, 0, modifiers.shift, cx),
            "up" => self.move_cursor(0, -1, modifiers.shift, cx),
            "down" => self.move_cursor(0, 1, modifiers.shift, cx),
            "pageup" => self.move_cursor(0, -(self.visible_rows() as i32), modifiers.shift, cx),
            "pagedown" => self.move_cursor(0, self.visible_rows() as i32, modifiers.shift, cx),
            "home" => self.select(Cell::new(0, self.model.cursor.row), modifiers.shift, cx),
            "end" => self.select(
                Cell::new(self.cols - 1, self.model.cursor.row),
                modifiers.shift,
                cx,
            ),
            "tab" => self.tab(modifiers.shift, window, cx),
            "enter" => self.move_cursor(0, if modifiers.shift { -1 } else { 1 }, false, cx),
            "backspace" | "delete" => self.clear_selection(cx),
            "f2" => self.start_edit(self.model.raw(self.model.cursor).into(), true, window, cx),
            "escape" => self.cancel_edit(window, cx),
            _ => {
                let Some(text) = &event.keystroke.key_char else {
                    return;
                };
                if text.chars().any(char::is_control) {
                    return;
                }
                self.start_edit(text.clone(), false, window, cx);
            }
        }
        cx.stop_propagation();
    }

    fn clear_selection(&mut self, cx: &mut Context<Self>) {
        let removed = self
            .model
            .cells
            .keys()
            .copied()
            .filter(|cell| self.model.selected(*cell))
            .collect::<Vec<_>>();
        self.model.clear_selection();
        for cell in removed {
            cx.emit(SheetEvent::Changed(cell));
        }
        self.sync_formula(cx);
        cx.notify();
    }

    fn cell_at(&self, point: Point<Pixels>) -> Option<Cell> {
        let bounds = self.grid_bounds?;
        let x = f32::from(point.x - bounds.left()) as i32 - ROW_HEADER - 2;
        let y = f32::from(point.y - bounds.top()) as i32 - CELL_HEIGHT - 2;
        if x < 0
            || y < 0
            || x >= self.visible_cols() as i32 * CELL_WIDTH
            || y >= self.visible_rows() as i32 * CELL_HEIGHT
        {
            return None;
        }
        let cell = Cell::new(
            self.left + (x / CELL_WIDTH) as usize,
            self.top + (y / CELL_HEIGHT) as usize,
        );
        (cell.col < self.cols && cell.row < self.rows).then_some(cell)
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(cell) = self.cell_at(event.position) else {
            return;
        };
        if self.cell_editing && cell == self.model.cursor {
            return;
        }
        window.focus(&self.focus, cx);
        self.select(cell, event.modifiers.shift, cx);
        if event.click_count == 2 {
            self.start_edit(self.model.raw(cell).into(), true, window, cx);
        } else {
            self.dragging = true;
        }
    }

    fn paint(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        let inactive = crate::window::content_is_inactive();
        let focused = (self.focus.is_focused(window) || self.cell_editing) && !inactive;
        let mut pen = Pen::new(self.grid_width(), self.grid_height());
        pen.language = self.language;
        pen.rect(0, 0, pen.width, pen.height, WHITE);
        pen.bevel(0, 0, pen.width, pen.height, GRAY_7, WHITE);
        pen.frame(1, 1, pen.width - 2, pen.height - 2, BLACK);
        let (first, last) = self.model.selection_bounds();
        for visible in 0..self.visible_rows().min(self.rows.saturating_sub(self.top)) {
            let row = self.top + visible;
            let y = 2 + CELL_HEIGHT + visible as i32 * CELL_HEIGHT;
            pen.set_dimmed(inactive);
            pen.rect(
                2,
                y,
                ROW_HEADER,
                CELL_HEIGHT,
                if (first.row..=last.row).contains(&row) {
                    GRAY_4
                } else {
                    GRAY_2
                },
            );
            pen.bevel(2, y, ROW_HEADER, CELL_HEIGHT, WHITE, GRAY_7);
            let number = (row + 1).to_string();
            pen.text(
                ROW_HEADER - Font::Small.measure(&number) - 2,
                y + 13,
                &number,
                Font::Small,
                BLACK,
            );
            pen.set_dimmed(false);
            for visible_col in 0..self.visible_cols().min(self.cols.saturating_sub(self.left)) {
                let cell = Cell::new(self.left + visible_col, row);
                let x = 2 + ROW_HEADER + visible_col as i32 * CELL_WIDTH;
                if self.model.selected(cell) {
                    pen.rect(
                        x,
                        y,
                        CELL_WIDTH,
                        CELL_HEIGHT,
                        if focused {
                            self.theme.accent.light
                        } else {
                            GRAY_3
                        },
                    );
                }
                pen.vline(x + CELL_WIDTH - 1, y, CELL_HEIGHT, GRAY_3);
                pen.hline(x, y + CELL_HEIGHT - 1, CELL_WIDTH, GRAY_3);
                let value = self.model.value(cell);
                let text = clipped(&value, CELL_WIDTH - 8, self.language);
                let x = if value.parse::<f64>().is_ok() {
                    x + CELL_WIDTH - 4 - Font::Small.measure_in(&text, self.language)
                } else {
                    x + 4
                };
                pen.text(x, y + 13, &text, Font::Small, BLACK);
            }
        }
        pen.set_dimmed(inactive);
        pen.rect(2, 2, ROW_HEADER, CELL_HEIGHT, GRAY_2);
        pen.bevel(2, 2, ROW_HEADER, CELL_HEIGHT, WHITE, GRAY_7);
        for visible in 0..self.visible_cols().min(self.cols.saturating_sub(self.left)) {
            let col = self.left + visible;
            let x = 2 + ROW_HEADER + visible as i32 * CELL_WIDTH;
            pen.rect(
                x,
                2,
                CELL_WIDTH,
                CELL_HEIGHT,
                if (first.col..=last.col).contains(&col) {
                    GRAY_4
                } else {
                    GRAY_2
                },
            );
            pen.bevel(x, 2, CELL_WIDTH, CELL_HEIGHT, WHITE, GRAY_7);
            let title = Cell::column_name(col);
            pen.text(
                x + (CELL_WIDTH - Font::Small.measure(&title)) / 2,
                15,
                &title,
                Font::Small,
                BLACK,
            );
        }
        pen.set_dimmed(false);
        let cell = self.model.cursor;
        if cell.col >= self.left
            && cell.col < self.left + self.visible_cols()
            && cell.row >= self.top
            && cell.row < self.top + self.visible_rows()
        {
            let x = 2 + ROW_HEADER + (cell.col - self.left) as i32 * CELL_WIDTH;
            let y = 2 + CELL_HEIGHT + (cell.row - self.top) as i32 * CELL_HEIGHT;
            let ink = if focused {
                self.theme.accent.dark
            } else {
                BLACK
            };
            pen.frame(x, y, CELL_WIDTH, CELL_HEIGHT, ink);
            pen.frame(x + 1, y + 1, CELL_WIDTH - 2, CELL_HEIGHT - 2, ink);
        }
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            pen.paint(
                point(bounds.origin.x.round(), bounds.origin.y.round()),
                window,
            )
        });
    }
}

impl Focusable for SheetView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for SheetView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.blur_subscription.is_none() {
            let focus = self.editor.focus_handle(cx);
            self.blur_subscription = Some(cx.on_blur(&focus, window, |this, _, cx| {
                if this.cell_editing {
                    this.finish_edit(cx);
                    this.sync_formula(cx);
                    cx.notify();
                }
            }));
        }
        if self.focus_after_edit {
            self.focus_after_edit = false;
            window.focus(&self.focus, cx);
        }
        let theme = self.theme;
        let language = self.language;
        let fills = self.fills_window;
        let formula_width = self.width - 62;
        if self.configured != Some((formula_width, theme, language)) {
            self.formula.update(cx, |field, cx| {
                field.set_size(formula_width as f32, 22., cx);
                field.set_theme(theme, cx);
                field.set_language(language, cx);
            });
            self.editor.update(cx, |field, cx| {
                field.set_theme(theme, cx);
                field.set_language(language, cx);
            });
            self.configured = Some((formula_width, theme, language));
        }
        let layout = cx.entity();
        let paint = cx.entity();
        let vertical = cx.entity();
        let horizontal = cx.entity();
        let cursor = self.model.cursor;
        let reference = cursor.to_string();
        let mut accessible_rows = Vec::new();
        let visible_cols = self.visible_cols().min(self.cols.saturating_sub(self.left));
        accessible_rows.push(
            div()
                .id("headers")
                .role(Role::Row)
                .absolute()
                .left(px((2 + ROW_HEADER) as f32))
                .top(px(2.))
                .h(px(CELL_HEIGHT as f32))
                .flex()
                .children((0..visible_cols).map(|visible| {
                    let col = self.left + visible;
                    div()
                        .id(col)
                        .role(Role::ColumnHeader)
                        .aria_column_index(col + 1)
                        .aria_label(Cell::column_name(col))
                        .w(px(CELL_WIDTH as f32))
                        .h(px(CELL_HEIGHT as f32))
                        .flex_shrink_0()
                })),
        );
        for visible in 0..self.visible_rows().min(self.rows.saturating_sub(self.top)) {
            let row = self.top + visible;
            let cells = (0..visible_cols)
                .map(|visible_col| {
                    let cell = Cell::new(self.left + visible_col, row);
                    let entity = cx.entity();
                    div()
                        .id(cell.col)
                        .role(Role::GridCell)
                        .aria_row_index(cell.row + 1)
                        .aria_column_index(cell.col + 1)
                        .aria_label(cell.to_string())
                        .aria_value(self.model.value(cell))
                        .aria_selected(self.model.selected(cell))
                        .w(px(CELL_WIDTH as f32))
                        .h(px(CELL_HEIGHT as f32))
                        .flex_shrink_0()
                        .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                            entity.update(cx, |this, cx| {
                                this.select(cell, false, cx);
                                window.focus(&this.focus, cx);
                            });
                        })
                })
                .collect::<Vec<_>>();
            accessible_rows.push(
                div()
                    .id(row)
                    .role(Role::Row)
                    .aria_row_index(row + 1)
                    .absolute()
                    .left(px(2.))
                    .top(px((2 + CELL_HEIGHT + visible as i32 * CELL_HEIGHT) as f32))
                    .h(px(CELL_HEIGHT as f32))
                    .flex()
                    .child(
                        div()
                            .id("row-header")
                            .role(Role::RowHeader)
                            .aria_label((row + 1).to_string())
                            .w(px(ROW_HEADER as f32))
                            .h(px(CELL_HEIGHT as f32))
                            .flex_shrink_0(),
                    )
                    .children(cells),
            );
        }
        let grid = div()
            .id("sheet-grid")
            .relative()
            .role(Role::Grid)
            .aria_label(self.label.clone())
            .aria_value(format!("{reference} {}", self.model.value(cursor)))
            .aria_row_count(self.rows)
            .aria_column_count(self.cols)
            .track_focus(&self.focus)
            .tab_stop(true)
            .w(px(self.grid_width() as f32))
            .h(px(self.grid_height() as f32))
            .flex_shrink_0()
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if this.dragging
                    && event.dragging()
                    && let Some(cell) = this.cell_at(event.position)
                {
                    this.select(cell, true, cx);
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = false),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                let delta = event.delta.pixel_delta(px(CELL_HEIGHT as f32));
                this.top = this
                    .top
                    .saturating_add_signed(
                        -(f32::from(delta.y) / CELL_HEIGHT as f32).round() as isize
                    )
                    .min(this.rows.saturating_sub(this.visible_rows()));
                this.left = this
                    .left
                    .saturating_add_signed(
                        -(f32::from(delta.x) / CELL_WIDTH as f32).round() as isize
                    )
                    .min(this.cols.saturating_sub(this.visible_cols()));
                cx.notify();
                cx.stop_propagation();
            }))
            .child(
                canvas(
                    move |bounds, _, cx| {
                        layout.update(cx, |this, _| this.grid_bounds = Some(bounds));
                    },
                    move |bounds, _, window, cx| paint.read(cx).paint(bounds, window),
                )
                .size_full(),
            )
            .children(accessible_rows)
            .when(self.cell_editing, |grid| {
                grid.child(
                    div()
                        .absolute()
                        .left(px((2
                            + ROW_HEADER
                            + cursor.col.saturating_sub(self.left) as i32 * CELL_WIDTH)
                            as f32))
                        .top(px((CELL_HEIGHT
                            + cursor.row.saturating_sub(self.top) as i32 * CELL_HEIGHT)
                            as f32))
                        .child(self.editor.clone()),
                )
            });
        div()
            .id("platinum-sheet")
            .flex()
            .flex_col()
            .flex_shrink_0()
            .w(px(self.width as f32))
            .h(px(self.height as f32))
            .capture_key_down(cx.listener(Self::capture_key))
            .on_key_down(cx.listener(Self::key_down))
            .when(self.formula_bar, |root| {
                root.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .px(px(6.))
                        .h(px(FORMULA_HEIGHT as f32))
                        .flex_shrink_0()
                        .bg(gpui::rgb(GRAY_2))
                        .child(drawing(44., 22., move |pen, _, _| {
                            pen.rect(0, 0, 44, 22, GRAY_2);
                            pen.bevel(0, 0, 44, 22, WHITE, GRAY_7);
                            pen.text_centered(&reference, Font::Bold, BLACK);
                        }))
                        .child(self.formula.clone()),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .when(fills, |row| row.ml(px(-2.)))
                    .child(grid)
                    .child(
                        div().when(fills, |bar| bar.ml(px(-2.)).mt(px(1.))).child(
                            ScrollBar::new(
                                "sheet-vertical",
                                self.top as f64 * CELL_HEIGHT as f64,
                                self.visible_rows() as f64 * CELL_HEIGHT as f64,
                                self.rows as f64 * CELL_HEIGHT as f64,
                            )
                            .tab_stop(false)
                            .step(CELL_HEIGHT as f64)
                            .length(self.grid_height() - if fills { 2 } else { 0 })
                            .theme(theme)
                            .label("Rows")
                            .on_change(move |value, _, cx| {
                                vertical.update(cx, |this, cx| {
                                    this.top = (value / CELL_HEIGHT as f64).round() as usize;
                                    cx.notify();
                                })
                            }),
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .h(px(SCROLLBAR as f32))
                    .when(fills, |row| row.ml(px(-1.)).mt(px(-2.)))
                    .child(if fills {
                        // On the window's lines and the grid's outline.
                        drawing((ROW_HEADER + 1) as f32, SCROLLBAR as f32, |pen, _, _| {
                            pen.rect(0, 0, pen.width, pen.height, GRAY_2);
                            pen.hline(0, 0, pen.width, BLACK);
                            pen.hline(0, pen.height - 1, pen.width, BLACK);
                            pen.vline(0, 0, pen.height, BLACK);
                        })
                        .into_any_element()
                    } else {
                        div()
                            .w(px(ROW_HEADER as f32))
                            .h_full()
                            .bg(gpui::rgb(GRAY_2))
                            .into_any_element()
                    })
                    .child(
                        ScrollBar::new(
                            "sheet-horizontal",
                            self.left as f64 * CELL_WIDTH as f64,
                            self.visible_cols() as f64 * CELL_WIDTH as f64,
                            self.cols as f64 * CELL_WIDTH as f64,
                        )
                        .tab_stop(false)
                        .horizontal()
                        .step(CELL_WIDTH as f64)
                        .length(if fills {
                            // To the resize corner's edge.
                            self.width - GROW_BOX_SIZE + 1 - ROW_HEADER
                        } else {
                            self.grid_width() - ROW_HEADER
                        })
                        .theme(theme)
                        .label("Columns")
                        .on_change(move |value, _, cx| {
                            horizontal.update(cx, |this, cx| {
                                this.left = (value / CELL_WIDTH as f64).round() as usize;
                                cx.notify();
                            })
                        }),
                    ),
            )
    }
}

fn clipped(text: &str, width: i32, language: Language) -> String {
    if Font::Small.measure_in(text, language) <= width {
        return text.into();
    }
    let mut value = String::new();
    for ch in text.chars() {
        if Font::Small.measure_in(&value, language)
            + Font::Small.measure_in(&ch.to_string(), language)
            + Font::Small.measure("…")
            > width
        {
            break;
        }
        value.push(ch);
    }
    value.push('…');
    value
}

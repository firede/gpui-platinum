//! An editable sheet with a formula bar and a rectangular selection.

use gpui::{
    AccessibleAction, App, Bounds, ClipboardItem, Context, Entity, EventEmitter, FocusHandle,
    Focusable, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Point, Role,
    ScrollWheelEvent, SharedString, Subscription, Window, canvas, div, point, prelude::*, px,
};

use std::rc::Rc;

use crate::{
    Font, InputEvent, Language, ScrollBar, TextField, Theme,
    list::{text_centered, text_middle},
    models::{Cell, Sheet},
    paint::{Pen, Rect, drawing},
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
    formula_gap: i32,
    fills_window: bool,
    cell_editing: bool,
    dragging: bool,
    focus_after_edit: bool,
    grid_bounds: Option<Bounds<Pixels>>,
    theme: Theme,
    language: Language,
    label: SharedString,
    configured: Option<(i32, Theme, Language)>,
    /// The formula bar's width, from the last layout.
    bar_width: Rc<std::cell::Cell<i32>>,
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
            formula_gap: 0,
            fills_window: false,
            cell_editing: false,
            dragging: false,
            focus_after_edit: false,
            grid_bounds: None,
            theme: Theme::default(),
            language: Language::English,
            label: "Sheet".into(),
            configured: None,
            bar_width: Rc::default(),
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
    /// The room between the formula bar and the sheet, as a column with a
    /// gap leaves between them. The size given includes it.
    pub fn formula_gap(mut self, gap: i32) -> Self {
        self.formula_gap = gap.max(0);
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

    /// The width of the sheet: its ring, cells and scroll bar.
    fn grid_width(&self) -> i32 {
        if self.fills_window {
            // Its ring under the window's lines at either side, its outline
            // on them.
            self.width + 4
        } else {
            self.width
        }
    }
    fn grid_height(&self) -> i32 {
        let formula = if self.formula_bar {
            FORMULA_HEIGHT + self.formula_gap
        } else {
            0
        };
        if self.fills_window {
            // The outline on the resize corner's top edge.
            self.height - formula - (GROW_BOX_SIZE - 2)
        } else {
            self.height - formula
        }
    }
    fn visible_cols(&self) -> usize {
        ((self.grid_width() - 4 - ROW_HEADER - SCROLLBAR) / CELL_WIDTH).max(1) as usize
    }
    fn visible_rows(&self) -> usize {
        ((self.grid_height() - 4 - CELL_HEIGHT) / CELL_HEIGHT).max(1) as usize
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

    /// The cell under a point, the nearest one of the sheet's beyond its
    /// edges; none over the headers or the scroll bar.
    fn cell_at(&self, point: Point<Pixels>) -> Option<Cell> {
        let bounds = self.grid_bounds?;
        let x = f32::from(point.x - bounds.left()) as i32 - ROW_HEADER - 2;
        let y = f32::from(point.y - bounds.top()) as i32 - CELL_HEIGHT - 2;
        if x < 0 || y < 0 || x + ROW_HEADER + 2 >= self.grid_width() - SCROLLBAR {
            return None;
        }
        Some(Cell::new(
            (self.left + (x / CELL_WIDTH) as usize).min(self.cols - 1),
            (self.top + (y / CELL_HEIGHT) as usize).min(self.rows - 1),
        ))
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

    fn focused(&self, window: &Window) -> bool {
        (self.focus.is_focused(window) || self.cell_editing)
            && !crate::window::content_is_inactive()
    }

    fn paint(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        let inactive = crate::window::content_is_inactive();
        let focused = self.focused(window);
        let (width, height) = (self.grid_width(), self.grid_height());
        let mut pen = Pen::new(width, height);
        pen.bevel(0, 0, width, height, GRAY_7, WHITE);
        let cells_width = width - 2 - SCROLLBAR + 1;
        let mut cells = Pen::new(cells_width, height - 2);
        cells.language = self.language;
        self.draw_sheet(&mut cells, focused, false);
        if inactive {
            // In the background, its headers gray, the cells as they are.
            for clip in [
                Rect {
                    x: 1,
                    y: 1,
                    width: cells_width - 2,
                    height: CELL_HEIGHT,
                },
                Rect {
                    x: 1,
                    y: 1,
                    width: ROW_HEADER,
                    height: height - 4,
                },
            ] {
                let mut headers = Pen::new(cells_width, height - 2);
                headers.language = self.language;
                headers.set_dimmed(true);
                self.draw_sheet(&mut headers, focused, true);
                headers.clip(clip);
                cells.append(headers, 0, 0);
            }
        }
        pen.append(cells, 1, 1);
        if focused && !self.fills_window {
            crate::widgets::draw_focus_ring(&mut pen, 0, self.theme.accent.dark);
        }
        pen.paint(
            point(bounds.origin.x.round(), bounds.origin.y.round()),
            window,
        );
    }

    /// Draws the cells from the first in view in the pen's box, which holds
    /// their outline, as the original's DrawSheet does.
    fn draw_sheet(&self, pen: &mut Pen, focused: bool, headers_only: bool) {
        let (width, height) = (pen.width, pen.height);
        let accent = self.theme.accent;
        let language = self.language;
        pen.rect(1, 1, width - 2, height - 2, WHITE);
        pen.frame(0, 0, width, height, BLACK);
        let (first, last) = self.model.selection_bounds();
        let cursor = self.model.cursor;
        let mut inner = Pen::new(width, height);
        inner.language = language;
        let header = |pen: &mut Pen, x: i32, y: i32, cell_width: i32, label: &str, on: bool| {
            let mut header = Pen::new(cell_width, CELL_HEIGHT);
            header.language = language;
            header.rect(
                0,
                0,
                cell_width,
                CELL_HEIGHT,
                if on { GRAY_4 } else { GRAY_2 },
            );
            header.bevel(0, 0, cell_width - 1, CELL_HEIGHT - 1, WHITE, GRAY_7);
            header.hline(0, CELL_HEIGHT - 1, cell_width, GRAY_9);
            header.vline(cell_width - 1, 0, CELL_HEIGHT, GRAY_9);
            text_centered(
                &mut header,
                0,
                0,
                cell_width - 1,
                CELL_HEIGHT - 1,
                label,
                Font::Small,
                BLACK,
            );
            pen.append(header, x, y);
        };
        for row in self.top..self.rows {
            let y = 1 + CELL_HEIGHT + (row - self.top) as i32 * CELL_HEIGHT;
            if y >= height {
                break;
            }
            for col in self.left..self.cols {
                let x = 1 + ROW_HEADER + (col - self.left) as i32 * CELL_WIDTH;
                if x >= width || headers_only {
                    break;
                }
                let cell = Cell::new(col, row);
                if self.model.selected(cell) && cell != cursor {
                    inner.rect(
                        x,
                        y,
                        CELL_WIDTH,
                        CELL_HEIGHT,
                        if focused { accent.light } else { GRAY_3 },
                    );
                }
                inner.hline(x, y + CELL_HEIGHT - 1, CELL_WIDTH, GRAY_3);
                inner.vline(x + CELL_WIDTH - 1, y, CELL_HEIGHT, GRAY_3);
                let editing = self.model.editing.as_ref().filter(|_| cell == cursor);
                let value = editing.cloned().unwrap_or_else(|| self.model.value(cell));
                if value.is_empty() || (editing.is_some() && self.cell_editing) {
                    continue;
                }
                let fit = Font::Small.truncate_in(&value, CELL_WIDTH - 8, language);
                let text_x = if editing.is_none() && value.parse::<f64>().is_ok() {
                    x + CELL_WIDTH - 4 - Font::Small.measure_in(&fit, language)
                } else {
                    x + 4
                };
                text_middle(
                    &mut inner,
                    text_x,
                    y,
                    CELL_HEIGHT - 1,
                    &fit,
                    Font::Small,
                    BLACK,
                );
            }
            header(
                &mut inner,
                1,
                y,
                ROW_HEADER,
                &(row + 1).to_string(),
                (first.row..=last.row).contains(&row),
            );
        }
        for col in self.left..self.cols {
            let x = 1 + ROW_HEADER + (col - self.left) as i32 * CELL_WIDTH;
            if x >= width {
                break;
            }
            header(
                &mut inner,
                x,
                1,
                CELL_WIDTH,
                &Cell::column_name(col),
                (first.col..=last.col).contains(&col),
            );
        }
        header(&mut inner, 1, 1, ROW_HEADER, "", false);
        // The cursor: a frame two pixels wide.
        if !headers_only && cursor.row >= self.top && cursor.col >= self.left {
            let x = 1 + ROW_HEADER + (cursor.col - self.left) as i32 * CELL_WIDTH;
            let y = 1 + CELL_HEIGHT + (cursor.row - self.top) as i32 * CELL_HEIGHT;
            let ink = if focused { accent.dark } else { BLACK };
            inner.frame(x - 1, y - 1, CELL_WIDTH + 1, CELL_HEIGHT + 1, ink);
            inner.frame(x, y, CELL_WIDTH - 1, CELL_HEIGHT - 1, ink);
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
        // The field takes what the bar leaves it.
        let bar = match self.bar_width.get() {
            0 => self.width,
            width => width,
        };
        let formula_width = bar - 12 - 44 - 6;
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
            })
            .child(
                div()
                    .absolute()
                    .top(px(1.))
                    .left(px((self.grid_width() - 1 - SCROLLBAR) as f32))
                    .child(
                        ScrollBar::new(
                            "sheet-vertical",
                            self.top as f64,
                            self.visible_rows() as f64,
                            self.rows as f64,
                        )
                        .tab_stop(false)
                        .step(1.)
                        .length(self.grid_height() - 2)
                        .theme(theme)
                        .label("Rows")
                        .on_change(move |value, _, cx| {
                            vertical.update(cx, |this, cx| {
                                this.top = value.round() as usize;
                                cx.notify();
                            })
                        }),
                    ),
            );
        let bar_width = self.bar_width.clone();
        let measured = self.bar_width.get();
        div()
            .id("platinum-sheet")
            .flex()
            .flex_col()
            .flex_shrink_0()
            .min_w(px(self.width as f32))
            .when(fills, |root| root.w(px(self.width as f32)))
            .capture_key_down(cx.listener(Self::capture_key))
            .on_key_down(cx.listener(Self::key_down))
            .when(self.formula_bar, |root| {
                // The bar spans what holds the sheet, as the original's.
                root.child(
                    div()
                        .relative()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .px(px(6.))
                        .h(px(FORMULA_HEIGHT as f32))
                        .flex_shrink_0()
                        .child(
                            canvas(
                                move |bounds, window, _| {
                                    let width = f32::from(bounds.size.width).round() as i32;
                                    if bar_width.replace(width) != width && width != measured {
                                        window.refresh();
                                    }
                                },
                                |bounds, _, window, _| {
                                    let mut pen = Pen::new(
                                        f32::from(bounds.size.width).round() as i32,
                                        f32::from(bounds.size.height).round() as i32,
                                    );
                                    pen.rect(0, 0, pen.width, pen.height, GRAY_2);
                                    pen.hline(0, pen.height - 2, pen.width, GRAY_7);
                                    pen.hline(0, pen.height - 1, pen.width, WHITE);
                                    pen.paint(
                                        point(bounds.origin.x.round(), bounds.origin.y.round()),
                                        window,
                                    );
                                },
                            )
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full(),
                        )
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
                    .when(self.formula_bar, |row| row.mt(px(self.formula_gap as f32)))
                    .child(grid),
            )
    }
}

//! Bitmap text fields with native input methods and clipboard editing.

use std::{
    ops::Range,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, ClipboardItem, ContentMask, Context, CursorStyle, ElementInputHandler,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, KeyBinding, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Role, ScrollWheelEvent,
    SharedString, Task, UTF16Selection, Window, actions, canvas, div, point, prelude::*, px, size,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::rich_text::{Alignment, FormatState, RichFormat, Run, StyleAttribute, TextStyle};
use crate::theme::{BLACK, GRAY_2, GRAY_3, GRAY_4, GRAY_6, GRAY_7, GRAY_8, WHITE};
use crate::{Font, Language, Theme};

#[path = "text_buffer.rs"]
pub(crate) mod text_buffer;
use text_buffer::TextBuffer;

const LINE_HEIGHT: f32 = 14.;
const INSET: f32 = 5.;
/// How long the caret shows, and then hides.
const CARET_BLINK: Duration = Duration::from_millis(530);
const SCROLLBAR_WIDTH: f32 = 16.;

actions!(
    platinum_text,
    [
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
        DeleteWordLeft,
        DeleteWordRight,
        Home,
        End,
        SelectHome,
        SelectEnd,
        DocumentStart,
        DocumentEnd,
        SelectDocumentStart,
        SelectDocumentEnd,
        PageUp,
        PageDown,
        SelectAll,
        Copy,
        Cut,
        Paste,
        Undo,
        Redo,
        Enter,
        ShowCharacterPalette,
        ToggleBold,
        ToggleItalic,
        ToggleUnderline,
        LargerText,
        SmallerText,
    ]
);

/// Register the text editing bindings once when the application starts.
pub fn init(cx: &mut App) {
    crate::font::init(cx);
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("PlatinumText")),
        KeyBinding::new("delete", Delete, Some("PlatinumText")),
        KeyBinding::new("left", Left, Some("PlatinumText")),
        KeyBinding::new("right", Right, Some("PlatinumText")),
        KeyBinding::new("up", Up, Some("PlatinumText")),
        KeyBinding::new("down", Down, Some("PlatinumText")),
        KeyBinding::new("shift-left", SelectLeft, Some("PlatinumText")),
        KeyBinding::new("shift-right", SelectRight, Some("PlatinumText")),
        KeyBinding::new("shift-up", SelectUp, Some("PlatinumText")),
        KeyBinding::new("shift-down", SelectDown, Some("PlatinumText")),
        KeyBinding::new("alt-left", WordLeft, Some("PlatinumText")),
        KeyBinding::new("alt-right", WordRight, Some("PlatinumText")),
        KeyBinding::new("alt-shift-left", SelectWordLeft, Some("PlatinumText")),
        KeyBinding::new("alt-shift-right", SelectWordRight, Some("PlatinumText")),
        KeyBinding::new("alt-backspace", DeleteWordLeft, Some("PlatinumText")),
        KeyBinding::new("alt-delete", DeleteWordRight, Some("PlatinumText")),
        KeyBinding::new("home", Home, Some("PlatinumText")),
        KeyBinding::new("end", End, Some("PlatinumText")),
        KeyBinding::new("shift-home", SelectHome, Some("PlatinumText")),
        KeyBinding::new("shift-end", SelectEnd, Some("PlatinumText")),
        KeyBinding::new("cmd-left", Home, Some("PlatinumText")),
        KeyBinding::new("cmd-right", End, Some("PlatinumText")),
        KeyBinding::new("cmd-shift-left", SelectHome, Some("PlatinumText")),
        KeyBinding::new("cmd-shift-right", SelectEnd, Some("PlatinumText")),
        KeyBinding::new("cmd-up", DocumentStart, Some("PlatinumText")),
        KeyBinding::new("cmd-down", DocumentEnd, Some("PlatinumText")),
        KeyBinding::new("cmd-shift-up", SelectDocumentStart, Some("PlatinumText")),
        KeyBinding::new("cmd-shift-down", SelectDocumentEnd, Some("PlatinumText")),
        KeyBinding::new("pageup", PageUp, Some("PlatinumText")),
        KeyBinding::new("pagedown", PageDown, Some("PlatinumText")),
        KeyBinding::new("cmd-a", SelectAll, Some("PlatinumText")),
        KeyBinding::new("cmd-c", Copy, Some("PlatinumText")),
        KeyBinding::new("cmd-x", Cut, Some("PlatinumText")),
        KeyBinding::new("cmd-v", Paste, Some("PlatinumText")),
        KeyBinding::new("cmd-z", Undo, Some("PlatinumText")),
        KeyBinding::new("cmd-shift-z", Redo, Some("PlatinumText")),
        KeyBinding::new("ctrl-a", SelectAll, Some("PlatinumText")),
        KeyBinding::new("ctrl-c", Copy, Some("PlatinumText")),
        KeyBinding::new("ctrl-x", Cut, Some("PlatinumText")),
        KeyBinding::new("ctrl-v", Paste, Some("PlatinumText")),
        KeyBinding::new("ctrl-z", Undo, Some("PlatinumText")),
        KeyBinding::new("ctrl-shift-z", Redo, Some("PlatinumText")),
        KeyBinding::new("enter", Enter, Some("PlatinumText")),
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, Some("PlatinumText")),
        KeyBinding::new("cmd-b", ToggleBold, Some("PlatinumText")),
        KeyBinding::new("cmd-i", ToggleItalic, Some("PlatinumText")),
        KeyBinding::new("cmd-u", ToggleUnderline, Some("PlatinumText")),
        KeyBinding::new("cmd-shift-.", LargerText, Some("PlatinumText")),
        KeyBinding::new("cmd-shift-,", SmallerText, Some("PlatinumText")),
    ]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    Changed,
    Submitted,
}

#[derive(Clone, Debug)]
struct Cell {
    range: Range<usize>,
    x: f32,
    width: f32,
    style: TextStyle,
}

#[derive(Clone, Debug)]
struct Line {
    start: usize,
    end: usize,
    cells: Vec<Cell>,
    width: f32,
    y: f32,
    height: f32,
    baseline: f32,
    offset_x: f32,
    paragraph: usize,
}

impl Line {
    fn new(start: usize) -> Self {
        Self {
            start,
            end: start,
            cells: Vec::new(),
            width: 0.,
            y: 0.,
            height: LINE_HEIGHT,
            baseline: Font::Small.ascent() as f32,
            offset_x: 0.,
            paragraph: 0,
        }
    }

    fn index_at(&self, x: f32) -> usize {
        let x = x - self.offset_x;
        self.cells
            .iter()
            .find(|cell| x < cell.x + cell.width / 2.)
            .map_or(self.end, |cell| cell.range.start)
    }

    fn x_at(&self, index: usize) -> f32 {
        self.offset_x
            + self
                .cells
                .iter()
                .find(|cell| index < cell.range.end)
                .map_or(self.width, |cell| cell.x)
    }
}

/// A sunken text field. Enable `multiline` for a wrapping text area.
pub struct TextField {
    focus_handle: FocusHandle,
    buffer: TextBuffer,
    placeholder: SharedString,
    label: SharedString,
    multiline: bool,
    document_left: f32,
    document_width: Option<f32>,
    first_indent: f32,
    password: bool,
    disabled: bool,
    read_only: bool,
    borderless: bool,
    document: bool,
    width: f32,
    height: f32,
    language: Language,
    theme: Theme,
    bounds: Option<Bounds<Pixels>>,
    lines: Vec<Line>,
    scroll_x: f32,
    scroll_y: f32,
    selecting: bool,
    preferred_x: Option<f32>,
    preceding_line: bool,
    reveal_caret: bool,
    focused: bool,
    activity: Instant,
    blink_on: bool,
    _blink_task: Task<()>,
}

impl TextField {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // The caret shows for CARET_BLINK after it last moved, then hides
        // for as long, and so on: the task draws it again as each turns.
        let blink_task = cx.spawn(async |field, cx| {
            let mut delay = CARET_BLINK;
            loop {
                cx.background_executor().timer(delay).await;
                let result = field.update(cx, |field, cx| {
                    let idle = field.idle(cx);
                    if field.focused {
                        cx.notify();
                    }
                    CARET_BLINK
                        - Duration::from_nanos((idle.as_nanos() % CARET_BLINK.as_nanos()) as u64)
                });
                match result {
                    Ok(next) => delay = next,
                    Err(_) => break,
                }
            }
        });
        Self {
            focus_handle: cx.focus_handle(),
            buffer: TextBuffer::default(),
            placeholder: "".into(),
            label: "".into(),
            multiline: false,
            document_left: INSET,
            document_width: None,
            first_indent: 0.,
            password: false,
            disabled: false,
            read_only: false,
            borderless: false,
            document: false,
            width: 160.,
            height: 22.,
            language: Language::English,
            theme: Theme::default(),
            bounds: None,
            lines: vec![Line::new(0)],
            scroll_x: 0.,
            scroll_y: 0.,
            selecting: false,
            preferred_x: None,
            preceding_line: false,
            reveal_caret: false,
            focused: false,
            activity: cx.background_executor().now(),
            blink_on: true,
            _blink_task: blink_task,
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.buffer.set(self.normalize_text(&text.into()));
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// This label is exposed to accessibility clients without drawing extra text.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        if multiline {
            self.width = 200.;
            self.height = 120.;
        }
        self
    }

    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn borderless(mut self, borderless: bool) -> Self {
        self.borderless = borderless;
        self
    }

    /// Make a text area the content of a document window: white to the
    /// window's edges, its scroll bar laid over the window's line at its
    /// right and the lines above and below it as a window's own scroll bar
    /// is, no ring or outline of its own, and no focus ring: the window
    /// shows where typing goes. `set_document_layout` sets its margins.
    pub fn document(mut self, document: bool) -> Self {
        self.document = document;
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width.max(24.);
        self.height = height.max(if self.borderless { 18. } else { 22. });
        self
    }

    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub(crate) fn rich(mut self) -> Self {
        self.buffer.rich = Some(RichFormat::new(&self.buffer.text));
        self
    }

    pub fn selection(&self) -> Range<usize> {
        self.buffer.selection()
    }

    /// Replace UTF-8 ranges while preserving the editor's styles and undo history.
    pub fn replace_ranges(
        &mut self,
        ranges: &[Range<usize>],
        text: &str,
        cx: &mut Context<Self>,
    ) -> usize {
        if self.disabled || self.read_only {
            return 0;
        }
        let text = self.normalize_text(text);
        let count = self.buffer.replace_many(ranges, &text);
        self.changed(count > 0, cx);
        count
    }

    pub fn select(&mut self, anchor: usize, caret: usize, cx: &mut Context<Self>) {
        self.buffer.select(anchor, false);
        self.buffer.select(caret, true);
        self.activity(cx);
    }

    pub fn format_state(&self) -> Option<FormatState> {
        self.buffer.format_state()
    }

    pub fn runs(&self) -> Vec<Run> {
        self.buffer.rich.as_ref().map_or_else(
            || {
                vec![Run {
                    text: self.buffer.text.clone(),
                    style: TextStyle::default(),
                }]
            },
            |rich| rich.runs(&self.buffer.text),
        )
    }

    pub fn alignments(&self) -> Vec<Alignment> {
        self.buffer
            .rich
            .as_ref()
            .map(|rich| rich.alignments.clone())
            .unwrap_or_default()
    }

    /// Restore paragraph alignments when loading a styled document.
    pub fn set_alignments(&mut self, alignments: &[Alignment], cx: &mut Context<Self>) {
        if let Some(rich) = &mut self.buffer.rich {
            rich.alignments = alignments.to_vec();
            rich.resize(&self.buffer.text);
            self.activity(cx);
        }
    }

    pub fn set_runs(&mut self, runs: &[Run], cx: &mut Context<Self>) {
        self.buffer.set_runs(runs);
        self.activity(cx);
    }

    pub fn toggle_style(&mut self, attribute: StyleAttribute, cx: &mut Context<Self>) {
        if self.disabled || self.read_only || self.buffer.rich.is_none() {
            return;
        }
        self.buffer.toggle_style(attribute);
        self.changed(true, cx);
    }

    pub fn set_scale(&mut self, scale: u8, cx: &mut Context<Self>) {
        if self.disabled || self.read_only || self.buffer.rich.is_none() {
            return;
        }
        self.buffer.set_scale(scale);
        self.changed(true, cx);
    }

    pub fn set_alignment(&mut self, alignment: Alignment, cx: &mut Context<Self>) {
        if self.disabled || self.read_only || self.buffer.rich.is_none() {
            return;
        }
        self.buffer.set_alignment(alignment);
        self.changed(true, cx);
    }

    /// Set document margins and the first-line indent, in bitmap pixels.
    /// A zero wrap width lets the text fill its viewport.
    pub fn set_document_layout(
        &mut self,
        left: f32,
        width: f32,
        first: f32,
        cx: &mut Context<Self>,
    ) {
        self.document_left = left.max(0.);
        self.document_width = (width > 0.).then_some(width.max(24.));
        self.first_indent = first.max(0.);
        self.activity(cx);
    }

    pub fn set_size(&mut self, width: f32, height: f32, cx: &mut Context<Self>) {
        self.width = width.max(24.);
        self.height = height.max(if self.borderless { 18. } else { 22. });
        self.activity(cx);
    }

    pub fn value(&self) -> &str {
        &self.buffer.text
    }

    /// Set the value from the application, clearing the field's undo history.
    pub fn set_value(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        let text = self.normalize_text(&text.into());
        if self.buffer.text != text {
            self.buffer.set(text);
            self.activity(cx);
        }
    }

    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        cx.notify();
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }

    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        self.selecting = false;
        cx.notify();
    }

    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.label = label.into();
        cx.notify();
    }

    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    pub fn set_borderless(&mut self, borderless: bool, cx: &mut Context<Self>) {
        self.borderless = borderless;
        cx.notify();
    }

    pub fn set_document(&mut self, document: bool, cx: &mut Context<Self>) {
        self.document = document;
        cx.notify();
    }

    fn normalize_text(&self, text: &str) -> String {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        if self.multiline {
            text
        } else {
            text.replace('\n', " ")
        }
    }

    /// How long since the caret last moved.
    fn idle(&self, cx: &App) -> Duration {
        cx.background_executor()
            .now()
            .saturating_duration_since(self.activity)
    }

    fn activity(&mut self, cx: &mut Context<Self>) {
        self.activity = cx.background_executor().now();
        self.blink_on = true;
        self.reveal_caret = true;
        self.preferred_x = None;
        self.preceding_line = false;
        cx.notify();
    }

    fn changed(&mut self, changed: bool, cx: &mut Context<Self>) {
        if changed {
            cx.emit(InputEvent::Changed);
        }
        self.activity(cx);
    }

    fn move_horizontal(&mut self, forward: bool, extend: bool, word: bool, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let selection = self.buffer.selection();
        let offset = if !extend && !selection.is_empty() && !word {
            if forward {
                selection.end
            } else {
                selection.start
            }
        } else if word {
            if forward {
                self.buffer.next_word()
            } else {
                self.buffer.previous_word()
            }
        } else if forward {
            self.buffer.next(self.buffer.caret)
        } else {
            self.buffer.previous(self.buffer.caret)
        };
        self.buffer.select(offset, extend);
        self.activity(cx);
    }

    fn cursor_position(&self) -> (usize, f32) {
        let row = line_index(&self.lines, self.buffer.caret, self.preceding_line);
        (row, self.lines[row].x_at(self.buffer.caret))
    }

    fn move_vertical(&mut self, delta: isize, extend: bool, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let (row, x) = self.cursor_position();
        let x = self.preferred_x.unwrap_or(x);
        let target = (row as isize + delta).clamp(0, self.lines.len() as isize - 1) as usize;
        self.buffer.select(self.lines[target].index_at(x), extend);
        self.activity(cx);
        self.preferred_x = Some(x);
        self.preceding_line = self.buffer.caret == self.lines[target].end;
    }

    fn move_end(&mut self, end: bool, document: bool, extend: bool, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let row = self.cursor_position().0;
        let offset = match (end, document) {
            (true, true) => self.buffer.text.len(),
            (false, true) => 0,
            (true, false) => self.lines[row].end,
            (false, false) => self.lines[row].start,
        };
        self.buffer.select(offset, extend);
        self.activity(cx);
        self.preceding_line = end && !document;
    }

    fn move_page(&mut self, down: bool, cx: &mut Context<Self>) {
        let row = self.cursor_position().0;
        let target_y = self.lines[row].y
            + if down {
                self.viewport_height()
            } else {
                -self.viewport_height()
            };
        let target = self
            .lines
            .iter()
            .rposition(|line| line.y <= target_y)
            .unwrap_or(0);
        self.move_vertical(target as isize - row as isize, false, cx);
    }

    fn delete_text(&mut self, forward: bool, word: bool, cx: &mut Context<Self>) {
        if self.disabled || self.read_only {
            return;
        }
        let mut range = self.buffer.selection();
        if range.is_empty() {
            if forward {
                range.end = if word {
                    self.buffer.next_word()
                } else {
                    self.buffer.next(range.end)
                };
            } else {
                range.start = if word {
                    self.buffer.previous_word()
                } else {
                    self.buffer.previous(range.start)
                };
            }
        }
        let changed = self.buffer.replace(range, "");
        self.changed(changed, cx);
    }

    fn copy(&mut self, cx: &mut Context<Self>) {
        if self.disabled || self.password {
            return;
        }
        let selection = self.buffer.selection();
        if !selection.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.buffer.text[selection].to_string(),
            ));
        }
    }

    fn cut(&mut self, cx: &mut Context<Self>) {
        if self.disabled || self.read_only || self.password {
            return;
        }
        self.copy(cx);
        let changed = self.buffer.replace(self.buffer.selection(), "");
        self.changed(changed, cx);
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.read_only {
            return;
        }
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn history(&mut self, redo: bool, cx: &mut Context<Self>) {
        if self.disabled || self.read_only {
            return;
        }
        let changed = if redo {
            self.buffer.redo()
        } else {
            self.buffer.undo()
        };
        self.changed(changed, cx);
    }

    fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.read_only {
            return;
        }
        if self.multiline {
            self.replace_text_in_range(None, "\n", window, cx);
        } else {
            cx.emit(InputEvent::Submitted);
            cx.propagate();
        }
    }

    fn text_top(&self, height: f32) -> f32 {
        if self.multiline {
            4.
        } else {
            ((height - Font::Small.cap_height() as f32) / 2.).floor()
        }
    }

    fn viewport_height(&self) -> f32 {
        self.bounds
            .map_or(self.height, |bounds| f32::from(bounds.size.height))
            - 8.
    }

    fn max_scroll(&self) -> f32 {
        (self.content_height() - self.viewport_height()).max(0.)
    }

    fn content_height(&self) -> f32 {
        self.lines
            .last()
            .map_or(LINE_HEIGHT, |line| line.y + line.height)
    }

    fn index_at(&self, position: Point<Pixels>) -> usize {
        self.position_at(position).1
    }

    fn position_at(&self, position: Point<Pixels>) -> (usize, usize) {
        let Some(bounds) = self.bounds else {
            return (0, 0);
        };
        let y = f32::from(position.y - bounds.top()) - self.text_top(f32::from(bounds.size.height))
            + self.scroll_y;
        let row = self.lines.iter().rposition(|line| line.y <= y).unwrap_or(0);
        let x = f32::from(position.x - bounds.left()) - self.document_left + self.scroll_x;
        (row, self.lines[row].index_at(x))
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            window.prevent_default();
            cx.stop_propagation();
            return;
        }
        window.focus(&self.focus_handle, cx);
        let (row, index) = self.position_at(event.position);
        if event.click_count >= 3 {
            let line = &self.lines[row];
            self.buffer.anchor = line.start;
            self.buffer.caret = line.end;
        } else if event.click_count == 2 {
            let word = self.buffer.word_at(index);
            self.buffer.anchor = word.start;
            self.buffer.caret = word.end;
        } else {
            self.buffer.select(index, event.modifiers.shift);
        }
        self.selecting = true;
        self.activity(cx);
        self.preceding_line = self.buffer.caret == self.lines[row].end;
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || !event.dragging() {
            return;
        }
        if self.selecting {
            let (row, index) = self.position_at(event.position);
            self.buffer.select(index, true);
            self.activity(cx);
            self.preceding_line = self.buffer.caret == self.lines[row].end;
        }
    }

    fn on_scroll(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || !self.multiline {
            return;
        }
        self.scroll_y = (self.scroll_y - f32::from(event.delta.pixel_delta(px(LINE_HEIGHT)).y))
            .clamp(0., self.max_scroll());
        cx.stop_propagation();
        cx.notify();
    }

    fn layout(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = Some(bounds);
        let width = f32::from(bounds.size.width)
            - self.document_left
            - INSET
            - if self.multiline { SCROLLBAR_WIDTH } else { 0. };
        self.lines = layout_document_lines(
            &self.buffer.text,
            self.language,
            self.password,
            self.multiline
                .then_some(self.document_width.unwrap_or(width).min(width).max(1.)),
            self.buffer.rich.as_ref(),
            self.first_indent,
        );
        self.scroll_y = self.scroll_y.clamp(0., self.max_scroll());
        if self.reveal_caret {
            let (row, x) = self.cursor_position();
            if self.multiline {
                let y = self.lines[row].y;
                let height = self.lines[row].height;
                if y < self.scroll_y {
                    self.scroll_y = y;
                }
                if y + height > self.scroll_y + self.viewport_height() {
                    self.scroll_y = y + height - self.viewport_height();
                }
                self.scroll_x = 0.;
            } else {
                if x < self.scroll_x {
                    self.scroll_x = x;
                }
                if x + 1. > self.scroll_x + width {
                    self.scroll_x = x + 1. - width;
                }
                self.scroll_x = self
                    .scroll_x
                    .clamp(0., (self.lines[0].width + 1. - width).max(0.));
            }
            self.reveal_caret = false;
        }
    }

    fn range_bounds(&self, range: Range<usize>) -> Option<Bounds<Pixels>> {
        let bounds = self.bounds?;
        let row = line_index(
            &self.lines,
            range.start,
            range.is_empty() && self.preceding_line,
        );
        let line = &self.lines[row];
        let left = line.x_at(range.start);
        let right = line.x_at(range.end.min(line.end));
        Some(Bounds::new(
            point(
                bounds.left() + px(self.document_left + left - self.scroll_x),
                bounds.top()
                    + px(self.text_top(f32::from(bounds.size.height)) + line.y - self.scroll_y),
            ),
            size(px((right - left).max(1.)), px(line.height)),
        ))
    }
}

impl EventEmitter<InputEvent> for TextField {}

impl Focusable for TextField {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for TextField {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.buffer.byte_range(range);
        *actual_range = Some(self.buffer.utf16_range(range.clone()));
        Some(self.buffer.text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        ignore_disabled: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if (self.disabled || self.read_only) && !ignore_disabled {
            return None;
        }
        Some(UTF16Selection {
            range: self.buffer.utf16_range(self.buffer.selection()),
            reversed: self.buffer.caret < self.buffer.anchor,
        })
    }

    fn set_selected_text_range(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = self.buffer.byte_range(range);
        self.select(range.start, range.end, cx);
    }

    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.buffer.text.encode_utf16().count())
    }

    fn accepts_text_input(&self, window: &mut Window, _: &mut Context<Self>) -> bool {
        !self.disabled && !self.read_only && self.focus_handle.is_focused(window)
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.buffer
            .marked
            .clone()
            .map(|range| self.buffer.utf16_range(range))
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.buffer.marked = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled || self.read_only || !self.focus_handle.is_focused(window) {
            return;
        }
        let text = self.normalize_text(text);
        let range = range
            .map(|range| self.buffer.byte_range(range))
            .or(self.buffer.marked.clone())
            .unwrap_or_else(|| self.buffer.selection());
        let changed = self.buffer.replace(range, &text);
        self.changed(changed, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selection: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled || self.read_only || !self.focus_handle.is_focused(window) {
            return;
        }
        let text = self.normalize_text(text);
        let range = range
            .map(|range| self.buffer.byte_range(range))
            .or(self.buffer.marked.clone())
            .unwrap_or_else(|| self.buffer.selection());
        let changed = self.buffer.compose(range, &text, selection);
        self.changed(changed, cx);
    }

    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        // The input method can ask before the next paint after changing marked text.
        self.layout(element_bounds);
        self.range_bounds(self.buffer.byte_range(range))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        self.layout(self.bounds?);
        let index = self.index_at(point);
        Some(self.buffer.utf16_range(index..index).start)
    }
}

impl TextField {
    fn paint(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        use crate::paint::Pen;

        let width = f32::from(bounds.size.width).round() as i32;
        let height = f32::from(bounds.size.height).round() as i32;
        let origin = point(bounds.origin.x.round(), bounds.origin.y.round());
        let face = if self.disabled { GRAY_2 } else { WHITE };
        let ink = if self.disabled { GRAY_8 } else { BLACK };
        let inactive = crate::window::content_is_inactive();
        let mut pen = Pen::new(width, height);
        if self.document && self.multiline {
            // A document's content: white to the window's edges, its scroll
            // bar over the window's line at its right.
            pen.rect(0, 0, width - SCROLLBAR_WIDTH as i32 + 1, height, WHITE);
        } else if self.borderless {
            pen.rect(0, 0, width, height, face);
        } else {
            pen.bevel(0, 0, width, height, GRAY_7, WHITE);
            pen.frame(
                1,
                1,
                width - 2,
                height - 2,
                if inactive {
                    GRAY_8
                } else if self.disabled {
                    GRAY_6
                } else {
                    BLACK
                },
            );
            pen.rect(2, 2, width - 4, height - 4, face);
            if self.focused && !self.disabled && !inactive {
                pen.frame(0, 0, width, height, self.theme.accent.dark);
            }
        }
        let content_right = width
            - if self.multiline {
                SCROLLBAR_WIDTH as i32 + 1
            } else {
                2
            };
        pen.paint(origin, window);

        let mut text = Pen::new(width, height);
        let selection = self.buffer.selection();
        let top = self.text_top(height as f32);
        let selection_color = if inactive {
            GRAY_4
        } else if self.focused {
            self.theme.accent.light
        } else {
            GRAY_3
        };
        for line in &self.lines {
            let y = (top + line.y - self.scroll_y).round() as i32;
            if y + (line.height as i32) < 2 || y >= height - 2 {
                continue;
            }
            let left = (self.document_left - self.scroll_x).round() as i32;
            if !selection.is_empty() && selection.start <= line.end && selection.end > line.start {
                let x = line.x_at(selection.start.max(line.start));
                let right = if selection.end > line.end {
                    line.offset_x + line.width + 3.
                } else {
                    line.x_at(selection.end)
                };
                text.rect(
                    left + x as i32,
                    y - 1,
                    (right - x).round() as i32,
                    line.height as i32,
                    selection_color,
                );
            }
            for cell in &line.cells {
                let visible = if self.password {
                    "•"
                } else {
                    &self.buffer.text[cell.range.clone()]
                };
                paint_styled_text(
                    &mut text,
                    left + (line.offset_x + cell.x).round() as i32,
                    y + line.baseline as i32,
                    visible,
                    self.language,
                    ink,
                    cell.style,
                );
                if cell.style.underline {
                    let scale = cell.style.scale.clamp(1, 4) as i32;
                    text.rect(
                        left + (line.offset_x + cell.x).round() as i32,
                        y + line.baseline as i32 + scale,
                        cell.width as i32,
                        scale,
                        ink,
                    );
                }
            }
            if let Some(marked) = &self.buffer.marked
                && marked.start <= line.end
                && marked.end > line.start
            {
                let x = line.x_at(marked.start.max(line.start));
                let right = line.x_at(marked.end.min(line.end));
                text.hline(
                    left + x as i32,
                    y + line.height as i32 - 2,
                    (right - x) as i32,
                    ink,
                );
            }
        }
        if self.buffer.text.is_empty() && !self.placeholder.is_empty() {
            paint_text(
                &mut text,
                self.document_left as i32,
                top as i32 + Font::Small.ascent(),
                &self.placeholder,
                self.language,
                GRAY_7,
            );
        }
        if self.focused && !self.disabled && !inactive && self.blink_on && selection.is_empty() {
            let (row, x) = self.cursor_position();
            text.vline(
                (self.document_left + x - self.scroll_x).round() as i32,
                (top + self.lines[row].y - self.scroll_y - 1.).round() as i32,
                self.lines[row].height as i32,
                BLACK,
            );
        }
        let mask = ContentMask {
            bounds: Bounds::new(
                origin + point(px(2.), px(2.)),
                size(px((content_right - 2) as f32), px((height - 4) as f32)),
            ),
        };
        window.with_content_mask(Some(mask), |window| text.paint(origin, window));
    }
}

fn paint_text(
    pen: &mut crate::paint::Pen,
    x: i32,
    baseline: i32,
    text: &str,
    language: Language,
    color: u32,
) {
    paint_styled_text(
        pen,
        x,
        baseline,
        text,
        language,
        color,
        TextStyle::default(),
    );
}

fn paint_styled_text(
    pen: &mut crate::paint::Pen,
    x: i32,
    baseline: i32,
    text: &str,
    language: Language,
    color: u32,
    style: TextStyle,
) {
    // Each cell is one grapheme; the field underlines whole cells itself.
    let style = TextStyle {
        underline: false,
        ..style
    };
    pen.styled_text_in(x, baseline, text, Font::Small, color, style, language);
}

fn line_index(lines: &[Line], offset: usize, preceding: bool) -> usize {
    // A soft wrap has one byte offset at both the end and the start of a line.
    // Preserve which side was reached by End, vertical motion or a mouse click.
    if preceding {
        lines.iter().position(|line| line.end >= offset)
    } else {
        lines.iter().rposition(|line| line.start <= offset)
    }
    .unwrap_or(0)
}

#[cfg(test)]
fn layout_lines(text: &str, language: Language, password: bool, wrap: Option<f32>) -> Vec<Line> {
    layout_styled_lines(text, language, password, wrap, None)
}

#[cfg(test)]
fn layout_styled_lines(
    text: &str,
    language: Language,
    password: bool,
    wrap: Option<f32>,
    rich: Option<&RichFormat>,
) -> Vec<Line> {
    layout_document_lines(text, language, password, wrap, rich, 0.)
}

fn layout_document_lines(
    text: &str,
    language: Language,
    password: bool,
    wrap: Option<f32>,
    rich: Option<&RichFormat>,
    first_indent: f32,
) -> Vec<Line> {
    let first_indent = first_indent.min((wrap.unwrap_or(f32::MAX) - 1.).max(0.));
    let mut lines = Vec::new();
    let mut line = Line::new(0);
    line.offset_x = first_indent;
    let mut character = 0;
    let mut paragraph = 0;
    for (start, grapheme) in text.grapheme_indices(true) {
        let style = rich
            .and_then(|rich| rich.styles.get(character))
            .copied()
            .unwrap_or_default();
        character += grapheme.chars().count();
        if grapheme == "\n" || grapheme == "\r\n" {
            line.end = start;
            lines.push(line);
            line = Line::new(start + grapheme.len());
            line.offset_x = first_indent;
            paragraph += 1;
            line.paragraph = paragraph;
            continue;
        }
        let visible = if password { "•" } else { grapheme };
        let font = style.font();
        let scale = i32::from(style.scale.clamp(1, 4));
        let mut advance = font.styled_advance(visible, scale, style.italic, language);
        let next_style = rich.and_then(|rich| rich.styles.get(character)).copied();
        if style.italic && next_style.is_none_or(|next| !next.italic || next.scale != style.scale) {
            advance += font.italic_slant() * scale;
        }
        let width = advance as f32;
        let trailing_space = grapheme.chars().all(char::is_whitespace)
            && line
                .cells
                .last()
                .is_some_and(|cell| !text[cell.range.clone()].chars().all(char::is_whitespace));
        if wrap.is_some_and(|wrap| line.offset_x + line.width + width > wrap)
            && !line.cells.is_empty()
            && !trailing_space
        {
            let word_break = line
                .cells
                .iter()
                .rposition(|cell| text[cell.range.clone()].chars().all(char::is_whitespace));
            if let Some(index) = word_break.filter(|index| *index + 1 < line.cells.len()) {
                let mut tail = line.cells.split_off(index + 1);
                let next_start = tail[0].range.start;
                let x = tail[0].x;
                for cell in &mut tail {
                    cell.x -= x;
                }
                let next_width = line.width - x;
                line.end = next_start;
                line.width = x;
                lines.push(line);
                line = Line::new(next_start);
                line.end = start;
                line.cells = tail;
                line.width = next_width;
            } else {
                line.end = start;
                lines.push(line);
                line = Line::new(start);
            }
            line.paragraph = paragraph;
        }
        line.cells.push(Cell {
            range: start..start + grapheme.len(),
            x: line.width,
            width,
            style,
        });
        line.width += width;
        line.end = start + grapheme.len();
    }
    lines.push(line);
    let mut y = 0.;
    let (above, below) = Font::Small.line_ink(language);
    for line in &mut lines {
        let scale = line
            .cells
            .iter()
            .map(|cell| cell.style.scale)
            .max()
            .unwrap_or(1)
            .clamp(1, 4) as f32;
        line.y = y;
        line.height = LINE_HEIGHT + (scale - 1.) * (above + below) as f32;
        if wrap.is_some() {
            line.baseline =
                ((LINE_HEIGHT - (above + below) as f32) / 2.).floor() + above as f32 * scale;
        }
        let alignment = rich.map_or(Alignment::Left, |rich| rich.alignments[line.paragraph]);
        let spare = (wrap.unwrap_or(line.width) - line.offset_x - line.width).max(0.);
        line.offset_x += match alignment {
            Alignment::Left => 0.,
            Alignment::Center => (spare / 2.).floor(),
            Alignment::Right => spare,
        };
        y += line.height;
    }
    lines
}

impl Render for TextField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.focused = self.focus_handle.is_focused(window);
        self.blink_on = (self.idle(cx).as_nanos() / CARET_BLINK.as_nanos()).is_multiple_of(2);
        let bounds = self.bounds.unwrap_or_else(|| {
            Bounds::new(point(px(0.), px(0.)), size(px(self.width), px(self.height)))
        });
        self.layout(Bounds::new(
            bounds.origin,
            size(px(self.width), px(self.height)),
        ));
        let layout_entity = cx.entity();
        let paint_entity = cx.entity();
        let scroll_entity = cx.entity();
        let role = if self.password {
            Role::PasswordInput
        } else if self.multiline {
            Role::MultilineTextInput
        } else {
            Role::TextInput
        };
        let value = if self.password {
            "•".repeat(self.buffer.text.graphemes(true).count())
        } else {
            self.buffer.text.clone()
        };
        let disabled = self.disabled;
        let read_only = self.read_only;
        div()
            .id("platinum-text")
            .relative()
            .w(px(self.width))
            .h(px(self.height))
            .flex_shrink_0()
            .key_context("PlatinumText")
            .track_focus(&self.focus_handle)
            .tab_index(0)
            .tab_stop(!self.disabled)
            .role(role)
            .aria_label(self.label.clone())
            .aria_placeholder(self.placeholder.clone())
            .aria_value(value)
            .a11y_synthetic_children(move |builder| {
                let node = builder.parent_node();
                if disabled {
                    node.set_disabled();
                }
                if read_only {
                    node.set_read_only();
                }
            })
            .cursor(if self.disabled {
                CursorStyle::Arrow
            } else {
                CursorStyle::IBeam
            })
            .on_action(
                cx.listener(|this, _: &Left, _, cx| this.move_horizontal(false, false, false, cx)),
            )
            .on_action(
                cx.listener(|this, _: &Right, _, cx| this.move_horizontal(true, false, false, cx)),
            )
            .on_action(cx.listener(|this, _: &SelectLeft, _, cx| {
                this.move_horizontal(false, true, false, cx)
            }))
            .on_action(cx.listener(|this, _: &SelectRight, _, cx| {
                this.move_horizontal(true, true, false, cx)
            }))
            .on_action(
                cx.listener(|this, _: &WordLeft, _, cx| {
                    this.move_horizontal(false, false, true, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &WordRight, _, cx| {
                    this.move_horizontal(true, false, true, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &SelectWordLeft, _, cx| {
                this.move_horizontal(false, true, true, cx)
            }))
            .on_action(cx.listener(|this, _: &SelectWordRight, _, cx| {
                this.move_horizontal(true, true, true, cx)
            }))
            .on_action(cx.listener(|this, _: &Up, _, cx| this.move_vertical(-1, false, cx)))
            .on_action(cx.listener(|this, _: &Down, _, cx| this.move_vertical(1, false, cx)))
            .on_action(cx.listener(|this, _: &SelectUp, _, cx| this.move_vertical(-1, true, cx)))
            .on_action(cx.listener(|this, _: &SelectDown, _, cx| this.move_vertical(1, true, cx)))
            .on_action(cx.listener(|this, _: &PageUp, _, cx| this.move_page(false, cx)))
            .on_action(cx.listener(|this, _: &PageDown, _, cx| this.move_page(true, cx)))
            .on_action(cx.listener(|this, _: &Home, _, cx| this.move_end(false, false, false, cx)))
            .on_action(cx.listener(|this, _: &End, _, cx| this.move_end(true, false, false, cx)))
            .on_action(
                cx.listener(|this, _: &SelectHome, _, cx| this.move_end(false, false, true, cx)),
            )
            .on_action(
                cx.listener(|this, _: &SelectEnd, _, cx| this.move_end(true, false, true, cx)),
            )
            .on_action(
                cx.listener(|this, _: &DocumentStart, _, cx| this.move_end(false, true, false, cx)),
            )
            .on_action(
                cx.listener(|this, _: &DocumentEnd, _, cx| this.move_end(true, true, false, cx)),
            )
            .on_action(cx.listener(|this, _: &SelectDocumentStart, _, cx| {
                this.move_end(false, true, true, cx)
            }))
            .on_action(
                cx.listener(|this, _: &SelectDocumentEnd, _, cx| {
                    this.move_end(true, true, true, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &SelectAll, _, cx| {
                if !this.disabled {
                    this.buffer.anchor = 0;
                    this.buffer.caret = this.buffer.text.len();
                    this.activity(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Backspace, _, cx| this.delete_text(false, false, cx)))
            .on_action(cx.listener(|this, _: &Delete, _, cx| this.delete_text(true, false, cx)))
            .on_action(
                cx.listener(|this, _: &DeleteWordLeft, _, cx| this.delete_text(false, true, cx)),
            )
            .on_action(
                cx.listener(|this, _: &DeleteWordRight, _, cx| this.delete_text(true, true, cx)),
            )
            .on_action(cx.listener(|this, _: &Copy, _, cx| this.copy(cx)))
            .on_action(cx.listener(|this, _: &Cut, _, cx| this.cut(cx)))
            .on_action(cx.listener(|this, _: &Paste, window, cx| this.paste(window, cx)))
            .on_action(cx.listener(|this, _: &Undo, _, cx| this.history(false, cx)))
            .on_action(cx.listener(|this, _: &Redo, _, cx| this.history(true, cx)))
            .on_action(cx.listener(|this, _: &Enter, window, cx| this.enter(window, cx)))
            .on_action(cx.listener(|this, _: &ShowCharacterPalette, window, _| {
                if !this.disabled && !this.read_only {
                    window.show_character_palette();
                }
            }))
            .on_action(cx.listener(|this, _: &ToggleBold, _, cx| {
                this.toggle_style(StyleAttribute::Bold, cx)
            }))
            .on_action(cx.listener(|this, _: &ToggleItalic, _, cx| {
                this.toggle_style(StyleAttribute::Italic, cx)
            }))
            .on_action(cx.listener(|this, _: &ToggleUnderline, _, cx| {
                this.toggle_style(StyleAttribute::Underline, cx)
            }))
            .on_action(cx.listener(|this, _: &LargerText, _, cx| {
                if let Some(state) = this.format_state() {
                    this.set_scale(state.scale.saturating_add(1), cx);
                }
            }))
            .on_action(cx.listener(|this, _: &SmallerText, _, cx| {
                if let Some(state) = this.format_state() {
                    this.set_scale(state.scale.saturating_sub(1), cx);
                }
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .child(
                canvas(
                    move |bounds, _, cx| {
                        layout_entity.update(cx, |field, _| field.layout(bounds));
                    },
                    move |bounds, _, window, cx| {
                        let drag_entity = paint_entity.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                            if phase.bubble() && event.dragging() {
                                drag_entity.update(cx, |field, cx| {
                                    field.on_mouse_move(event, window, cx);
                                });
                            }
                        });
                        paint_entity.update(cx, |field, cx| {
                            if !field.disabled && !field.read_only {
                                window.handle_input(
                                    &field.focus_handle,
                                    ElementInputHandler::new(bounds, paint_entity.clone()),
                                    cx,
                                );
                            }
                            field.paint(bounds, window);
                        });
                    },
                )
                .size_full(),
            )
            .when(self.multiline, |element| {
                // A document's scroll bar lies over the window's lines at its
                // right, top and bottom.
                let (inset, length) = if self.document {
                    (-1., self.height + 2.)
                } else {
                    (1., self.height - 2.)
                };
                element.child(
                    div().absolute().right(px(inset)).top(px(inset)).child(
                        crate::ScrollBar::new(
                            "text-scroll",
                            self.scroll_y as f64,
                            self.viewport_height() as f64,
                            self.content_height() as f64,
                        )
                        .length(length as i32)
                        .step(LINE_HEIGHT as f64)
                        .theme(self.theme)
                        .tab_stop(false)
                        .disabled(self.disabled)
                        .on_change(move |value, _, cx| {
                            scroll_entity.update(cx, |field, cx| {
                                field.scroll_y = (value as f32).clamp(0., field.max_scroll());
                                cx.notify();
                            })
                        }),
                    ),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_line_indentation_wraps_and_hit_tests_in_document_coordinates() {
        let text = "one two three\none two";
        let width = Font::Small.measure("one two") as f32 + 1.;
        let lines = layout_document_lines(text, Language::English, false, Some(width), None, 16.);
        assert_eq!(lines[0].offset_x, 16.);
        assert_eq!(&text[lines[0].start..lines[0].end], "one ");
        assert_eq!(lines[1].offset_x, 0.);
        assert_eq!(
            lines
                .iter()
                .find(|line| line.paragraph == 1)
                .unwrap()
                .offset_x,
            16.
        );
        for line in &lines {
            assert_eq!(line.index_at(line.x_at(line.start)), line.start);
        }
    }

    #[test]
    fn wrapping_keeps_words_together_and_indices_in_the_original_text() {
        let text = "one two three";
        let width = Font::Small.measure("one two") as f32 + 1.;
        let lines = layout_lines(text, Language::English, false, Some(width));
        assert_eq!(&text[lines[0].start..lines[0].end], "one two ");
        assert_eq!(&text[lines[1].start..lines[1].end], "three");
        assert_eq!(lines[1].cells[0].x, 0.);
        assert_eq!(lines[1].index_at(0.), 8);
    }

    #[test]
    fn a_soft_wrap_retains_the_side_reached_by_line_end_or_line_start() {
        let lines = layout_lines("one two", Language::English, false, Some(25.));
        let offset = lines[1].start;
        assert_eq!(lines[0].end, offset);
        assert_eq!(line_index(&lines, offset, true), 0);
        assert_eq!(line_index(&lines, offset, false), 1);
    }

    #[test]
    fn explicit_newlines_keep_empty_lines_and_a_final_caret_line() {
        let lines = layout_lines("a\n\n中\n", Language::English, false, Some(100.));
        assert_eq!(
            lines
                .iter()
                .map(|line| line.start..line.end)
                .collect::<Vec<_>>(),
            vec![0..1, 2..2, 3..6, 7..7]
        );
    }

    #[test]
    fn password_cells_retain_grapheme_offsets_with_equal_display_widths() {
        let lines = layout_lines("a👩‍💻e\u{301}", Language::English, true, None);
        assert_eq!(
            lines[0]
                .cells
                .iter()
                .map(|cell| cell.range.clone())
                .collect::<Vec<_>>(),
            vec![0..1, 1..12, 12..15]
        );
        let widths = lines[0]
            .cells
            .iter()
            .map(|cell| cell.width)
            .collect::<Vec<_>>();
        assert_eq!(widths[0], widths[1]);
        assert_eq!(widths[1], widths[2]);
    }

    #[test]
    fn rich_layout_uses_scaled_line_heights_and_aligned_hit_positions() {
        let text = "small\nBIG";
        let mut rich = RichFormat::new(text);
        rich.styles[6..].fill(TextStyle {
            bold: true,
            scale: 3,
            ..TextStyle::default()
        });
        rich.alignments[1] = Alignment::Right;
        let lines = layout_styled_lines(text, Language::English, false, Some(180.), Some(&rich));
        assert_eq!(lines[0].height, 14.);
        assert_eq!(lines[1].height, 40.);
        assert_eq!(lines[1].y, 14.);
        assert_eq!(lines[1].width + lines[1].offset_x, 180.);
        assert_eq!(lines[1].index_at(lines[1].offset_x), 6);
        assert_eq!(lines[1].x_at(text.len()), 180.);
        assert_eq!(
            lines[1].cells[0].width,
            Font::SmallBold
                .glyph_or_replacement('B', Language::English)
                .advance as f32
                * 3.
        );
    }

    #[test]
    fn scaled_italic_drawing_uses_pixel_blocks_and_keeps_cjk_upright() {
        let mut plain = crate::paint::Pen::new(100, 100);
        let mut italic = crate::paint::Pen::new(100, 100);
        let style = TextStyle {
            italic: true,
            scale: 2,
            ..TextStyle::default()
        };
        paint_styled_text(
            &mut plain,
            0,
            30,
            "A中",
            Language::English,
            BLACK,
            TextStyle::default().scaled(2),
        );
        paint_styled_text(&mut italic, 0, 30, "A中", Language::English, BLACK, style);
        assert!(
            italic
                .fills
                .iter()
                .all(|fill| fill.rect.height == 2 && fill.rect.width % 2 == 0)
        );
        assert_ne!(plain.fills[0].rect.x, italic.fills[0].rect.x);
        assert_eq!(
            plain.fills.last().unwrap().rect,
            italic.fills.last().unwrap().rect
        );
    }
}

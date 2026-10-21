use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, ImageSource, KeyDownEvent, MouseButton,
    Role, SharedString, Window, div, img, prelude::*, px,
};

use crate::{
    Font, Language, ScrollBar, Theme,
    paint::{Pen, drawing},
    theme::*,
};

const CELL_WIDTH: i32 = 84;
const CELL_HEIGHT: i32 = 64;
const GRID_TOP: i32 = 4;
const SCROLL_BAR_WIDTH: i32 = 16;
/// The scroll bar's arrows move a quarter of a row.
const SCROLL_STEP: i32 = CELL_HEIGHT / 4;

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

#[derive(Clone)]
pub struct IconItem {
    pub title: SharedString,
    pub kind: IconKind,
    image: Option<ImageSource>,
    selected_image: Option<ImageSource>,
    data: Option<usize>,
    draggable: bool,
}
impl IconItem {
    pub fn new(title: impl Into<SharedString>, kind: IconKind) -> Self {
        Self {
            title: title.into(),
            kind,
            image: None,
            selected_image: None,
            data: None,
            draggable: false,
        }
    }
    /// Replaces the built-in icon with a custom image, fitted into 32 pixels.
    pub fn image(mut self, image: impl Into<ImageSource>) -> Self {
        self.image = Some(image.into());
        self
    }
    pub fn from_image(title: impl Into<SharedString>, image: impl Into<ImageSource>) -> Self {
        Self::new(title, IconKind::Document).image(image)
    }
    /// Supplies a selected variant while preserving the image's transparency.
    pub fn selected_image(mut self, image: impl Into<ImageSource>) -> Self {
        self.selected_image = Some(image.into());
        self
    }
    pub fn data(mut self, data: usize) -> Self {
        self.data = Some(data);
        self
    }
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }
}

#[derive(Clone)]
pub struct IconDrag {
    pub data: usize,
    title: SharedString,
    kind: IconKind,
    language: Language,
}
impl Render for IconDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let title = self.title.clone();
        let kind = self.kind;
        let language = self.language;
        drawing(84., 64., move |pen, _, _| {
            pen.language = language;
            draw_icon(pen, 26, 4, kind, true);
            let name = fit_title(&title, 80, language);
            let width = Font::Small.measure_in(&name, language);
            pen.rect((84 - width) / 2 - 2, 38, width + 4, 13, BLACK);
            pen.text((84 - width) / 2, 48, &name, Font::Small, WHITE);
        })
    }
}

impl std::fmt::Debug for IconItem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IconItem")
            .field("title", &self.title)
            .field("kind", &self.kind)
            .field("custom_image", &self.image.is_some())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconEvent {
    Selected(usize),
    Activated(usize),
    Dropped {
        source: usize,
        target: Option<usize>,
        copying: bool,
    },
    DragHovered(Option<usize>),
}

pub struct IconView {
    focus: FocusHandle,
    items: Vec<IconItem>,
    selected: Option<usize>,
    width: i32,
    height: i32,
    frame: bool,
    always_scrollbar: bool,
    offset: i32,
    drag_hover: Option<usize>,
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
            frame: true,
            always_scrollbar: false,
            offset: 0,
            drag_hover: None,
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
    /// Draws the sunken outline used by a standalone icon view.
    pub fn frame(mut self, frame: bool) -> Self {
        self.frame = frame;
        self
    }
    /// Reserves a permanent scrollbar, as a folder window does. Otherwise the
    /// bar appears only when the rows do not all fit.
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
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = (index < self.items.len()).then_some(index);
        self
    }
    pub fn selection(&self) -> Option<usize> {
        self.selected
    }
    /// Updates the selection without emitting a user selection event.
    pub fn set_selection(&mut self, selected: Option<usize>, cx: &mut Context<Self>) {
        self.selected = selected.filter(|index| *index < self.items.len());
        self.reveal_selection();
        cx.notify();
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
    fn layout(&self) -> IconLayout {
        icon_layout(
            self.width,
            self.height,
            self.frame,
            self.always_scrollbar,
            self.items.len(),
        )
    }
    fn columns(&self) -> usize {
        self.layout().columns
    }
    fn inset(&self) -> i32 {
        if self.frame { 2 } else { 0 }
    }
    fn content_width(&self) -> i32 {
        self.layout().width
    }
    fn content_viewport_height(&self) -> i32 {
        self.layout().height
    }
    fn content_height(&self) -> i32 {
        self.layout().total
    }
    fn max_offset(&self) -> i32 {
        (self.content_height() - self.content_viewport_height()).max(0)
    }
    pub fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.items.len() {
            return;
        }
        if self.selected != Some(index) {
            self.selected = Some(index);
            cx.emit(IconEvent::Selected(index));
        }
        self.reveal_selection();
        cx.notify();
    }
    fn reveal_selection(&mut self) {
        let Some(index) = self.selected else {
            return;
        };
        let top = GRID_TOP + (index / self.columns()) as i32 * CELL_HEIGHT;
        let bottom = top + CELL_HEIGHT;
        let height = self.content_viewport_height();
        if top < self.offset {
            self.offset = top;
        } else if bottom > self.offset + height {
            self.offset = top.min(bottom - height);
        }
        self.offset = self.offset.clamp(0, self.max_offset());
    }
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        let next = match event.keystroke.key.as_str() {
            "enter" => {
                if let Some(index) = self.selected {
                    cx.emit(IconEvent::Activated(index));
                }
                cx.stop_propagation();
                return;
            }
            key => {
                let layout = self.layout();
                let Some(index) = icon_key_target(
                    self.selected,
                    key,
                    self.items.len(),
                    layout.columns,
                    layout.page(),
                ) else {
                    return;
                };
                index
            }
        };
        self.select(next, cx);
        cx.stop_propagation();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IconLayout {
    width: i32,
    height: i32,
    columns: usize,
    total: i32,
    scrolls: bool,
}

impl IconLayout {
    /// Page Up and Down move by the rows in view, less one.
    fn page(&self) -> usize {
        (self.height / CELL_HEIGHT - 1).max(1) as usize * self.columns
    }
}

/// Lays the grid out in the box: rows start four down and leave four under
/// the last, and when they do not all fit a scroll bar stands at the right,
/// the columns filling what is left.
fn icon_layout(
    width: i32,
    height: i32,
    frame: bool,
    always_scrollbar: bool,
    count: usize,
) -> IconLayout {
    let inset = if frame { 2 } else { 0 };
    let total = |columns: usize| {
        if count == 0 {
            0
        } else {
            GRID_TOP * 2 + count.div_ceil(columns) as i32 * CELL_HEIGHT
        }
    };
    let mut layout = IconLayout {
        width: (width - 2 * inset).max(0),
        height: (height - 2 * inset).max(0),
        columns: 1,
        total: 0,
        scrolls: always_scrollbar,
    };
    layout.columns = (layout.width / CELL_WIDTH).max(1) as usize;
    layout.total = total(layout.columns);
    layout.scrolls |= layout.total > layout.height;
    if layout.scrolls {
        // A framed view's bar shares the content's right outline.
        let bar = if frame {
            SCROLL_BAR_WIDTH - 1
        } else {
            SCROLL_BAR_WIDTH
        };
        layout.width = (layout.width - bar).max(0);
        layout.columns = (layout.width / CELL_WIDTH).max(1) as usize;
        layout.total = total(layout.columns);
    }
    layout
}

fn icon_key_target(
    selected: Option<usize>,
    key: &str,
    count: usize,
    columns: usize,
    page: usize,
) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let at = selected.unwrap_or(0);
    let target = match key {
        "left" | "right" | "up" | "down" | "pageup" | "pagedown" if selected.is_none() => 0,
        "left" => at.saturating_sub(1),
        "right" => at.saturating_add(1).min(count - 1),
        "up" => at.saturating_sub(columns),
        "down" => at.saturating_add(columns).min(count - 1),
        "pageup" => at.saturating_sub(page),
        "pagedown" => at.saturating_add(page).min(count - 1),
        "home" => 0,
        "end" => count - 1,
        _ => return None,
    };
    Some(target)
}

fn icon_index_at(x: i32, y: i32, columns: usize, count: usize) -> Option<usize> {
    if x < 0 || y < GRID_TOP || x >= columns as i32 * CELL_WIDTH {
        return None;
    }
    let index = ((y - GRID_TOP) / CELL_HEIGHT) as usize * columns + (x / CELL_WIDTH) as usize;
    (index < count).then_some(index)
}

impl Render for IconView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let layout = self.layout();
        let columns = layout.columns;
        let offset = self.offset;
        let theme = self.theme;
        let language = self.language;
        let focused = self.focus.is_focused(window);
        let width = self.width;
        let height = self.height;
        let inset = self.inset();
        let viewport_height = layout.height;
        let bar_top = if self.frame { 1 } else { 0 };
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
            .relative()
            .p(px(inset as f32))
            .bg(gpui::rgb(WHITE))
            .on_key_down(cx.listener(Self::key_down))
            .on_drag_move::<IconDrag>(cx.listener(
                |this, event: &gpui::DragMoveEvent<IconDrag>, _, cx| {
                    let local = event.event.position - event.bounds.origin;
                    let x = f32::from(local.x) as i32 - this.inset();
                    let y = f32::from(local.y) as i32 - this.inset();
                    let hovered = (x >= 0
                        && x < this.content_width()
                        && y >= 0
                        && y < this.content_viewport_height())
                    .then(|| icon_index_at(x, y + this.offset, this.columns(), this.items.len()))
                    .flatten();
                    if this.drag_hover != hovered {
                        this.drag_hover = hovered;
                        cx.emit(IconEvent::DragHovered(hovered));
                        cx.notify();
                    }
                },
            ))
            .on_drop(cx.listener(|this, drag: &IconDrag, window, cx| {
                cx.emit(IconEvent::Dropped {
                    source: drag.data,
                    target: None,
                    copying: window.modifiers().alt,
                });
                this.drag_hover = None;
                cx.emit(IconEvent::DragHovered(None));
                cx.notify();
                cx.stop_propagation();
            }))
            .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _, cx| {
                this.offset = (this.offset
                    - f32::from(event.delta.pixel_delta(px(16.)).y).round() as i32)
                    .clamp(0, this.max_offset());
                cx.notify();
                cx.stop_propagation();
            }))
            .when(self.frame, |view| {
                view.child(div().absolute().inset_0().child(drawing(
                    width as f32,
                    height as f32,
                    move |pen, _, _| {
                        pen.bevel(0, 0, width, height, GRAY_7, WHITE);
                        pen.frame(1, 1, width - 2, height - 2, BLACK);
                        if focused && !crate::window::content_is_inactive() {
                            crate::widgets::draw_focus_ring(pen, 0, theme.accent.dark);
                        }
                    },
                )))
            })
            .child(
                div()
                    .relative()
                    .w(px(layout.width as f32))
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
                                let top =
                                    GRID_TOP + (*index / columns) as i32 * CELL_HEIGHT - offset;
                                top + CELL_HEIGHT > 0 && top < viewport_height
                            })
                            .map(|(index, item)| {
                                let title = item.title.clone();
                                let kind = item.kind;
                                let selected =
                                    self.selected == Some(index) || self.drag_hover == Some(index);
                                let image = if selected {
                                    item.selected_image.as_ref().or(item.image.as_ref())
                                } else {
                                    item.image.as_ref()
                                }
                                .cloned();
                                let custom_image = image.is_some();
                                div()
                                    .id(index)
                                    .role(Role::ListBoxOption)
                                    .aria_label(title.clone())
                                    .aria_selected(selected)
                                    .absolute()
                                    .left(px((index % columns) as f32 * CELL_WIDTH as f32))
                                    .top(px((GRID_TOP + (index / columns) as i32 * CELL_HEIGHT
                                        - offset)
                                        as f32))
                                    .w(px(CELL_WIDTH as f32))
                                    .h(px(CELL_HEIGHT as f32))
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
                                            },
                                        ),
                                    )
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        cx.listener(move |this, _, window, cx| {
                                            window.focus(&this.focus, cx);
                                            this.select(index, cx);
                                        }),
                                    )
                                    .on_drop(cx.listener(
                                        move |this, drag: &IconDrag, window, cx| {
                                            cx.emit(IconEvent::Dropped {
                                                source: drag.data,
                                                target: Some(index),
                                                copying: window.modifiers().alt,
                                            });
                                            this.drag_hover = None;
                                            cx.emit(IconEvent::DragHovered(None));
                                            cx.notify();
                                            cx.stop_propagation();
                                        },
                                    ))
                                    .when(item.draggable && item.data.is_some(), |cell| {
                                        let drag = IconDrag {
                                            data: item.data.unwrap(),
                                            title: item.title.clone(),
                                            kind: item.kind,
                                            language,
                                        };
                                        cell.on_drag(drag, |drag, _, _, cx| {
                                            cx.new(|_| drag.clone())
                                        })
                                    })
                                    .child(drawing(84., 64., move |pen, _, _| {
                                        pen.language = language;
                                        if !custom_image {
                                            draw_icon(pen, 26, 4, kind, selected);
                                        }
                                        let name = fit_title(&title, 80, language);
                                        let width = Font::Small.measure_in(&name, language);
                                        pen.rect(
                                            (84 - width) / 2 - 2,
                                            38,
                                            width + 4,
                                            13,
                                            if selected {
                                                if focused && !crate::window::content_is_inactive()
                                                {
                                                    theme.accent.dark
                                                } else {
                                                    BLACK
                                                }
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
                                    .when_some(image, |cell, image| {
                                        cell.child(
                                            div()
                                                .absolute()
                                                .left(px(26.))
                                                .top(px(4.))
                                                .size(px(32.))
                                                .child(img(image).size_full()),
                                        )
                                    })
                            }),
                    ),
            )
            .when(layout.scrolls, |view| {
                view.child(
                    div()
                        .absolute()
                        .left(px((inset + layout.width) as f32))
                        .top(px(bar_top as f32))
                        .child(
                            ScrollBar::new(
                                "icon-scroll",
                                self.offset as f64,
                                viewport_height as f64,
                                layout.total as f64,
                            )
                            .length(height - 2 * bar_top)
                            .step(SCROLL_STEP as f64)
                            .tab_stop(false)
                            .theme(theme)
                            .on_change(move |value, _, cx| {
                                scroll_entity.update(cx, |this, cx| {
                                    this.offset = value.round() as i32;
                                    cx.notify();
                                });
                            }),
                        ),
                )
            })
    }
}

pub(super) fn fit_title(title: &str, width: i32, language: Language) -> String {
    Font::Small.truncate_in(title, width, language)
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Modifiers, TestAppContext, point};

    #[test]
    fn grid_hit_testing_keeps_the_top_gutter_and_row_boundaries() {
        assert_eq!(icon_index_at(20, 3, 2, 5), None);
        assert_eq!(icon_index_at(20, 4, 2, 5), Some(0));
        assert_eq!(icon_index_at(20, 67, 2, 5), Some(0));
        assert_eq!(icon_index_at(20, 68, 2, 5), Some(2));
        assert_eq!(icon_index_at(84, 132, 2, 5), None);
        assert_eq!(icon_index_at(168, 4, 2, 5), None);
    }

    #[test]
    fn first_arrow_selects_the_first_icon_and_subsequent_arrows_follow_the_grid() {
        for key in ["left", "right", "up", "down", "pageup", "pagedown"] {
            assert_eq!(icon_key_target(None, key, 8, 3, 3), Some(0));
        }
        assert_eq!(icon_key_target(Some(0), "down", 8, 3, 3), Some(3));
        assert_eq!(icon_key_target(Some(6), "right", 8, 3, 3), Some(7));
        assert_eq!(icon_key_target(Some(7), "down", 8, 3, 3), Some(7));
        assert_eq!(icon_key_target(None, "right", 0, 3, 3), None);
    }

    #[test]
    fn page_keys_move_by_the_rows_in_view_less_one() {
        // 260×110 shows one whole row in two columns: a page is one row.
        let short = icon_layout(260, 110, true, false, 12);
        assert_eq!(short.page(), 2);
        // 280×300 shows four rows in three columns: a page is three rows.
        let tall = icon_layout(280, 300, true, false, 24);
        assert_eq!(tall.page(), 9);
        assert_eq!(icon_key_target(Some(0), "pagedown", 12, 2, 2), Some(2));
        assert_eq!(icon_key_target(Some(2), "pageup", 12, 2, 2), Some(0));
        assert_eq!(icon_key_target(Some(10), "pagedown", 12, 2, 2), Some(11));
        assert_eq!(icon_key_target(Some(1), "pageup", 12, 2, 2), Some(0));
    }

    #[test]
    fn a_scroll_bar_appears_when_the_rows_do_not_fit() {
        // Two rows in two columns fit 206×140 exactly, as in the gallery.
        assert_eq!(
            icon_layout(206, 140, true, false, 4),
            IconLayout {
                width: 202,
                height: 136,
                columns: 2,
                total: 136,
                scrolls: false,
            }
        );
        // At 130 they do not, and the bar takes the room of its width less
        // the outline it shares.
        let scrolled = icon_layout(206, 130, true, false, 4);
        assert!(scrolled.scrolls);
        assert_eq!(scrolled.width, 187);
        assert_eq!(scrolled.columns, 2);
        // A bar can cost a column, and with it a row.
        let fits = icon_layout(280, 300, true, false, 12);
        assert_eq!((fits.scrolls, fits.columns, fits.total), (false, 3, 264));
        let narrowed = icon_layout(260, 110, true, false, 12);
        assert_eq!(
            (narrowed.scrolls, narrowed.width, narrowed.columns),
            (true, 241, 2)
        );
        assert_eq!(narrowed.total, 8 + 6 * CELL_HEIGHT);
        // An unframed view with a permanent bar gives it the full width.
        let folder = icon_layout(328, 174, false, true, 1);
        assert_eq!((folder.scrolls, folder.width), (true, 312));
        assert_eq!(icon_layout(206, 130, true, false, 0).total, 0);
    }

    fn numbered(count: usize) -> Vec<IconItem> {
        (1..=count)
            .map(|index| IconItem::new(format!("Icon {index}"), IconKind::Document))
            .collect()
    }

    #[gpui::test]
    fn page_keys_choose_a_page_away_and_scroll_it_into_view(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| IconView::new(numbered(12), cx).size(260, 110));
        cx.update(|window, cx| window.focus(&view.focus_handle(cx), cx));
        for (key, selection, offset) in [
            ("right", 0, 0),
            ("pagedown", 2, 26),
            ("pagedown", 4, 90),
            ("end", 11, 282),
            ("pageup", 9, 260),
            ("pageup", 7, 196),
            ("home", 0, 4),
        ] {
            cx.simulate_keystrokes(key);
            cx.read_entity(&view, |view, _| {
                assert_eq!(
                    (view.selection(), view.offset),
                    (Some(selection), offset),
                    "{key}"
                );
            });
        }
    }

    #[gpui::test]
    fn the_scroll_bar_arrow_steps_a_quarter_row(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| IconView::new(numbered(12), cx).size(280, 110));
        cx.run_until_parked();
        cx.simulate_click(point(px(271.), px(101.)), Modifiers::default());
        cx.read_entity(&view, |view, _| {
            assert_eq!(SCROLL_STEP, 16);
            assert_eq!(view.offset, SCROLL_STEP);
            assert_eq!(view.selection(), None);
        });
    }
}

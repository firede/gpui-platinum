//! A list box with one keyboard focus stop and scrollable rows.

use crate::{
    Font, Language, ScrollBar, Theme,
    paint::{Pen, Rect, drawing},
    theme::*,
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

/// The height of a row of a list box.
pub const LIST_ROW_HEIGHT: i32 = 16;
const LIST_TEXT_LEFT: i32 = 4;
const SCROLL_BAR_WIDTH: i32 = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem {
    pub label: SharedString,
    pub disabled: bool,
}

impl From<&str> for ListItem {
    fn from(label: &str) -> Self {
        Self {
            label: label.to_owned().into(),
            disabled: false,
        }
    }
}

impl From<String> for ListItem {
    fn from(label: String) -> Self {
        Self {
            label: label.into(),
            disabled: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListEvent {
    Selected(usize),
    Activated(usize),
}

pub struct ListBox {
    focus: FocusHandle,
    items: Vec<ListItem>,
    selected: Option<usize>,
    /// The first row in view, in rows; the wheel moves it by fractions.
    top: f64,
    width: i32,
    height: i32,
    /// Where the layout put the box, from the last frame.
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    fill_height: bool,
    dragging: bool,
    label: SharedString,
    theme: Theme,
    language: Language,
    prefix: String,
    typed_at: Instant,
    disabled: bool,
    focus_ring: bool,
    always_scrollbar: bool,
    arrows: Vec<bool>,
    icons: Vec<Vec<String>>,
}

impl EventEmitter<ListEvent> for ListBox {}

impl ListBox {
    pub fn new(
        items: impl IntoIterator<Item = impl Into<ListItem>>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            items: items.into_iter().map(Into::into).collect(),
            selected: None,
            top: 0.,
            width: 200,
            height: 120,
            bounds: Rc::new(Cell::new(None)),
            fill_height: false,
            dragging: false,
            label: "List".into(),
            theme: Theme::default(),
            language: Language::English,
            prefix: String::new(),
            typed_at: cx.background_executor().now(),
            disabled: false,
            focus_ring: true,
            always_scrollbar: false,
            arrows: Vec::new(),
            icons: Vec::new(),
        }
    }
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    pub fn fill_height(mut self) -> Self {
        self.fill_height = true;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn focus_ring(mut self, enabled: bool) -> Self {
        self.focus_ring = enabled;
        self
    }
    pub fn scroll_bar_always(mut self) -> Self {
        self.always_scrollbar = true;
        self
    }
    pub fn arrows(mut self, arrows: Vec<bool>) -> Self {
        self.arrows = arrows;
        self
    }
    pub fn icons<I, S>(mut self, icons: impl IntoIterator<Item = I>) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.icons = icons
            .into_iter()
            .map(|icon| icon.into_iter().map(Into::into).collect())
            .collect();
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
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        cx.notify();
    }
    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = self
            .items
            .get(selected)
            .filter(|item| !item.disabled)
            .map(|_| selected);
        self
    }
    pub fn selection(&self) -> Option<usize> {
        self.selected
    }
    pub fn items(&self) -> &[ListItem] {
        &self.items
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.label = label.into();
        cx.notify();
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        cx.notify();
    }
    pub fn set_arrows(&mut self, arrows: Vec<bool>, cx: &mut Context<Self>) {
        self.arrows = arrows;
        cx.notify();
    }
    pub fn set_icons(&mut self, icons: Vec<Vec<String>>, cx: &mut Context<Self>) {
        self.icons = icons;
        cx.notify();
    }
    pub fn set_selection(&mut self, selected: Option<usize>, cx: &mut Context<Self>) {
        self.selected =
            selected.filter(|index| self.items.get(*index).is_some_and(|item| !item.disabled));
        if let Some(index) = self.selected {
            self.reveal(index);
        }
        cx.notify();
    }
    pub fn set_items(&mut self, items: Vec<ListItem>, cx: &mut Context<Self>) {
        self.items = items;
        if self
            .selected
            .is_some_and(|i| self.items.get(i).is_none_or(|item| item.disabled))
        {
            self.selected = None;
        }
        self.top = self.top.min(self.max_top() as f64);
        cx.notify();
    }
    pub fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.items.get(index).is_none_or(|item| item.disabled) {
            return;
        }
        if self.selected != Some(index) {
            self.selected = Some(index);
            cx.emit(ListEvent::Selected(index));
        }
        self.reveal(index);
        cx.notify();
    }

    /// The box's height: its own, or the one its parent gave it.
    fn box_height(&self) -> i32 {
        match self.bounds.get() {
            Some(bounds) if self.fill_height => f32::from(bounds.size.height).round() as i32,
            _ => self.height,
        }
    }

    /// The rows that show whole.
    fn rows(&self) -> usize {
        ((self.box_height() - 4) / LIST_ROW_HEIGHT).max(1) as usize
    }

    fn max_top(&self) -> usize {
        self.items.len().saturating_sub(self.rows())
    }

    /// Scrolls the fewest rows that bring the row into view.
    fn reveal(&mut self, index: usize) {
        let rows = self.rows();
        if (index as f64) < self.top {
            self.top = index as f64;
        } else if index as f64 >= self.top + rows as f64 {
            self.top = (index + 1 - rows) as f64;
        }
    }

    fn scrolls(&self) -> bool {
        self.items.len() > self.rows()
    }

    /// The row under a point of the box, from its top.
    fn row_at(&self, y: f32) -> usize {
        let row = ((y - 2.) / LIST_ROW_HEIGHT as f32).floor().max(0.) as usize;
        self.top as usize + row
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let count = self.items.len();
        if count == 0 {
            return;
        }
        let current = self.selected.unwrap_or(0);
        let page = self.rows().saturating_sub(1).max(1);
        let key = event.keystroke.key.as_str();
        let target = match key {
            "up" => Some(current.saturating_sub(1)),
            "down" => Some(
                self.selected
                    .map_or(0, |current| (current + 1).min(count - 1)),
            ),
            "home" => Some(0),
            "end" => Some(count - 1),
            "pageup" => Some(current.saturating_sub(page)),
            "pagedown" => Some((current + page).min(count - 1)),
            "enter" => {
                if let Some(index) = self.selected {
                    cx.emit(ListEvent::Activated(index));
                }
                cx.stop_propagation();
                return;
            }
            _ => None,
        };
        if let Some(mut index) = target {
            let backwards = matches!(key, "up" | "home" | "pageup");
            while self.items[index].disabled {
                if backwards && index > 0 {
                    index -= 1;
                } else if !backwards && index + 1 < count {
                    index += 1;
                } else {
                    return;
                }
            }
            self.select(index, cx);
            cx.stop_propagation();
        } else if !event.keystroke.modifiers.platform
            && !event.keystroke.modifiers.control
            && !event.keystroke.modifiers.alt
            && let Some(text) = &event.keystroke.key_char
        {
            let now = cx.background_executor().now();
            if now.duration_since(self.typed_at) > Duration::from_secs(1) {
                self.prefix.clear();
            }
            self.prefix.push_str(&text.to_lowercase());
            self.typed_at = now;
            if let Some(index) = self.items.iter().position(|item| {
                !item.disabled && item.label.to_lowercase().starts_with(&self.prefix)
            }) {
                self.select(index, cx);
            }
        }
    }
}

impl Focusable for ListBox {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// Draws the rows of a list box from row `top` in the pen's box, row
/// `selected` highlighted: in the accent while the list has the keyboard
/// focus, in gray otherwise.
pub fn draw_list_box(
    pen: &mut Pen,
    items: &[&str],
    top: usize,
    selected: Option<usize>,
    focused: bool,
    disabled: bool,
    accent: crate::Accent,
) {
    let items = items
        .iter()
        .map(|&label| ListItem::from(label))
        .collect::<Vec<_>>();
    draw_list_rows(
        pen,
        &items,
        &[],
        top,
        selected,
        focused,
        disabled,
        accent,
        pen.language,
    );
}

/// Draws the white, black-framed rows of a list, each with its icon when
/// there are icons, the rows taking no choosing in Gray8.
#[allow(clippy::too_many_arguments)]
fn draw_list_rows(
    pen: &mut Pen,
    items: &[ListItem],
    icons: &[Vec<String>],
    top: usize,
    selected: Option<usize>,
    focused: bool,
    disabled: bool,
    accent: crate::Accent,
    language: Language,
) {
    let (width, height) = (pen.width, pen.height);
    pen.rect(1, 1, width - 2, height - 2, WHITE);
    pen.frame(0, 0, width, height, BLACK);
    let mut rows = Pen::new(width, height);
    rows.language = language;
    for (index, item) in items.iter().enumerate().skip(top) {
        let y = 1 + (index - top) as i32 * LIST_ROW_HEIGHT;
        if y >= height - 1 {
            break;
        }
        let mut ink = if disabled || item.disabled {
            GRAY_8
        } else {
            BLACK
        };
        let mut highlighted = false;
        if selected == Some(index) {
            let mut face = GRAY_4;
            if focused && !disabled {
                (face, ink, highlighted) = (accent.dark, WHITE, true);
            }
            rows.rect(1, y, width - 2, LIST_ROW_HEIGHT, face);
        }
        let mut x = LIST_TEXT_LEFT;
        if let Some(icon) = icons.get(index).filter(|icon| !icon.is_empty()) {
            let icon = icon.iter().map(String::as_str).collect::<Vec<_>>();
            crate::bitmap_icons::draw_bitmap_icon(
                &mut rows,
                &icon,
                x - 1,
                y + (LIST_ROW_HEIGHT - icon.len() as i32) / 2,
                accent,
                highlighted,
                item.disabled,
            );
        }
        if !icons.is_empty() {
            x += 19;
        }
        let label = Font::Small.truncate_in(&item.label, width - x - LIST_TEXT_LEFT, language);
        text_middle(&mut rows, x, y, LIST_ROW_HEIGHT, &label, Font::Small, ink);
    }
    rows.clip(Rect {
        x: 1,
        y: 1,
        width: width - 2,
        height: height - 2,
    });
    pen.append(rows, 0, 0);
}

/// The capital height of a face, as the original sets it.
pub(crate) fn cap_height(font: Font) -> i32 {
    match font {
        Font::Small | Font::SmallBold => 8,
        Font::Sans | Font::Bold => 9,
    }
}

/// Draws text from x, its capitals centered on the middle of the `height`
/// pixels from y, and returns the x after it.
pub(crate) fn text_middle(
    pen: &mut Pen,
    x: i32,
    y: i32,
    height: i32,
    text: &str,
    font: Font,
    color: u32,
) {
    let cap = cap_height(font);
    pen.text(x, y + (height - cap) / 2 + cap, text, font, color);
}

/// Draws text centered in the box, its capitals on the box's middle.
#[allow(clippy::too_many_arguments)]
pub(crate) fn text_centered(
    pen: &mut Pen,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    text: &str,
    font: Font,
    color: u32,
) {
    let text_width = font.measure_in(text, pen.language);
    text_middle(
        pen,
        x + (width - text_width) / 2,
        y,
        height,
        text,
        font,
        color,
    );
}

const ROW_ARROW: [&str; 7] = ["#...", "##..", "###.", "####", "###.", "##..", "#..."];

/// Draws a list box filling the pen: its sunken bevel round the rows from
/// `top`, in the room the scroll bar leaves when `with_bar`, with an arrow
/// at the right of each row `arrows` marks.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_list_frame(
    pen: &mut Pen,
    items: &[ListItem],
    icons: &[Vec<String>],
    arrows: &[bool],
    top: usize,
    selected: Option<usize>,
    focused: bool,
    disabled: bool,
    accent: crate::Accent,
    with_bar: bool,
) {
    let language = pen.language;
    let list_width = pen.width - 2 - if with_bar { SCROLL_BAR_WIDTH - 1 } else { 0 };
    pen.bevel(0, 0, pen.width, pen.height, GRAY_7, WHITE);
    let inner_height = pen.height - 2;
    let mut list = Pen::new(list_width, inner_height);
    draw_list_rows(
        &mut list, items, icons, top, selected, focused, disabled, accent, language,
    );
    for (index, &arrow) in arrows.iter().enumerate().skip(top) {
        let y = 1 + (index - top) as i32 * LIST_ROW_HEIGHT;
        if !arrow || y + LIST_ROW_HEIGHT > inner_height - 1 {
            continue;
        }
        let ink = if selected == Some(index) && focused {
            WHITE
        } else {
            BLACK
        };
        for (dy, row) in ROW_ARROW.iter().enumerate() {
            let ink_width = row.bytes().filter(|&b| b == b'#').count() as i32;
            list.hline(
                list_width - 11,
                y + (LIST_ROW_HEIGHT - 7) / 2 + dy as i32,
                ink_width,
                ink,
            );
        }
    }
    pen.append(list, 1, 1);
}

impl Render for ListBox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window) && !self.disabled;
        let accent = self.theme.accent;
        let language = self.language;
        let height = self.box_height();
        let rows = self.rows();
        let scrolls = self.scrolls();
        let with_bar = scrolls || self.always_scrollbar;
        self.top = self.top.clamp(0., self.max_top() as f64);
        let top = self.top as usize;
        let items = self.items.clone();
        let icons = self.icons.clone();
        let arrows = self.arrows.clone();
        let selected = self.selected;
        let disabled = self.disabled;
        let ring = self.focus_ring;
        let fill_height = self.fill_height;
        let laid_out = self.bounds.clone();
        let list_width = self.width - 2 - if with_bar { SCROLL_BAR_WIDTH - 1 } else { 0 };
        let entity = cx.entity();
        let count = self.items.len();
        div()
            .id("list")
            .role(Role::ListBox)
            .aria_label(self.label.clone())
            .track_focus(&self.focus)
            .tab_stop(!self.disabled)
            .relative()
            .w(px(self.width as f32))
            .when(self.fill_height, |el| el.h_full())
            .when(!self.fill_height, |el| el.h(px(self.height as f32)))
            .flex_shrink_0()
            .on_key_down(cx.listener(Self::key_down))
            .on_scroll(cx.listener(move |this, event: &ScrollEvent, _, cx| {
                let delta = f32::from(event.delta.pixel_delta(px(16.)).y) as f64;
                this.top =
                    (this.top - delta / LIST_ROW_HEIGHT as f64).clamp(0., this.max_top() as f64);
                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    let Some(bounds) = this.focus_bounds(window) else {
                        return;
                    };
                    let local = event.position - bounds.origin;
                    if this.disabled || f32::from(local.x) >= (list_width + 1) as f32 {
                        return;
                    }
                    let row = this.row_at(f32::from(local.y));
                    if row < this.items.len() && !this.items[row].disabled {
                        this.select(row, cx);
                        if event.click_count == 2 {
                            cx.emit(ListEvent::Activated(row));
                        }
                    }
                    this.dragging = true;
                }),
            )
            .on_mouse_move(
                cx.listener(move |this, event: &MouseMoveEvent, window, cx| {
                    if !this.dragging || event.pressed_button != Some(MouseButton::Left) {
                        return;
                    }
                    let Some(bounds) = this.focus_bounds(window) else {
                        return;
                    };
                    let height = this.box_height() as f32;
                    let y = f32::from(event.position.y - bounds.origin.y).clamp(2., height - 3.);
                    let row = this.row_at(y).min(this.items.len().saturating_sub(1));
                    if this.items.get(row).is_some_and(|item| !item.disabled) {
                        this.select(row, cx);
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = false),
            )
            .child(
                canvas(
                    move |bounds, window, _| {
                        let before = laid_out.replace(Some(bounds));
                        if fill_height && before.is_none_or(|b| b.size != bounds.size) {
                            // Laid out anew: the rows that fit change.
                            window.refresh();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(div().absolute().top_0().left_0().child(drawing(
                self.width as f32,
                height as f32,
                move |pen, _, _| {
                    let inactive = crate::window::content_is_inactive();
                    let focused = focused && !inactive;
                    pen.language = language;
                    draw_list_frame(
                        pen, &items, &icons, &arrows, top, selected, focused, disabled, accent,
                        with_bar,
                    );
                    if focused && ring {
                        crate::widgets::draw_focus_ring(pen, 0, accent.dark);
                    }
                },
            )))
            .children((top..(top + rows).min(count)).map(|index| {
                let item = &self.items[index];
                div()
                    .id(index)
                    .role(Role::ListBoxOption)
                    .aria_label(item.label.clone())
                    .aria_selected(self.selected == Some(index))
                    .absolute()
                    .left(px(1.))
                    .top(px((2 + (index - top) as i32 * LIST_ROW_HEIGHT) as f32))
                    .w(px((list_width - 2) as f32))
                    .h(px(LIST_ROW_HEIGHT as f32))
                    .on_a11y_action(AccessibleAction::Click, {
                        let entity = entity.clone();
                        let row_disabled = disabled || item.disabled;
                        move |_, window, cx| {
                            entity.update(cx, |this, cx| {
                                if row_disabled {
                                    return;
                                }
                                window.focus(&this.focus, cx);
                                this.select(index, cx);
                            });
                        }
                    })
            }))
            .when(with_bar, |root| {
                root.child(
                    div()
                        .absolute()
                        .top(px(1.))
                        .left(px((self.width - 1 - SCROLL_BAR_WIDTH) as f32))
                        .child(
                            ScrollBar::new(
                                "list-scroll",
                                self.top,
                                rows as f64,
                                count.max(rows) as f64,
                            )
                            .step(1.)
                            .length(height - 2)
                            .theme(self.theme)
                            .inactive(!scrolls)
                            .disabled(self.disabled)
                            .on_change(move |value, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.top = value;
                                    cx.notify();
                                })
                            }),
                        ),
                )
            })
    }
}

impl ListBox {
    /// Where the box is on the window, from the last frame.
    fn focus_bounds(&self, _: &Window) -> Option<Bounds<Pixels>> {
        self.bounds.get()
    }
}

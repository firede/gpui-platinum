//! Pop-up menus with keyboard selection and a floating menu surface.

use crate::{
    Font, Language, Theme,
    paint::{Pen, Rect, drawing},
    theme::*,
};
use gpui::{prelude::*, *};

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub title: SharedString,
    pub disabled: bool,
    pub separator: bool,
    pub checked: bool,
    pub key: Option<SharedString>,
    pub submenu: Vec<MenuItem>,
}

impl MenuItem {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            disabled: false,
            separator: false,
            checked: false,
            key: None,
            submenu: Vec::new(),
        }
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn separator() -> Self {
        Self {
            disabled: true,
            separator: true,
            ..Self::new("")
        }
    }
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
    pub fn shortcut(mut self, key: impl Into<SharedString>) -> Self {
        self.key = Some(key.into());
        self
    }
    pub fn submenu(mut self, items: Vec<MenuItem>) -> Self {
        self.submenu = items;
        self
    }
}

impl From<&str> for MenuItem {
    fn from(title: &str) -> Self {
        Self::new(title.to_owned())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuEvent(pub usize);

pub struct PopupMenu {
    focus: FocusHandle,
    items: Vec<MenuItem>,
    selected: usize,
    has_selection: bool,
    highlighted: Option<usize>,
    open: bool,
    disabled: bool,
    width: i32,
    label: SharedString,
    theme: Theme,
    language: Language,
}

impl EventEmitter<MenuEvent> for PopupMenu {}
impl Focusable for PopupMenu {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl PopupMenu {
    pub fn new(
        items: impl IntoIterator<Item = impl Into<MenuItem>>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            items: items.into_iter().map(Into::into).collect(),
            selected: 0,
            has_selection: true,
            highlighted: None,
            open: false,
            disabled: false,
            width: 180,
            label: "Pop-up menu".into(),
            theme: Theme::default(),
            language: Language::English,
        }
    }
    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected.min(self.items.len().saturating_sub(1));
        self.has_selection = true;
        self
    }
    pub fn width(mut self, width: i32) -> Self {
        self.width = width;
        self
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
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
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    pub fn selection(&self) -> usize {
        self.selected
    }
    pub fn set_selection(&mut self, selected: usize, cx: &mut Context<Self>) {
        self.selected = selected.min(self.items.len().saturating_sub(1));
        self.has_selection = true;
        cx.notify();
    }
    /// Shows the menu label and no checked item until another item is selected.
    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.has_selection = false;
        cx.notify();
    }
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.label = label.into();
        cx.notify();
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        if disabled {
            self.open = false;
            self.highlighted = None;
        }
        cx.notify();
    }
    pub fn set_items(&mut self, items: Vec<MenuItem>, cx: &mut Context<Self>) {
        self.items = items;
        self.selected = self.selected.min(self.items.len().saturating_sub(1));
        self.open = false;
        cx.notify();
    }
    pub fn choose(&mut self, selected: usize, cx: &mut Context<Self>) {
        if self
            .items
            .get(selected)
            .is_none_or(|item| item.disabled || item.separator)
        {
            return;
        }
        self.selected = selected;
        self.has_selection = true;
        self.open = false;
        self.highlighted = None;
        cx.emit(MenuEvent(selected));
        cx.notify();
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.items.is_empty() {
            return;
        }
        match event.keystroke.key.as_str() {
            "escape" => {
                self.open = false;
                self.highlighted = None;
            }
            "enter" | "space" => {
                if self.open {
                    if let Some(index) = self.highlighted {
                        self.choose(index, cx);
                    }
                } else {
                    self.open = true;
                    self.highlighted = Some(self.selected);
                }
            }
            "up" | "down" | "home" | "end" => {
                let was_open = self.open;
                self.open = true;
                let enabled: Vec<_> = self
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| !item.disabled && !item.separator)
                    .map(|(i, _)| i)
                    .collect();
                if enabled.is_empty() {
                    return;
                }
                if !was_open && matches!(event.keystroke.key.as_str(), "up" | "down") {
                    self.highlighted = enabled
                        .iter()
                        .copied()
                        .find(|index| self.has_selection && *index == self.selected)
                        .or_else(|| enabled.first().copied());
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                let pos = enabled
                    .iter()
                    .position(|i| Some(*i) == self.highlighted)
                    .unwrap_or(0);
                let next = match event.keystroke.key.as_str() {
                    "home" => 0,
                    "end" => enabled.len() - 1,
                    "up" => pos.saturating_sub(1),
                    _ => (pos + 1).min(enabled.len() - 1),
                };
                self.highlighted = Some(enabled[next]);
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }
}

impl Render for PopupMenu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = self.width;
        let current = if self.has_selection {
            self.items
                .get(self.selected)
                .map(|item| item.title.clone())
                .unwrap_or_default()
        } else {
            self.label.clone()
        };
        let disabled = self.disabled;
        let accent = self.theme.accent;
        let open = self.open;
        let language = self.language;
        let focus = self.focus.clone();
        let mut root = div()
            .id("popup")
            .role(Role::ComboBox)
            .aria_label(self.label.clone())
            .aria_value(current.clone())
            .aria_expanded(open)
            .relative()
            .flex_shrink_0()
            .text_color(rgb(BLACK))
            .when(!disabled, |el| {
                el.track_focus(&self.focus)
                    .tab_stop(true)
                    .active(|style| style.text_color(rgb(WHITE)))
            })
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    if this.disabled {
                        return;
                    }
                    window.focus(&this.focus, cx);
                    this.open = !this.open;
                    this.highlighted = None;
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .child(drawing(width as f32, 20., move |pen, window, _| {
                let inactive = crate::window::content_is_inactive();
                pen.language = language;
                let pressed =
                    !disabled && !inactive && window.text_style().color == rgb(WHITE).into();
                draw_popup_button(pen, &current, disabled, pressed);
                if !disabled
                    && !inactive
                    && focus.is_focused(window)
                    && window.last_input_was_keyboard()
                {
                    crate::widgets::draw_focus_ring(pen, 3, accent.dark);
                }
            }));
        if self.open {
            let selected_top: i32 = self
                .items
                .iter()
                .take(self.selected)
                .map(|item| if item.separator { 8 } else { 18 })
                .sum();
            let menu = div()
                .id("menu")
                .role(Role::Menu)
                .relative()
                .w(px(width as f32))
                .p(px(2.))
                .child(menu_frame())
                .occlude()
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.open = false;
                    cx.notify();
                    cx.stop_propagation();
                }))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .children(self.items.iter().enumerate().map(|(index, item)| {
                    let title = item.title.clone();
                    let disabled = item.disabled;
                    let separator = item.separator;
                    let selected = self.has_selection && index == self.selected;
                    let highlighted = self.highlighted == Some(index) && !disabled;
                    let mut item = item.clone();
                    item.checked = selected;
                    div()
                        .id(index)
                        .role(Role::MenuItem)
                        .aria_label(title.clone())
                        .aria_selected(selected)
                        .h(px(if separator { 8. } else { 18. }))
                        .w_full()
                        .on_mouse_move(cx.listener(move |this, _, _, cx| {
                            if !disabled && this.highlighted != Some(index) {
                                this.highlighted = Some(index);
                                cx.notify();
                            }
                        }))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.choose(index, cx);
                                cx.stop_propagation();
                            }),
                        )
                        .child(drawing(
                            (width - 4) as f32,
                            if separator { 8. } else { 18. },
                            move |pen, _, _| {
                                pen.language = language;
                                draw_menu_item(pen, &item, highlighted, accent);
                            },
                        ))
                }));
            root = root.child(
                div().absolute().left_0().top_0().child(
                    deferred(
                        anchored()
                            .anchor(Anchor::TopLeft)
                            .offset(point(px(0.), px((-selected_top - 1) as f32)))
                            .snap_to_window_with_margin(px(0.))
                            .child(menu),
                    )
                    .priority(2),
                ),
            );
        }
        root
    }
}

pub(crate) fn menu_frame() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let mut pen = Pen::new(
                f32::from(bounds.size.width).round() as i32,
                f32::from(bounds.size.height).round() as i32,
            );
            draw_menu_frame(&mut pen);
            pen.paint(bounds.origin, window);
        },
    )
    .absolute()
    .inset_0()
    .size_full()
}

/// Draws the frame shared by pop-up, context and menu bar panels.
pub fn draw_menu_frame(pen: &mut Pen) {
    pen.rect(0, 0, pen.width, pen.height, GRAY_2);
    pen.frame(0, 0, pen.width, pen.height, BLACK);
    pen.bevel(1, 1, pen.width - 2, pen.height - 2, WHITE, GRAY_7);
}

/// Draws one eighteen-pixel menu row, or an eight-pixel separator.
pub fn draw_menu_item(pen: &mut Pen, item: &MenuItem, highlighted: bool, accent: Accent) {
    if item.separator {
        pen.hline(0, 3, pen.width, GRAY_7);
        pen.hline(0, 4, pen.width, WHITE);
        return;
    }
    let ink = if item.disabled {
        GRAY_8
    } else if highlighted {
        pen.rect(0, 0, pen.width, pen.height, accent.dark);
        WHITE
    } else {
        BLACK
    };
    let baseline = (pen.height - Font::Bold.cap_height()) / 2 + Font::Bold.cap_height();
    if item.checked {
        pen.text(4, baseline, "✓", Font::Bold, ink);
    }
    pen.text(16, baseline, &item.title, Font::Bold, ink);
    if let Some(key) = &item.key {
        let shortcut = format!("⌘{}", key.to_uppercase());
        pen.text(
            pen.width - 10 - Font::Bold.measure_in(&shortcut, pen.language),
            baseline,
            &shortcut,
            Font::Bold,
            ink,
        );
    } else if !item.submenu.is_empty() {
        for row in 0..7 {
            pen.hline(
                pen.width - 12,
                (pen.height - 7) / 2 + row,
                4 - (3 - row).abs(),
                ink,
            );
        }
    }
}

/// Draws the beveled pop-up button and its separate arrow box.
pub fn draw_popup_button(pen: &mut Pen, text: &str, disabled: bool, pressed: bool) {
    crate::widgets::draw_button(pen, "", disabled, pressed);
    let inactive = crate::window::content_is_inactive();
    let (ink, line, highlight) = if disabled || inactive {
        (GRAY_8, GRAY_4, GRAY_1)
    } else if pressed {
        (WHITE, GRAY_11, GRAY_8)
    } else {
        (BLACK, GRAY_7, WHITE)
    };
    let arrow_left = pen.width - 18;
    pen.vline(arrow_left, 3, pen.height - 6, line);
    pen.vline(arrow_left + 1, 3, pen.height - 6, highlight);
    for row in 0..4 {
        pen.hline(
            arrow_left + 6 + row,
            (pen.height - 4) / 2 + row,
            7 - row * 2,
            ink,
        );
    }
    let mut label = Pen::new(pen.width, pen.height);
    label.language = pen.language;
    label.text(
        9,
        (pen.height - Font::Bold.cap_height()) / 2 + Font::Bold.cap_height(),
        &Font::Bold.truncate_in(text, arrow_left - 13, pen.language),
        Font::Bold,
        ink,
    );
    label.clip(Rect {
        x: 2,
        y: 0,
        width: arrow_left - 4,
        height: pen.height,
    });
    pen.append(label, 0, 0);
}

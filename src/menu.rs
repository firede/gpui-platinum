//! Pop-up menus with keyboard selection and a floating menu surface.

use crate::{
    Font, Language, Theme,
    paint::{Rect, drawing},
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
        if self.disabled {
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
        let mut root = div()
            .id("popup")
            .role(Role::ComboBox)
            .aria_label(self.label.clone())
            .aria_value(current.clone())
            .aria_expanded(open)
            .relative()
            .flex_shrink_0()
            .p(px(3.))
            .text_color(rgb(BLACK))
            .when(!disabled, |el| {
                el.track_focus(&self.focus)
                    .tab_stop(true)
                    .focus_visible(move |style| style.bg(rgb(accent.dark)))
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
            .child(drawing(width as f32, 20., move |pen, _, _| {
                pen.language = language;
                pen.round_frame(
                    Rect {
                        x: 0,
                        y: 0,
                        width,
                        height: 20,
                    },
                    2,
                    if disabled { GRAY_6 } else { BLACK },
                    Some(GRAY_2),
                );
                pen.bevel(1, 1, width - 2, 18, WHITE, GRAY_7);
                pen.text(
                    8,
                    14,
                    &current,
                    Font::Bold,
                    if disabled { GRAY_8 } else { BLACK },
                );
                for row in 0..4 {
                    pen.hline(
                        width - 15 + row,
                        8 + row,
                        7 - row * 2,
                        if disabled { GRAY_8 } else { BLACK },
                    );
                }
            }));
        if self.open {
            let menu = div()
                .id("menu")
                .role(Role::Menu)
                .w(px(width as f32))
                .p(px(1.))
                .bg(rgb(GRAY_1))
                .border_1()
                .border_color(rgb(BLACK))
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
                    div()
                        .id(index)
                        .role(Role::MenuItem)
                        .aria_label(title.clone())
                        .aria_selected(selected)
                        .h(px(if separator { 6. } else { 20. }))
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
                            if separator { 6. } else { 20. },
                            move |pen, _, _| {
                                pen.language = language;
                                if separator {
                                    pen.hline(2, 2, pen.width - 4, GRAY_7);
                                    pen.hline(2, 3, pen.width - 4, WHITE);
                                    return;
                                }
                                pen.rect(
                                    0,
                                    0,
                                    pen.width,
                                    pen.height,
                                    if highlighted { accent.dark } else { GRAY_1 },
                                );
                                let ink = if disabled {
                                    GRAY_8
                                } else if highlighted {
                                    WHITE
                                } else {
                                    BLACK
                                };
                                if selected {
                                    pen.text(3, 14, "✓", Font::Small, ink);
                                }
                                pen.text(18, 14, &title, Font::Bold, ink);
                            },
                        ))
                }));
            root = root.child(
                deferred(
                    anchored()
                        .anchor(Anchor::TopLeft)
                        .snap_to_window_with_margin(px(4.))
                        .child(menu),
                )
                .priority(2),
            );
        }
        root
    }
}

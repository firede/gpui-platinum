//! A list box with one keyboard focus stop and scrollable rows.

use crate::{Font, Language, ScrollBar, Theme, paint::drawing, theme::*};
use gpui::{prelude::*, *};
use std::time::{Duration, Instant};

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
    scroll: ScrollHandle,
    items: Vec<ListItem>,
    selected: Option<usize>,
    width: i32,
    height: i32,
    fill_height: bool,
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
            scroll: ScrollHandle::new(),
            items: items.into_iter().map(Into::into).collect(),
            selected: None,
            width: 200,
            height: 120,
            fill_height: false,
            label: "List".into(),
            theme: Theme::default(),
            language: Language::English,
            prefix: String::new(),
            typed_at: Instant::now(),
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
            self.scroll.scroll_to_item(index);
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
        self.scroll.scroll_to_item(index);
        cx.notify();
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
        let page = ((self.height - 4) / 16).max(1) as usize;
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
            if self.typed_at.elapsed() > Duration::from_secs(1) {
                self.prefix.clear();
            }
            self.prefix.push_str(&text.to_lowercase());
            self.typed_at = Instant::now();
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

impl Render for ListBox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window) && !self.disabled;
        let accent = self.theme.accent;
        let language = self.language;
        let viewport = if self.fill_height && self.scroll.bounds().size.height > px(0.) {
            f32::from(self.scroll.bounds().size.height) as i32
        } else {
            self.height - 6
        };
        let scrolls = self.always_scrollbar || self.items.len() as i32 * 16 > viewport;
        let width = self.width - if scrolls { 16 } else { 0 };
        let handle = self.scroll.clone();
        let entity = cx.entity();
        div()
            .id("list")
            .role(Role::ListBox)
            .aria_label(self.label.clone())
            .track_focus(&self.focus)
            .tab_stop(!self.disabled)
            .w(px(self.width as f32))
            .flex()
            .when(self.fill_height, |el| el.h_full())
            .when(!self.fill_height, |el| el.h(px(self.height as f32)))
            .flex_shrink_0()
            .p(px(2.))
            .border_1()
            .border_color(rgb(if focused && self.focus_ring {
                accent.dark
            } else {
                GRAY_7
            }))
            .bg(rgb(WHITE))
            .on_key_down(cx.listener(Self::key_down))
            .child(
                div()
                    .id("rows")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_y_scroll()
                    .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.notify()))
                    .track_scroll(&self.scroll)
                    .children(self.items.iter().enumerate().map(|(index, item)| {
                        let selected = self.selected == Some(index);
                        let text = item.label.clone();
                        let disabled = self.disabled || item.disabled;
                        let icon = self.icons.get(index).cloned();
                        let has_icons = !self.icons.is_empty();
                        let arrow = self.arrows.get(index).copied().unwrap_or(false);
                        let entity = cx.entity();
                        div()
                            .id(index)
                            .role(Role::ListBoxOption)
                            .aria_label(text.clone())
                            .aria_selected(selected)
                            .h(px(16.))
                            .w_full()
                            .flex_shrink_0()
                            .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                                entity.update(cx, |this, cx| {
                                    if disabled {
                                        return;
                                    }
                                    window.focus(&this.focus, cx);
                                    this.select(index, cx);
                                });
                            })
                            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                                window.focus(&this.focus, cx);
                                if disabled {
                                    return;
                                }
                                this.select(index, cx);
                                if event.click_count() == 2 {
                                    cx.emit(ListEvent::Activated(index));
                                }
                            }))
                            .child(drawing((width - 6) as f32, 16., move |pen, _, _| {
                                let focused = focused && !crate::window::content_is_inactive();
                                pen.set_dimmed(crate::window::content_is_inactive());
                                pen.language = language;
                                pen.rect(
                                    0,
                                    0,
                                    pen.width,
                                    16,
                                    if selected {
                                        if focused { accent.dark } else { GRAY_4 }
                                    } else {
                                        WHITE
                                    },
                                );
                                if let Some(icon) = &icon {
                                    let rows = icon.iter().map(String::as_str).collect::<Vec<_>>();
                                    crate::bitmap_icons::draw_bitmap_icon(
                                        pen,
                                        &rows,
                                        3,
                                        (16 - rows.len() as i32) / 2,
                                        accent,
                                        selected && focused,
                                        disabled,
                                    );
                                }
                                let left = if has_icons { 23 } else { 4 };
                                let label = Font::Small.truncate_in(
                                    &text,
                                    pen.width - left - if arrow { 12 } else { 4 },
                                    language,
                                );
                                pen.text(
                                    left,
                                    12,
                                    &label,
                                    Font::Small,
                                    if disabled {
                                        GRAY_8
                                    } else if selected && focused {
                                        WHITE
                                    } else {
                                        BLACK
                                    },
                                );
                                if arrow {
                                    let ink = if disabled {
                                        GRAY_8
                                    } else if selected && focused {
                                        WHITE
                                    } else {
                                        BLACK
                                    };
                                    for row in 0..7_i32 {
                                        pen.hline(pen.width - 8, 4 + row, 4 - (3 - row).abs(), ink);
                                    }
                                }
                            }))
                    })),
            )
            .when(scrolls, |root| {
                root.child(
                    ScrollBar::new(
                        "list-scroll",
                        -f32::from(self.scroll.offset().y) as f64,
                        viewport as f64,
                        (self.items.len() * 16) as f64,
                    )
                    .length(viewport)
                    .theme(self.theme)
                    .disabled(self.disabled)
                    .on_change(move |value, _, cx| {
                        handle.set_offset(point(px(0.), px(-value as f32)));
                        entity.update(cx, |_, cx| cx.notify());
                    }),
                )
            })
    }
}

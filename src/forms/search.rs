use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, MouseButton, Role, SharedString,
    Subscription, Window, div, prelude::*, px,
};

use super::FieldStyle;
use crate::{InputEvent, Language, TextField, Theme, paint::drawing, theme::*};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchChanged(pub String);

/// A text field with a magnifier and a clear button. Escape clears the query.
pub struct SearchField {
    field: Entity<TextField>,
    style: FieldStyle,
    placeholder: SharedString,
    _subscription: Subscription,
}

impl EventEmitter<SearchChanged> for SearchField {}
impl Focusable for SearchField {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.field.focus_handle(cx)
    }
}

impl SearchField {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let field = cx.new(|cx| TextField::new(cx).borderless(true).size(148., 22.));
        let subscription = cx.subscribe(&field, |_, field, event, cx| {
            if *event == InputEvent::Changed {
                cx.emit(SearchChanged(field.read(cx).value().to_owned()));
                cx.notify();
            }
        });
        Self {
            field,
            style: FieldStyle::new(160),
            placeholder: "".into(),
            _subscription: subscription,
        }
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.style.theme = theme;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.style.language = language;
        self
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.style.label = label.into();
        self
    }
    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }
    pub fn width(mut self, width: i32) -> Self {
        self.style.width = width.max(64);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.style.disabled = disabled;
        self
    }
    pub fn value<'a>(&'a self, cx: &'a App) -> &'a str {
        self.field.read(cx).value()
    }
    pub fn text_field(&self) -> &Entity<TextField> {
        &self.field
    }
    pub fn set_value(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        self.field
            .update(cx, |field, cx| field.set_value(value, cx));
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.style.theme = theme;
        self.style.dirty = true;
        cx.notify();
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.style.disabled = disabled;
        self.style.dirty = true;
        cx.notify();
    }
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.style.label = label.into();
        self.style.dirty = true;
        cx.notify();
    }
    pub fn set_placeholder(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.placeholder = text.into();
        self.style.dirty = true;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.style.language = language;
        self.style.dirty = true;
        cx.notify();
    }
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        if self.style.disabled || self.field.read(cx).value().is_empty() {
            return;
        }
        self.set_value("", cx);
        cx.emit(SearchChanged(String::new()));
    }
}

impl Render for SearchField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.style.dirty {
            let placeholder = if self.placeholder.is_empty() {
                super::word(self.style.language, "search").into()
            } else {
                self.placeholder.clone()
            };
            self.field
                .update(cx, |field, cx| field.set_placeholder(placeholder, cx));
            self.style.apply(&self.field, self.style.width - 28, cx);
            self.field.update(cx, |field, cx| {
                field.set_size((self.style.width - 28) as f32, 18., cx)
            });
        }
        let width = self.style.width;
        let disabled = self.style.disabled;
        let focused = self.field.focus_handle(cx).is_focused(window);
        let accent = self.style.theme.accent.dark;
        let has_text = !self.field.read(cx).value().is_empty();
        let mut root = div()
            .id("search-field")
            .role(Role::Group)
            .aria_label(self.style.label.clone())
            .relative()
            .w(px(width as f32))
            .h(px(22.))
            .flex_shrink_0()
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" && !this.style.disabled {
                    this.clear(cx);
                    cx.stop_propagation();
                }
            }))
            .child(drawing(width as f32, 22., move |pen, _, _| {
                let inactive = crate::window::content_is_inactive();
                pen.sunken();
                if inactive {
                    pen.frame(1, 1, width - 2, 20, GRAY_8);
                }
                if disabled {
                    pen.frame(1, 1, width - 2, 20, GRAY_6);
                    pen.rect(2, 2, width - 4, 18, GRAY_2);
                }
                if focused && !inactive {
                    crate::draw_focus_ring(pen, 0, accent);
                }
                let ink = if disabled {
                    GRAY_6
                } else if inactive {
                    GRAY_8
                } else {
                    GRAY_9
                };
                for (y, row) in [
                    ".###..", "#...#.", "#...#.", "#...#.", ".####.", "....##", ".....#",
                ]
                .iter()
                .enumerate()
                {
                    for (x, byte) in row.bytes().enumerate() {
                        if byte == b'#' {
                            pen.dot(5 + x as i32, 7 + y as i32, ink);
                        }
                    }
                }
            }))
            .child(
                div()
                    .absolute()
                    .left(px(14.))
                    .top(px(2.))
                    .child(self.field.clone()),
            );
        if has_text && !disabled {
            root = root.child(
                div()
                    .id("clear")
                    .role(Role::Button)
                    .aria_label("Clear search")
                    .absolute()
                    .right(px(3.))
                    .top(px(2.))
                    .w(px(14.))
                    .h(px(18.))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.clear(cx);
                        window.focus(&this.field.focus_handle(cx), cx);
                    }))
                    .child(drawing(14., 18., |pen, _, _| {
                        for (y, row) in [
                            "..###..", ".#####.", "##.#.##", "###.###", "##.#.##", ".#####.",
                            "..###..",
                        ]
                        .iter()
                        .enumerate()
                        {
                            for (x, byte) in row.bytes().enumerate() {
                                if byte == b'#' {
                                    pen.dot(4 + x as i32, 5 + y as i32, GRAY_7);
                                }
                            }
                        }
                    })),
            );
        }
        root
    }
}

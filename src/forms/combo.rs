use gpui::{
    Anchor, App, Context, Entity, EventEmitter, FocusHandle, Focusable, MouseButton, Role,
    SharedString, Subscription, Window, anchored, deferred, div, point, prelude::*, px,
};

use super::FieldStyle;
use crate::{InputEvent, Language, MenuItem, TextField, Theme, paint::drawing, theme::*};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboChanged(pub String);

/// An editable field whose suggestions match the beginning of its text.
pub struct Combobox {
    field: Entity<TextField>,
    menu_focus: FocusHandle,
    options: Vec<String>,
    matches: Vec<usize>,
    highlighted: Option<usize>,
    open: bool,
    style: FieldStyle,
    _subscription: Subscription,
}

impl EventEmitter<ComboChanged> for Combobox {}
impl Focusable for Combobox {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.field.focus_handle(cx)
    }
}

impl Combobox {
    pub fn new(
        options: impl IntoIterator<Item = impl Into<String>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let field = cx.new(|cx| TextField::new(cx).size(140., 22.));
        let subscription = cx.subscribe(&field, |this, field, event, cx| {
            if *event == InputEvent::Changed {
                // Typing leaves the menu to Down or the arrow button.
                let text = field.read(cx).value().to_owned();
                if this.open {
                    this.matches = matching_options(&this.options, &text);
                    this.highlighted = (!this.matches.is_empty()).then_some(0);
                    this.open = !this.matches.is_empty();
                }
                cx.emit(ComboChanged(text));
                cx.notify();
            } else if *event == InputEvent::Submitted
                && this.open
                && let Some(index) = this.highlighted
            {
                this.choose_value(index, cx);
            }
        });
        Self {
            field,
            menu_focus: cx.focus_handle(),
            options: options.into_iter().map(Into::into).collect(),
            matches: Vec::new(),
            highlighted: None,
            open: false,
            // A field of 140 and the arrow button of 20 over its edge.
            style: FieldStyle::new(159),
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
    pub fn width(mut self, width: i32) -> Self {
        self.style.width = width.max(64);
        self.style.dirty = true;
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
        self.open = false;
        cx.notify();
    }
    pub fn set_options(&mut self, options: Vec<String>, cx: &mut Context<Self>) {
        self.options = options;
        self.open = false;
        cx.notify();
    }
    pub fn set_width(&mut self, width: i32, cx: &mut Context<Self>) {
        self.style.width = width.max(64);
        self.style.dirty = true;
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
        if disabled {
            self.open = false;
        }
        cx.notify();
    }
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.style.label = label.into();
        self.style.dirty = true;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.style.language = language;
        self.style.dirty = true;
        cx.notify();
    }

    /// Opens the menu of the choices that begin with the text, the first
    /// highlighted. The menu keeps the keys while it is open, as the
    /// original's menus do, and gives them back to the field as it closes.
    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.style.disabled {
            return;
        }
        self.matches = matching_options(&self.options, self.field.read(cx).value());
        self.open = !self.matches.is_empty();
        self.highlighted = self.open.then_some(0);
        if self.open {
            window.focus(&self.menu_focus, cx);
        }
        cx.notify();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        if self.menu_focus.is_focused(window) {
            window.focus(&self.field.focus_handle(cx), cx);
        }
        cx.notify();
    }

    fn choose(&mut self, position: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.choose_value(position, cx);
        window.focus(&self.field.focus_handle(cx), cx);
    }

    fn choose_value(&mut self, position: usize, cx: &mut Context<Self>) {
        let Some(&index) = self.matches.get(position) else {
            return;
        };
        let value = self.options[index].clone();
        let changed = self.field.read(cx).value() != value;
        self.set_value(value.clone(), cx);
        if changed {
            cx.emit(ComboChanged(value));
        }
    }
}

impl Render for Combobox {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.style.apply(&self.field, self.style.width - 19, cx);
        let width = self.style.width;
        let disabled = self.style.disabled;
        let open = self.open;
        let accent = self.style.theme.accent;
        let language = self.style.language;
        let mut root = div()
            .id("combobox")
            .role(Role::ComboBox)
            .aria_label(self.style.label.clone())
            .aria_value(self.field.read(cx).value().to_owned())
            .aria_expanded(open)
            .track_focus(&self.menu_focus)
            .relative()
            .w(px(width as f32))
            .h(px(22.))
            .flex()
            .items_center()
            .flex_shrink_0()
            // Down in the field, which the field binds, opens the menu.
            .capture_action(
                cx.listener(|this, _: &crate::text_input::Down, window, cx| {
                    if !this.open && !this.style.disabled {
                        this.open(window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .capture_key_down(cx.listener(|this, event, window, cx| {
                if this.style.disabled || super::modified(event) {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "down" | "up" => {
                        if !this.open {
                            if event.keystroke.key == "down" {
                                this.open(window, cx);
                            }
                        } else if !this.matches.is_empty() {
                            let current = this.highlighted.unwrap_or(0);
                            this.highlighted = Some(if event.keystroke.key == "up" {
                                current.saturating_sub(1)
                            } else {
                                (current + 1).min(this.matches.len() - 1)
                            });
                        }
                    }
                    "enter" if this.open => {
                        if let Some(index) = this.highlighted {
                            this.choose(index, window, cx);
                        }
                    }
                    "escape" if this.open => this.close(window, cx),
                    "tab" => {
                        this.close(window, cx);
                        return;
                    }
                    _ => return,
                }
                cx.notify();
                cx.stop_propagation();
            }))
            .child(self.field.clone())
            .child(
                div()
                    .id("choices")
                    .role(Role::Button)
                    .aria_label(super::word(language, "choices"))
                    .w(px(20.))
                    // On the field's outline, inside its ring.
                    .h(px(20.))
                    .ml(px(-1.))
                    .flex_shrink_0()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window, cx| {
                            if this.style.disabled {
                                return;
                            }
                            if this.open {
                                this.close(window, cx);
                            } else {
                                this.open(window, cx);
                            }
                            cx.stop_propagation();
                        }),
                    )
                    .child(drawing(20., 20., move |pen, _, _| {
                        crate::draw_bevel_button(pen, disabled, open, false);
                        for row in 0..4 {
                            pen.hline(
                                6 + row,
                                8 + row,
                                7 - row * 2,
                                if disabled {
                                    GRAY_6
                                } else if crate::window::content_is_inactive() {
                                    GRAY_8
                                } else {
                                    BLACK
                                },
                            );
                        }
                    })),
            );
        if open && !disabled {
            // A menu of the choices under the field, over its bottom edge,
            // the one the field holds checked.
            let value = self.field.read(cx).value().to_owned();
            let items: Vec<MenuItem> = self
                .matches
                .iter()
                .map(|&index| {
                    let title = self.options[index].clone();
                    let checked = title == value;
                    MenuItem::new(title).checked(checked)
                })
                .collect();
            let menu_width = width.max(crate::menu_bar::menu_width(&items, language));
            let menu = div()
                .id("suggestions")
                .role(Role::ListBox)
                .aria_label(super::word(language, "choices"))
                .relative()
                .w(px(menu_width as f32))
                .p(px(2.))
                .child(crate::menu::menu_frame())
                .occlude()
                .on_mouse_down_out(cx.listener(|this, _, window, cx| this.close(window, cx)))
                .children(items.into_iter().enumerate().map(|(position, item)| {
                    let selected = self.highlighted == Some(position);
                    div()
                        .id(position)
                        .role(Role::ListBoxOption)
                        .aria_label(item.title.clone())
                        .aria_selected(selected)
                        .w_full()
                        .h(px(18.))
                        .on_mouse_down(MouseButton::Left, |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                        })
                        .on_mouse_move(cx.listener(move |this, _, _, cx| {
                            if this.highlighted != Some(position) {
                                this.highlighted = Some(position);
                                cx.notify();
                            }
                        }))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(move |this, _, window, cx| {
                                this.choose(position, window, cx);
                                cx.stop_propagation();
                            }),
                        )
                        .child(drawing((menu_width - 4) as f32, 18., move |pen, _, _| {
                            pen.language = language;
                            crate::menu::draw_menu_item(pen, &item, selected, accent);
                        }))
                }));
            root = root.child(
                div().absolute().left_0().top_0().child(
                    deferred(
                        anchored()
                            .anchor(Anchor::TopLeft)
                            .offset(point(px(0.), px(21.)))
                            .snap_to_window_with_margin(px(0.))
                            .child(menu),
                    )
                    .priority(3),
                ),
            );
        }
        root
    }
}

fn matching_options(options: &[String], text: &str) -> Vec<usize> {
    let prefix = text.to_lowercase();
    options
        .iter()
        .enumerate()
        .filter(|(_, option)| option.to_lowercase().starts_with(&prefix))
        .map(|(index, _)| index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suggestions_match_unicode_prefixes_without_reordering() {
        let options = ["Apple", "Apricot", "Éclair", "Banana"].map(String::from);
        assert_eq!(matching_options(&options, "aP"), [0, 1]);
        assert_eq!(matching_options(&options, "é"), [2]);
        assert_eq!(matching_options(&options, ""), [0, 1, 2, 3]);
        assert!(matching_options(&options, "pear").is_empty());
    }
}

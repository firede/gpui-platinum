use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, Role, SharedString, Window, div,
    prelude::*, px,
};

use super::FieldStyle;
use crate::{
    Font, Language, Slider, Theme, button,
    dialogs::modal::{ModalState, ModalView, modal},
    paint::drawing,
    theme::*,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorChanged(pub u32);

/// A color swatch opening a palette and RGB picker. Only OK commits the color.
pub struct ColorWell {
    focus: FocusHandle,
    palette_focus: FocusHandle,
    buttons: [FocusHandle; 2],
    value: u32,
    draft: u32,
    palette_index: usize,
    modal: ModalState,
    style: FieldStyle,
}

impl EventEmitter<ColorChanged> for ColorWell {}
impl Focusable for ColorWell {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl ColorWell {
    pub fn new(color: u32, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            palette_focus: cx.focus_handle().tab_stop(true),
            buttons: [cx.focus_handle(), cx.focus_handle()],
            value: color & 0xffffff,
            draft: color & 0xffffff,
            palette_index: 0,
            modal: ModalState::new(300., cx),
            style: FieldStyle::new(44),
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
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.style.disabled = disabled;
        self
    }
    pub fn value(&self) -> u32 {
        self.value
    }
    pub fn set_value(&mut self, color: u32, cx: &mut Context<Self>) {
        self.value = color & 0xffffff;
        cx.notify();
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.style.disabled = disabled;
        if disabled {
            self.modal.open = false;
        }
        cx.notify();
    }
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.style.label = label.into();
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.style.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.style.language = language;
        cx.notify();
    }

    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.style.disabled {
            return;
        }
        self.draft = self.value;
        self.palette_index = palette()
            .iter()
            .position(|color| *color == self.value)
            .unwrap_or(0);
        self.modal.theme = self.style.theme;
        self.modal.language = self.style.language;
        self.modal.stops = vec![
            self.palette_focus.clone(),
            self.buttons[0].clone(),
            self.buttons[1].clone(),
        ];
        self.modal.show(Some(&self.palette_focus), window, cx);
        cx.notify();
    }

    fn finish(&mut self, commit: bool, window: &mut Window, cx: &mut Context<Self>) {
        if commit && self.value != self.draft {
            self.value = self.draft;
            cx.emit(ColorChanged(self.value));
        }
        self.modal.close(window, cx);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn accept(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish(true, window, cx);
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish(false, window, cx);
    }

    fn choose_swatch(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(color) = palette().get(index) {
            self.draft = *color;
            self.palette_index = index;
            window.focus(&self.palette_focus, cx);
            cx.notify();
        }
    }
}

impl Render for ColorWell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.style.language;
        let title: SharedString = if self.style.label.is_empty() {
            super::word(language, "color").into()
        } else {
            self.style.label.clone()
        };
        let shown = self.value;
        let disabled = self.style.disabled;
        let open = self.modal.open;
        let focus_visible = self.focus.is_focused(window) && window.last_input_was_keyboard();
        let theme = self.style.theme;
        let mut root = div()
            .id("color-well")
            .role(Role::ColorWell)
            .aria_label(title.clone())
            .aria_value(format!("#{shown:06X}"))
            .relative()
            .w(px(44.))
            .h(px(22.))
            .flex_shrink_0()
            .when(!disabled, |element| element.track_focus(&self.focus))
            .on_click(cx.listener(|this, _, window, cx| this.open(window, cx)))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "space" | "enter") && !this.modal.open {
                    this.open(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(drawing(44., 22., move |pen, _, _| {
                crate::draw_bevel_button(pen, disabled, open, false);
                pen.set_dimmed(crate::window::content_is_inactive());
                let offset = i32::from(open);
                pen.frame(5 + offset, 5 + offset, 34, 12, BLACK);
                pen.rect(6 + offset, 6 + offset, 32, 10, shown);
                if focus_visible && !disabled && !crate::window::content_is_inactive() {
                    crate::draw_focus_ring(pen, 0, theme.accent.dark);
                }
            }));
        if open {
            let draft = self.draft;
            let palette = palette();
            let chosen = self.palette_index;
            let focused = self.palette_focus.is_focused(window);
            let grid = div()
                .id("palette")
                .role(Role::Grid)
                .aria_label(super::word(language, "palette"))
                .track_focus(&self.palette_focus)
                .relative()
                .w(px(116.))
                .h(px(88.))
                .flex_shrink_0()
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    let next = match event.keystroke.key.as_str() {
                        "left" => this.palette_index.saturating_sub(1),
                        "right" => (this.palette_index + 1).min(47),
                        "up" => this.palette_index.saturating_sub(8),
                        "down" => (this.palette_index + 8).min(47),
                        "home" => 0,
                        "end" => 47,
                        _ => return,
                    };
                    this.choose_swatch(next, window, cx);
                    cx.stop_propagation();
                }))
                .child(drawing(116., 88., move |pen, _, _| {
                    pen.sunken();
                    for (index, color) in palette.iter().copied().enumerate() {
                        let x = 2 + index as i32 % 8 * 14;
                        let y = 2 + index as i32 / 8 * 14;
                        pen.rect(x, y, 14, 14, color);
                        if color == draft || (focused && index == chosen) {
                            pen.frame(x, y, 14, 14, BLACK);
                            pen.frame(x + 1, y + 1, 12, 12, WHITE);
                        }
                    }
                }))
                .children((0..48).map(|index| {
                    div()
                        .id(index)
                        .role(Role::GridCell)
                        .aria_label(format!("#{:06X}", palette[index]))
                        .aria_selected(palette[index] == draft)
                        .absolute()
                        .left(px((2 + index % 8 * 14) as f32))
                        .top(px((2 + index / 8 * 14) as f32))
                        .w(px(14.))
                        .h(px(14.))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.choose_swatch(index, window, cx)
                        }))
                }));
            let preview = div()
                .id("preview")
                .role(Role::Image)
                .aria_label(super::word(language, "newAndOld"))
                .child(drawing(60., 50., move |pen, _, _| {
                    pen.sunken();
                    pen.rect(3, 3, 54, 22, draft);
                    pen.rect(3, 25, 54, 22, shown);
                }));
            // The picker is a dialog of the well's name: the palette and the
            // new color over the old, sliders of red, green and blue, and
            // Cancel and OK.
            let mut dialog = div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "enter" {
                        this.finish(true, window, cx);
                        cx.stop_propagation();
                    }
                }))
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(12.))
                        .child(grid)
                        .child(preview),
                );
            for (index, key) in ["red", "green", "blue"].iter().enumerate() {
                let channel = ((draft >> (16 - 8 * index)) & 255) as f64;
                let name = super::word(language, key);
                let picker = cx.weak_entity();
                dialog = dialog.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(div().w(px(42.)).child(super::form_label(
                            super::word(language, "labelled").replace("{name}", &name),
                            language,
                        )))
                        .child(
                            Slider::new(("channel", index), channel, 0., 255.)
                                .width(160)
                                .step(1.)
                                .theme(theme)
                                .label(name)
                                .on_change(move |value, _, cx| {
                                    picker
                                        .update(cx, |this, cx| {
                                            this.draft = with_channel(this.draft, index, value);
                                            cx.notify();
                                        })
                                        .ok();
                                }),
                        )
                        // A line of the text's own height, centered on the row.
                        .child(drawing(
                            24.,
                            Font::Small.line_height() as f32,
                            move |pen, _, _| {
                                let text = format!("{}", channel as u8);
                                pen.text(0, Font::Small.ascent(), &text, Font::Small, BLACK)
                            },
                        )),
                );
            }
            dialog = dialog.child(
                div()
                    .flex()
                    .justify_end()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        button("cancel", super::word(language, "cancel"))
                            .focus_handle(&self.buttons[0])
                            .theme(theme)
                            .language(language)
                            .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                    )
                    .child(
                        button("ok", super::word(language, "ok"))
                            .focus_handle(&self.buttons[1])
                            .default_button()
                            .theme(theme)
                            .language(language)
                            .on_click(cx.listener(|this, _, window, cx| this.accept(window, cx))),
                    ),
            );
            root = root.child(modal(
                &mut self.modal,
                ModalView::new(title, true, dialog),
                window,
                cx,
                |this| &mut this.modal,
                Self::accept,
                Self::cancel,
            ));
        }
        root
    }
}

fn with_channel(color: u32, index: usize, value: f64) -> u32 {
    let shift = 16 - index.min(2) * 8;
    let value = if value.is_finite() {
        value.round().clamp(0., 255.) as u32
    } else {
        0
    };
    (color & !(255 << shift)) | (value << shift)
}

fn palette() -> [u32; 48] {
    let hues = [
        0xcc2222, 0xee8822, 0xeecc22, 0x449933, 0x229999, 0x3355cc, 0x6633aa, 0xbb4499,
    ];
    let shades = [0.6, 0.35, 0., -0.25, -0.5];
    let grays = [
        WHITE, GRAY_2, GRAY_4, GRAY_6, GRAY_8, GRAY_10, GRAY_12, BLACK,
    ];
    std::array::from_fn(|index| {
        if index >= 40 {
            return grays[index - 40];
        }
        let color = hues[index % 8];
        let shade: f32 = shades[index / 8];
        let target = if shade > 0. { 255. } else { 0. };
        let mut result = 0;
        for shift in [0, 8, 16] {
            let channel = ((color >> shift) & 255) as f32;
            let amount = shade.abs();
            result |= ((channel * (1. - amount) + target * amount + 0.5) as u32) << shift;
        }
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn channel_updates_leave_other_channels_unchanged() {
        assert_eq!(with_channel(0x123456, 0, 255.), 0xff3456);
        assert_eq!(with_channel(0x123456, 1, -10.), 0x120056);
        assert_eq!(with_channel(0x123456, 2, 12.6), 0x12340d);
    }
    #[test]
    fn palette_keeps_upstream_hues_and_gray_row() {
        let colors = palette();
        assert_eq!(colors[16], 0xcc2222);
        assert_eq!(colors[23], 0xbb4499);
        assert_eq!(colors[40], WHITE);
        assert_eq!(colors[47], BLACK);
    }
}

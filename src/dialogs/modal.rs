use gpui::{
    AnyElement, App, Context, FocusHandle, KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, Render, Role, SharedString, Subscription, Window, actions,
    anchored, canvas, deferred, div, point, prelude::*, px,
};

use crate::{Font, Language, Theme, paint::Pen, theme::*};

actions!(platinum_modal, [Next, Previous, Accept, Cancel]);

/// Register the modal keyboard bindings once when the application starts.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", Next, Some("PlatinumModal")),
        KeyBinding::new("shift-tab", Previous, Some("PlatinumModal")),
        KeyBinding::new("escape", Cancel, Some("PlatinumModal")),
        KeyBinding::new("cmd-.", Cancel, Some("PlatinumModal")),
    ]);
}

pub(super) struct ModalState {
    pub open: bool,
    pub width: f32,
    pub position: Point<Pixels>,
    pub focus: FocusHandle,
    pub stops: Vec<FocusHandle>,
    pub theme: Theme,
    pub language: Language,
    previous: Option<FocusHandle>,
    drag: Option<(Point<Pixels>, Point<Pixels>)>,
    subscription: Option<Subscription>,
}

impl ModalState {
    pub fn new(width: f32, cx: &mut App) -> Self {
        Self {
            open: false,
            width,
            position: point(px(0.), px(0.)),
            focus: cx.focus_handle().tab_stop(false),
            stops: Vec::new(),
            theme: Theme::default(),
            language: Language::English,
            previous: None,
            drag: None,
            subscription: None,
        }
    }

    pub fn show(&mut self, first: Option<&FocusHandle>, window: &mut Window, cx: &mut App) {
        if !self.open {
            self.previous = window.focused(cx);
            let viewport = window.viewport_size();
            self.position = point(
                px(((f32::from(viewport.width) - self.width) / 2.)
                    .max(0.)
                    .round()),
                px((f32::from(viewport.height) / 5.).round()),
            );
        }
        self.open = true;
        window.focus(first.unwrap_or(&self.focus), cx);
    }

    pub fn close(&mut self, window: &mut Window, cx: &mut App) {
        self.open = false;
        self.drag = None;
        if let Some(previous) = self.previous.take() {
            window.focus(&previous, cx);
        } else {
            window.blur(cx);
        }
    }

    fn cycle(&self, backwards: bool, window: &mut Window, cx: &mut App) {
        let current = self
            .stops
            .iter()
            .position(|handle| handle.contains_focused(window, cx));
        if let Some(next) = cycle_index(self.stops.len(), current, backwards) {
            window.focus(&self.stops[next], cx);
        } else {
            window.focus(&self.focus, cx);
        }
        cx.stop_propagation();
    }
}

pub(super) fn cycle_index(count: usize, current: Option<usize>, backwards: bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some(match (current, backwards) {
        (Some(index), true) => (index + count - 1) % count,
        (Some(index), false) => (index + 1) % count,
        (None, true) => count - 1,
        (None, false) => 0,
    })
}

pub(super) struct ModalView {
    title: SharedString,
    movable: bool,
    content: AnyElement,
}

impl ModalView {
    pub fn new(title: impl Into<SharedString>, movable: bool, content: impl IntoElement) -> Self {
        Self {
            title: title.into(),
            movable,
            content: content.into_any_element(),
        }
    }
}

pub(super) fn modal<T: Render + 'static>(
    modal_state: &mut ModalState,
    view: ModalView,
    window: &mut Window,
    cx: &mut Context<T>,
    state: fn(&mut T) -> &mut ModalState,
    accept: fn(&mut T, &mut Window, &mut Context<T>),
    cancel: fn(&mut T, &mut Window, &mut Context<T>),
) -> AnyElement {
    let ModalView {
        title,
        movable,
        content,
    } = view;
    if modal_state.subscription.is_none() {
        modal_state.subscription =
            Some(
                cx.on_focus_out(&modal_state.focus, window, move |this, _, window, cx| {
                    let modal = state(this);
                    if modal.open {
                        window.focus(modal.stops.first().unwrap_or(&modal.focus), cx);
                    }
                }),
            );
    }
    let (open, focus, position, width, theme, language) = (
        modal_state.open,
        modal_state.focus.clone(),
        modal_state.position,
        modal_state.width,
        modal_state.theme,
        modal_state.language,
    );
    if !open {
        return div().into_any_element();
    }
    let drag = cx.weak_entity();
    let frame_title = title.clone();
    let frame = canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let mut pen = Pen::new(
                f32::from(bounds.size.width).round() as i32,
                f32::from(bounds.size.height).round() as i32,
            );
            pen.language = language;
            if movable {
                crate::window::draw_window_frame(
                    &mut pen,
                    &frame_title,
                    crate::window::PlatinumWindowOptions {
                        modal: true,
                        ..Default::default()
                    },
                    false,
                    crate::window::WindowPart::None,
                );
            } else {
                draw_frame(&mut pen, theme);
            }
            pen.paint(bounds.origin, window);
            let moving = drag.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if phase.bubble() && event.dragging() {
                    moving
                        .update(cx, |this, cx| {
                            let modal = state(this);
                            if let Some((mouse, origin)) = modal.drag {
                                let viewport = window.viewport_size();
                                modal.position.x = (origin.x + event.position.x - mouse.x)
                                    .clamp(px(0.), (viewport.width - px(40.)).max(px(0.)))
                                    .round();
                                modal.position.y = (origin.y + event.position.y - mouse.y)
                                    .clamp(px(0.), (viewport.height - px(24.)).max(px(0.)))
                                    .round();
                                cx.notify();
                            }
                        })
                        .ok();
                }
            });
            window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
                if phase.bubble() {
                    drag.update(cx, |this, _| state(this).drag = None).ok();
                }
            });
        },
    )
    .absolute()
    .inset_0()
    .size_full();
    let mut panel = div()
        .id("modal-panel")
        .role(if movable {
            Role::Dialog
        } else {
            Role::AlertDialog
        })
        .aria_label(title)
        .relative()
        .when(movable, |element| {
            element.absolute().left(position.x).top(position.y)
        })
        .w(px(width))
        .flex()
        .flex_col()
        .gap(px(10.))
        .px(px(if movable { 20. } else { 15. }))
        .pb(px(if movable { 20. } else { 15. }))
        .pt(px(if movable { 31. } else { 15. }))
        .child(frame)
        .child(content);
    if movable {
        panel = panel.child(
            div()
                .id("modal-title")
                .absolute()
                .left(px(0.))
                .right(px(0.))
                .top(px(0.))
                .h(px(19.))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                        let modal = state(this);
                        modal.drag = Some((event.position, modal.position));
                        window.prevent_default();
                        cx.stop_propagation();
                    }),
                ),
        );
    }
    let viewport = window.viewport_size();
    let backdrop = div()
        .id("modal-backdrop")
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .w(viewport.width)
        .h(viewport.height)
        .occlude()
        .key_context("PlatinumModal")
        .track_focus(&focus)
        .tab_stop(false)
        .on_mouse_down(MouseButton::Left, |_, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
        .on_action(
            cx.listener(move |this, _: &Next, window, cx| state(this).cycle(false, window, cx)),
        )
        .on_action(
            cx.listener(move |this, _: &Previous, window, cx| state(this).cycle(true, window, cx)),
        )
        .on_action(cx.listener(move |this, _: &Accept, window, cx| accept(this, window, cx)))
        .on_action(cx.listener(move |this, _: &Cancel, window, cx| cancel(this, window, cx)))
        .child(panel);
    deferred(
        anchored()
            .position(point(px(0.), px(0.)))
            .child(crate::button_scope("modal-buttons", backdrop)),
    )
    .priority(20)
    .into_any_element()
}

pub(super) fn draw_frame(pen: &mut Pen, theme: Theme) {
    let (w, h) = (pen.width, pen.height);
    pen.rect(0, 0, w, h, theme.background);
    pen.frame(0, 0, w, h, BLACK);
    pen.bevel(1, 1, w - 2, h - 2, WHITE, GRAY_7);
    pen.bevel(3, 3, w - 6, h - 6, GRAY_7, WHITE);
    pen.bevel(4, 4, w - 8, h - 8, WHITE, GRAY_5);
}

pub(super) fn text(
    text: impl Into<SharedString>,
    font: Font,
    language: Language,
) -> impl IntoElement {
    crate::text_in_language(text, font, BLACK, language)
}

pub(super) fn paragraph(
    text_value: impl Into<SharedString>,
    font: Font,
    width: i32,
    language: Language,
) -> impl IntoElement {
    let value = text_value.into();
    let lines = crate::display::wrap(&value, font, width);
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .pb(px(2.))
        .w(px(width as f32))
        .children(
            lines
                .into_iter()
                .map(move |line| text(line, font, language)),
        )
}

#[cfg(test)]
mod tests {
    use super::cycle_index;
    #[test]
    fn modal_tab_order_wraps_without_reaching_background_controls() {
        assert_eq!(cycle_index(3, Some(2), false), Some(0));
        assert_eq!(cycle_index(3, Some(0), true), Some(2));
        assert_eq!(cycle_index(3, None, false), Some(0));
        assert_eq!(cycle_index(3, None, true), Some(2));
        assert_eq!(cycle_index(0, None, false), None);
    }
}

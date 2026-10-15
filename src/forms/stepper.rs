use std::time::Duration;

use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, MouseButton, Role, SharedString, Task,
    Window, div, prelude::*, px,
};

use crate::{
    Theme,
    paint::{Rect, drawing},
    theme::*,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step(pub i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepperState {
    Normal,
    Disabled,
    Inactive,
}

/// Little arrows that repeat while held. Positive steps point up.
pub struct Stepper {
    focus: FocusHandle,
    label: SharedString,
    theme: Theme,
    disabled: bool,
    pressed: i32,
    inside: i32,
    repeat: Option<Task<()>>,
}

impl EventEmitter<Step> for Stepper {}
impl Focusable for Stepper {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Stepper {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            label: "".into(),
            theme: Theme::default(),
            disabled: false,
            pressed: 0,
            inside: 0,
            repeat: None,
        }
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn tab_stop(mut self, enabled: bool) -> Self {
        self.focus = self.focus.tab_stop(enabled);
        self
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        if disabled {
            self.stop(cx);
        }
        cx.notify();
    }
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.label = label.into();
        cx.notify();
    }

    fn start(&mut self, direction: i32, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        window.focus(&self.focus, cx);
        self.pressed = direction;
        self.inside = direction;
        cx.emit(Step(direction));
        self.repeat = Some(cx.spawn(async |stepper, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(300))
                .await;
            loop {
                let keep = stepper
                    .update(cx, |stepper, cx| {
                        if stepper.disabled || stepper.pressed == 0 {
                            return false;
                        }
                        if stepper.inside == stepper.pressed {
                            cx.emit(Step(stepper.pressed));
                        }
                        true
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(60))
                    .await;
            }
        }));
        cx.notify();
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        self.pressed = 0;
        self.repeat = None;
        cx.notify();
    }
}

impl Render for Stepper {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.disabled;
        let pressed = if self.inside == self.pressed {
            self.pressed
        } else {
            0
        };
        let focused = self.focus.is_focused(window) && window.last_input_was_keyboard();
        let theme = self.theme;
        let mut root = div()
            .id("stepper")
            .relative()
            .role(Role::SpinButton)
            .aria_label(self.label.clone())
            .w(px(13.))
            .h(px(23.))
            .flex_shrink_0()
            .when(!disabled, |element| element.track_focus(&self.focus))
            .on_key_down(cx.listener(|this, event, _, cx| {
                if this.disabled || super::modified(event) {
                    return;
                }
                let direction = match event.keystroke.key.as_str() {
                    "up" => 1,
                    "down" => -1,
                    _ => return,
                };
                cx.emit(Step(direction));
                cx.stop_propagation();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.stop(cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.stop(cx)),
            )
            .on_mouse_exit(cx.listener(|this, _, _, cx| {
                this.inside = 0;
                cx.notify();
            }))
            .child(drawing(13., 23., move |pen, _, _| {
                draw_stepper(
                    pen,
                    pressed,
                    if disabled {
                        StepperState::Disabled
                    } else {
                        StepperState::Normal
                    },
                );
                if focused && !disabled {
                    crate::draw_focus_ring(pen, 1, theme.accent.dark);
                }
            }));
        for (direction, top) in [(1, 0.), (-1, 11.)] {
            root = root.child(
                div()
                    .id(if direction > 0 { "up" } else { "down" })
                    .absolute()
                    .left(px(0.))
                    .top(px(top))
                    .w(px(13.))
                    .h(px(12.))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            this.start(direction, window, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .on_mouse_move(cx.listener(
                        move |this, event: &gpui::MouseMoveEvent, _, cx| {
                            if this.pressed != 0 && !event.dragging() {
                                this.stop(cx);
                            }
                            if this.inside != direction {
                                this.inside = direction;
                                cx.notify();
                            }
                        },
                    )),
            );
        }
        root
    }
}

/// Draws the thirteen-by-twenty-three-pixel arrows in the pen's corner.
pub fn draw_stepper(pen: &mut crate::paint::Pen, pressed: i32, state: StepperState) {
    let line = match state {
        StepperState::Normal => BLACK,
        StepperState::Disabled => GRAY_6,
        StepperState::Inactive => GRAY_8,
    };
    pen.round_frame(
        Rect {
            x: 0,
            y: 0,
            width: 13,
            height: 23,
        },
        2,
        line,
        Some(GRAY_2),
    );
    pen.hline(1, 11, 11, line);
    for (direction, top, bottom) in [(1, 1, 11), (-1, 12, 22)] {
        let (face, mut light, mut dark) = if pressed == direction {
            (GRAY_6, GRAY_9, GRAY_4)
        } else {
            (GRAY_2, WHITE, GRAY_7)
        };
        if state != StepperState::Normal {
            light = GRAY_2;
            dark = GRAY_2;
        }
        pen.rect(2, top + 1, 9, bottom - top - 2, face);
        pen.bevel(1, top, 11, bottom - top, light, dark);
        for row in 0..3 {
            let y = top + 3 + if direction > 0 { row } else { 2 - row };
            pen.hline(6 - row, y, 1 + row * 2, line);
        }
    }
}

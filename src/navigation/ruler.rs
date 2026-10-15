use gpui::{
    App, Bounds, Context, DispatchPhase, EventEmitter, FocusHandle, Focusable, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Role, Window, canvas, div,
    prelude::*, px,
};

use crate::{Font, Language, Theme, paint::Pen, theme::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RulerState {
    pub left: i32,
    pub right: i32,
    pub first: i32,
}
impl Default for RulerState {
    fn default() -> Self {
        Self {
            left: 18,
            right: 270,
            first: 18,
        }
    }
}
impl RulerState {
    fn normalize(&mut self, width: i32) {
        self.right = self.right.clamp(36, width.max(37) - 1);
        self.left = self.left.clamp(0, self.right - 36);
        self.first = self.first.clamp(0, self.right - self.left - 36);
    }
    fn move_marker(&mut self, marker: Marker, x: i32, width: i32) {
        match marker {
            Marker::Left => self.left = (x / 9 * 9).clamp(0, self.right - 36),
            Marker::Right => {
                self.right =
                    self.left + ((x - self.left + 4) / 9 * 9).clamp(36, width - 1 - self.left)
            }
            Marker::First => {
                self.first = ((x - self.left) / 9 * 9).clamp(0, self.right - self.left - 36)
            }
        }
        self.normalize(width);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RulerChanged(pub RulerState);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Marker {
    Left,
    Right,
    First,
}

pub struct Ruler {
    focus: FocusHandle,
    state: RulerState,
    width: i32,
    bounds: Option<Bounds<Pixels>>,
    active: Marker,
    dragging: bool,
    theme: Theme,
    language: Language,
}
impl EventEmitter<RulerChanged> for Ruler {}
impl Focusable for Ruler {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Ruler {
    pub fn new(state: RulerState, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            state,
            width: 300,
            bounds: None,
            active: Marker::Left,
            dragging: false,
            theme: Theme::default(),
            language: Language::English,
        }
    }
    pub fn width(mut self, width: i32) -> Self {
        self.width = width.max(60);
        self.state.normalize(self.width);
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
    pub fn state(&self) -> RulerState {
        self.state
    }
    pub fn set_width(&mut self, width: i32, cx: &mut Context<Self>) {
        self.width = width.max(60);
        self.state.normalize(self.width);
        cx.notify();
    }
    pub fn set_state(&mut self, mut state: RulerState, cx: &mut Context<Self>) {
        state.normalize(self.width);
        self.state = state;
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
    fn move_marker(&mut self, x: i32, cx: &mut Context<Self>) {
        let previous = self.state;
        self.state.move_marker(self.active, x, self.width);
        if self.state != previous {
            cx.emit(RulerChanged(self.state));
            cx.notify();
        }
    }
    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(bounds) = self.bounds else {
            return;
        };
        let x = f32::from(event.position.x - bounds.left()).round() as i32;
        let y = f32::from(event.position.y - bounds.top()).round() as i32;
        let marker = if y < 7 && (x - self.state.left - self.state.first).abs() <= 4 {
            Some(Marker::First)
        } else if (x - self.state.left).abs() <= 5 {
            Some(Marker::Left)
        } else if (x - self.state.right).abs() <= 5 {
            Some(Marker::Right)
        } else {
            None
        };
        window.focus(&self.focus, cx);
        if let Some(marker) = marker {
            self.active = marker;
            self.dragging = true;
            cx.notify();
            cx.stop_propagation();
        }
    }
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "up" | "down" => {
                self.active = match (self.active, event.keystroke.key.as_str()) {
                    (Marker::Left, "down") | (Marker::Right, "up") => Marker::First,
                    (Marker::First, "down") | (Marker::Left, "up") => Marker::Right,
                    _ => Marker::Left,
                };
                cx.notify();
            }
            "left" | "right" | "home" | "end" => {
                let current = match self.active {
                    Marker::Left => self.state.left,
                    Marker::Right => self.state.right,
                    Marker::First => self.state.left + self.state.first,
                };
                let next = match event.keystroke.key.as_str() {
                    "home" => 0,
                    "end" => self.width - 1,
                    "left" => current - 9,
                    _ => current + 9,
                };
                self.move_marker(next, cx);
            }
            _ => return,
        }
        cx.stop_propagation();
    }
    fn paint(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        let mut pen = Pen::new(self.width, 22);
        pen.set_dimmed(crate::window::content_is_inactive());
        pen.language = self.language;
        let RulerState { left, right, first } = self.state;
        pen.rect(0, 0, self.width, 22, GRAY_2);
        pen.hline(0, 21, self.width, BLACK);
        pen.rect(0, 2, self.width, 12, GRAY_1);
        pen.rect(left, 2, right - left, 12, WHITE);
        pen.hline(0, 1, self.width, GRAY_7);
        pen.hline(0, 14, self.width, GRAY_7);
        let mut tick = -(left / 9);
        while left + tick * 9 < self.width {
            let x = left + tick * 9;
            let height = if tick % 8 == 0 {
                12
            } else if tick % 4 == 0 {
                5
            } else {
                2
            };
            pen.vline(x, 14 - height, height, BLACK);
            if tick % 8 == 0 && tick > 0 {
                pen.text(x + 3, 11, &(tick / 8).to_string(), Font::Small, BLACK);
            }
            tick += 1;
        }
        let focused = self.focus.is_focused(window) && window.last_input_was_keyboard();
        let color = |marker| {
            if focused && self.active == marker {
                self.theme.accent.dark
            } else {
                BLACK
            }
        };
        for row in 0..6 {
            pen.hline(left, 15 + row, row + 2, color(Marker::Left));
            pen.hline(right - row - 1, 15 + row, row + 2, color(Marker::Right));
        }
        for row in 0..3 {
            pen.hline(
                left + first - 2 + row,
                2 + row,
                5 - row * 2,
                color(Marker::First),
            );
        }
        pen.paint(bounds.origin, window);
    }
}
impl Render for Ruler {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.state.normalize(self.width);
        let layout = cx.entity();
        let paint = cx.entity();
        div()
            .id("ruler")
            .role(Role::Slider)
            .aria_label(crate::tr(self.language, "ruler"))
            .aria_numeric_value((self.state.right - self.state.left) as f64)
            .aria_min_numeric_value(36.)
            .aria_max_numeric_value(self.width as f64)
            .track_focus(&self.focus)
            .tab_stop(true)
            .w(px(self.width as f32))
            .h(px(22.))
            .flex_shrink_0()
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .child(
                canvas(
                    move |bounds, _, cx| {
                        layout.update(cx, |this, _| this.bounds = Some(bounds));
                    },
                    move |bounds, _, window, cx| {
                        paint.read(cx).paint(bounds, window);
                        let movement = paint.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                            if phase != DispatchPhase::Bubble {
                                return;
                            }
                            movement.update(cx, |this, cx| {
                                if !this.dragging {
                                    return;
                                }
                                if !event.dragging() {
                                    this.dragging = false;
                                    return;
                                }
                                if let Some(bounds) = this.bounds {
                                    this.move_marker(
                                        f32::from(event.position.x - bounds.left()).round() as i32,
                                        cx,
                                    );
                                    cx.stop_propagation();
                                }
                            });
                        });
                        let release = paint.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase != DispatchPhase::Bubble || event.button != MouseButton::Left {
                                return;
                            }
                            release.update(cx, |this, cx| {
                                if this.dragging {
                                    this.dragging = false;
                                    cx.notify();
                                    cx.stop_propagation();
                                }
                            });
                        });
                    },
                )
                .size_full(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Marker, RulerState};
    #[test]
    fn dragging_margins_snaps_and_keeps_a_minimum_text_width() {
        let mut state = RulerState {
            left: 18,
            right: 270,
            first: 180,
        };
        state.move_marker(Marker::Right, 85, 300);
        assert_eq!(
            state,
            RulerState {
                left: 18,
                right: 81,
                first: 27
            }
        );
        state.move_marker(Marker::Left, 290, 300);
        assert_eq!(
            state,
            RulerState {
                left: 45,
                right: 81,
                first: 0
            }
        );
        state.move_marker(Marker::Right, 500, 300);
        assert_eq!(state.right, 299);
    }
}

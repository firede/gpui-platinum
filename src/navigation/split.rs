use gpui::{
    AnyElement, App, AvailableSpace, Bounds, DispatchPhase, Element, ElementId, FocusHandle,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseMoveEvent,
    MouseUpEvent, Pixels, RenderOnce, Role, Style, Window, div, prelude::*, px, size,
};
use std::{cell::RefCell, rc::Rc};

use crate::{Language, Theme, paint::Pen, theme::*};

type ChangeHandler = Rc<dyn Fn(f32, &mut Window, &mut App)>;
#[derive(Clone)]
struct Config {
    position: f32,
    width: i32,
    height: i32,
    minimum: f32,
    vertical: bool,
    theme: Theme,
    language: Language,
    on_change: Option<ChangeHandler>,
}
impl Config {
    fn length(&self) -> f32 {
        if self.vertical {
            self.height as f32
        } else {
            self.width as f32
        }
    }
    fn minimum(&self) -> f32 {
        self.minimum.min((self.length() - 7.) / 2.).max(0.)
    }
    fn maximum(&self) -> f32 {
        (self.length() - 7. - self.minimum()).max(self.minimum())
    }
    fn clamp(&self, position: f32) -> f32 {
        position.clamp(self.minimum(), self.maximum())
    }
    fn dimensions(&self) -> (i32, i32) {
        if self.vertical {
            (self.width, 7)
        } else {
            (7, self.height)
        }
    }
}

/// Two clipped panes separated by a pointer- and keyboard-operated divider.
#[derive(IntoElement)]
pub struct Split {
    id: ElementId,
    first: AnyElement,
    second: AnyElement,
    config: Config,
}
impl Split {
    pub fn new(
        id: impl Into<ElementId>,
        position: f32,
        first: impl IntoElement,
        second: impl IntoElement,
    ) -> Self {
        Self {
            id: id.into(),
            first: first.into_any_element(),
            second: second.into_any_element(),
            config: Config {
                position,
                width: 320,
                height: 160,
                minimum: 40.,
                vertical: false,
                theme: Theme::default(),
                language: Language::English,
                on_change: None,
            },
        }
    }
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.config.width = width.max(7);
        self.config.height = height.max(7);
        self
    }
    pub fn vertical(mut self) -> Self {
        self.config.vertical = true;
        self
    }
    pub fn minimum(mut self, minimum: f32) -> Self {
        self.config.minimum = minimum.max(0.);
        self
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.config.theme = theme;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.config.language = language;
        self
    }
    pub fn on_change(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.config.on_change = Some(Rc::new(handler));
        self
    }
}
impl RenderOnce for Split {
    fn render(mut self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.config.position = self.config.clamp(self.config.position);
        let position = self.config.position;
        let first = div()
            .flex_shrink_0()
            .overflow_hidden()
            .when(self.config.vertical, |el| el.w_full().h(px(position)))
            .when(!self.config.vertical, |el| el.h_full().w(px(position)))
            .child(self.first);
        div()
            .id(self.id.clone())
            .flex()
            .when(self.config.vertical, |el| el.flex_col())
            .w(px(self.config.width as f32))
            .h(px(self.config.height as f32))
            .flex_shrink_0()
            .child(first)
            .child(Divider {
                id: "divider".into(),
                config: self.config,
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .overflow_hidden()
                    .child(self.second),
            )
    }
}

struct Divider {
    id: ElementId,
    config: Config,
}
struct DividerState {
    focus: FocusHandle,
    config: Config,
    dragging: bool,
    start_pointer: f32,
    start_position: f32,
}
type SharedState = Rc<RefCell<DividerState>>;
struct PrepaintState {
    element: AnyElement,
    state: SharedState,
}
impl IntoElement for Divider {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Divider {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let (width, height) = self.config.dimensions();
        let mut style = Style::default();
        style.size.width = px(width as f32).into();
        style.size.height = px(height as f32).into();
        style.flex_shrink = 0.;
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> PrepaintState {
        let config = self.config.clone();
        let state = window.with_element_state(
            id.expect("split divider id"),
            |previous: Option<SharedState>, _| {
                let state = previous.unwrap_or_else(|| {
                    Rc::new(RefCell::new(DividerState {
                        focus: cx.focus_handle().tab_stop(true),
                        config: config.clone(),
                        dragging: false,
                        start_pointer: 0.,
                        start_position: 0.,
                    }))
                });
                state.borrow_mut().config = config.clone();
                (state.clone(), state)
            },
        );
        let focus = state.borrow().focus.clone();
        let down = state.clone();
        let keyboard = state.clone();
        let mut element = div()
            .id("split-input")
            .role(Role::Splitter)
            .aria_label(crate::tr(config.language, "divider"))
            .aria_numeric_value(config.position as f64)
            .aria_min_numeric_value(config.minimum() as f64)
            .aria_max_numeric_value(config.maximum() as f64)
            .aria_numeric_value_step(8.)
            .track_focus(&focus)
            .tab_stop(true)
            .w(bounds.size.width)
            .h(bounds.size.height)
            .cursor(if config.vertical {
                gpui::CursorStyle::ResizeUpDown
            } else {
                gpui::CursorStyle::ResizeLeftRight
            })
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                let mut state = down.borrow_mut();
                window.focus(&state.focus, cx);
                state.dragging = true;
                state.start_pointer = f32::from(if state.config.vertical {
                    event.position.y
                } else {
                    event.position.x
                });
                state.start_position = state.config.position;
                cx.stop_propagation();
            })
            .on_key_down(move |event, window, cx| {
                let next = {
                    let state = keyboard.borrow();
                    let key = event.keystroke.key.as_str();
                    match (state.config.vertical, key) {
                        (false, "left") | (true, "up") => Some(state.config.position - 8.),
                        (false, "right") | (true, "down") => Some(state.config.position + 8.),
                        (_, "home") => Some(state.config.minimum()),
                        (_, "end") => Some(state.config.maximum()),
                        _ => None,
                    }
                };
                if let Some(position) = next {
                    change(&keyboard, position, window, cx);
                    cx.stop_propagation();
                }
            })
            .into_any_element();
        element.prepaint_as_root(
            bounds.origin,
            size(
                AvailableSpace::Definite(bounds.size.width),
                AvailableSpace::Definite(bounds.size.height),
            ),
            window,
            cx,
        );
        PrepaintState { element, state }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        prepaint.element.paint(window, cx);
        {
            let state = prepaint.state.borrow();
            let (width, height) = state.config.dimensions();
            let mut pen = Pen::new(width, height);
            pen.rect(0, 0, width, height, GRAY_2);
            if state.config.vertical {
                pen.hline(0, 0, width, BLACK);
                pen.hline(0, 1, width, WHITE);
                pen.hline(0, 5, width, GRAY_7);
                pen.hline(0, 6, width, BLACK);
                for i in -2..=2 {
                    pen.dot(width / 2 + i * 4, 2, GRAY_7);
                    pen.dot(width / 2 + i * 4 + 1, 3, WHITE);
                }
            } else {
                pen.vline(0, 0, height, BLACK);
                pen.vline(1, 0, height, WHITE);
                pen.vline(5, 0, height, GRAY_7);
                pen.vline(6, 0, height, BLACK);
                for i in -2..=2 {
                    pen.dot(2, height / 2 + i * 4, GRAY_7);
                    pen.dot(3, height / 2 + i * 4 + 1, WHITE);
                }
            }
            if state.focus.is_focused(window) && window.last_input_was_keyboard() {
                pen.frame(0, 0, width, height, state.config.theme.accent.dark);
            }
            pen.paint(bounds.origin, window);
        }
        let movement = prepaint.state.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble {
                return;
            }
            let next = {
                let mut state = movement.borrow_mut();
                if !state.dragging {
                    return;
                }
                if !event.dragging() {
                    state.dragging = false;
                    return;
                }
                let coordinate = f32::from(if state.config.vertical {
                    event.position.y
                } else {
                    event.position.x
                });
                state.start_position + coordinate - state.start_pointer
            };
            change(&movement, next, window, cx);
            cx.stop_propagation();
        });
        let release = prepaint.state.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
            if phase != DispatchPhase::Bubble || event.button != MouseButton::Left {
                return;
            }
            let mut state = release.borrow_mut();
            if state.dragging {
                state.dragging = false;
                cx.stop_propagation();
            }
        });
    }
}
fn change(state: &SharedState, position: f32, window: &mut Window, cx: &mut App) {
    let (position, callback) = {
        let mut state = state.borrow_mut();
        let position = state.config.clamp(position.round());
        if position == state.config.position {
            return;
        }
        state.config.position = position;
        (position, state.config.on_change.clone())
    };
    if let Some(callback) = callback {
        callback(position, window, cx);
    }
    window.refresh();
}

#[cfg(test)]
mod tests {
    use super::Config;
    use crate::{Language, Theme};
    #[test]
    fn narrow_splits_keep_both_panes_nonnegative() {
        let config = Config {
            position: 100.,
            width: 60,
            height: 120,
            minimum: 40.,
            vertical: false,
            theme: Theme::default(),
            language: Language::English,
            on_change: None,
        };
        assert_eq!(config.clamp(-20.), 26.5);
        assert_eq!(config.clamp(1000.), 26.5);
    }
}

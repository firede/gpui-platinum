//! Sliders and scroll bars with bitmap thumbs and native pointer capture.

use std::{cell::RefCell, rc::Rc, time::Duration};

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, DispatchPhase, Element, ElementId, FocusHandle,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, RenderOnce, Role, ScrollDelta, SharedString, Style, Task, Window,
    div, point, prelude::*, px, size,
};

use crate::{Theme, paint::Pen, theme::*};

const SLIDER_HEIGHT: i32 = 18;
const THUMB_WIDTH: i32 = 11;
const BAR_WIDTH: i32 = 16;

type ChangeHandler = Rc<dyn Fn(f64, &mut Window, &mut App)>;
type SharedState = Rc<RefCell<RangeState>>;

/// A horizontal slider. The owner supplies the value on each render.
#[derive(IntoElement)]
pub struct Slider {
    element: RangeElement,
}

impl Slider {
    pub fn new(id: impl Into<ElementId>, value: f64, min: f64, max: f64) -> Self {
        Self {
            element: RangeElement::new(id.into(), value, Range::new(min, max), Kind::Slider),
        }
    }

    /// Stops at multiples of `step` from the minimum, as well as both endpoints.
    pub fn step(mut self, step: f64) -> Self {
        self.element.config.step = positive(step);
        self
    }

    pub fn width(mut self, width: i32) -> Self {
        self.element.config.length = width.max(60);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.element.config.disabled = disabled;
        self
    }

    pub fn inactive(mut self, inactive: bool) -> Self {
        self.element.config.inactive = inactive;
        self
    }

    /// Draws the slider's current value without installing input handlers.
    pub fn draw(&self, pen: &mut Pen, pressed: bool) {
        draw_slider(pen, &self.element.config, self.element.value, pressed);
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.element.config.theme = theme;
        self
    }

    /// Names the slider for assistive technology.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.element.config.label = label.into();
        self
    }

    pub fn ticks(mut self, count: u32) -> Self {
        self.element.config.ticks = count;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.element.config.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Slider {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.element
    }
}

/// A proportional scroll bar. `value` is the content offset in pixels.
#[derive(IntoElement)]
pub struct ScrollBar {
    element: RangeElement,
}

impl ScrollBar {
    pub fn new(id: impl Into<ElementId>, value: f64, viewport: f64, content: f64) -> Self {
        let viewport = nonnegative(viewport);
        let content = nonnegative(content);
        let mut element = RangeElement::new(
            id.into(),
            value,
            Range::new(0., (content - viewport).max(0.)),
            Kind::ScrollBar {
                viewport,
                content,
                horizontal: false,
            },
        );
        element.config.step = 16.;
        Self { element }
    }

    pub fn length(mut self, length: i32) -> Self {
        self.element.config.length = length.max(BAR_WIDTH * 2);
        self
    }

    /// Keeps an embedded bar out of the tab order while retaining pointer input.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.element.config.tab_stop = tab_stop;
        self
    }

    pub fn horizontal(mut self) -> Self {
        if let Kind::ScrollBar { horizontal, .. } = &mut self.element.config.kind {
            *horizontal = true;
        }
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.element.config.step = positive(step);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.element.config.disabled = disabled;
        self
    }

    /// A background window's flat track, without arrows or a thumb.
    pub fn inactive(mut self, inactive: bool) -> Self {
        self.element.config.inactive = inactive;
        self
    }

    /// Draws the bar's current value without installing input handlers.
    pub fn draw(&self, pen: &mut Pen, pressed: ScrollPart) {
        draw_scroll_bar(pen, &self.element.config, self.element.value, pressed);
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.element.config.theme = theme;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.element.config.label = label.into();
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.element.config.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ScrollBar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.element
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Slider,
    ScrollBar {
        viewport: f64,
        content: f64,
        horizontal: bool,
    },
}

#[derive(Clone)]
struct Config {
    range: Range,
    kind: Kind,
    step: f64,
    length: i32,
    ticks: u32,
    disabled: bool,
    inactive: bool,
    tab_stop: bool,
    theme: Theme,
    label: SharedString,
    on_change: Option<ChangeHandler>,
}

impl Config {
    fn role(&self) -> Role {
        match self.kind {
            Kind::Slider => Role::Slider,
            Kind::ScrollBar { .. } => Role::ScrollBar,
        }
    }

    fn orientation(&self) -> gpui::Orientation {
        match self.kind {
            Kind::ScrollBar {
                horizontal: false, ..
            } => gpui::Orientation::Vertical,
            _ => gpui::Orientation::Horizontal,
        }
    }

    fn enabled(&self) -> bool {
        !self.disabled && !self.inactive && self.range.max > self.range.min
    }

    fn dimensions(&self) -> (i32, i32) {
        match self.kind {
            Kind::Slider => (
                self.length,
                SLIDER_HEIGHT + if self.ticks > 1 { 5 } else { 0 },
            ),
            Kind::ScrollBar {
                horizontal: true, ..
            } => (self.length, BAR_WIDTH),
            Kind::ScrollBar {
                horizontal: false, ..
            } => (BAR_WIDTH, self.length),
        }
    }

    fn input_value(&self, value: f64) -> f64 {
        match self.kind {
            Kind::Slider => self.range.snap(value, self.step),
            Kind::ScrollBar { .. } => self.range.clamp(value),
        }
    }

    fn key_step(&self) -> f64 {
        if self.step > 0. {
            self.step
        } else {
            (self.range.max - self.range.min) / 100.
        }
    }

    fn page_step(&self) -> f64 {
        match self.kind {
            Kind::Slider => ((self.range.max - self.range.min) / 10.).max(self.key_step()),
            Kind::ScrollBar { viewport, .. } => (viewport - self.key_step()).max(self.key_step()),
        }
    }

    fn coordinate(&self, position: Point<Pixels>, bounds: Bounds<Pixels>) -> f32 {
        match self.kind {
            Kind::ScrollBar {
                horizontal: false, ..
            } => f32::from(position.y - bounds.top()),
            _ => f32::from(position.x - bounds.left()),
        }
    }

    fn part_at(&self, at: f32, value: f64) -> ScrollPart {
        if !self.enabled() {
            return ScrollPart::None;
        }
        match self.kind {
            Kind::Slider => ScrollPart::Thumb,
            Kind::ScrollBar {
                viewport, content, ..
            } => {
                if at < 0. || at >= self.length as f32 {
                    return ScrollPart::None;
                }
                if at < BAR_WIDTH as f32 {
                    return ScrollPart::Back;
                }
                if at >= (self.length - BAR_WIDTH) as f32 {
                    return ScrollPart::Forward;
                }
                let Some(thumb) = scroll_thumb(self.length, value, viewport, content) else {
                    return ScrollPart::None;
                };
                if at < thumb.position as f32 {
                    ScrollPart::PageBack
                } else if at >= (thumb.position + thumb.length) as f32 {
                    ScrollPart::PageForward
                } else {
                    ScrollPart::Thumb
                }
            }
        }
    }

    fn drag_value(&self, at: f32, grab: f32) -> f64 {
        let fraction = match self.kind {
            Kind::Slider => (at - grab - 5.) as f64 / (self.length - THUMB_WIDTH).max(1) as f64,
            Kind::ScrollBar {
                viewport, content, ..
            } => {
                let Some(thumb) = scroll_thumb(self.length, self.range.min, viewport, content)
                else {
                    return self.range.min;
                };
                let travel = self.length - 2 * BAR_WIDTH + 2 - thumb.length;
                if travel <= 0 {
                    return self.range.min;
                }
                (at - grab - (BAR_WIDTH - 1) as f32) as f64 / travel as f64
            }
        };
        self.input_value(self.range.at_fraction(fraction))
    }

    fn key_value(&self, key: &str, value: f64) -> Option<f64> {
        let slider = matches!(self.kind, Kind::Slider);
        let next = match key {
            "left" => value - self.key_step(),
            "right" => value + self.key_step(),
            "up" => {
                value
                    + if slider {
                        self.key_step()
                    } else {
                        -self.key_step()
                    }
            }
            "down" => {
                value
                    + if slider {
                        -self.key_step()
                    } else {
                        self.key_step()
                    }
            }
            "pageup" => {
                value
                    + if slider {
                        self.page_step()
                    } else {
                        -self.page_step()
                    }
            }
            "pagedown" => {
                value
                    + if slider {
                        -self.page_step()
                    } else {
                        self.page_step()
                    }
            }
            "home" => self.range.min,
            "end" => self.range.max,
            _ => return None,
        };
        Some(self.input_value(next))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Range {
    min: f64,
    max: f64,
}

impl Range {
    fn new(min: f64, max: f64) -> Self {
        let min = if min.is_finite() { min } else { 0. };
        let max = if max.is_finite() { max.max(min) } else { min };
        Self { min, max }
    }

    fn clamp(self, value: f64) -> f64 {
        if value.is_finite() {
            value.clamp(self.min, self.max)
        } else {
            self.min
        }
    }

    fn snap(self, value: f64, step: f64) -> f64 {
        let value = self.clamp(value);
        if step <= 0. || !step.is_finite() || value == self.min || value == self.max {
            return value;
        }
        self.clamp(self.min + ((value - self.min) / step).round() * step)
    }

    fn fraction(self, value: f64) -> f64 {
        if self.max <= self.min {
            return 0.;
        }
        (self.clamp(value) - self.min) / (self.max - self.min)
    }

    fn at_fraction(self, fraction: f64) -> f64 {
        self.clamp(self.min + fraction.clamp(0., 1.) * (self.max - self.min))
    }
}

fn positive(value: f64) -> f64 {
    if value.is_finite() && value > 0. {
        value
    } else {
        0.
    }
}

fn nonnegative(value: f64) -> f64 {
    positive(value)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollPart {
    #[default]
    None,
    Back,
    Forward,
    PageBack,
    PageForward,
    Thumb,
}

struct RangeState {
    focus: FocusHandle,
    config: Config,
    value: f64,
    bounds: Bounds<Pixels>,
    pointer: Point<Pixels>,
    pressed: ScrollPart,
    grab: f32,
    repeat: Option<Task<()>>,
}

struct RangeElement {
    id: ElementId,
    value: f64,
    config: Config,
}

impl RangeElement {
    fn new(id: ElementId, value: f64, range: Range, kind: Kind) -> Self {
        Self {
            id,
            value,
            config: Config {
                range,
                kind,
                step: 0.,
                length: 160,
                ticks: 0,
                disabled: false,
                inactive: false,
                tab_stop: true,
                theme: Theme::default(),
                label: SharedString::default(),
                on_change: None,
            },
        }
    }
}

impl IntoElement for RangeElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

struct PrepaintState {
    element: AnyElement,
    state: SharedState,
}

impl Element for RangeElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    // Enabled ranges use the inner div's focusable accessibility node. GPUI's
    // div has no disabled property, so disabled ranges describe themselves.
    fn a11y_role(&self) -> Option<Role> {
        (!self.config.enabled()).then(|| self.config.role())
    }

    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        node.set_label(self.config.label.to_string());
        node.set_numeric_value(self.config.range.clamp(self.value));
        node.set_min_numeric_value(self.config.range.min);
        node.set_max_numeric_value(self.config.range.max);
        node.set_orientation(self.config.orientation());
        node.set_disabled();
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
        let value = config.range.clamp(self.value);
        let state = window.with_element_state(
            id.expect("range element id"),
            |state: Option<SharedState>, _| {
                let state = state.unwrap_or_else(|| {
                    Rc::new(RefCell::new(RangeState {
                        focus: cx.focus_handle().tab_stop(config.tab_stop),
                        config: config.clone(),
                        value,
                        bounds,
                        pointer: Point::default(),
                        pressed: ScrollPart::None,
                        grab: 0.,
                        repeat: None,
                    }))
                });
                {
                    let mut current = state.borrow_mut();
                    current.config = config.clone();
                    current.focus = current.focus.clone().tab_stop(config.tab_stop);
                    current.value = value;
                    current.bounds = bounds;
                    if !config.enabled() {
                        current.pressed = ScrollPart::None;
                        current.repeat = None;
                    }
                }
                (state.clone(), state)
            },
        );
        let focus = state.borrow().focus.clone();
        let mut element = div()
            .id("input")
            .w(bounds.size.width)
            .h(bounds.size.height)
            .flex_shrink_0()
            .when(config.enabled(), |element| element.role(config.role()))
            .aria_label(config.label.clone())
            .aria_numeric_value(value)
            .aria_min_numeric_value(config.range.min)
            .aria_max_numeric_value(config.range.max)
            .aria_numeric_value_step(config.key_step())
            .aria_orientation(config.orientation());
        if config.enabled() {
            let down_state = state.clone();
            let key_state = state.clone();
            let wheel_state = state.clone();
            element = element
                .track_focus(&focus)
                .tab_stop(config.tab_stop)
                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    pointer_down(&down_state, event.position, window, cx);
                    cx.stop_propagation();
                })
                .on_key_down(move |event, window, cx| {
                    let next = {
                        let state = key_state.borrow();
                        if event.keystroke.modifiers.platform
                            || event.keystroke.modifiers.control
                            || event.keystroke.modifiers.alt
                        {
                            return;
                        }
                        state.config.key_value(&event.keystroke.key, state.value)
                    };
                    if let Some(next) = next {
                        set_value(&key_state, next, window, cx);
                        cx.stop_propagation();
                    }
                });
            if matches!(config.kind, Kind::ScrollBar { .. }) {
                element = element.on_scroll(move |event, window, cx| {
                    let next = {
                        let state = wheel_state.borrow();
                        let delta = match event.delta {
                            ScrollDelta::Pixels(delta) => {
                                point(f32::from(delta.x), f32::from(delta.y))
                            }
                            ScrollDelta::Lines(delta) => delta * state.config.key_step() as f32,
                        };
                        let horizontal = matches!(
                            state.config.kind,
                            Kind::ScrollBar {
                                horizontal: true,
                                ..
                            }
                        );
                        let amount = if horizontal && delta.x != 0. {
                            delta.x
                        } else {
                            delta.y
                        };
                        state.value - amount as f64
                    };
                    set_value(&wheel_state, next, window, cx);
                    cx.stop_propagation();
                });
            }
        }
        let mut element = element.into_any_element();
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
            let mut config = state.config.clone();
            config.inactive |= crate::window::content_is_inactive();
            let (width, height) = config.dimensions();
            let mut pen = Pen::new(width, height);
            let at = state.config.coordinate(state.pointer, bounds);
            let over_pressed_part = bounds.contains(&state.pointer)
                && state.config.part_at(at, state.value) == state.pressed;
            let pressed = if state.pressed == ScrollPart::Thumb || over_pressed_part {
                state.pressed
            } else {
                ScrollPart::None
            };
            match config.kind {
                Kind::Slider => {
                    draw_slider(&mut pen, &config, state.value, pressed == ScrollPart::Thumb);
                    if config.disabled {
                        fade(&mut pen);
                    }
                }
                Kind::ScrollBar { .. } => draw_scroll_bar(&mut pen, &config, state.value, pressed),
            }
            if config.enabled()
                && state.focus.is_focused(window)
                && window.last_input_was_keyboard()
            {
                if matches!(state.config.kind, Kind::Slider) {
                    let mut focus = Pen::new(width, SLIDER_HEIGHT);
                    crate::draw_focus_ring(&mut focus, 0, state.config.theme.accent.dark);
                    pen.fills.extend(focus.fills);
                } else {
                    pen.frame(0, 0, width, height, state.config.theme.accent.dark);
                }
            }
            pen.paint(point(bounds.left().round(), bounds.top().round()), window);
        }
        let move_state = prepaint.state.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble {
                return;
            }
            let next = {
                let mut state = move_state.borrow_mut();
                if state.pressed == ScrollPart::None {
                    return;
                }
                state.pointer = event.position;
                if !event.dragging() {
                    state.pressed = ScrollPart::None;
                    state.repeat = None;
                    window.refresh();
                    return;
                }
                if state.pressed == ScrollPart::Thumb {
                    let at = state.config.coordinate(event.position, state.bounds);
                    Some(state.config.drag_value(at, state.grab))
                } else {
                    None
                }
            };
            if let Some(next) = next {
                set_value(&move_state, next, window, cx);
            }
            window.refresh();
            cx.stop_propagation();
        });
        let up_state = prepaint.state.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble || event.button != MouseButton::Left {
                return;
            }
            let mut state = up_state.borrow_mut();
            if state.pressed != ScrollPart::None {
                state.pressed = ScrollPart::None;
                state.repeat = None;
                window.refresh();
                cx.stop_propagation();
            }
        });
    }
}

fn set_value(state: &SharedState, value: f64, window: &mut Window, cx: &mut App) {
    let change = {
        let mut state = state.borrow_mut();
        if !state.config.enabled() {
            return;
        }
        let value = state.config.input_value(value);
        if state.value == value {
            return;
        }
        state.value = value;
        (value, state.config.on_change.clone())
    };
    if let (value, Some(handler)) = change {
        handler(value, window, cx);
    }
    window.refresh();
}

fn pointer_down(state: &SharedState, position: Point<Pixels>, window: &mut Window, cx: &mut App) {
    let (part, next) = {
        let mut state = state.borrow_mut();
        let config = state.config.clone();
        if !config.enabled() {
            return;
        }
        if config.tab_stop {
            state.focus.focus(window, cx);
        } else {
            window.prevent_default();
        }
        state.pointer = position;
        let at = config.coordinate(position, state.bounds);
        state.pressed = config.part_at(at, state.value);
        if state.pressed == ScrollPart::Thumb {
            let next = match config.kind {
                Kind::Slider => {
                    let center = 5.
                        + config.range.fraction(state.value) as f32
                            * (config.length - THUMB_WIDTH) as f32;
                    if (at - center).abs() <= 5. {
                        state.grab = at - center;
                        None
                    } else {
                        state.grab = 0.;
                        Some(config.drag_value(at, 0.))
                    }
                }
                Kind::ScrollBar {
                    viewport, content, ..
                } => {
                    if let Some(thumb) = scroll_thumb(config.length, state.value, viewport, content)
                    {
                        state.grab = at - thumb.position as f32;
                    }
                    None
                }
            };
            (ScrollPart::Thumb, next)
        } else {
            (state.pressed, None)
        }
    };
    if let Some(next) = next {
        set_value(state, next, window, cx);
    }
    if part != ScrollPart::Thumb && part != ScrollPart::None {
        act(state, part, window, cx);
        start_repeat(state, window, cx);
    }
    window.refresh();
}

fn act(state: &SharedState, part: ScrollPart, window: &mut Window, cx: &mut App) {
    let next = {
        let state = state.borrow();
        let amount = match part {
            ScrollPart::Back => -state.config.key_step(),
            ScrollPart::Forward => state.config.key_step(),
            ScrollPart::PageBack => -state.config.page_step(),
            ScrollPart::PageForward => state.config.page_step(),
            _ => return,
        };
        state.value + amount
    };
    set_value(state, next, window, cx);
}

fn start_repeat(state: &SharedState, window: &mut Window, cx: &mut App) {
    let weak = Rc::downgrade(state);
    let task = window.spawn(cx, async move |cx| {
        cx.background_executor()
            .timer(Duration::from_millis(300))
            .await;
        loop {
            let Some(state) = weak.upgrade() else {
                break;
            };
            let keep_going = cx
                .update(|window, cx| {
                    let part = {
                        let state = state.borrow();
                        if !state.config.enabled()
                            || state.pressed == ScrollPart::None
                            || !window.is_window_active()
                        {
                            return false;
                        }
                        let at = state.config.coordinate(state.pointer, state.bounds);
                        if !state.bounds.contains(&state.pointer)
                            || state.config.part_at(at, state.value) != state.pressed
                        {
                            return true;
                        }
                        state.pressed
                    };
                    act(&state, part, window, cx);
                    true
                })
                .unwrap_or(false);
            if !keep_going {
                break;
            }
            drop(state);
            cx.background_executor()
                .timer(Duration::from_millis(60))
                .await;
        }
    });
    state.borrow_mut().repeat = Some(task);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Thumb {
    position: i32,
    length: i32,
}

fn scroll_thumb(length: i32, value: f64, viewport: f64, content: f64) -> Option<Thumb> {
    let viewport = nonnegative(viewport);
    let content = nonnegative(content);
    let track = length - 2 * BAR_WIDTH + 2;
    if content <= viewport || track < BAR_WIDTH {
        return None;
    }
    let thumb_length = ((track as f64 * viewport / content) as i32).clamp(BAR_WIDTH, track);
    let fraction = Range::new(0., content - viewport).fraction(value);
    Some(Thumb {
        position: BAR_WIDTH - 1 + (fraction * (track - thumb_length) as f64).round() as i32,
        length: thumb_length,
    })
}

fn thumb_face(theme: Theme) -> u32 {
    let a = theme.accent.light;
    let b = theme.accent.mid;
    let mut result = 0;
    for shift in [0, 8, 16] {
        result |= (((a >> shift) & 255) + ((b >> shift) & 255)).div_ceil(2) << shift;
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderState {
    Normal,
    Pressed,
    Disabled,
    Inactive,
}

fn draw_slider(pen: &mut Pen, config: &Config, value: f64, pressed: bool) {
    let track_width = pen.width - 10;
    let state = if config.disabled || config.range.max <= config.range.min {
        SliderState::Disabled
    } else if config.inactive {
        SliderState::Inactive
    } else if pressed {
        SliderState::Pressed
    } else {
        SliderState::Normal
    };
    let (line, empty) = match state {
        SliderState::Disabled => (GRAY_6, GRAY_3),
        SliderState::Inactive => (GRAY_8, GRAY_3),
        _ => (GRAY_11, GRAY_4),
    };
    pen.bevel(3, 4, track_width + 4, 6, GRAY_7, WHITE);
    pen.frame(4, 5, track_width + 2, 4, line);
    pen.rect(5, 6, track_width, 2, empty);
    if config.ticks > 1 {
        // More ticks than pixels cannot add detail, and would waste drawing work.
        let ticks = config.ticks.min(track_width as u32);
        for tick in 0..ticks {
            let x = 5 + (tick as i64 * (track_width - 1) as i64 / (ticks - 1) as i64) as i32;
            pen.vline(x, 18, 4, line);
        }
    }
    let x = (config.range.fraction(value) * (track_width - 1) as f64).round() as i32;
    let mut thumb = Pen::new(THUMB_WIDTH, 16);
    draw_slider_thumb(&mut thumb, state, config.theme);
    for mut fill in thumb.fills {
        fill.rect.x += x;
        pen.fills.push(fill);
    }
}

/// Draws the pointed eleven-by-sixteen-pixel thumb used by a slider.
pub fn draw_slider_thumb(pen: &mut Pen, state: SliderState, theme: Theme) {
    let (frame, face, top, bottom) = match state {
        SliderState::Disabled => (GRAY_6, GRAY_2, GRAY_1, GRAY_4),
        SliderState::Inactive => (GRAY_8, GRAY_2, GRAY_2, GRAY_2),
        SliderState::Pressed => (
            BLACK,
            theme.accent.mid,
            theme.accent.dark,
            theme.accent.light,
        ),
        SliderState::Normal => (BLACK, thumb_face(theme), WHITE, theme.accent.dark),
    };
    // Each pixel drawn once, as a faded thumb shows what is under it.
    for y in 0..16 {
        let inset = (y - 10).max(0);
        let width = THUMB_WIDTH - inset * 2;
        if y == 0 || width <= 2 {
            pen.hline(inset, y, width, frame);
            continue;
        }
        pen.dot(inset, y, frame);
        pen.dot(inset + width - 1, y, frame);
        let (left, right) = (inset + 1, inset + width - 2);
        if y == 1 {
            pen.hline(left, y, right - left + 1, top);
            continue;
        }
        for x in left..=right {
            let color = if x == right {
                bottom
            } else if x == left && y < 11 {
                top
            } else {
                face
            };
            pen.dot(x, y, color);
        }
    }
    if matches!(state, SliderState::Normal | SliderState::Pressed) {
        for ridge in 0..3 {
            pen.hline(3, 4 + 2 * ridge, 5, theme.accent.dark);
            pen.hline(4, 5 + 2 * ridge, 5, WHITE);
        }
    }
}

/// Paints what the pen drew at half over what is under it, as the original
/// paints a control it disables, each color made so that over the window's
/// face it comes out as asked: white, lighter than the face, comes out
/// Gray1.
fn fade(pen: &mut Pen) {
    let unfade = |color: u32| {
        let channel = |shift: u32| {
            let (value, face) = ((color >> shift) & 255, (GRAY_2 >> shift) & 255);
            (2 * value as i32 - face as i32).clamp(0, 255) as u32
        };
        channel(16) << 16 | channel(8) << 8 | channel(0)
    };
    for fill in &mut pen.fills {
        fill.color = unfade(fill.color);
        fill.alpha = 128;
    }
}

fn draw_scroll_bar(pen: &mut Pen, config: &Config, value: f64, pressed: ScrollPart) {
    if config.inactive {
        pen.rect(0, 0, pen.width, pen.height, GRAY_2);
        pen.frame(0, 0, pen.width, pen.height, BLACK);
        return;
    }
    let Kind::ScrollBar {
        viewport,
        content,
        horizontal,
    } = config.kind
    else {
        return;
    };
    let length = config.length;
    let active = config.enabled();
    let thumb = scroll_thumb(length, value, viewport, content);
    // Draw vertically, transposing rectangles for the horizontal bar.
    let mut rect = |x: i32, y: i32, width: i32, height: i32, color| {
        if horizontal {
            pen.rect(y, x, height, width, color);
        } else {
            pen.rect(x, y, width, height, color);
        }
    };
    rect(0, 0, BAR_WIDTH, length, BLACK);
    rect(
        1,
        BAR_WIDTH,
        BAR_WIDTH - 2,
        length - 2 * BAR_WIDTH,
        if active { GRAY_4 } else { GRAY_2 },
    );
    if active {
        rect(1, BAR_WIDTH, 1, length - 2 * BAR_WIDTH, GRAY_7);
        rect(2, BAR_WIDTH, BAR_WIDTH - 3, 1, GRAY_7);
        rect(
            BAR_WIDTH - 2,
            BAR_WIDTH + 1,
            1,
            length - 2 * BAR_WIDTH - 1,
            GRAY_3,
        );
        if let Some(thumb) = thumb {
            if pressed == ScrollPart::PageBack {
                rect(
                    2,
                    BAR_WIDTH + 1,
                    BAR_WIDTH - 4,
                    thumb.position - BAR_WIDTH - 1,
                    GRAY_6,
                );
            }
            if pressed == ScrollPart::PageForward {
                rect(
                    2,
                    thumb.position + thumb.length,
                    BAR_WIDTH - 4,
                    length - BAR_WIDTH - 1 - thumb.position - thumb.length,
                    GRAY_6,
                );
            }
        }
    }
    for (y, part) in [
        (0, ScrollPart::Back),
        (length - BAR_WIDTH, ScrollPart::Forward),
    ] {
        let (face, top, bottom) = if pressed == part {
            (GRAY_5, GRAY_7, GRAY_3)
        } else {
            (GRAY_2, WHITE, GRAY_7)
        };
        rect(1, y + 1, 14, 14, face);
        rect(1, y + 1, 13, 1, top);
        rect(1, y + 2, 1, 12, top);
        rect(2, y + 14, 13, 1, bottom);
        rect(14, y + 2, 1, 12, bottom);
        let ink = if active { BLACK } else { GRAY_6 };
        for row in 0..4 {
            let draw_y = if part == ScrollPart::Back {
                y + 6 + row
            } else {
                y + 9 - row
            };
            rect(7 - row, draw_y, 1 + row * 2, 1, ink);
        }
    }
    if active && let Some(thumb) = thumb {
        let pos = thumb.position;
        let length = thumb.length;
        let face = if pressed == ScrollPart::Thumb {
            config.theme.accent.mid
        } else {
            thumb_face(config.theme)
        };
        let dark = config.theme.accent.dark;
        rect(1, pos, 14, length, BLACK);
        rect(1, pos + 1, 14, length - 2, face);
        rect(1, pos + 1, 13, 1, WHITE);
        rect(1, pos + 2, 1, length - 4, WHITE);
        rect(2, pos + length - 2, 13, 1, dark);
        rect(14, pos + 2, 1, length - 4, dark);
        let middle = pos + length / 2;
        for ridge in -2..2 {
            rect(4, middle + ridge * 2, 8, 1, dark);
            rect(5, middle + ridge * 2 + 1, 8, 1, WHITE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slider(min: f64, max: f64) -> Config {
        Slider::new("test", min, min, max).element.config
    }

    #[test]
    fn slider_maps_endpoints_and_clamps_outside_drags() {
        let config = slider(-20., 80.);
        assert_eq!(config.drag_value(5., 0.), -20.);
        assert_eq!(config.drag_value(154., 0.), 80.);
        assert_eq!(config.drag_value(-100., 0.), -20.);
        assert_eq!(config.drag_value(500., 0.), 80.);
        assert_eq!(config.drag_value(81.5, 2.), 30.);
    }

    #[test]
    fn steps_start_at_minimum_and_keep_both_endpoints() {
        let range = Range::new(3., 12.);
        assert_eq!(range.snap(7., 5.), 8.);
        assert_eq!(range.snap(-1., 5.), 3.);
        assert_eq!(range.snap(12., 5.), 12.);
        assert_eq!(range.snap(10.9, 0.), 10.9);
    }

    #[test]
    fn keyboard_directions_follow_each_control() {
        let slider = slider(0., 100.);
        assert_eq!(slider.key_value("up", 50.), Some(51.));
        assert_eq!(slider.key_value("pageup", 50.), Some(60.));
        assert_eq!(slider.key_value("home", 50.), Some(0.));
        assert_eq!(slider.key_value("end", 50.), Some(100.));
        let bar = ScrollBar::new("test", 100., 80., 800.).element.config;
        assert_eq!(bar.key_value("up", 100.), Some(84.));
        assert_eq!(bar.key_value("pageup", 100.), Some(36.));
        assert_eq!(bar.key_value("pagedown", 700.), Some(720.));
    }

    #[test]
    fn scrollbar_thumb_matches_content_and_reaches_both_ends() {
        assert_eq!(
            scroll_thumb(190, 0., 100., 400.),
            Some(Thumb {
                position: 15,
                length: 40
            })
        );
        assert_eq!(
            scroll_thumb(190, 300., 100., 400.),
            Some(Thumb {
                position: 135,
                length: 40
            })
        );
        let bar = ScrollBar::new("test", 0., 100., 400.)
            .length(190)
            .element
            .config;
        assert_eq!(bar.drag_value(19., 4.), 0.);
        assert_eq!(bar.drag_value(139., 4.), 300.);
        assert_eq!(bar.drag_value(79., 4.), 150.);
        assert_eq!(bar.part_at(80., 0.), ScrollPart::PageForward);
    }

    #[test]
    fn empty_invalid_and_small_ranges_do_not_divide_by_zero() {
        assert_eq!(scroll_thumb(100, 0., 100., 100.), None);
        assert_eq!(scroll_thumb(40, 0., 10., 100.), None);
        assert_eq!(scroll_thumb(100, 0., 0., 100.).unwrap().length, BAR_WIDTH);
        let range = Range::new(5., 2.);
        assert_eq!(range.fraction(9.), 0.);
        assert_eq!(range.at_fraction(0.5), 5.);
        assert_eq!(range.clamp(f64::NAN), 5.);
        assert!(!slider(5., 5.).enabled());
        assert!(
            !ScrollBar::new("empty", 0., 10., 0.)
                .element
                .config
                .enabled()
        );
    }
}

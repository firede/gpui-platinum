//! Tool strips, help balloons, scrolling surfaces, and docked windows.

pub use crate::toolbar::{Toolbar, toolbar_separator};
use crate::{
    Control, Font, Language, ScrollBar, Theme, bevel_button,
    paint::{Pen, Rect},
    theme::*,
};
use gpui::{prelude::*, *};
use std::time::{Duration, Instant};

#[derive(IntoElement)]
pub struct Balloon {
    id: ElementId,
    text: SharedString,
    content: AnyElement,
    language: Language,
    anchor_focus: Option<FocusHandle>,
}
impl Balloon {
    pub fn new(
        id: impl Into<ElementId>,
        text: impl Into<SharedString>,
        content: impl IntoElement,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            content: content.into_any_element(),
            language: Language::English,
            anchor_focus: None,
        }
    }
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }
    /// Reuses the anchor's focus handle without adding another keyboard stop.
    pub fn anchor_focus(mut self, focus: &FocusHandle) -> Self {
        self.anchor_focus = Some(focus.clone());
        self
    }
}

#[derive(Default)]
struct BalloonVisibility {
    focused: bool,
    hovered: bool,
    dismissed: bool,
}
impl BalloonVisibility {
    fn reset_when_inactive(&mut self) {
        if !self.focused && !self.hovered {
            self.dismissed = false;
        }
    }
}

struct BalloonState {
    visibility: BalloonVisibility,
    _subscriptions: Vec<Subscription>,
}
impl BalloonState {
    fn new(focus: Option<&FocusHandle>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut subscriptions = Vec::new();
        if let Some(focus) = focus {
            subscriptions.push(cx.on_focus_in(focus, window, |this, window, cx| {
                this.visibility.focused = true;
                cx.notify();
                window.on_next_frame(|window, _| window.refresh());
            }));
            subscriptions.push(cx.on_focus_out(focus, window, |this, _, window, cx| {
                this.visibility.focused = false;
                this.visibility.reset_when_inactive();
                cx.notify();
                window.on_next_frame(|window, _| window.refresh());
            }));
        }
        Self {
            visibility: BalloonVisibility {
                focused: focus.is_some_and(|focus| focus.contains_focused(window, cx)),
                ..Default::default()
            },
            _subscriptions: subscriptions,
        }
    }
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.visibility.dismissed = true;
        cx.notify();
    }
}

struct HelpBalloon(SharedString, Language);
impl Render for HelpBalloon {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let width = Font::Small.measure_in(&self.0, self.1).min(200) + 20;
        div()
            .id("help-balloon")
            .role(Role::Tooltip)
            .child(crate::balloon_in(self.0.clone(), width, self.1))
    }
}
impl RenderOnce for Balloon {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |window, cx| {
            BalloonState::new(self.anchor_focus.as_ref(), window, cx)
        });
        let focused = self
            .anchor_focus
            .as_ref()
            .is_some_and(|focus| focus.contains_focused(window, cx));
        state.update(cx, |state, _| {
            state.visibility.focused = focused;
            state.visibility.reset_when_inactive();
        });
        let visible = &state.read(cx).visibility;
        let by_focus = visible.focused && window.last_input_was_keyboard() && !visible.dismissed;
        let hover_enabled = !visible.dismissed && !by_focus;
        let hovering = state.clone();
        let keyboard = state.clone();
        let mouse = state.clone();
        let mut root = div()
            .id(self.id)
            .relative()
            .on_hover(move |hovered, _, cx| {
                hovering.update(cx, |state, cx| {
                    state.visibility.hovered = *hovered;
                    state.visibility.reset_when_inactive();
                    cx.notify();
                });
            })
            .capture_key_down(move |event, _, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "escape" => {
                        let visible = &keyboard.read(cx).visibility;
                        if !visible.dismissed && (visible.focused || visible.hovered) {
                            keyboard.update(cx, BalloonState::dismiss);
                            cx.stop_propagation();
                        }
                    }
                    "enter" | "space" => keyboard.update(cx, BalloonState::dismiss),
                    _ => {}
                }
            })
            .child(self.content)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        window.on_mouse_event(move |event: &MouseDownEvent, phase, _, cx| {
                            if phase.capture() && bounds.contains(&event.position) {
                                mouse.update(cx, BalloonState::dismiss);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0()
                .size_full(),
            );
        if by_focus {
            let tip = cx.new(|_| HelpBalloon(self.text.clone(), self.language));
            root = root.child(
                deferred(
                    anchored()
                        .anchor(Anchor::TopLeft)
                        .offset(point(px(0.), px(4.)))
                        .snap_to_window_with_margin(px(4.))
                        .child(tip),
                )
                .priority(10),
            );
        }
        if hover_enabled {
            root = root
                .tooltip_show_delay(Duration::from_millis(600))
                .tooltip(move |_, cx| {
                    cx.new(|_| HelpBalloon(self.text.clone(), self.language))
                        .into()
                });
        }
        root
    }
}

#[derive(IntoElement)]
pub struct ScrollView {
    id: ElementId,
    content: AnyElement,
    handle: ScrollHandle,
    width: i32,
    height: i32,
    theme: Theme,
}
impl ScrollView {
    pub fn new(id: impl Into<ElementId>, handle: &ScrollHandle, content: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            content: content.into_any_element(),
            handle: handle.clone(),
            width: 240,
            height: 120,
            theme: Theme::default(),
        }
    }
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width.max(34);
        self.height = height.max(34);
        self
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}
impl RenderOnce for ScrollView {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        let position = -f32::from(self.handle.offset().y);
        let extent = f32::from(self.handle.max_offset().y).abs();
        let height = self.height;
        let handle = self.handle.clone();
        let measured = self.handle.clone();
        window.on_next_frame(move |window, _| {
            if f32::from(measured.max_offset().y).abs() != extent {
                window.refresh();
            }
        });
        div()
            .id(self.id)
            .flex()
            .flex_shrink_0()
            .w(px(self.width as f32))
            .h(px(height as f32))
            .border_1()
            .border_color(rgb(BLACK))
            .child(
                div()
                    .id("viewport")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_x_hidden()
                    .overflow_y_scroll()
                    .track_scroll(&self.handle)
                    .on_scroll_wheel(|_, window, _| window.refresh())
                    .child(self.content),
            )
            .child(
                ScrollBar::new(
                    "scroll",
                    position as f64,
                    (height - 2) as f64,
                    (extent + height as f32 - 2.) as f64,
                )
                .length(height - 2)
                .theme(self.theme)
                .on_change(move |position, window, _| {
                    handle.set_offset(point(px(0.), px(-position as f32)));
                    window.refresh();
                }),
            )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupWindowEvent {
    Opened,
    Closed,
    Moved,
    TornOff,
}

pub struct PopupWindow {
    title: SharedString,
    content: Option<AnyView>,
    focus: FocusHandle,
    x: i32,
    width: i32,
    height: i32,
    open: bool,
    theme: Theme,
    language: Language,
    drag: Option<(Point<Pixels>, i32)>,
    moved: bool,
    spring_at: Option<Instant>,
}
impl EventEmitter<PopupWindowEvent> for PopupWindow {}
impl Focusable for PopupWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl PopupWindow {
    pub fn new(title: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            title: title.into(),
            content: None,
            focus: cx.focus_handle().tab_stop(true),
            x: 8,
            width: 180,
            height: 100,
            open: true,
            theme: Theme::default(),
            language: Language::English,
            drag: None,
            moved: false,
            spring_at: None,
        }
    }
    pub fn content(mut self, content: impl Into<AnyView>) -> Self {
        self.content = Some(content.into());
        self
    }
    pub fn position(mut self, x: i32) -> Self {
        self.x = x.max(0);
        self
    }
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width.max(104);
        self.height = height.max(32);
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
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_title(&mut self, title: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.title = title.into();
        cx.notify();
    }
    pub fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.open != open {
            self.open = open;
            self.spring_at = None;
            cx.emit(if open {
                PopupWindowEvent::Opened
            } else {
                PopupWindowEvent::Closed
            });
            cx.notify();
        }
    }
    /// Opens a closed tab when a drag has rested over it for half a second.
    pub fn spring_open(&mut self, dragging_over: bool, cx: &mut Context<Self>) {
        if !dragging_over || self.open {
            self.spring_at = None;
            return;
        }
        if self.spring_at.is_some() {
            return;
        }
        let start = Instant::now();
        self.spring_at = Some(start);
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(500))
                .await;
            this.update(cx, |this, cx| {
                if this.spring_at == Some(start) && !this.open {
                    this.set_open(true, cx);
                }
            })
            .ok();
        })
        .detach();
    }
}
impl Render for PopupWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = self.title.clone();
        let width = self.width;
        let height = if self.open { self.height + 19 } else { 20 };
        let tab_width = (Font::Bold.measure_in(&title, self.language) + 32).clamp(80, width - 16);
        let open = self.open;
        let language = self.language;
        let focused = self.focus.is_focused(window) && window.last_input_was_keyboard();
        let accent = self.theme.accent.dark;
        let entity = cx.entity().downgrade();
        let pressed = self.drag.is_some() && !self.moved;
        let frame = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let mut pen = Pen::new(width, height);
                pen.language = language;
                draw_popup_window(&mut pen, &title, tab_width, open, pressed);
                if focused {
                    pen.rect(24, 15, tab_width - 24, 2, accent);
                }
                pen.paint(bounds.origin, window);
                let moving = entity.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    moving
                        .update(cx, |this, cx| {
                            if let Some((start, x)) = this.drag {
                                if !event.dragging() {
                                    this.drag = None;
                                    this.moved = false;
                                    cx.notify();
                                    return;
                                }
                                let dx = f32::from(event.position.x - start.x);
                                let dy = f32::from(event.position.y - start.y);
                                this.moved |= dx.abs() > 3. || dy.abs() > 3.;
                                if this.moved {
                                    this.x = (x + dx.round() as i32).max(0);
                                    cx.notify();
                                }
                            } else {
                                let tab = Bounds::new(
                                    bounds.origin + point(px(12.), px(0.)),
                                    size(px(tab_width as f32), px(20.)),
                                );
                                this.spring_open(
                                    event.dragging() && tab.contains(&event.position),
                                    cx,
                                );
                            }
                        })
                        .ok();
                });
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                    if phase != DispatchPhase::Bubble || event.button != MouseButton::Left {
                        return;
                    }
                    entity
                        .update(cx, |this, cx| {
                            this.spring_at = None;
                            if let Some((start, _)) = this.drag.take() {
                                if f32::from(start.y - event.position.y) > 24. {
                                    cx.emit(PopupWindowEvent::TornOff);
                                } else if this.moved {
                                    cx.emit(PopupWindowEvent::Moved);
                                } else {
                                    this.set_open(!this.open, cx);
                                }
                                this.moved = false;
                                cx.notify();
                            }
                        })
                        .ok();
                });
            },
        )
        .absolute()
        .size_full();
        let accessible = cx.entity();
        div()
            .id("popup-window")
            .role(Role::Window)
            .aria_label(self.title.clone())
            .absolute()
            .bottom_0()
            .left(px(self.x as f32))
            .w(px(width as f32))
            .h(px(height as f32))
            .occlude()
            .child(frame)
            .child(
                div()
                    .id("tab")
                    .role(Role::Button)
                    .aria_label(self.title.clone())
                    .aria_expanded(open)
                    .track_focus(&self.focus)
                    .tab_stop(true)
                    .absolute()
                    .left(px(12.))
                    .top_0()
                    .w(px(tab_width as f32))
                    .h(px(20.))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            window.focus(&this.focus, cx);
                            this.drag = Some((event.position, this.x));
                            this.moved = false;
                            cx.notify();
                            cx.stop_propagation();
                        }),
                    )
                    .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                        accessible.update(cx, |this, cx| {
                            window.focus(&this.focus, cx);
                            this.set_open(!this.open, cx);
                        });
                    })
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        match event.keystroke.key.as_str() {
                            "enter" | "space" => this.set_open(!this.open, cx),
                            "w" if event.keystroke.modifiers.platform => this.set_open(false, cx),
                            _ => return,
                        }
                        cx.stop_propagation();
                    })),
            )
            .when(open, |root| {
                root.child(
                    div()
                        .absolute()
                        .left(px(6.))
                        .right(px(6.))
                        .top(px(25.))
                        .bottom_0()
                        .overflow_hidden()
                        .when_some(self.content.clone(), |root, content| root.child(content)),
                )
            })
    }
}

pub fn draw_popup_window(pen: &mut Pen, title: &str, tab_width: i32, open: bool, pressed: bool) {
    let (face, light, dark) = if pressed {
        (GRAY_5, GRAY_7, GRAY_3)
    } else {
        (GRAY_2, WHITE, GRAY_7)
    };
    let width = pen.width;
    if open {
        let height = pen.height - 19;
        pen.rect(0, 19, width, height, GRAY_2);
        pen.hline(0, 19, width, BLACK);
        pen.vline(0, 19, height, BLACK);
        pen.vline(width - 1, 19, height, BLACK);
        pen.hline(1, 20, width - 2, light);
        pen.vline(1, 20, height - 1, light);
        pen.vline(width - 2, 21, height - 2, dark);
        pen.hline(4, 23, width - 9, dark);
        pen.vline(4, 23, height, dark);
        pen.vline(width - 5, 24, height, light);
        pen.hline(5, 24, width - 10, BLACK);
        pen.vline(5, 24, height, BLACK);
        pen.vline(width - 6, 24, height, BLACK);
    }
    pen.rect(13, 1, tab_width - 2, 19, face);
    pen.hline(14, 0, tab_width - 4, BLACK);
    pen.dot(13, 1, BLACK);
    pen.dot(12 + tab_width - 2, 1, BLACK);
    pen.vline(12, 2, 18, BLACK);
    pen.vline(12 + tab_width - 1, 2, 18, BLACK);
    pen.hline(14, 1, tab_width - 4, light);
    pen.vline(13, 2, 18, light);
    pen.vline(12 + tab_width - 2, 2, 18, dark);
    if open {
        pen.hline(13, 19, tab_width - 2, face);
    }
    let mut label = Pen::new(tab_width - 4, 20);
    label.language = pen.language;
    label.text(
        (tab_width - 4 - Font::Bold.measure_in(title, pen.language)) / 2,
        14,
        title,
        Font::Bold,
        BLACK,
    );
    for fill in label.fills {
        let left = fill.rect.x.max(0);
        let right = (fill.rect.x + fill.rect.width).min(tab_width - 4);
        pen.rect(
            14 + left,
            fill.rect.y,
            right - left,
            fill.rect.height,
            fill.color,
        );
    }
}

pub fn zoom_rects(
    from: Rect,
    to: Rect,
    start: Instant,
    width: i32,
    height: i32,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let progress = start.elapsed().as_secs_f32() / 0.18;
            if progress >= 1. {
                return;
            }
            let mut pen = Pen::new(width, height);
            for step in 0..4 {
                let amount = progress - step as f32 * 0.12;
                if amount < 0. {
                    continue;
                }
                let lerp = |a: i32, b: i32| (a as f32 + (b - a) as f32 * amount).round() as i32;
                let rect = Rect {
                    x: lerp(from.x, to.x),
                    y: lerp(from.y, to.y),
                    width: lerp(from.width, to.width),
                    height: lerp(from.height, to.height),
                };
                for x in (0..rect.width).step_by(2) {
                    pen.dot(rect.x + x, rect.y, GRAY_10);
                    pen.dot(rect.x + x, rect.y + rect.height - 1, GRAY_10);
                }
                for y in (0..rect.height).step_by(2) {
                    pen.dot(rect.x, rect.y + y, GRAY_10);
                    pen.dot(rect.x + rect.width - 1, rect.y + y, GRAY_10);
                }
            }
            pen.paint(bounds.origin, window);
            window.request_animation_frame();
        },
    )
    .w(px(width as f32))
    .h(px(height as f32))
}

pub fn icon_button(
    id: impl Into<ElementId>,
    icon: &'static [&'static str],
    label: impl Into<SharedString>,
) -> Control {
    bevel_button(id, label).icon_only(icon)
}

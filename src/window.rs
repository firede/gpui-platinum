//! Movable, resizable windows inside a Platinum desktop.

use crate::{Font, Theme, paint::Pen, theme::*};
use gpui::{prelude::*, *};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub collapsed: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: 20.,
            y: 20.,
            width: 300.,
            height: 180.,
            collapsed: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowEvent {
    Activated,
    Closed,
    Moved,
    Resized,
}

#[derive(Clone, Copy)]
enum Drag {
    Move {
        mouse: Point<Pixels>,
        state: WindowState,
    },
    Resize {
        mouse: Point<Pixels>,
        state: WindowState,
    },
}

pub struct PlatinumWindow {
    title: SharedString,
    pub state: WindowState,
    active: bool,
    theme: Theme,
    content: Option<AnyView>,
    drag: Option<Drag>,
    saved_size: Option<WindowState>,
    zoom_to: WindowState,
}

impl EventEmitter<WindowEvent> for PlatinumWindow {}

impl PlatinumWindow {
    pub fn new(title: impl Into<SharedString>, _: &mut Context<Self>) -> Self {
        Self {
            title: title.into(),
            state: WindowState::default(),
            active: true,
            theme: Theme::default(),
            content: None,
            drag: None,
            saved_size: None,
            zoom_to: WindowState {
                x: 4.,
                y: 4.,
                width: 400.,
                height: 260.,
                collapsed: false,
            },
        }
    }
    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.state.x = x;
        self.state.y = y;
        self
    }
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.state.width = width;
        self.state.height = height;
        self
    }
    pub fn content(mut self, view: impl Into<AnyView>) -> Self {
        self.content = Some(view.into());
        self
    }
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    pub fn zoom_to(mut self, state: WindowState) -> Self {
        self.zoom_to = state;
        self
    }
    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        self.active = active;
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_title(&mut self, title: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.title = title.into();
        cx.notify();
    }
    pub fn zoom(&mut self, cx: &mut Context<Self>) {
        if let Some(state) = self.saved_size.take() {
            self.state = state;
        } else {
            self.saved_size = Some(self.state);
            self.state = self.zoom_to;
        }
        cx.emit(WindowEvent::Resized);
        cx.notify();
    }
    pub fn collapse(&mut self, cx: &mut Context<Self>) {
        self.state.collapsed = !self.state.collapsed;
        cx.notify();
    }

    fn move_mouse(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if event.pressed_button != Some(MouseButton::Left) {
            self.drag = None;
            return;
        }
        match self.drag {
            Some(Drag::Move { mouse, state }) => {
                self.state.x = (state.x + f32::from(event.position.x - mouse.x))
                    .round()
                    .max(0.);
                self.state.y = (state.y + f32::from(event.position.y - mouse.y))
                    .round()
                    .max(0.);
                cx.emit(WindowEvent::Moved);
                cx.notify();
            }
            Some(Drag::Resize { mouse, state }) => {
                self.state.width = (state.width + f32::from(event.position.x - mouse.x))
                    .round()
                    .max(140.);
                self.state.height = (state.height + f32::from(event.position.y - mouse.y))
                    .round()
                    .max(70.);
                cx.emit(WindowEvent::Resized);
                cx.notify();
            }
            None => {}
        }
    }
}

impl Render for PlatinumWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state;
        let active = self.active;
        let title = self.title.clone();
        let width = state.width;
        let height = if state.collapsed { 25. } else { state.height };
        let theme = self.theme;
        let entity = cx.entity().downgrade();
        let frame = canvas(
            |_, _, _| (),
            move |bounds, _, window, _cx| {
                let mut pen = Pen::new(width as i32, height as i32);
                let (w, h) = (pen.width, pen.height);
                pen.raised(theme.background);
                pen.frame(2, 2, w - 4, h - 4, GRAY_5);
                if active {
                    for row in (5..20).step_by(2) {
                        pen.hline(5, row, w - 10, GRAY_7);
                    }
                }
                let title_width = Font::Bold.measure(&title);
                pen.rect((w - title_width) / 2 - 5, 3, title_width + 10, 19, GRAY_2);
                pen.text(
                    (w - title_width) / 2,
                    16,
                    &title,
                    Font::Bold,
                    if active { BLACK } else { GRAY_8 },
                );
                pen.hline(3, 22, w - 6, BLACK);
                if !state.collapsed {
                    pen.bevel(5, 25, w - 10, h - 30, GRAY_7, WHITE);
                    if active {
                        for i in 0..3 {
                            let offset = i * 4;
                            pen.hline(w - 14 + offset, h - 4 - offset, 10 - offset, GRAY_7);
                            pen.vline(w - 4 - offset, h - 14 + offset, 10 - offset, GRAY_7);
                        }
                    }
                }
                pen.paint(bounds.origin, window);
                let moving = entity.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble {
                        moving
                            .update(cx, |this, cx| this.move_mouse(event, cx))
                            .ok();
                    }
                });
                window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble {
                        entity.update(cx, |this, _| this.drag = None).ok();
                    }
                });
            },
        )
        .absolute()
        .size_full();
        let mut root = div()
            .id("window")
            .role(Role::Window)
            .aria_label(self.title.clone())
            .absolute()
            .left(px(state.x))
            .top(px(state.y))
            .w(px(width))
            .h(px(height))
            .occlude()
            .child(frame)
            .child(
                div()
                    .id("title")
                    .absolute()
                    .left(px(23.))
                    .right(px(42.))
                    .top(px(3.))
                    .h(px(19.))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _, cx| {
                            if !this.active {
                                cx.emit(WindowEvent::Activated);
                                cx.stop_propagation();
                                return;
                            }
                            if event.click_count == 2 {
                                this.collapse(cx);
                            } else {
                                this.drag = Some(Drag::Move {
                                    mouse: event.position,
                                    state: this.state,
                                });
                            }
                            cx.stop_propagation();
                        }),
                    ),
            );
        if self.active {
            root = root
                .child(
                    title_box("close", 7., TitleBox::Close)
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(WindowEvent::Closed))),
                )
                .child(
                    title_box("zoom", width - 37., TitleBox::Zoom)
                        .on_click(cx.listener(|this, _, _, cx| this.zoom(cx))),
                )
                .child(
                    title_box("collapse", width - 20., TitleBox::Collapse)
                        .on_click(cx.listener(|this, _, _, cx| this.collapse(cx))),
                );
        }
        if !state.collapsed {
            root = root
                .child(
                    div()
                        .absolute()
                        .left(px(7.))
                        .top(px(27.))
                        .right(px(7.))
                        .bottom(px(8.))
                        .overflow_hidden()
                        .when_some(self.content.clone(), |el, content| el.child(content)),
                )
                .child(
                    div()
                        .id("resize")
                        .absolute()
                        .right(px(2.))
                        .bottom(px(2.))
                        .size(px(14.))
                        .cursor(CursorStyle::ResizeUpLeftDownRight)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                if this.active {
                                    this.drag = Some(Drag::Resize {
                                        mouse: event.position,
                                        state: this.state,
                                    });
                                }
                                cx.stop_propagation();
                            }),
                        ),
                );
        }
        if !self.active {
            root = root.child(div().absolute().inset_0().occlude().on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| {
                    cx.emit(WindowEvent::Activated);
                    cx.stop_propagation();
                }),
            ));
        }
        root
    }
}

#[derive(Clone, Copy)]
enum TitleBox {
    Close,
    Zoom,
    Collapse,
}

fn title_box(id: &'static str, left: f32, kind: TitleBox) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(match kind {
            TitleBox::Close => "Close",
            TitleBox::Zoom => "Zoom",
            TitleBox::Collapse => "Collapse",
        })
        .absolute()
        .left(px(left))
        .top(px(6.))
        .w(px(12.))
        .h(px(12.))
        .text_color(rgb(BLACK))
        .active(|style| style.text_color(rgb(WHITE)))
        .child(crate::paint::drawing(12., 12., move |pen, window, _| {
            let pressed = window.text_style().color == rgb(WHITE).into();
            pen.raised(if pressed { GRAY_4 } else { GRAY_2 });
            match kind {
                TitleBox::Close => {}
                TitleBox::Zoom => {
                    pen.frame(3, 3, 6, 6, BLACK);
                    pen.hline(3, 4, 6, BLACK);
                }
                TitleBox::Collapse => {
                    pen.hline(3, 5, 6, BLACK);
                    pen.hline(3, 6, 6, BLACK);
                }
            }
        }))
}

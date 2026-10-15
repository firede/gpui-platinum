//! Movable, resizable windows inside a Platinum desktop.

use crate::{
    Font, Language, Theme,
    paint::{Pen, Rect},
    theme::*,
};
use gpui::{prelude::*, *};

pub(crate) use crate::inactive::content_is_inactive;

pub const TITLE_BAR_HEIGHT: i32 = 19;
pub const FRAME_WIDTH: i32 = 6;
pub const TITLE_BOX_SIZE: i32 = 11;
pub const GROW_BOX_SIZE: i32 = 15;
pub const UTILITY_BAR_HEIGHT: i32 = 13;
pub const UTILITY_BOX_SIZE: i32 = 9;
const TITLE_BOX_INSET: i32 = 8;
const TITLE_BOX_GAP: i32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatinumWindowOptions {
    pub active: bool,
    pub no_close: bool,
    pub no_zoom: bool,
    pub no_collapse: bool,
    pub no_grow: bool,
    pub modal: bool,
    pub interactive: bool,
    pub dragging: bool,
    pub utility: bool,
}

impl Default for PlatinumWindowOptions {
    fn default() -> Self {
        Self {
            active: true,
            no_close: false,
            no_zoom: false,
            no_collapse: false,
            no_grow: false,
            modal: false,
            interactive: false,
            dragging: false,
            utility: false,
        }
    }
}

impl PlatinumWindowOptions {
    pub fn normalized(mut self) -> Self {
        if self.utility {
            self.no_zoom = true;
            self.no_grow = true;
        }
        if self.modal {
            self.no_close = true;
            self.no_zoom = true;
            self.no_collapse = true;
            self.no_grow = true;
        }
        self
    }

    pub fn title_bar_height(self) -> i32 {
        if self.utility {
            UTILITY_BAR_HEIGHT
        } else {
            TITLE_BAR_HEIGHT
        }
    }

    pub fn frame_width(self) -> i32 {
        if self.utility {
            3
        } else if self.modal {
            FRAME_WIDTH + 2
        } else {
            FRAME_WIDTH
        }
    }

    fn title_box_metrics(self) -> (i32, i32, i32) {
        if self.utility {
            (UTILITY_BOX_SIZE, 2, 4)
        } else {
            (TITLE_BOX_SIZE, 4, TITLE_BOX_INSET)
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowPart {
    #[default]
    None,
    Title,
    Close,
    Zoom,
    Collapse,
    Grow,
    Content,
}

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
    /// Index of an ancestor in the outermost-first path supplied to the frame.
    PathChosen(usize),
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
    options: PlatinumWindowOptions,
    theme: Theme,
    language: Language,
    content: Option<AnyView>,
    drag: Option<Drag>,
    saved_size: Option<WindowState>,
    zoom_to: WindowState,
    path: Vec<String>,
    path_menu: Entity<crate::MenuBar>,
    minimum_width: f32,
    minimum_height: f32,
    bounds: Option<Bounds<Pixels>>,
}

impl EventEmitter<WindowEvent> for PlatinumWindow {}

impl PlatinumWindow {
    pub fn new(title: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        let path_menu = cx.new(|cx| crate::MenuBar::context_menu(Vec::new(), cx));
        cx.subscribe(&path_menu, |this, _, choice: &crate::MenuChoice, cx| {
            if let Some(index) = choice.path.first()
                && *index > 0
                && *index <= this.path.len()
            {
                cx.emit(WindowEvent::PathChosen(this.path.len() - index));
            }
        })
        .detach();
        Self {
            title: title.into(),
            state: WindowState::default(),
            options: PlatinumWindowOptions::default(),
            theme: Theme::default(),
            language: Language::English,
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
            path: Vec::new(),
            path_menu,
            minimum_width: 120.,
            minimum_height: 80.,
            bounds: None,
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
        self.options.active = active;
        self
    }
    pub fn options(mut self, options: PlatinumWindowOptions) -> Self {
        self.options = options.normalized();
        self
    }
    pub fn frame_options(&self) -> PlatinumWindowOptions {
        self.options.normalized()
    }
    pub fn set_options(&mut self, options: PlatinumWindowOptions, cx: &mut Context<Self>) {
        self.options = options.normalized();
        cx.notify();
    }
    pub fn minimum_size(mut self, width: f32, height: f32) -> Self {
        self.minimum_width = width.max(1.);
        self.minimum_height = height.max(1.);
        self
    }
    pub fn content_size(&self) -> Size<Pixels> {
        let options = self.options.normalized();
        size(
            px((self.state.width - (options.frame_width() * 2) as f32).max(0.)),
            px(
                (self.state.height - (options.title_bar_height() + options.frame_width()) as f32)
                    .max(0.),
            ),
        )
    }
    pub fn is_zoomed(&self) -> bool {
        self.saved_size.is_some()
    }
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }
    /// Ancestor folders in outermost-first order. Command-clicking the title
    /// displays the current title followed by its parents.
    pub fn path(mut self, path: Vec<String>) -> Self {
        self.path = path;
        self
    }
    pub fn set_path(&mut self, path: Vec<String>, cx: &mut Context<Self>) {
        self.path = path;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        cx.notify();
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
        self.options.active = active;
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
        if self.options.normalized().no_zoom {
            return;
        }
        let collapsed = self.state.collapsed;
        if let Some(state) = self.saved_size.take() {
            self.state = state;
        } else {
            self.saved_size = Some(self.state);
            self.state = self.zoom_to;
        }
        self.state.collapsed = collapsed;
        cx.emit(WindowEvent::Resized);
        cx.notify();
    }
    pub fn collapse(&mut self, cx: &mut Context<Self>) {
        if self.options.normalized().no_collapse {
            return;
        }
        self.state.collapsed = !self.state.collapsed;
        cx.emit(WindowEvent::Resized);
        cx.notify();
    }

    fn move_mouse(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if event.pressed_button != Some(MouseButton::Left) {
            self.drag = None;
            return;
        }
        match self.drag {
            Some(Drag::Move { mouse, state }) => {
                self.state.x = (state.x + f32::from(event.position.x - mouse.x)).round();
                self.state.y = (state.y + f32::from(event.position.y - mouse.y))
                    .round()
                    .max(0.);
                cx.emit(WindowEvent::Moved);
                cx.notify();
            }
            Some(Drag::Resize { mouse, state }) => {
                self.state.width = (state.width + f32::from(event.position.x - mouse.x))
                    .round()
                    .max(self.minimum_width);
                self.state.height = (state.height + f32::from(event.position.y - mouse.y))
                    .round()
                    .max(self.minimum_height);
                self.saved_size = None;
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
        let options = self.options.normalized();
        let title = self.title.clone();
        let width = state.width;
        let height = if state.collapsed {
            options.title_bar_height() as f32
        } else {
            state.height
        };
        let language = self.language;
        let entity = cx.entity().downgrade();
        let layout = cx.entity().downgrade();
        let frame = canvas(
            move |bounds, _, cx| {
                layout.update(cx, |this, _| this.bounds = Some(bounds)).ok();
            },
            move |bounds, _, window, _| {
                let mut pen = Pen::new(width.round() as i32, height.round() as i32);
                pen.language = language;
                draw_window_frame(&mut pen, &title, options, state.collapsed, WindowPart::None);
                window.with_content_mask(Some(ContentMask { bounds }), |window| {
                    pen.paint(bounds.origin, window)
                });
                let moving = entity.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                    if phase.bubble() {
                        moving
                            .update(cx, |this, cx| this.move_mouse(event, cx))
                            .ok();
                    }
                });
                window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
                    if phase.bubble() {
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
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.platform
                    && event.keystroke.key == "w"
                    && !this.options.normalized().no_close
                {
                    cx.emit(WindowEvent::Closed);
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if !this.options.active {
                        cx.emit(WindowEvent::Activated);
                    }
                }),
            )
            .child(frame)
            .child(
                div()
                    .id("title")
                    .absolute()
                    .left(px(0.))
                    .right(px(0.))
                    .top(px(0.))
                    .h(px(options.title_bar_height() as f32))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            if !this.options.active {
                                cx.emit(WindowEvent::Activated);
                            }
                            if event.modifiers.platform && !this.path.is_empty() {
                                let mut items =
                                    vec![crate::MenuItem::new(this.title.clone()).checked(true)];
                                items.extend(
                                    this.path
                                        .iter()
                                        .rev()
                                        .map(|name| crate::MenuItem::new(name.clone())),
                                );
                                let theme = this.theme;
                                let language = this.language;
                                let options = this.options.normalized();
                                let font = if options.utility {
                                    Font::Small
                                } else {
                                    Font::Bold
                                };
                                let title_width = font.measure_in(&this.title, language) as f32;
                                let origin = this
                                    .bounds
                                    .map_or(point(px(this.state.x), px(this.state.y)), |bounds| {
                                        bounds.origin
                                    });
                                let position = point(
                                    origin.x + px((this.state.width - title_width) / 2. - 16.),
                                    origin.y + px(options.title_bar_height() as f32),
                                );
                                this.path_menu.update(cx, |menu, cx| {
                                    menu.set_menus(vec![crate::Menu::new("", items)], cx);
                                    menu.set_theme(theme, cx);
                                    menu.set_language(language, cx);
                                    menu.show_at(0, position, window, cx);
                                });
                            } else if event.click_count == 2
                                && !this.options.normalized().no_collapse
                            {
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
        if options.active {
            let (box_size, _, inset) = options.title_box_metrics();
            if !options.no_close {
                root = root.child(
                    title_box("close", inset as f32, WindowPart::Close, options)
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(WindowEvent::Closed))),
                );
            }
            let mut right = width - (inset + box_size) as f32;
            if !options.no_collapse {
                root = root.child(
                    title_box("collapse", right, WindowPart::Collapse, options)
                        .on_click(cx.listener(|this, _, _, cx| this.collapse(cx))),
                );
                right -= (box_size + TITLE_BOX_GAP) as f32;
            }
            if !options.no_zoom {
                root = root.child(
                    title_box("zoom", right, WindowPart::Zoom, options)
                        .on_click(cx.listener(|this, _, _, cx| this.zoom(cx))),
                );
            }
        }
        if !state.collapsed {
            let side = options.frame_width() as f32;
            let background = !options.active && !options.interactive;
            let mut content = div()
                .absolute()
                .left(px(side))
                .top(px(options.title_bar_height() as f32))
                .right(px(side))
                .bottom(px(side))
                .overflow_hidden()
                .when_some(self.content.clone(), |el, content| {
                    el.child(crate::inactive::Inactive::new(
                        background,
                        crate::button_scope("window-buttons", content),
                    ))
                });
            if background && !options.dragging && !cx.has_active_drag() {
                content = content.child(div().absolute().inset_0().occlude().on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_, _, _, cx| {
                        cx.emit(WindowEvent::Activated);
                        cx.stop_propagation();
                    }),
                ));
            }
            root = root.child(content);
            if !options.no_grow {
                root = root.child(
                    div()
                        .id("resize")
                        .absolute()
                        .right(px(1.))
                        .bottom(px(1.))
                        .size(px(GROW_BOX_SIZE as f32))
                        .cursor(CursorStyle::ResizeUpLeftDownRight)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                if !this.options.active {
                                    cx.emit(WindowEvent::Activated);
                                }
                                this.drag = Some(Drag::Resize {
                                    mouse: event.position,
                                    state: this.state,
                                });
                                cx.stop_propagation();
                            }),
                        ),
                );
            }
        }
        root.child(self.path_menu.clone())
    }
}

fn title_box(
    id: &'static str,
    left: f32,
    part: WindowPart,
    options: PlatinumWindowOptions,
) -> Stateful<Div> {
    let (box_size, top, _) = options.title_box_metrics();
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(match part {
            WindowPart::Close => "Close",
            WindowPart::Zoom => "Zoom",
            WindowPart::Collapse => "Collapse",
            _ => "",
        })
        .absolute()
        .left(px(left))
        .top(px(top as f32))
        .w(px(box_size as f32))
        .h(px(box_size as f32))
        .text_color(rgb(BLACK))
        .active(|style| style.text_color(rgb(WHITE)))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(crate::paint::drawing(
            box_size as f32,
            box_size as f32,
            move |pen, window, _| {
                draw_title_box(pen, part, window.text_style().color == rgb(WHITE).into());
            },
        ))
}

/// The part of a frame under a position relative to its upper-left corner.
pub fn window_part_at(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    options: PlatinumWindowOptions,
    collapsed: bool,
) -> WindowPart {
    if x < 0. || y < 0. || x >= width || y >= height {
        return WindowPart::None;
    }
    let options = options.normalized();
    let (box_size, top, inset) = options.title_box_metrics();
    let in_box = |left: f32| {
        x >= left && x < left + box_size as f32 && y >= top as f32 && y < (top + box_size) as f32
    };
    if y < options.title_bar_height() as f32 {
        if !options.no_close && in_box(inset as f32) {
            return WindowPart::Close;
        }
        let mut right = width - (inset + box_size) as f32;
        if !options.no_collapse {
            if in_box(right) {
                return WindowPart::Collapse;
            }
            right -= (box_size + TITLE_BOX_GAP) as f32;
        }
        if !options.no_zoom && in_box(right) {
            return WindowPart::Zoom;
        }
        return WindowPart::Title;
    }
    if !collapsed
        && !options.no_grow
        && x >= width - GROW_BOX_SIZE as f32 - 1.
        && y >= height - GROW_BOX_SIZE as f32 - 1.
    {
        return WindowPart::Grow;
    }
    WindowPart::Content
}

/// Draws the normal, inactive, utility, modal or rolled-up window frame.
pub fn draw_window_frame(
    pen: &mut Pen,
    title: &str,
    options: PlatinumWindowOptions,
    collapsed: bool,
    pressed: WindowPart,
) {
    let options = options.normalized();
    let (width, height) = (pen.width, pen.height);
    let side = options.frame_width();
    let bar = options.title_bar_height();
    let (box_size, box_top, inset) = options.title_box_metrics();
    pen.rect(0, 0, width, height, GRAY_2);
    pen.frame(0, 0, width, height, BLACK);
    let (light, dark) = if options.active {
        (WHITE, GRAY_7)
    } else {
        (GRAY_1, GRAY_5)
    };
    pen.bevel(1, 1, width - 2, height - 2, light, dark);
    if !collapsed {
        if options.utility {
            pen.frame(
                side - 1,
                bar - 1,
                width - side * 2 + 2,
                height - bar - side + 2,
                BLACK,
            );
        } else {
            let (light, dark) = if options.active {
                (GRAY_7, WHITE)
            } else {
                (GRAY_5, GRAY_1)
            };
            pen.bevel(
                side - 2,
                bar - 2,
                width - side * 2 + 4,
                height - bar - side + 4,
                light,
                dark,
            );
            pen.frame(
                side - 1,
                bar - 1,
                width - side * 2 + 2,
                height - bar - side + 2,
                BLACK,
            );
        }
        if options.modal {
            pen.bevel(3, bar - 1, width - 6, height - bar - 2, GRAY_5, WHITE);
        }
    }
    let font = if options.utility {
        Font::Small
    } else {
        Font::Bold
    };
    let title_width = font.measure_in(title, pen.language);
    let title_x = (width - title_width) / 2;
    if options.active {
        let mut left = if options.no_close {
            4
        } else {
            inset + box_size + 4
        };
        let mut right = width - inset;
        if !options.no_collapse {
            right -= box_size + 4;
        }
        if !options.no_zoom {
            right -= box_size + TITLE_BOX_GAP;
        }
        if options.no_collapse && options.no_zoom {
            right = width - 4;
        }
        let (text_left, text_right) = if title.is_empty() {
            (left, left)
        } else {
            (title_x - 6, title_x + title_width + 6)
        };
        let (stripes, top) = if options.utility { (4, 2) } else { (6, 4) };
        for index in 0..stripes {
            let y = top + 2 * index;
            for (start, end) in [(left, text_left), (text_right, right)] {
                if end > start {
                    pen.hline(start, y, end - start, GRAY_7);
                    pen.hline(start + 1, y + 1, end - start, WHITE);
                }
            }
        }
        if !options.no_close {
            let mut button = Pen::new(box_size, box_size);
            draw_title_box(&mut button, WindowPart::Close, pressed == WindowPart::Close);
            pen.append(button, inset, box_top);
        }
        left = width - inset - box_size;
        if !options.no_collapse {
            let mut button = Pen::new(box_size, box_size);
            draw_title_box(
                &mut button,
                WindowPart::Collapse,
                pressed == WindowPart::Collapse,
            );
            pen.append(button, left, box_top);
            left -= box_size + TITLE_BOX_GAP;
        }
        if !options.no_zoom {
            let mut button = Pen::new(box_size, box_size);
            draw_title_box(&mut button, WindowPart::Zoom, pressed == WindowPart::Zoom);
            pen.append(button, left, box_top);
        }
    }
    let baseline = if options.utility {
        (bar - font.cap_height()) / 2 + font.cap_height()
    } else {
        1 + (bar - 2 - font.cap_height()) / 2 + font.cap_height()
    };
    let mut title_pen = Pen::new(width, bar);
    title_pen.language = pen.language;
    title_pen.text(
        title_x,
        baseline,
        title,
        font,
        if options.active { BLACK } else { GRAY_8 },
    );
    title_pen.clip(Rect {
        x: 0,
        y: 0,
        width,
        height: bar,
    });
    pen.append(title_pen, 0, 0);
    if !collapsed && !options.no_grow {
        let mut grow = Pen::new(GROW_BOX_SIZE, GROW_BOX_SIZE);
        draw_grow_box(&mut grow, options.active);
        pen.append(
            grow,
            width - side - GROW_BOX_SIZE + 1,
            height - side - GROW_BOX_SIZE + 1,
        );
    }
}

pub fn draw_title_box(pen: &mut Pen, part: WindowPart, pressed: bool) {
    let side = pen.width;
    pen.bevel(0, 0, side, side, GRAY_7, WHITE);
    pen.frame(1, 1, side - 2, side - 2, BLACK);
    let (face, light, dark) = if pressed {
        (GRAY_6, GRAY_9, GRAY_4)
    } else {
        (GRAY_2, WHITE, GRAY_7)
    };
    pen.rect(2, 2, side - 4, side - 4, face);
    pen.bevel(2, 2, side - 4, side - 4, light, dark);
    match part {
        WindowPart::Zoom => pen.frame(2, 2, 5, 5, BLACK),
        WindowPart::Collapse => {
            pen.hline(2, 4, side - 4, BLACK);
            pen.hline(2, 6, side - 4, BLACK);
        }
        _ => {}
    }
}

pub fn draw_grow_box(pen: &mut Pen, active: bool) {
    let side = GROW_BOX_SIZE;
    pen.rect(0, 0, side, side, GRAY_2);
    pen.hline(0, 0, side, BLACK);
    pen.vline(0, 0, side, BLACK);
    if active {
        for index in 0..3 {
            let offset = 4 + 3 * index;
            for pixel in 0..offset {
                pen.dot(side - offset + pixel, side - 2 - pixel, GRAY_7);
                pen.dot(side - offset + pixel + 1, side - 2 - pixel, WHITE);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    #[test]
    fn hit_testing_respects_hidden_parts_and_utility_metrics() {
        let normal = PlatinumWindowOptions::default();
        assert_eq!(
            window_part_at(13., 9., 180., 120., normal, false),
            WindowPart::Close
        );
        assert_eq!(
            window_part_at(151., 9., 180., 120., normal, false),
            WindowPart::Zoom
        );
        assert_eq!(
            window_part_at(166., 9., 180., 120., normal, false),
            WindowPart::Collapse
        );
        assert_eq!(
            window_part_at(176., 116., 180., 120., normal, false),
            WindowPart::Grow
        );
        assert_eq!(
            window_part_at(20., 19., 180., 120., normal, false),
            WindowPart::Content
        );
        let utility = PlatinumWindowOptions {
            utility: true,
            ..normal
        };
        assert_eq!(
            window_part_at(8., 6., 90., 60., utility, false),
            WindowPart::Close
        );
        assert_eq!(
            window_part_at(80., 56., 90., 60., utility, false),
            WindowPart::Content
        );
        let modal = PlatinumWindowOptions {
            modal: true,
            ..normal
        };
        assert_eq!(
            window_part_at(13., 9., 180., 120., modal, false),
            WindowPart::Title
        );
        assert_eq!(
            window_part_at(176., 116., 180., 120., modal, false),
            WindowPart::Content
        );
        assert_eq!(
            window_part_at(-1., 5., 180., 120., normal, false),
            WindowPart::None
        );
    }
}

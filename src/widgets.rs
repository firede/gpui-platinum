//! Small controls, built from GPUI elements and the shared pixel pen.

use crate::{
    Font, Language, Theme,
    paint::{Pen, Rect, drawing},
    theme::*,
};
use gpui::{prelude::*, *};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckState {
    Off,
    On,
    Mixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind {
    Button,
    Bevel,
    Checkbox(CheckState),
    Radio(bool),
    Disclosure(bool),
    Tab(bool),
    MenuItem(bool),
    Swatch(u32),
}

pub(crate) type ClickHandler = std::rc::Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(Clone)]
pub struct Control {
    id: ElementId,
    pub(crate) label: SharedString,
    pub(crate) kind: ControlKind,
    width: Option<i32>,
    height: Option<i32>,
    icon: Option<&'static [&'static str]>,
    icon_only: bool,
    pub(crate) disabled: bool,
    default: bool,
    pub(crate) selected: bool,
    toggle: bool,
    theme: Theme,
    language: Language,
    focus: Option<FocusHandle>,
    tab_stop: bool,
    pub(crate) click: Option<ClickHandler>,
}

impl Control {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        kind: ControlKind,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind,
            width: None,
            height: None,
            icon: None,
            icon_only: false,
            disabled: false,
            default: false,
            selected: false,
            toggle: false,
            theme: Theme::default(),
            language: Language::English,
            focus: None,
            tab_stop: true,
            click: None,
        }
    }

    pub fn width(mut self, width: i32) -> Self {
        self.width = Some(width);
        self
    }
    pub fn focus_handle(mut self, focus: &FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }
    pub fn height(mut self, height: i32) -> Self {
        self.height = Some(height);
        self
    }
    pub fn icon(mut self, rows: &'static [&'static str]) -> Self {
        self.icon = Some(rows);
        self
    }
    pub fn icon_only(mut self, rows: &'static [&'static str]) -> Self {
        self.icon = Some(rows);
        self.icon_only = true;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn default_button(mut self) -> Self {
        self.default = true;
        self
    }
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self.toggle = true;
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
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.click = Some(std::rc::Rc::new(handler));
        self
    }

    pub(crate) fn dimensions(&self) -> (i32, i32) {
        let label_width = if self.icon_only {
            0
        } else {
            Font::Bold.measure_in(&self.label, self.language)
        };
        let (icon_width, icon_height) = self.icon.map_or((0, 0), |rows| {
            (
                rows.iter().map(|row| row.len()).max().unwrap_or(0) as i32,
                rows.len() as i32,
            )
        });
        let (width, height) = match self.kind {
            ControlKind::Button => ((label_width + 24).max(58), 20),
            ControlKind::Bevel if self.icon_only => {
                let side = 22.max(icon_width.max(icon_height) + 6);
                (side, side)
            }
            ControlKind::Bevel => (
                label_width
                    + icon_width
                    + if icon_width > 0 && label_width > 0 {
                        4
                    } else {
                        0
                    }
                    + 16,
                22.max(icon_height + 10),
            ),
            ControlKind::Checkbox(_) | ControlKind::Radio(_) | ControlKind::Disclosure(_) => (
                if label_width > 0 {
                    label_width + 18
                } else {
                    12
                },
                18,
            ),
            ControlKind::Swatch(_) => (42, 24),
            _ => (label_width + 20, 24),
        };
        (self.width.unwrap_or(width), self.height.unwrap_or(height))
    }

    fn role(&self) -> Role {
        match self.kind {
            ControlKind::Checkbox(_) => Role::CheckBox,
            ControlKind::Radio(_) => Role::RadioButton,
            ControlKind::Tab(_) => Role::Tab,
            ControlKind::MenuItem(_) => Role::MenuItem,
            _ => Role::Button,
        }
    }

    fn toggled(&self) -> Option<Toggled> {
        match self.kind {
            ControlKind::Checkbox(CheckState::Mixed) => Some(Toggled::Mixed),
            ControlKind::Checkbox(state) => Some(if state == CheckState::On {
                Toggled::True
            } else {
                Toggled::False
            }),
            ControlKind::Radio(on) => Some(if on { Toggled::True } else { Toggled::False }),
            ControlKind::Bevel if self.toggle => Some(if self.selected {
                Toggled::True
            } else {
                Toggled::False
            }),
            _ => None,
        }
    }
}

impl IntoElement for Control {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for Control {
    type RequestLayoutState = ();
    type PrepaintState = AnyElement;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn a11y_role(&self) -> Option<Role> {
        self.disabled.then(|| self.role())
    }
    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        node.set_label(self.label.to_string());
        node.set_disabled();
        if let Some(toggled) = self.toggled() {
            node.set_toggled(toggled);
        }
        if let ControlKind::Tab(on) = self.kind {
            node.set_selected(on);
        }
        if let ControlKind::Disclosure(open) = self.kind {
            node.set_expanded(open);
        }
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let (width, height) = self.dimensions();
        let extra = if self.default { 8 } else { 0 };
        let mut style = Style::default();
        style.size.width = px((width + extra) as f32).into();
        style.size.height = px((height + extra) as f32).into();
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
    ) -> AnyElement {
        let focus = window.with_element_state(
            id.expect("control id"),
            |previous: Option<FocusHandle>, _| {
                let focus = self
                    .focus
                    .clone()
                    .or(previous)
                    .unwrap_or_else(|| cx.focus_handle());
                let focus = focus.tab_stop(!self.disabled && self.tab_stop);
                (focus.clone(), focus)
            },
        );
        let (width, height) = self.dimensions();
        let extra = if self.default { 8 } else { 0 };
        let mut element = div()
            .id("input")
            .w(bounds.size.width)
            .h(bounds.size.height)
            .flex_shrink_0()
            .text_color(rgb(BLACK));
        if !self.disabled {
            element = element
                .role(self.role())
                .aria_label(self.label.clone())
                .track_focus(&focus)
                .active(|style| style.text_color(rgb(WHITE)));
            if let Some(toggled) = self.toggled() {
                element = element.aria_toggled(toggled);
            }
            if let ControlKind::Tab(on) = self.kind {
                element = element.aria_selected(on);
            }
            if let ControlKind::Disclosure(open) = self.kind {
                element = element.aria_expanded(open);
            }
            if let Some(click) = self.click.clone() {
                element = element.on_click({
                    let click = click.clone();
                    move |event, window, cx| click(event, window, cx)
                });
                if let ControlKind::Disclosure(open) = self.kind {
                    element = element.on_key_down(move |event, window, cx| {
                        if event.keystroke.modifiers.modified() {
                            return;
                        }
                        if (event.keystroke.key == "right" && !open)
                            || (event.keystroke.key == "left" && open)
                        {
                            click(
                                &ClickEvent::Keyboard(KeyboardClickEvent {
                                    button: KeyboardButton::Enter,
                                    bounds,
                                }),
                                window,
                                cx,
                            );
                            cx.stop_propagation();
                        }
                    });
                }
            }
        }
        let label = self.label.clone();
        let kind = self.kind;
        let disabled = self.disabled;
        let default = self.default;
        let selected = self.selected;
        let theme = self.theme;
        let language = self.language;
        let icon = self.icon;
        let icon_only = self.icon_only;
        let mut element = element
            .child(drawing(
                (width + extra) as f32,
                (height + extra) as f32,
                move |pen, window, _| {
                    let pressed = !disabled
                        && (window.text_style().color == rgb(WHITE).into()
                            || (selected && kind != ControlKind::Bevel));
                    let mut inner = Pen::new(width, height);
                    inner.language = language;
                    if kind == ControlKind::Bevel {
                        draw_bevel_content(
                            &mut inner,
                            if icon_only { "" } else { &label },
                            icon,
                            disabled,
                            pressed,
                            selected,
                            theme,
                        );
                    } else {
                        draw_control(&mut inner, &label, kind, disabled, pressed, theme);
                    }
                    if !disabled && focus.is_focused(window) && window.last_input_was_keyboard() {
                        draw_control_focus(&mut inner, kind, theme.accent.dark);
                    }
                    if default {
                        draw_default_ring(pen, disabled);
                        pen.append(inner, 4, 4);
                    } else {
                        pen.append(inner, 0, 0);
                    }
                },
            ))
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
        element
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        element: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx);
    }
}

fn draw_control_focus(pen: &mut Pen, kind: ControlKind, color: u32) {
    match kind {
        ControlKind::Checkbox(_) | ControlKind::Radio(_) | ControlKind::Disclosure(_) => {
            let mut mark = Pen::new(12, 12);
            if matches!(kind, ControlKind::Radio(_)) {
                draw_focus_circle(&mut mark, color);
            } else {
                draw_focus_ring(&mut mark, 0, color);
            }
            pen.append(mark, 0, (pen.height - 12) / 2);
        }
        ControlKind::Button => draw_focus_ring(pen, 3, color),
        _ => draw_focus_ring(pen, 0, color),
    }
}

pub fn draw_bevel_content(
    pen: &mut Pen,
    label: &str,
    icon: Option<&[&str]>,
    disabled: bool,
    pressed: bool,
    on: bool,
    theme: Theme,
) {
    draw_bevel_button(pen, disabled, pressed, on);
    let label_width = Font::Bold.measure_in(label, pen.language);
    let icon_width = icon.map_or(0, |rows| {
        rows.iter().map(|row| row.len()).max().unwrap_or(0) as i32
    });
    let content_width = label_width
        + icon_width
        + if label_width > 0 && icon_width > 0 {
            4
        } else {
            0
        };
    let offset = i32::from(pressed || on);
    let mut x = (pen.width - content_width) / 2 + offset;
    if let Some(rows) = icon {
        let ink_top = rows
            .iter()
            .position(|row| row.bytes().any(|byte| byte != b'.' && byte != b' '))
            .unwrap_or(0);
        let ink_bottom = rows
            .iter()
            .rposition(|row| row.bytes().any(|byte| byte != b'.' && byte != b' '))
            .map_or(ink_top, |y| y + 1);
        let y = (pen.height - (ink_bottom - ink_top) as i32) / 2 - ink_top as i32 + offset;
        crate::bitmap_icons::draw_bitmap_icon(pen, rows, x, y, theme.accent, false, disabled);
        x += icon_width + 4;
    }
    pen.text(
        x,
        (pen.height - 9) / 2 + 9 + offset,
        label,
        Font::Bold,
        if disabled { GRAY_8 } else { BLACK },
    );
}

pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Control {
    Control::new(id, label, ControlKind::Button)
}
pub fn bevel_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Control {
    Control::new(id, label, ControlKind::Bevel)
}
pub fn checkbox(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    state: CheckState,
) -> Control {
    Control::new(id, label, ControlKind::Checkbox(state))
}
pub fn radio(id: impl Into<ElementId>, label: impl Into<SharedString>, selected: bool) -> Control {
    Control::new(id, label, ControlKind::Radio(selected))
}
pub fn disclosure(id: impl Into<ElementId>, label: impl Into<SharedString>, open: bool) -> Control {
    Control::new(id, label, ControlKind::Disclosure(open))
}
pub fn tab(id: impl Into<ElementId>, label: impl Into<SharedString>, selected: bool) -> Control {
    Control::new(id, label, ControlKind::Tab(selected))
}

#[track_caller]
pub fn label(text: impl Into<SharedString>) -> impl IntoElement {
    text_in(text, Font::Bold, BLACK)
}

#[track_caller]
pub fn text_in(text: impl Into<SharedString>, font: Font, color: u32) -> impl IntoElement {
    text_in_language(text, font, color, Language::English)
}

#[track_caller]
pub fn label_in(text: impl Into<SharedString>, language: Language) -> impl IntoElement {
    text_in_language(text, Font::Bold, BLACK, language)
}

#[track_caller]
pub fn text_in_language(
    text: impl Into<SharedString>,
    font: Font,
    color: u32,
    language: Language,
) -> impl IntoElement {
    let text = text.into();
    let width = font.measure_in(&text, language).max(1);
    div()
        .id(ElementId::NamedChild(
            std::sync::Arc::new(ElementId::from(std::panic::Location::caller())),
            text.clone(),
        ))
        .role(Role::Label)
        .aria_label(text.clone())
        .flex_shrink_0()
        .child(drawing(width as f32, 18., move |pen, _, _| {
            pen.language = language;
            pen.text(0, 13, &text, font, color)
        }))
}

pub fn separator(width: i32) -> impl IntoElement {
    drawing(width as f32, 2., |pen, _, _| {
        pen.hline(0, 0, pen.width, GRAY_7);
        pen.hline(0, 1, pen.width, WHITE);
    })
}

#[track_caller]
pub fn group_box(title: impl Into<SharedString>, width: i32) -> Div {
    group_box_in(title, width, Language::English)
}

#[track_caller]
pub fn group_box_in(title: impl Into<SharedString>, width: i32, language: Language) -> Div {
    let title = title.into();
    div()
        .relative()
        .w(px(width as f32))
        .flex()
        .flex_col()
        .gap(px(6.))
        .p(px(12.))
        .pt(px(18.))
        .border_1()
        .border_color(rgb(GRAY_7))
        .child(
            div()
                .absolute()
                .left(px(8.))
                .top(px(-9.))
                .px(px(4.))
                .bg(rgb(GRAY_2))
                .child(label_in(title, language)),
        )
}

pub fn progress_bar(value: f64, width: i32, theme: Theme) -> impl IntoElement {
    drawing(width as f32, 14., move |pen, _, _| {
        draw_progress(pen, value, theme.accent)
    })
}

pub fn draw_progress(pen: &mut Pen, value: f64, accent: Accent) {
    progress_channel(pen);
    let width = ((pen.width - 2) as f64 * value.clamp(0., 1.)).round() as i32;
    if width <= 0 {
        return;
    }
    let height = pen.height - 2;
    pen.rect(1, 1, width, height, accent.mid);
    pen.hline(1, 1, width, accent.light);
    pen.hline(1, 2, width, mix(accent.light, accent.mid, 0.5));
    pen.hline(1, height - 1, width, mix(accent.mid, accent.dark, 0.5));
    pen.hline(1, height, width, accent.dark);
    if width < pen.width - 2 {
        pen.vline(1 + width, 1, height, accent.dark);
    }
}

pub fn draw_progress_indeterminate(pen: &mut Pen, phase: i32, accent: Accent) {
    progress_channel(pen);
    let (width, height) = (pen.width - 2, pen.height - 2);
    for y in 0..height {
        for x in 0..width {
            let color = if (x + y - phase).rem_euclid(16) < 8 {
                accent.mid
            } else {
                accent.light
            };
            let color = if y == 0 {
                mix(color, WHITE, 0.5)
            } else if y == height - 1 {
                blend(color, accent.dark, 0.6)
            } else {
                color
            };
            pen.dot(1 + x, 1 + y, color);
        }
    }
}

fn progress_channel(pen: &mut Pen) {
    let (w, h) = (pen.width, pen.height);
    pen.frame(0, 0, w, h, BLACK);
    pen.rect(1, 1, w - 2, h - 2, GRAY_4);
    pen.hline(1, 1, w - 2, GRAY_7);
    pen.vline(1, 2, h - 3, GRAY_7);
    pen.hline(2, h - 2, w - 3, GRAY_1);
    pen.vline(w - 2, 2, h - 4, GRAY_1);
}

fn blend(background: u32, foreground: u32, alpha: f64) -> u32 {
    let mut color = 0;
    for shift in [16, 8, 0] {
        let a = ((background >> shift) & 255) as f64;
        let b = ((foreground >> shift) & 255) as f64;
        color |= (((a * (1. - alpha)).round() + (b * alpha).round()) as u32) << shift;
    }
    color
}

fn mix(from: u32, to: u32, amount: f64) -> u32 {
    let mut color = 0;
    for shift in [16, 8, 0] {
        let a = ((from >> shift) & 255) as f64;
        let b = ((to >> shift) & 255) as f64;
        color |= ((a + (b - a) * amount).round() as u32) << shift;
    }
    color
}

pub fn draw_control(
    pen: &mut Pen,
    label: &str,
    kind: ControlKind,
    disabled: bool,
    pressed: bool,
    theme: Theme,
) {
    let (w, h) = (pen.width, pen.height);
    let text = if disabled {
        GRAY_8
    } else if pressed {
        WHITE
    } else {
        BLACK
    };
    match kind {
        ControlKind::Button => draw_button(pen, label, disabled, pressed),
        ControlKind::Bevel => {
            draw_bevel_button(pen, disabled, pressed, false);
            let offset = i32::from(pressed);
            pen.text(
                (w - Font::Bold.measure_in(label, pen.language)) / 2 + offset,
                (h - 9) / 2 + 9 + offset,
                label,
                Font::Bold,
                if disabled { GRAY_8 } else { BLACK },
            );
        }
        ControlKind::Tab(_) => {
            let selected = matches!(kind, ControlKind::Tab(true));
            let face = if pressed {
                GRAY_9
            } else if selected {
                GRAY_1
            } else {
                GRAY_2
            };
            pen.raised(face);
            if pressed {
                pen.bevel(1, 1, w - 2, h - 2, GRAY_11, GRAY_7);
            }
            if selected {
                pen.hline(1, 1, w - 2, theme.accent.dark);
            }
            pen.text_centered(label, Font::Bold, text);
        }
        ControlKind::Checkbox(state) => {
            let mut mark = Pen::new(12, 12);
            draw_checkbox(&mut mark, state, disabled, pressed);
            pen.append(mark, 0, 3);
            pen.text(
                18,
                13,
                label,
                Font::Bold,
                if disabled { GRAY_8 } else { BLACK },
            );
        }
        ControlKind::Radio(on) => {
            let mut mark = Pen::new(12, 12);
            draw_radio(&mut mark, on, disabled, pressed);
            pen.append(mark, 0, 3);
            pen.text(
                18,
                13,
                label,
                Font::Bold,
                if disabled { GRAY_8 } else { BLACK },
            );
        }
        ControlKind::Disclosure(open) => {
            let mut mark = Pen::new(12, 12);
            draw_disclosure(&mut mark, open, disabled, pressed);
            pen.append(mark, 0, (h - 12) / 2);
            pen.text(
                18,
                (h - 9) / 2 + 9,
                label,
                Font::Bold,
                if disabled { GRAY_8 } else { BLACK },
            );
        }
        ControlKind::MenuItem(selected) => {
            pen.rect(
                0,
                0,
                w,
                h,
                if selected || pressed {
                    theme.accent.dark
                } else {
                    GRAY_1
                },
            );
            pen.text(
                8,
                (h - 9) / 2 + 9,
                label,
                Font::Bold,
                if disabled {
                    GRAY_8
                } else if selected || pressed {
                    WHITE
                } else {
                    BLACK
                },
            );
        }
        ControlKind::Swatch(color) => {
            pen.sunken();
            pen.rect(3, 3, w - 6, h - 6, color);
        }
    }
}

pub fn draw_disclosure(pen: &mut Pen, open: bool, disabled: bool, pressed: bool) {
    let (line, fill, highlight) = if disabled {
        (GRAY_6, GRAY_3, GRAY_3)
    } else if pressed {
        (BLACK, BLACK, BLACK)
    } else {
        (BLACK, GRAY_6, GRAY_4)
    };
    let (rows, left, top): (&[&str], i32, i32) = if open {
        (
            &[
                "ooooooooooo",
                ".o-------o.",
                "..o-----o..",
                "...o---o...",
                "....o-o....",
                ".....o.....",
            ],
            0,
            3,
        )
    } else {
        (
            &[
                "o......", "oo.....", "o-o....", "o--o...", "o---o..", "o----o.", "o---o..",
                "o--o...", "o-o....", "oo.....", "o......",
            ],
            3,
            0,
        )
    };
    for (y, row) in rows.iter().enumerate() {
        for (x, byte) in row.bytes().enumerate() {
            let color = match byte {
                b'o' => line,
                b'-' if (!open && y < rows.len() / 2 && row.as_bytes()[x + 1] == b'o')
                    || (open && y == 1) =>
                {
                    highlight
                }
                b'-' => fill,
                _ => continue,
            };
            pen.dot(left + x as i32, top + y as i32, color);
        }
    }
}

pub fn draw_bevel_button(pen: &mut Pen, disabled: bool, pressed: bool, on: bool) {
    let (w, h) = (pen.width, pen.height);
    let (mut frame, mut face, mut top, mut bottom, mut inner_top, mut inner_bottom) =
        (GRAY_10, GRAY_2, WHITE, GRAY_7, GRAY_1, GRAY_5);
    if pressed {
        (face, top, bottom, inner_top, inner_bottom) = (GRAY_6, GRAY_9, GRAY_4, GRAY_8, GRAY_5);
    } else if on {
        (face, top, bottom, inner_top, inner_bottom) = (GRAY_4, GRAY_7, GRAY_1, GRAY_6, GRAY_3);
    }
    if disabled {
        (frame, top, bottom, inner_top, inner_bottom) = (GRAY_6, GRAY_1, GRAY_4, GRAY_2, GRAY_2);
        face = if on { GRAY_3 } else { GRAY_2 };
    }
    pen.rect(1, 1, w - 2, h - 2, face);
    pen.frame(0, 0, w, h, frame);
    pen.bevel(1, 1, w - 2, h - 2, top, bottom);
    pen.bevel(2, 2, w - 4, h - 4, inner_top, inner_bottom);
}

pub fn draw_button(pen: &mut Pen, label: &str, disabled: bool, pressed: bool) {
    let (w, h) = (pen.width, pen.height);
    let (line, face, top, bottom, inner_top, inner_bottom, text) = if disabled {
        (GRAY_6, GRAY_2, GRAY_1, GRAY_4, GRAY_2, GRAY_2, GRAY_8)
    } else if pressed {
        (BLACK, GRAY_9, GRAY_11, GRAY_7, GRAY_10, GRAY_8, WHITE)
    } else {
        (BLACK, GRAY_2, WHITE, GRAY_7, GRAY_1, GRAY_5, BLACK)
    };
    pen.round_frame(
        Rect {
            x: 0,
            y: 0,
            width: w,
            height: h,
        },
        3,
        line,
        Some(face),
    );
    pen.hline(3, 1, w - 6, top);
    pen.vline(1, 3, h - 6, top);
    pen.dot(2, 2, top);
    pen.hline(3, h - 2, w - 6, bottom);
    pen.vline(w - 2, 3, h - 6, bottom);
    pen.dot(w - 3, h - 3, bottom);
    pen.hline(3, 2, w - 7, inner_top);
    pen.vline(2, 3, h - 7, inner_top);
    pen.hline(4, h - 3, w - 7, inner_bottom);
    pen.vline(w - 3, 4, h - 7, inner_bottom);
    pen.text_centered(label, Font::Bold, text);
}

pub fn draw_default_ring(pen: &mut Pen, disabled: bool) {
    let (w, h) = (pen.width, pen.height);
    let face = if disabled { GRAY_2 } else { GRAY_4 };
    let top = if disabled { GRAY_1 } else { WHITE };
    let bottom = if disabled { GRAY_4 } else { GRAY_7 };
    pen.round_frame(
        Rect {
            x: 0,
            y: 0,
            width: w,
            height: h,
        },
        4,
        if disabled { GRAY_6 } else { BLACK },
        Some(face),
    );
    pen.hline(4, 1, w - 8, top);
    pen.vline(1, 4, h - 8, top);
    pen.dot(2, 2, top);
    pen.dot(3, 2, top);
    pen.dot(2, 3, top);
    pen.hline(4, h - 2, w - 8, bottom);
    pen.vline(w - 2, 4, h - 8, bottom);
    pen.dot(w - 3, h - 3, bottom);
    pen.dot(w - 4, h - 3, bottom);
    pen.dot(w - 3, h - 4, bottom);
    pen.rect(3, 3, w - 6, h - 6, GRAY_5);
}

pub fn draw_focus_ring(pen: &mut Pen, radius: usize, color: u32) {
    pen.round_frame(
        Rect {
            x: -3,
            y: -3,
            width: pen.width + 6,
            height: pen.height + 6,
        },
        (radius + 1).min(4),
        color,
        None,
    );
    pen.round_frame(
        Rect {
            x: -2,
            y: -2,
            width: pen.width + 4,
            height: pen.height + 4,
        },
        radius,
        color,
        None,
    );
}

/// Draw a twelve-pixel check box at the pen's origin.
pub fn draw_checkbox(pen: &mut Pen, state: CheckState, disabled: bool, pressed: bool) {
    let (frame, face, light, dark, ink) = mark_colors(disabled, pressed);
    pen.frame(0, 0, 12, 12, frame);
    pen.rect(1, 1, 10, 10, face);
    pen.bevel(1, 1, 10, 10, light, dark);
    match state {
        CheckState::Off => {}
        CheckState::Mixed => pen.rect(3, 5, 6, 2, ink),
        CheckState::On => {
            for (y, row) in [
                "#......#", "##....##", ".##..##.", "..####..", "..####..", ".##..##.", "##....##",
                "#......#",
            ]
            .iter()
            .enumerate()
            {
                for (x, byte) in row.bytes().enumerate() {
                    if byte == b'#' {
                        pen.dot(2 + x as i32, 2 + y as i32, ink);
                    }
                }
            }
        }
    }
}

pub fn draw_radio(pen: &mut Pen, on: bool, disabled: bool, pressed: bool) {
    const MASK: [&[u8; 12]; 12] = [
        b"....oooo....",
        b"..oo----oo..",
        b".o--------o.",
        b".o--------o.",
        b"o----------o",
        b"o----------o",
        b"o----------o",
        b"o----------o",
        b".o--------o.",
        b".o--------o.",
        b"..oo----oo..",
        b"....oooo....",
    ];
    let (frame, face, light, dark, ink) = mark_colors(disabled, pressed);
    let inside = |x: i32, y: i32| {
        (0..12).contains(&x) && (0..12).contains(&y) && MASK[y as usize][x as usize] == b'-'
    };
    for y in 0..12 {
        for x in 0..12 {
            match MASK[y as usize][x as usize] {
                b'o' => pen.dot(x, y, frame),
                b'-' => {
                    let mut color = face;
                    if !inside(x - 1, y) || !inside(x, y - 1) || !inside(x - 1, y - 1) {
                        if x + y < 11 {
                            color = light;
                        } else if x + y > 11 {
                            color = dark;
                        }
                    }
                    if !inside(x + 1, y) || !inside(x, y + 1) || !inside(x + 1, y + 1) {
                        if x + y > 11 {
                            color = dark;
                        } else if x + y < 11 && color == face {
                            color = light;
                        }
                    }
                    pen.dot(x, y, color);
                }
                _ => {}
            }
        }
    }
    if on {
        for (y, row) in [".##.", "####", "####", ".##."].iter().enumerate() {
            for (x, byte) in row.bytes().enumerate() {
                if byte == b'#' {
                    pen.dot(4 + x as i32, 4 + y as i32, ink);
                }
            }
        }
    }
}

fn mark_colors(disabled: bool, pressed: bool) -> (u32, u32, u32, u32, u32) {
    if disabled {
        (GRAY_6, GRAY_2, GRAY_2, GRAY_2, GRAY_6)
    } else if pressed {
        (BLACK, GRAY_4, GRAY_7, GRAY_1, BLACK)
    } else {
        (BLACK, GRAY_2, WHITE, GRAY_7, BLACK)
    }
}

pub fn draw_focus_circle(pen: &mut Pen, color: u32) {
    for y in -4..16 {
        for x in -4..16 {
            let dx = x as f64 + 0.5 - 6.;
            let dy = y as f64 + 0.5 - 6.;
            let distance = dx * dx + dy * dy;
            if (7.2 * 7.2..9.2 * 9.2).contains(&distance) {
                pen.dot(x, y, color);
            }
        }
    }
}

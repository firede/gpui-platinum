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

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Control {
    id: ElementId,
    label: SharedString,
    kind: ControlKind,
    width: i32,
    height: i32,
    disabled: bool,
    default: bool,
    selected: bool,
    theme: Theme,
    language: Language,
    focus: Option<FocusHandle>,
    click: Option<ClickHandler>,
}

impl Control {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        kind: ControlKind,
    ) -> Self {
        let label = label.into();
        let width = Font::Bold.measure(&label);
        let (width, height) = match kind {
            ControlKind::Button => ((width + 24).max(58), 20),
            ControlKind::Checkbox(_) | ControlKind::Radio(_) | ControlKind::Disclosure(_) => {
                (width + 18, 18)
            }
            ControlKind::Swatch(_) => (42, 24),
            _ => (width + 20, 24),
        };
        Self {
            id: id.into(),
            label,
            kind,
            width,
            height,
            disabled: false,
            default: false,
            selected: false,
            theme: Theme::default(),
            language: Language::English,
            focus: None,
            click: None,
        }
    }

    pub fn width(mut self, width: i32) -> Self {
        self.width = width;
        self
    }
    pub fn focus_handle(mut self, focus: &FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }
    pub fn height(mut self, height: i32) -> Self {
        self.height = height;
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
        self.click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Control {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            label,
            kind,
            width,
            height,
            disabled,
            default,
            selected,
            theme,
            language,
            focus,
            click,
        } = self;
        let role = match kind {
            ControlKind::Checkbox(_) => Role::CheckBox,
            ControlKind::Radio(_) => Role::RadioButton,
            ControlKind::Tab(_) => Role::Tab,
            ControlKind::MenuItem(_) => Role::MenuItem,
            _ => Role::Button,
        };
        let extra = if default { 8 } else { 0 };
        let mut element = div()
            .id(id)
            .role(role)
            .aria_label(label.clone())
            .flex_shrink_0()
            .border_2()
            .border_color(gpui::transparent_black())
            .p(px(1.))
            .text_color(rgb(BLACK))
            .when(!disabled, |e| {
                e.focusable()
                    .tab_stop(true)
                    .when_some(focus, |el, handle| el.track_focus(&handle))
                    .focus_visible(move |s| s.border_color(rgb(theme.accent.dark)))
                    .active(|s| s.text_color(rgb(WHITE)))
            });
        if let ControlKind::Checkbox(state) = kind {
            element = element.aria_toggled(match state {
                CheckState::Off => Toggled::False,
                CheckState::On => Toggled::True,
                CheckState::Mixed => Toggled::Mixed,
            });
        }
        if let ControlKind::Radio(on) | ControlKind::Tab(on) = kind {
            element = element.aria_selected(on);
        }
        if !disabled && let Some(click) = click {
            element = element.on_click(click);
        }
        element.child(drawing(
            (width + extra) as f32,
            (height + extra) as f32,
            move |pen, window, _| {
                pen.language = language;
                let pressed =
                    !disabled && (window.text_style().color == rgb(WHITE).into() || selected);
                if default {
                    draw_default_ring(pen, disabled);
                    let mut inner = Pen::new(width, height);
                    inner.language = language;
                    draw_control(&mut inner, &label, kind, disabled, pressed, theme);
                    for mut fill in inner.fills {
                        fill.rect.x += 4;
                        fill.rect.y += 4;
                        pen.fills.push(fill);
                    }
                } else {
                    draw_control(pen, &label, kind, disabled, pressed, theme);
                }
            },
        ))
    }
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

pub fn label(text: impl Into<SharedString>) -> impl IntoElement {
    text_in(text, Font::Bold, BLACK)
}

pub fn text_in(text: impl Into<SharedString>, font: Font, color: u32) -> impl IntoElement {
    let text = text.into();
    let width = font.measure(&text).max(1);
    div()
        .id(ElementId::Name(text.clone()))
        .role(Role::Label)
        .aria_label(text.clone())
        .flex_shrink_0()
        .child(drawing(width as f32, 18., move |pen, _, _| {
            pen.text(0, 13, &text, font, color)
        }))
}

pub fn separator(width: i32) -> impl IntoElement {
    drawing(width as f32, 2., |pen, _, _| {
        pen.hline(0, 0, pen.width, GRAY_7);
        pen.hline(0, 1, pen.width, WHITE);
    })
}

pub fn group_box(title: impl Into<SharedString>, width: i32) -> Div {
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
                .child(label(title)),
        )
}

pub fn progress_bar(value: f64, width: i32, theme: Theme) -> impl IntoElement {
    drawing(width as f32, 14., move |pen, _, _| {
        pen.sunken();
        let fill = ((pen.width - 4) as f64 * value.clamp(0., 1.)).round() as i32;
        pen.rect(2, 2, fill, pen.height - 4, theme.accent.mid);
        pen.hline(2, 2, fill, theme.accent.light);
        pen.hline(2, pen.height - 3, fill, theme.accent.dark);
        for x in (4..fill).step_by(6) {
            pen.vline(x, 3, pen.height - 7, theme.accent.light);
        }
    })
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
        ControlKind::Bevel | ControlKind::Tab(_) => {
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
            pen.rect(0, 0, w, h, GRAY_2);
            pen.frame(0, 3, 12, 12, if disabled { GRAY_6 } else { BLACK });
            pen.rect(1, 4, 10, 10, if pressed { GRAY_4 } else { GRAY_2 });
            if !disabled {
                pen.bevel(1, 4, 10, 10, WHITE, GRAY_7);
            }
            let ink = if disabled { GRAY_6 } else { BLACK };
            match state {
                CheckState::Mixed => pen.rect(3, 8, 6, 2, ink),
                CheckState::On => {
                    for i in 0..7 {
                        pen.rect(2 + i, 5 + i, 2, 1, ink);
                        pen.rect(8 - i, 5 + i, 2, 1, ink);
                    }
                }
                CheckState::Off => {}
            }
            pen.text(
                18,
                13,
                label,
                Font::Bold,
                if disabled { GRAY_8 } else { BLACK },
            );
        }
        ControlKind::Radio(on) => {
            pen.rect(0, 0, w, h, GRAY_2);
            pen.round_frame(
                Rect {
                    x: 0,
                    y: 3,
                    width: 12,
                    height: 12,
                },
                4,
                if disabled { GRAY_6 } else { BLACK },
                Some(if pressed { GRAY_4 } else { GRAY_2 }),
            );
            if on {
                pen.round_frame(
                    Rect {
                        x: 4,
                        y: 7,
                        width: 4,
                        height: 4,
                    },
                    1,
                    if disabled { GRAY_6 } else { BLACK },
                    Some(BLACK),
                );
            }
            pen.text(
                18,
                13,
                label,
                Font::Bold,
                if disabled { GRAY_8 } else { BLACK },
            );
        }
        ControlKind::Disclosure(open) => {
            pen.rect(0, 0, w, h, GRAY_2);
            let ink = if disabled { GRAY_6 } else { BLACK };
            for i in 0..5 {
                if open {
                    pen.hline(1 + i, 6 + i, 9 - 2 * i, ink);
                } else {
                    pen.vline(3 + i, 4 + i, 9 - 2 * i, ink);
                }
            }
            pen.text(
                18,
                13,
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
    pen.round_frame(
        Rect {
            x: 0,
            y: 0,
            width: w,
            height: h,
        },
        4,
        if disabled { GRAY_6 } else { BLACK },
        Some(GRAY_4),
    );
    pen.hline(4, 1, w - 8, WHITE);
    pen.vline(1, 4, h - 8, WHITE);
    pen.hline(4, h - 2, w - 8, GRAY_7);
    pen.vline(w - 2, 4, h - 8, GRAY_7);
    pen.rect(3, 3, w - 6, h - 6, GRAY_5);
}

//! The original's golden scenes of single controls, dialogs and frames.

use crate::Scene;
use gpui::{
    AnyElement, AnyView, AnyWindowHandle, App, AppContext as _, Bounds, Context, Entity, Focusable,
    HeadlessAppContext, IntoElement, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, PlatformInput, Render,
    Styled, Window, canvas, div, point, px, rgb,
};
use gpui_platinum::{
    CheckState, Slider, Theme,
    paint::{Pen, Rect, drawing},
    slider::{ScrollPart, SliderState, draw_slider_thumb},
    theme::*,
    *,
};

use std::{cell::RefCell, rc::Rc};

type Cell = Box<dyn Fn(&mut Pen)>;

/// Cells side by side, each `width`×`height` with a gap, as the original's
/// `board` draws a control's states.
fn board(width: i32, height: i32, cells: Vec<Cell>) -> AnyElement {
    let cells: Vec<std::rc::Rc<Cell>> = cells.into_iter().map(std::rc::Rc::new).collect();
    div()
        .flex()
        .p(px(6.))
        .gap(px(6.))
        .children(
            cells
                .into_iter()
                .map(move |cell| drawing(width as f32, height as f32, move |pen, _, _| cell(pen))),
        )
        .into_any_element()
}

/// Draws into a `width`×`height` pen at (`x`, `y`) of `pen`.
fn sub(pen: &mut Pen, x: i32, y: i32, width: i32, height: i32, draw: impl FnOnce(&mut Pen)) {
    let mut inner = Pen::new(width, height);
    inner.language = pen.language;
    draw(&mut inner);
    pen.append(inner, x, y);
}

type Layout<T> = Box<dyn Fn(&T, &mut Window, &mut Context<Held<T>>) -> AnyElement>;

/// A scene's retained controls, and how it lays them out each frame on the
/// window's Gray2.
pub struct Held<T: 'static> {
    state: T,
    layout: Layout<T>,
}

impl<T: 'static> Held<T> {
    fn new<E: IntoElement>(
        state: T,
        layout: impl Fn(&T, &mut Window, &mut Context<Self>) -> E + 'static,
    ) -> Self {
        Self {
            state,
            layout: Box::new(move |state, window, cx| layout(state, window, cx).into_any_element()),
        }
    }
}

impl<T: 'static> Render for Held<T> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(GRAY_2))
            .child((self.layout)(&self.state, window, cx))
    }
}

type Build = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Elements built anew each frame, without a background: a window's content.
struct Piece(Build);

impl Render for Piece {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        (self.0)(window, cx)
    }
}

fn piece<E: IntoElement>(
    cx: &mut App,
    build: impl Fn(&mut Window, &mut App) -> E + 'static,
) -> AnyView {
    cx.new(|_| {
        Piece(Box::new(move |window, cx| {
            build(window, cx).into_any_element()
        }))
    })
    .into()
}

/// Sends `event` to the scene's window, drawn first so that it hits what
/// it is over.
fn send(app: &mut HeadlessAppContext, window: AnyWindowHandle, event: PlatformInput) {
    app.update_window(window, |_, window, cx| {
        let _ = window.draw(cx);
        window.dispatch_event(event, cx);
    })
    .unwrap();
    app.run_until_parked();
}

fn press(
    app: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    x: f32,
    y: f32,
    modifiers: Modifiers,
) {
    send(
        app,
        window,
        PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Left,
            position: point(px(x), px(y)),
            modifiers,
            click_count: 1,
            first_mouse: false,
        }),
    );
}

fn release(
    app: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    x: f32,
    y: f32,
    modifiers: Modifiers,
) {
    send(
        app,
        window,
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position: point(px(x), px(y)),
            modifiers,
            click_count: 1,
        }),
    );
}

/// Presses a key, as "tab" or "shift-tab", in the scene's window.
fn key(app: &mut HeadlessAppContext, window: AnyWindowHandle, keystroke: &'static str) {
    let keystroke = Keystroke::parse(keystroke).unwrap();
    send(
        app,
        window,
        PlatformInput::KeyDown(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        }),
    );
    send(app, window, PlatformInput::KeyUp(KeyUpEvent { keystroke }));
}

fn hover(app: &mut HeadlessAppContext, window: AnyWindowHandle, x: f32, y: f32) {
    send(
        app,
        window,
        PlatformInput::MouseMove(MouseMoveEvent {
            position: point(px(x), px(y)),
            modifiers: Modifiers::default(),
            pressed_button: None,
        }),
    );
}

fn boards() -> Vec<Scene> {
    let lavender = ACCENTS[0];
    vec![
        Scene::elements("button-states", 400., 40., move |_, _| {
            board(
                70,
                28,
                vec![
                    Box::new(|pen| {
                        sub(pen, 4, 4, 62, 20, |p| {
                            draw_button(p, "Normal", false, false)
                        })
                    }),
                    Box::new(|pen| {
                        sub(pen, 4, 4, 62, 20, |p| {
                            draw_button(p, "Pressed", false, true)
                        })
                    }),
                    Box::new(|pen| {
                        sub(pen, 4, 4, 62, 20, |p| {
                            draw_button(p, "Disabled", true, false)
                        })
                    }),
                    Box::new(|pen| {
                        draw_default_ring(pen, false);
                        sub(pen, 4, 4, 62, 20, |p| {
                            draw_button(p, "Default", false, false)
                        });
                    }),
                    Box::new(move |pen| {
                        sub(pen, 4, 4, 62, 20, |p| {
                            draw_button(p, "Focus", false, false);
                            draw_focus_ring(p, 3, lavender.dark);
                        })
                    }),
                ],
            )
        }),
        Scene::elements("checkbox-states", 260., 30., move |_, _| {
            let mut cells: Vec<Cell> = Vec::new();
            for (disabled, pressed) in [(false, false), (false, true), (true, false)] {
                for mark in [CheckState::Off, CheckState::On, CheckState::Mixed] {
                    cells.push(Box::new(move |pen| {
                        sub(pen, 3, 3, 12, 12, |p| {
                            draw_checkbox(p, mark, disabled, pressed)
                        })
                    }));
                }
            }
            cells.push(Box::new(move |pen| {
                sub(pen, 3, 3, 12, 12, |p| {
                    draw_checkbox(p, CheckState::On, false, false);
                    draw_focus_ring(p, 0, lavender.dark);
                })
            }));
            board(18, 18, cells)
        }),
        Scene::elements("radio-states", 200., 30., move |_, _| {
            let mut cells: Vec<Cell> = Vec::new();
            for (disabled, pressed) in [(false, false), (false, true), (true, false)] {
                for on in [false, true] {
                    cells.push(Box::new(move |pen| {
                        sub(pen, 3, 3, 12, 12, |p| draw_radio(p, on, disabled, pressed))
                    }));
                }
            }
            cells.push(Box::new(move |pen| {
                sub(pen, 3, 3, 12, 12, |p| {
                    draw_radio(p, true, false, false);
                    draw_focus_circle(p, lavender.dark);
                })
            }));
            board(18, 18, cells)
        }),
        Scene::elements("slider-states", 520., 40., move |_, _| {
            let slider = |value: f64, ticks: u32, height: i32, disabled: bool| -> Cell {
                Box::new(move |pen| {
                    sub(pen, 0, 0, 120, height, |p| {
                        Slider::new("slider", value, 0., 1.)
                            .width(120)
                            .ticks(ticks)
                            .disabled(disabled)
                            .draw(p, false)
                    })
                })
            };
            board(
                120,
                24,
                vec![
                    slider(0.3, 0, 18, false),
                    slider(0.5, 5, 24, false),
                    slider(0.7, 0, 18, true),
                    Box::new(|pen| {
                        sub(pen, 10, 0, 11, 16, |p| {
                            draw_slider_thumb(p, SliderState::Pressed, Theme::default())
                        })
                    }),
                ],
            )
        }),
        Scene::elements("progress-states", 520., 30., move |_, _| {
            board(
                120,
                14,
                vec![
                    Box::new(move |pen| draw_progress(pen, 0., lavender)),
                    Box::new(move |pen| draw_progress(pen, 0.4, lavender)),
                    Box::new(move |pen| draw_progress(pen, 1., lavender)),
                    Box::new(move |pen| draw_progress_indeterminate(pen, 3, lavender)),
                ],
            )
        }),
        Scene::elements("disclosure-states", 160., 28., move |_, _| {
            let mut cells: Vec<Cell> = Vec::new();
            for (disabled, pressed) in [(false, false), (false, true), (true, false)] {
                for open in [false, true] {
                    cells.push(Box::new(move |pen| {
                        draw_disclosure(pen, open, disabled, pressed)
                    }));
                }
            }
            board(12, 12, cells)
        }),
        Scene::elements("tabs-states", 420., 44., move |_, _| {
            let tab = |label: &'static str, disabled: bool, pressed: bool| -> Cell {
                Box::new(move |pen| {
                    sub(pen, 0, 2, 70, 22, |p| {
                        choice::draw_tab(p, label, false, disabled, pressed)
                    })
                })
            };
            board(
                70,
                24,
                vec![
                    tab("Normal", false, false),
                    Box::new(|pen| choice::draw_tab(pen, "Selected", true, false, false)),
                    tab("Pressed", false, true),
                    tab("Disabled", true, false),
                    Box::new(move |pen| {
                        choice::draw_tab(pen, "Focused", true, false, false);
                        pen.rect(5, pen.height - 5, pen.width - 10, 2, lavender.dark);
                    }),
                ],
            )
        }),
        Scene::elements("titlebox-states", 220., 30., move |_, _| {
            let mut cells: Vec<Cell> = Vec::new();
            for part in [WindowPart::Close, WindowPart::Zoom, WindowPart::Collapse] {
                for pressed in [false, true] {
                    cells.push(Box::new(move |pen| {
                        sub(pen, 0, 0, 11, 11, |p| draw_title_box(p, part, pressed))
                    }));
                }
            }
            cells.push(Box::new(|pen| {
                sub(pen, 0, 0, 9, 9, |p| {
                    draw_title_box(p, WindowPart::Close, false)
                })
            }));
            cells.push(Box::new(|pen| {
                sub(pen, 0, 0, 15, 15, |p| draw_grow_box(p, true))
            }));
            board(15, 15, cells)
        }),
        Scene::elements("alert-icons", 140., 44., |_, _| {
            let icon =
                |kind: AlertKind| -> Cell { Box::new(move |pen| draw_alert_icon(pen, kind)) };
            board(
                32,
                32,
                vec![
                    icon(AlertKind::Note),
                    icon(AlertKind::Caution),
                    icon(AlertKind::Stop),
                ],
            )
        }),
        Scene::elements("scrollbar-states", 260., 130., |_, _| {
            let bar = |value: f64, content: f64, pressed: ScrollPart| -> Cell {
                Box::new(move |pen| {
                    ScrollBar::new("bar", value, 10., content)
                        .length(116)
                        .draw(pen, pressed)
                })
            };
            div()
                .relative()
                .size_full()
                .child(board(
                    16,
                    116,
                    vec![
                        bar(0., 40., ScrollPart::None),
                        bar(15., 40., ScrollPart::Thumb),
                        bar(30., 40., ScrollPart::Forward),
                        bar(10., 40., ScrollPart::PageForward),
                        bar(0., 5., ScrollPart::None),
                    ],
                ))
                .child(div().absolute().left(px(120.)).top(px(10.)).child(drawing(
                    130.,
                    16.,
                    |pen, _, _| {
                        ScrollBar::new("horizontal", 20., 10., 40.)
                            .length(130)
                            .horizontal()
                            .draw(pen, ScrollPart::Back)
                    },
                )))
        }),
    ]
}

/// Draws lines of text, each from the top `y` of its line as the original
/// places them, in the system (bold) or application (small) face.
fn lines(pen: &mut Pen, lines: &[(i32, &str, Font)]) {
    for &(y, text, font) in lines {
        pen.text(0, y + font.ascent(), text, font, BLACK);
    }
}

fn text() -> Vec<Scene> {
    const SAMPLES: &str = "€5 £5 ¥5 ¢5 ® ™ ±1 §2 ¶ ¿Qué? ¡Sí! «oui» ‹a› a·b 5µm ¬ ´ ¨ ¸";
    vec![Scene::new("accents", 400., 278., |_, cx| {
        let name = cx.new(|cx| {
            TextField::new(cx)
                .with_text("Café")
                .label("Name")
                .size(120., 22.)
        });
        let notes = cx.new(|cx| {
            TextField::new(cx)
                .multiline(true)
                .with_text(
                    "ÉLÈVE, Über, Ångström, Æble, Þór\nfaçade, naïve, señor, Straße, smørrebrød, cœur, ðað",
                )
                .label("Notes")
                .size(300., 40.)
        });
        Held::new((name, notes), |(name, notes), _, _| {
            div()
                .flex()
                .flex_col()
                .items_start()
                .p(px(8.))
                .gap(px(8.))
                .child(drawing(384., 140., |pen, _, _| {
                    let (system, app) = (Font::Bold, Font::Small);
                    lines(
                        pen,
                        &[
                            (46, SAMPLES, system),
                            (60, SAMPLES, app),
                            (76, "Łódź, Żółć, Český Krumlov, Říčany, Győr, Ğüneş, Ąę, ďť İı", system),
                            (92, "Łódź, Żółć, Český Krumlov, Říčany, Győr, Ğüneş, Ąę, ĺŕ ďť ĎŤ İı", app),
                            (108, "Iași, Țară; Rīga, Ķemeri, Liepāja, Ņ", system),
                            (124, "Iași, Țară; Rīga, Ķemeri, Liepāja, Ņ", app),
                            (4, "ÀÁÂÃÄÅÆ ÇÈÉÊË ÌÍÎÏ Ñ ÒÓÔÕÖØŒ ÙÚÛÜ Ý", system),
                            (18, "àáâãäåæ çèéêë ìíîï ñ òóôõöøœ ùúûü ýÿ ß Ðð Þþ", system),
                            (
                                32,
                                "ÀÁÂÃÄÅÆ ÇÈÉÊË ÌÍÎÏ Ñ ÒÓÔÕÖØŒ ÙÚÛÜ Ý àáâãäåæ çèéêë ìíîï ñ òóôõöøœ ùúûü ýÿ ß Ðð Þþ",
                                app,
                            ),
                        ],
                    )
                }))
                .child(button("resume", "Résumé"))
                .child(name.clone())
                .child(notes.clone())
        })
    })]
}

fn status() -> Vec<Scene> {
    vec![Scene::new("stepper-status", 420., 80., |_, cx| {
        let stepper = cx.new(|cx| Stepper::new(cx).label("Count"));
        let number = cx.new(|cx| {
            NumberField::new(12., 6., 72., 1., cx)
                .digits(0)
                .label("Size")
        });
        let search = cx.new(|cx| {
            let mut search = SearchField::new(cx).width(120);
            search.set_value("pix", cx);
            search
        });
        Held::new(
            (stepper, number, search),
            |(stepper, number, search), _, _| {
                div()
                    .flex()
                    .flex_col()
                    .p(px(8.))
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(stepper.clone())
                            .child(drawing(13., 23., |pen, _, _| {
                                draw_stepper(pen, 1, StepperState::Normal)
                            }))
                            .child(drawing(13., 23., |pen, _, _| {
                                draw_stepper(pen, 0, StepperState::Disabled)
                            }))
                            .child(number.clone())
                            .child(search.clone())
                            .child(drawing(16., 16., |pen, _, _| draw_chasing_arrows(pen, 0))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(placard("12 items", Font::Small.measure("12 items") + 12))
                            .child(badge("3", Theme::default()))
                            .child(badge("New", Theme::default()))
                            .child(tag("draft"))
                            .children((0..4).map(|step| {
                                drawing(16., 16., move |pen, _, _| {
                                    draw_chasing_arrows(pen, step * 2)
                                })
                            })),
                    )
            },
        )
    })]
}

/// The original's two pop-up windows, Files open and Tools closed: in front
/// as `files_front` says, Tools moved to `tools_x`.
fn popup_windows(name: &'static str, files_front: bool, tools_x: i32) -> Scene {
    Scene::new(name, 400., 140., move |_, cx| {
        let make = |title: &'static str, x: i32, (width, height): (i32, i32), cx: &mut App| {
            let content = piece(cx, |_, _| {
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .bg(rgb(WHITE))
                    .p(px(6.))
                    .child(label("Read Me"))
                    .child(label("Recipes"))
            });
            cx.new(|cx| {
                PopupWindow::new(title, cx)
                    .content(content)
                    .position(x)
                    .size(width, height)
            })
        };
        let files: Entity<PopupWindow> = make("Files", 10, (200, 90), cx);
        let tools: Entity<PopupWindow> = make("Tools", tools_x, (150, 80), cx);
        files.update(cx, |files, cx| files.set_active(files_front, cx));
        tools.update(cx, |tools, cx| {
            tools.set_open(false, cx);
            tools.set_active(!files_front, cx);
        });
        Held::new((files, tools), |(files, tools), _, _| {
            div()
                .relative()
                .size_full()
                .bg(rgb(GRAY_5))
                .child(files.clone())
                .child(tools.clone())
        })
    })
}

/// A tab control as the original builds one: the tabs, 8 in from the
/// pane's left, over the pane, whose content is 12 in from its sides.
fn tab_control(
    id: &'static str,
    selected: usize,
    labels: &'static [&'static str],
    (width, height): (i32, i32),
    content: impl IntoElement,
) -> impl IntoElement {
    let widths: Vec<i32> = labels.iter().map(|l| Font::Bold.measure(l) + 24).collect();
    let gap_x = 8 + widths[..selected].iter().map(|w| w - 1).sum::<i32>();
    let gap_width = widths[selected];
    let top = 23;
    div()
        .relative()
        .w(px(width as f32))
        .h(px(height as f32))
        .flex_shrink_0()
        .child(div().absolute().top(px(top as f32)).child(drawing(
            width as f32,
            (height - top) as f32,
            move |pen, _, _| draw_tab_pane(pen, gap_x, gap_width),
        )))
        .child(
            div()
                .absolute()
                .left(px(8.))
                .top_0()
                .child(ChoiceGroup::tabs(id, selected, labels.iter().copied())),
        )
        .child(
            div()
                .absolute()
                .left(px(12.))
                .right(px(12.))
                .top(px((top + 12) as f32))
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(content),
        )
}

const BOLD_ICON: &[&str] = &[
    "######..", ".##..##.", ".##..##.", ".#####..", ".##..##.", ".##..##.", "######..",
];

const PAGE_ICON: &[&str] = &[
    "#####...", "#...##..", "#...###.", "#.....#.", "#.###.#.", "#.....#.", "#.###.#.", "#.....#.",
    "#######.",
];

fn groups() -> Vec<Scene> {
    vec![
        Scene::elements("bevel-states", 460., 110., |_, _| {
            div()
                .flex()
                .flex_col()
                .p(px(8.))
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(bevel_button("normal", "Normal"))
                        .child(bevel_button("on", "On").selected(true))
                        .child(bevel_button("off", "Off").selected(false))
                        .child(bevel_button("disabled", "Disabled").disabled(true))
                        .child(bevel_button("page", "Page").icon(PAGE_ICON))
                        .child(icon_button("bold", BOLD_ICON, "Bold").selected(true))
                        .child(bevel_button("menu", "Menu").menu_arrow(true)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(drawing(60., 22., |pen, _, _| {
                            draw_bevel_button(pen, false, true, false);
                            let x = (pen.width - Font::Bold.measure("Pressed")) / 2 + 1;
                            let top = 1 + (pen.height - Font::Bold.cap_height()) / 2;
                            pen.text(x, top + Font::Bold.ascent(), "Pressed", Font::Bold, BLACK);
                        }))
                        .child(ChoiceGroup::segments(
                            "segments",
                            1,
                            ["Left", "Center", "Right"],
                        )),
                )
                .child(
                    Toolbar::new()
                        .item(icon_button("new", PAGE_ICON, "New"))
                        .separator()
                        .item(icon_button("bold-tool", BOLD_ICON, "Bold").selected(true))
                        .item(bevel_button("plain", "Plain")),
                )
        }),
        Scene::elements("check-radio-widgets", 220., 80., |_, _| {
            div()
                .flex()
                .flex_col()
                .p(px(6.))
                .gap(px(2.))
                .child(checkbox("show", "Show hidden files", CheckState::On))
                .child(checkbox("disabled", "Disabled option", CheckState::Off).disabled(true))
                .child(ChoiceGroup::radio("sort", 0, ["By name", "By date"]))
        }),
        Scene::elements("group-tabs", 420., 130., |_, _| {
            div()
                .flex()
                .items_start()
                .p(px(8.))
                .gap(px(10.))
                .child(
                    group_box("Options", 140)
                        .child(checkbox("sounds", "Sounds", CheckState::On))
                        .child(separator(140 - 24))
                        .child(checkbox("blink", "Blink", CheckState::On)),
                )
                .child(tab_control(
                    "tabs",
                    1,
                    &["General", "Views", "Labels"],
                    (240, 110),
                    checkbox("content", "Tab content", CheckState::On),
                ))
        }),
    ]
}

/// East Asian text in both faces, styled and scaled, beside Latin text,
/// in `language`'s forms.
fn east_asian(name: &'static str, language: Language) -> Scene {
    Scene::elements(name, 200., 90., move |_, _| {
        div()
            .flex()
            .flex_col()
            .items_start()
            .py(px(4.))
            .px(px(6.))
            .gap(px(6.))
            .child(label_in("System 系统设置：显示", language))
            .child(text_in_language(
                "App 文稿，3 个项目。",
                Font::Small,
                BLACK,
                language,
            ))
            .child(drawing(188., 44., move |pen, _, _| {
                pen.language = language;
                let baseline = Font::Small.ascent();
                let style = TextStyle::default();
                let bold = TextStyle {
                    bold: true,
                    ..style
                };
                let italic = TextStyle {
                    italic: true,
                    ..style
                };
                let underline = TextStyle {
                    underline: true,
                    ..style
                };
                let x = pen.text_styled(0, baseline, "粗体 ", Font::Small, BLACK, bold);
                let x = pen.text_styled(x, baseline, "斜体 ", Font::Small, BLACK, italic);
                pen.text_styled(x + 4, baseline, "下划线", Font::Small, BLACK, underline);
                pen.text_styled(
                    0,
                    18 + 2 * Font::Bold.ascent(),
                    "12:30 中",
                    Font::Bold,
                    BLACK,
                    style.scaled(2),
                );
            }))
    })
    .east_asian()
}

/// The controls whose words and dates follow the language, on 15 October
/// 2026 at 21:41.
fn languages(name: &'static str, language: Language) -> Scene {
    Scene::new(name, 420., 200., move |_, cx| {
        let day = models::Date::new(2026, 10, 15).unwrap();
        // The original's clock, frozen for its pictures.
        let epoch = models::Date::new(1970, 1, 1).unwrap();
        let calendar = cx.new(|cx| CalendarView::new(day, cx).today(epoch).language(language));
        let picker = cx.new(|cx| DatePicker::new(day, cx).language(language));
        let time = cx.new(|cx| TimeField::new(21, 41, 0, cx).language(language));
        let pages = cx.new(|cx| Pagination::new(2, 5, cx).language(language));
        let search = cx.new(|cx| SearchField::new(cx).language(language));
        Held::new(
            (calendar, picker, time, pages, search),
            |(calendar, picker, time, pages, search), _, _| {
                div()
                    .flex()
                    .items_start()
                    .p(px(10.))
                    .gap(px(12.))
                    .child(calendar.clone())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_start()
                            .gap(px(10.))
                            .child(picker.clone())
                            .child(time.clone())
                            .child(pages.clone())
                            .child(search.clone()),
                    )
            },
        )
    })
    .east_asian()
}

/// A control of each kind in a window in the background, beside a small
/// window in front.
fn inactive_window() -> Scene {
    Scene::new("inactive-window", 580., 240., |_, cx| {
        let text = cx.new(|cx| TextField::new(cx).with_text("Text").size(110., 22.));
        let stepper = cx.new(|cx| Stepper::new(cx).label("Count"));
        let list = cx.new(|cx| {
            ListBox::new(["A", "B", "C", "D", "E", "F"], cx)
                .size(110, 52)
                .selected(1)
        });
        // As wide as the original's fits its items: 9 before the text, 8
        // after it, and the arrow's 18.
        let text_width = Font::Bold.measure("One").max(Font::Bold.measure("Two"));
        let menu = cx.new(|cx| {
            PopupMenu::new(["One", "Two"], cx)
                .selected(1)
                .width(9 + text_width + 8 + 18)
        });
        let table = cx.new(|cx| {
            let mut table = Table::with_columns(
                vec![
                    TableColumn::new("Name").width(60),
                    TableColumn::new("Size").width(40),
                ],
                vec![
                    vec!["Letter".into(), "2 K".into()],
                    vec!["Recipes".into(), "4 K".into()],
                ],
                cx,
            )
            .size(110, 60);
            table.sort_by(0, false, cx);
            table
        });
        let content = piece(cx, move |_, _| {
            let column = || div().flex().flex_col().items_start().gap(px(6.));
            div()
                .flex()
                .items_start()
                .p(px(8.))
                .gap(px(10.))
                .child(
                    column()
                        .child(button("button", "Button"))
                        .child(button("default", "Default").default_button())
                        .child(checkbox("check", "Check", CheckState::On))
                        .child(radio("radio", "Radio", true))
                        .child(disclosure("more", "More", false))
                        .child(
                            Slider::new("slider", 40., 0., 100.)
                                .width(120)
                                .label("Slider"),
                        )
                        .child(menu.clone()),
                )
                .child(
                    column()
                        .child(text.clone())
                        .child(bevel_button("bevel", "Bevel").selected(true))
                        .child(stepper.clone())
                        .child(list.clone())
                        .child(
                            div()
                                .flex()
                                .gap(px(4.))
                                .child(ScrollBar::new("bar", 0., 10., 40.).length(44)),
                        ),
                )
                .child(
                    column()
                        .child(
                            group_box("Group", Font::Bold.measure("Inside") + 24)
                                .child(label("Inside")),
                        )
                        .child(table.clone()),
                )
        });
        let back = cx.new(|cx| {
            PlatinumWindow::new("In the background", cx)
                .position(10., 10.)
                .size(430., 220.)
                .active(false)
                .content(content)
        });
        let front = cx.new(|cx| {
            PlatinumWindow::new("In front", cx)
                .position(450., 10.)
                .size(120., 60.)
        });
        Held::new((back, front), |(back, front), _, _| {
            div()
                .relative()
                .size_full()
                .child(back.clone())
                .child(front.clone())
        })
    })
}

fn windows() -> Vec<Scene> {
    vec![
        popup_windows("popup-window", true, 230),
        popup_windows("popup-window-focus", false, 200).prepare(|app, window| {
            // The Tools tab, the second stop, from the keyboard.
            app.update_window(window, |_, window, cx| {
                window.focus_next(cx);
                window.focus_next(cx);
            })
            .unwrap();
            key(app, window, "f19");
        }),
        popup_windows("popup-window-pressed", true, 200).prepare(|app, window| {
            press(app, window, 42., 41., Modifiers::default());
        }),
        Scene::elements("zoomrects", 300., 140., |_, _| {
            drawing(300., 140., |pen, _, _| {
                let from = Rect {
                    x: 10,
                    y: 100,
                    width: 32,
                    height: 32,
                };
                let to = |x, y, width, height| Rect {
                    x,
                    y,
                    width,
                    height,
                };
                draw_zoom_rects(pen, from, to(60, 10, 90, 70), 0.35);
                draw_zoom_rects(pen, from, to(170, 10, 120, 100), 0.7);
            })
        }),
        Scene::elements("toolbar-overflow", 150., 140., |_, _| {
            div().flex().flex_col().w(px(150.)).child(
                Toolbar::new()
                    .item(bevel_button("one", "One"))
                    .item(bevel_button("two", "Two"))
                    .separator()
                    .item(bevel_button("three", "Three"))
                    .item(bevel_button("bold", "Bold").selected(false)),
            )
        }),
        Scene::new("window-frames", 300., 220., |_, cx| {
            let back = cx.new(|cx| {
                PlatinumWindow::new("Background", cx)
                    .position(120., 90.)
                    .size(160., 110.)
                    .active(false)
            });
            let inside = piece(cx, |_, _| {
                div().flex().flex_col().p(px(10.)).child(checkbox(
                    "inside",
                    "Inside",
                    CheckState::Off,
                ))
            });
            let front = cx.new(|cx| {
                PlatinumWindow::new("Untitled", cx)
                    .position(30., 30.)
                    .size(180., 120.)
                    .content(inside)
            });
            Held::new((back, front), |(back, front), _, _| {
                div()
                    .relative()
                    .size_full()
                    .bg(rgb(GRAY_5))
                    .child(back.clone())
                    .child(front.clone())
            })
        }),
        Scene::elements("window-header", 260., 34., |_, _| {
            div().flex().flex_col().p(px(8.)).child(window_header_parts(
                "4 items",
                "47.7 MB available",
                244,
                Language::English,
            ))
        }),
        Scene::new("window-path-menu", 260., 150., |_, cx| {
            let frame = cx.new(|cx| {
                PlatinumWindow::new("Letters", cx)
                    .position(20., 10.)
                    .size(220., 120.)
                    .path(vec!["Platinum Disk".into(), "Documents".into()])
            });
            Held::new(frame, |frame, _, _| {
                div().relative().size_full().child(frame.clone())
            })
        })
        .prepare(|app, window| {
            let command = Modifiers {
                platform: true,
                ..Modifiers::default()
            };
            press(app, window, 130., 19., command);
            release(app, window, 130., 19., command);
            hover(app, window, 130., 76.);
        }),
        Scene::elements("well-balloon", 260., 80., |_, _| {
            div()
                .flex()
                .items_center()
                .p(px(8.))
                .gap(px(12.))
                .child(image_well(48, 48).label("Picture"))
                .child(drawing(150., 50., |pen, _, _| {
                    draw_balloon(pen);
                    let top = 8 + 6;
                    pen.text(
                        10,
                        top + Font::Small.ascent(),
                        "Turns the page.",
                        Font::Small,
                        BLACK,
                    );
                }))
        }),
    ]
}

fn dialogs() -> Vec<Scene> {
    vec![
        Scene::new("alert-caution", 400., 200., |window, cx| {
            let alert = cx.new(|cx| {
                Alert::new(
                    AlertKind::Caution,
                    "Save changes to the document “Notes” before closing?",
                    "If you don’t save, your changes will be lost.",
                    ["Don’t Save", "Cancel", "Save"],
                    cx,
                )
            });
            alert.update(cx, |alert, cx| alert.show(window, cx));
            Held::new(alert, |alert, _, _| {
                div().relative().size_full().child(alert.clone())
            })
        }),
        Scene::new("dialog-modal", 360., 240., |window, cx| {
            let name = cx.new(|cx| TextField::new(cx).label("New name").size(220., 22.));
            let focus = name.focus_handle(cx);
            let content = piece(cx, move |_, _| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(label("New name:"))
                    .child(name.clone())
            });
            let dialog = cx.new(|cx| {
                Dialog::new("Rename", content, cx)
                    .accept_label("Rename")
                    .width(260.)
                    .focus_handles(vec![focus])
            });
            dialog.update(cx, |dialog, cx| dialog.show(window, cx));
            Held::new(dialog, |dialog, _, _| {
                div().relative().size_full().child(dialog.clone())
            })
        }),
        Scene::new("progress-dialog-banner", 420., 240., |window, cx| {
            let progress = cx.new(|cx| {
                ProgressDialog::new("Copying", "Copying 12 items to “Backup”…", cx)
                    .progress(0.4)
                    .detail("About a minute left")
            });
            progress.update(cx, |progress, cx| progress.show(window, cx));
            let banner = cx.new(|cx| Banner::new(AlertKind::Note, "The disk was backed up.", cx));
            Held::new((progress, banner), |(progress, banner), _, _| {
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .flex_col()
                    .p(px(6.))
                    .gap(px(8.))
                    .child(progress.clone())
                    .child(banner.clone())
                    .child(text_in("28 items, 3.2 MB available", Font::Small, BLACK))
            })
        }),
        Scene::new("toasts", 420., 160., |_, cx| {
            let toasts = cx.new(|cx| {
                let mut toasts = Toasts::new(cx);
                for toast in [
                    Toast::new("Saved"),
                    Toast::new("Note deleted")
                        .kind(AlertKind::Caution)
                        .action("Undo"),
                    Toast::new("The disk is full")
                        .description("Empty the Wastebasket")
                        .kind(AlertKind::Stop),
                ] {
                    toasts.push(toast.timeout(std::time::Duration::ZERO), cx);
                }
                toasts
            });
            Held::new(toasts, |toasts, _, _| {
                div()
                    .relative()
                    .size_full()
                    .bg(rgb(GRAY_5))
                    .child(toasts.clone())
            })
        }),
    ]
}

pub fn scenes() -> Vec<Scene> {
    let mut scenes = boards();
    scenes.extend(dialogs());
    scenes.extend(status());
    scenes.extend(text());
    scenes.extend(groups());
    scenes.push(east_asian("eastasian", Language::SimplifiedChinese));
    scenes.push(east_asian("eastasian-ja", Language::Japanese));
    scenes.push(languages("languages-zh-hans", Language::SimplifiedChinese));
    scenes.push(languages("languages-ja", Language::Japanese));
    scenes.extend(windows());
    scenes.push(inactive_window());
    scenes.push(focus_states());
    scenes
}

/// Where each measured control was laid out in the last frame.
type Places = Rc<RefCell<Vec<Bounds<Pixels>>>>;

/// `child`, with its bounds kept in `places[index]` as it is laid out.
fn measured(places: &Places, index: usize, child: impl IntoElement) -> impl IntoElement {
    let places = places.clone();
    div().relative().child(child).child(
        canvas(
            move |bounds, _, _| places.borrow_mut()[index] = bounds,
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full(),
    )
}

/// One run of a row of the montage: x, y, width and color.
type Run = (i32, i32, i32, u32);

/// The original's focus montage: Tab moves the focus through one control of
/// each kind, and each step's focused control, 5 pixels around, takes the
/// next 150×64 cell. The scene's window draws the column scrolled to each
/// control in turn, keeps what it shows, and at last draws the montage.
fn focus_states() -> Scene {
    const COUNT: usize = 12;
    const CELL: (i32, i32) = (150, 64);
    let places: Places = Rc::new(RefCell::new(vec![Bounds::default(); COUNT]));
    let scroll = Rc::new(std::cell::Cell::new(0.));
    let montage: Rc<RefCell<Option<Vec<Run>>>> = Rc::new(RefCell::new(None));
    let scene = {
        let (places, scroll, montage) = (places.clone(), scroll.clone(), montage.clone());
        Scene::new("focus-states", 600., 192., move |_, cx| {
            // The original's field keeps its caret where a new one starts.
            let field = cx.new(|cx| {
                let mut field = TextField::new(cx).with_text("Name").size(100., 22.);
                field.select(0, 0, cx);
                field
            });
            let list = cx.new(|cx| ListBox::new(["One", "Two"], cx).size(90, 38).selected(1));
            let text_width = Font::Bold.measure("Name").max(Font::Bold.measure("Date"));
            let menu = cx.new(|cx| {
                PopupMenu::new(["Name", "Date"], cx)
                    .selected(0)
                    .label("Sort")
                    .width(9 + text_width + 8 + 18)
            });
            let stepper = cx.new(|cx| Stepper::new(cx).label("Count"));
            Held::new(
                (field, list, menu, stepper),
                move |(field, list, menu, stepper), _, _| {
                    if let Some(runs) = montage.borrow().clone() {
                        return div()
                            .child(drawing(600., 192., move |pen, _, _| {
                                for &(x, y, width, color) in &runs {
                                    pen.hline(x, y, width, color);
                                }
                            }))
                            .into_any_element();
                    }
                    let controls: [AnyElement; COUNT] = [
                        button("button", "Button").into_any_element(),
                        checkbox("check", "Check", CheckState::On).into_any_element(),
                        ChoiceGroup::radio("radio", 0, ["Radio"]).into_any_element(),
                        Slider::new("volume", 40., 0., 100.)
                            .width(120)
                            .label("Volume")
                            .into_any_element(),
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            // The original's label, its capitals centered on
                            // the button's 20 pixels.
                            .child(drawing(
                                Font::Bold.measure("Sort") as f32,
                                20.,
                                |pen, _, _| {
                                    let cap = Font::Bold.cap_height();
                                    pen.text(0, (20 - cap) / 2 + cap, "Sort", Font::Bold, BLACK);
                                },
                            ))
                            .child(menu.clone())
                            .into_any_element(),
                        disclosure("more", "More", false).into_any_element(),
                        field.clone().into_any_element(),
                        list.clone().into_any_element(),
                        tab_control("tabs", 0, &["Tabs", "Two"], (130, 50), div())
                            .into_any_element(),
                        bevel_button("bevel", "Bevel").into_any_element(),
                        ChoiceGroup::segments("seg", 0, ["Seg", "Two"]).into_any_element(),
                        stepper.clone().into_any_element(),
                    ];
                    div()
                        .absolute()
                        .top(px(-scroll.get()))
                        .left_0()
                        .flex()
                        .flex_col()
                        .items_start()
                        .p(px(10.))
                        .gap(px(10.))
                        .children(
                            controls
                                .into_iter()
                                .enumerate()
                                .map(|(index, control)| measured(&places, index, control)),
                        )
                        .into_any_element()
                },
            )
        })
    };
    scene.prepare(move |app, window| {
        let mut picture = image::RgbaImage::from_pixel(
            4 * CELL.0 as u32,
            3 * CELL.1 as u32,
            image::Rgba([0xdd, 0xdd, 0xdd, 255]),
        );
        for index in 0..COUNT {
            app.update_window(window, |_, window, cx| window.focus_next(cx))
                .unwrap();
            key(app, window, "f19");
            // Scroll the column so that the control is 5 from the top.
            let place = places.borrow()[index];
            let top = f32::from(place.origin.y) + scroll.get();
            scroll.set((top - 5.).max(0.));
            app.update_window(window, |_, window, cx| {
                window.refresh();
                let _ = window.draw(cx);
            })
            .unwrap();
            let place = places.borrow()[index];
            let shot = app.capture_screenshot(window).unwrap();
            let scale = shot.width() / 600;
            let (x, y) = (
                f32::from(place.origin.x) as i32 - 5,
                f32::from(place.origin.y) as i32 - 5,
            );
            let width = (f32::from(place.size.width) as i32 + 10).min(CELL.0);
            let height = (f32::from(place.size.height) as i32 + 10).min(CELL.1);
            let (cell_x, cell_y) = (index as i32 % 4 * CELL.0, index as i32 / 4 * CELL.1);
            for row in 0..height {
                for column in 0..width {
                    let mut pixel =
                        *shot.get_pixel(((x + column) as u32) * scale, ((y + row) as u32) * scale);
                    pixel[3] = 255;
                    picture.put_pixel((cell_x + column) as u32, (cell_y + row) as u32, pixel);
                }
            }
        }
        let mut runs = Vec::new();
        for (y, row) in picture.rows().enumerate() {
            let mut start = 0;
            let colors: Vec<u32> = row
                .map(|pixel| u32::from_be_bytes([0, pixel[0], pixel[1], pixel[2]]))
                .collect();
            for x in 1..=colors.len() {
                if x == colors.len() || colors[x] != colors[start] {
                    runs.push((start as i32, y as i32, (x - start) as i32, colors[start]));
                    start = x;
                }
            }
        }
        *montage.borrow_mut() = Some(runs);
        app.update_window(window, |_, window, _| window.refresh())
            .unwrap();
    })
}

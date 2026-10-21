//! The original's golden scenes of single controls, dialogs and frames.

use crate::Scene;
use gpui::{
    AnyElement, AnyView, AnyWindowHandle, App, AppContext as _, Context, Focusable,
    HeadlessAppContext, IntoElement, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, PlatformInput, Render, Styled, Window, div, point, px, rgb,
};
use gpui_platinum::{
    CheckState, Slider, Theme,
    paint::{Pen, drawing},
    slider::{ScrollPart, SliderState, draw_slider_thumb},
    theme::*,
    *,
};

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

/// Elements built anew each frame, without a background: a window's content.
struct Piece(Box<dyn Fn(&mut Window, &mut App) -> AnyElement>);

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

fn windows() -> Vec<Scene> {
    vec![
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
        })
        .pending(),
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
    scenes.extend(windows());
    scenes
}

//! The original's golden scenes of single controls, dialogs and frames.

use crate::Scene;
use gpui::{AnyElement, IntoElement, ParentElement, Styled, div, px};
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
        .children(cells.into_iter().map(move |cell| {
            drawing(width as f32, height as f32, move |pen, _, _| cell(pen))
        }))
        .into_any_element()
}

/// Draws into a `width`×`height` pen at (`x`, `y`) of `pen`.
fn sub(pen: &mut Pen, x: i32, y: i32, width: i32, height: i32, draw: impl FnOnce(&mut Pen)) {
    let mut inner = Pen::new(width, height);
    inner.language = pen.language;
    draw(&mut inner);
    pen.append(inner, x, y);
}

fn boards() -> Vec<Scene> {
    let lavender = ACCENTS[0];
    vec![
        Scene::elements("button-states", 400., 40., move |_, _| {
            board(
                70,
                28,
                vec![
                    Box::new(|pen| sub(pen, 4, 4, 62, 20, |p| draw_button(p, "Normal", false, false))),
                    Box::new(|pen| sub(pen, 4, 4, 62, 20, |p| draw_button(p, "Pressed", false, true))),
                    Box::new(|pen| sub(pen, 4, 4, 62, 20, |p| draw_button(p, "Disabled", true, false))),
                    Box::new(|pen| {
                        draw_default_ring(pen, false);
                        sub(pen, 4, 4, 62, 20, |p| draw_button(p, "Default", false, false));
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
                        sub(pen, 3, 3, 12, 12, |p| draw_checkbox(p, mark, disabled, pressed))
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
                sub(pen, 0, 0, 9, 9, |p| draw_title_box(p, WindowPart::Close, false))
            }));
            cells.push(Box::new(|pen| sub(pen, 0, 0, 15, 15, |p| draw_grow_box(p, true))));
            board(15, 15, cells)
        }),
        Scene::elements("alert-icons", 140., 44., |_, _| {
            let icon = |kind: AlertKind| -> Cell { Box::new(move |pen| draw_alert_icon(pen, kind)) };
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

pub fn scenes() -> Vec<Scene> {
    let mut scenes = boards();
    scenes.extend([]);
    scenes
}

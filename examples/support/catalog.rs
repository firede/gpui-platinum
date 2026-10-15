use gpui::{prelude::*, *};
use gpui_platinum::{Font, theme::*, *};

actions!(controls, [NextFocus, PreviousFocus, Quit]);

const PAGES: &[&str] = &[
    "Button",
    "Checkbox",
    "Radio, RadioGroup",
    "Disclosure",
    "GroupBox, Separator",
    "TabControl",
    "BevelButton, IconButton",
    "Segmented",
    "ProgressBar",
    "Label, TextIn, Paragraph",
    "Slider",
    "PopupMenu",
    "TextField, TextArea",
    "ListBox",
    "ScrollBar",
    "TreeView",
    "Table",
    "Calendar",
    "Window",
    "Alert",
    "Dialog",
];

pub struct Controls {
    focus: FocusHandle,
    page: usize,
    checks: [CheckState; 3],
    radio: usize,
    open: bool,
    tab: usize,
    toggle: bool,
    segment: usize,
    alert: bool,
    theme: Theme,
    sidebar: Entity<ListBox>,
    slider: f64,
    scroll: f64,
    popup: Entity<PopupMenu>,
    disabled_popup: Entity<PopupMenu>,
    text: Entity<TextField>,
    password: Entity<TextField>,
    area: Entity<TextField>,
    list: Entity<ListBox>,
    tree: Entity<TreeView>,
    table: Entity<Table>,
    calendar: Entity<CalendarView>,
    windows: Vec<Entity<PlatinumWindow>>,
    modal_buttons: [FocusHandle; 2],
}

impl Controls {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|cx| {
            ListBox::new(PAGES.iter().copied(), cx)
                .label("Controls")
                .size(220, 420)
                .fill_height()
                .selected(0)
        });
        cx.subscribe(&sidebar, |this, _, event, cx| {
            if let ListEvent::Selected(index) = event {
                this.page = *index;
                cx.notify();
            }
        })
        .detach();
        let windows = vec![
            cx.new(|cx| {
                PlatinumWindow::new("Untitled", cx)
                    .position(6., 45.)
                    .size(240., 150.)
                    .active(false)
            }),
            cx.new(|cx| {
                PlatinumWindow::new("Window", cx)
                    .position(55., 12.)
                    .size(280., 180.)
            }),
        ];
        for child in &windows {
            cx.subscribe(child, |this, child, event, cx| {
                if *event == WindowEvent::Activated {
                    for window in &this.windows {
                        window.update(cx, |window, cx| window.set_active(false, cx));
                    }
                    child.update(cx, |window, cx| window.set_active(true, cx));
                    if let Some(index) = this.windows.iter().position(|window| *window == child) {
                        let window = this.windows.remove(index);
                        this.windows.push(window);
                    }
                } else if *event == WindowEvent::Closed {
                    this.windows.retain(|window| *window != child);
                }
                cx.notify();
            })
            .detach();
        }
        Self {
            focus: cx.focus_handle(),
            page: 0,
            checks: [CheckState::On, CheckState::Off, CheckState::Mixed],
            radio: 1,
            open: true,
            tab: 0,
            toggle: true,
            segment: 1,
            alert: false,
            theme: Theme::default(),
            sidebar,
            slider: 40.,
            scroll: 0.,
            popup: cx.new(|cx| PopupMenu::new(["One", "Two", "Three"], cx).label("Sort by")),
            disabled_popup: cx.new(|cx| PopupMenu::new(["One", "Two", "Three"], cx).disabled(true)),
            text: cx.new(|cx| {
                TextField::new(cx)
                    .placeholder("Untitled")
                    .label("Name")
                    .size(280., 22.)
            }),
            password: cx.new(|cx| {
                TextField::new(cx)
                    .password(true)
                    .label("Password")
                    .size(280., 22.)
            }),
            area: cx.new(|cx| {
                TextField::new(cx)
                    .multiline(true)
                    .with_text("Dear diary, today the controls\nall lined up on whole pixels.")
                    .label("Note")
                    .size(280., 90.)
            }),
            list: cx.new(|cx| {
                ListBox::new(
                    [
                        "Letter", "Recipes", "To Do", "Read Me", "Untitled", "Minutes", "Projects",
                        "Mail",
                    ],
                    cx,
                )
                .size(220, 100)
                .selected(0)
            }),
            tree: cx.new(|cx| {
                TreeView::new(models::FileTree::sample(), cx)
                    .size(300, 180)
                    .label("Platinum Disk")
            }),
            table: cx.new(|cx| {
                Table::new(
                    vec!["Name".into(), "Size".into(), "Kind".into()],
                    vec![
                        vec!["Letter".into(), "1200".into(), "Document".into()],
                        vec!["Recipes".into(), "84".into(), "Folder".into()],
                        vec!["Read Me".into(), "256".into(), "Document".into()],
                    ],
                    cx,
                )
                .size(420, 180)
            }),
            calendar: cx.new(|cx| CalendarView::new(models::Date::today_utc(), cx)),
            windows,
            modal_buttons: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
        }
    }

    fn page(&self, cx: &mut Context<Self>) -> AnyElement {
        let column = div().flex().flex_col().gap(px(12.)).items_start();
        match self.page {
            0 => column
                .child(
                    row()
                        .child(
                            button("plain", "Button").on_click(cx.listener(|this, _, _, cx| {
                                this.alert = true;
                                cx.notify();
                            })),
                        )
                        .child(button("disabled", "Disabled").disabled(true)),
                )
                .child(
                    row()
                        .child(button("cancel", "Cancel").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.alert = false;
                                cx.notify();
                            },
                        )))
                        .child(
                            button("default", "OK")
                                .default_button()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.alert = true;
                                    cx.notify();
                                })),
                        ),
                )
                .into_any_element(),
            1 => column
                .children(["One", "Two", "Three"].iter().enumerate().map(|(i, text)| {
                    checkbox(("check", i), *text, self.checks[i])
                        .theme(self.theme)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.checks[i] = if this.checks[i] == CheckState::On {
                                CheckState::Off
                            } else {
                                CheckState::On
                            };
                            cx.notify();
                        }))
                }))
                .child(checkbox("disabled", "Disabled", CheckState::Off).disabled(true))
                .into_any_element(),
            2 => {
                let entity = cx.entity();
                column
                    .child(
                        ChoiceGroup::radio(
                            "radio-group",
                            self.radio,
                            [
                                ChoiceItem::new("One"),
                                ChoiceItem::new("Two"),
                                ChoiceItem::new("Disabled").disabled(true),
                            ],
                        )
                        .theme(self.theme)
                        .on_change(move |value, _, cx| {
                            entity.update(cx, |this, cx| {
                                this.radio = value;
                                cx.notify();
                            })
                        }),
                    )
                    .into_any_element()
            }
            3 => column
                .child(
                    disclosure("details", "Show", self.open)
                        .theme(self.theme)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open = !this.open;
                            cx.notify();
                        })),
                )
                .when(self.open, |el| {
                    el.child(text_in(
                        "The quick brown fox jumps over the lazy dog.",
                        Font::Small,
                        BLACK,
                    ))
                })
                .into_any_element(),
            4 => column
                .child(
                    group_box("GroupBox", 280)
                        .child(label("One"))
                        .child(separator(250))
                        .child(label("Two")),
                )
                .into_any_element(),
            5 => {
                let entity = cx.entity();
                column
                    .child(
                        ChoiceGroup::tabs("tabs", self.tab, ["One", "Two", "Three"])
                            .width(90)
                            .theme(self.theme)
                            .on_change(move |value, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.tab = value;
                                    cx.notify();
                                })
                            }),
                    )
                    .child(text_in(
                        ["One", "Two", "Three"][self.tab],
                        Font::Small,
                        BLACK,
                    ))
                    .into_any_element()
            }
            6 => column
                .child(
                    row()
                        .child(bevel_button("plain", "Button").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.alert = true;
                                cx.notify();
                            },
                        )))
                        .child(
                            bevel_button("toggle", "Toggle")
                                .selected(self.toggle)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle = !this.toggle;
                                    cx.notify();
                                })),
                        )
                        .child(bevel_button("disabled", "Disabled").disabled(true)),
                )
                .into_any_element(),
            7 => {
                let entity = cx.entity();
                column
                    .child(
                        ChoiceGroup::segments("segments", self.segment, ["One", "Two", "Three"])
                            .theme(self.theme)
                            .on_change(move |value, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.segment = value;
                                    cx.notify();
                                })
                            }),
                    )
                    .into_any_element()
            }
            8 => column
                .child(progress_bar(0.4, 280, self.theme))
                .child(progress_bar(0., 280, self.theme))
                .child(progress_bar(1., 280, self.theme))
                .into_any_element(),
            9 => column
                .child(label("The quick brown fox jumps over the lazy dog."))
                .child(text_in(
                    "The quick brown fox jumps over the lazy dog.",
                    Font::Small,
                    BLACK,
                ))
                .child(text_in("Platinum — À bientôt, 世界。", Font::Bold, BLACK))
                .into_any_element(),
            10 => {
                let first = cx.entity();
                let second = cx.entity();
                column
                    .child(
                        Slider::new("slider", self.slider, 0., 100.)
                            .width(280)
                            .theme(self.theme)
                            .label("Volume")
                            .on_change(move |value, _, cx| {
                                first.update(cx, |this, cx| {
                                    this.slider = value;
                                    cx.notify();
                                })
                            }),
                    )
                    .child(
                        Slider::new("stepped", self.slider, 0., 100.)
                            .width(280)
                            .ticks(5)
                            .step(25.)
                            .theme(self.theme)
                            .label("Speed")
                            .on_change(move |value, _, cx| {
                                second.update(cx, |this, cx| {
                                    this.slider = value;
                                    cx.notify();
                                })
                            }),
                    )
                    .child(
                        Slider::new("disabled-slider", self.slider, 0., 100.)
                            .width(280)
                            .disabled(true)
                            .label("Disabled"),
                    )
                    .into_any_element()
            }
            11 => column
                .child(self.popup.clone())
                .child(self.disabled_popup.clone())
                .into_any_element(),
            12 => column
                .child(self.text.clone())
                .child(self.password.clone())
                .child(self.area.clone())
                .into_any_element(),
            13 => column.child(self.list.clone()).into_any_element(),
            14 => {
                let vertical = cx.entity();
                let horizontal = cx.entity();
                column
                    .child(
                        row()
                            .child(
                                ScrollBar::new("vertical", self.scroll, 20., 100.)
                                    .length(160)
                                    .theme(self.theme)
                                    .label("Vertical scroll")
                                    .on_change(move |value, _, cx| {
                                        vertical.update(cx, |this, cx| {
                                            this.scroll = value;
                                            cx.notify();
                                        })
                                    }),
                            )
                            .child(
                                ScrollBar::new("horizontal", self.scroll, 20., 100.)
                                    .length(240)
                                    .horizontal()
                                    .theme(self.theme)
                                    .label("Horizontal scroll")
                                    .on_change(move |value, _, cx| {
                                        horizontal.update(cx, |this, cx| {
                                            this.scroll = value;
                                            cx.notify();
                                        })
                                    }),
                            ),
                    )
                    .into_any_element()
            }
            15 => column.child(self.tree.clone()).into_any_element(),
            16 => column.child(self.table.clone()).into_any_element(),
            17 => column.child(self.calendar.clone()).into_any_element(),
            18 => div()
                .relative()
                .w(px(460.))
                .h(px(290.))
                .overflow_hidden()
                .bg(rgb(GRAY_5))
                .children(self.windows.iter().cloned())
                .into_any_element(),
            _ => column
                .child(
                    button("show", "Show").on_click(cx.listener(|this, _, window, cx| {
                        this.alert = true;
                        window.focus(&this.modal_buttons[1], cx);
                        cx.notify();
                    })),
                )
                .into_any_element(),
        }
    }
}

fn row() -> Div {
    div().flex().items_center().gap(px(10.))
}

impl Render for Controls {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div()
            .id("controls")
            .key_context("Controls")
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(GRAY_2))
            .on_action(cx.listener(|this, _: &NextFocus, window, cx| {
                if this.alert {
                    let next = usize::from(this.modal_buttons[0].is_focused(window));
                    window.focus(&this.modal_buttons[next], cx);
                } else {
                    window.focus_next(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &PreviousFocus, window, cx| {
                if this.alert {
                    let next = usize::from(this.modal_buttons[0].is_focused(window));
                    window.focus(&this.modal_buttons[next], cx);
                } else {
                    window.focus_prev(cx);
                }
            }))
            .on_action(|_: &Quit, _, cx| cx.quit())
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "escape" | "enter") && this.alert {
                    this.alert = false;
                    cx.notify();
                }
            }))
            .child(
                div()
                    .h(px(30.))
                    .w_full()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .border_b_1()
                    .border_color(rgb(BLACK))
                    .child(label("Controls")),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .p(px(12.))
                    .gap(px(16.))
                    .child(self.sidebar.clone())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .flex_1()
                            .pt(px(8.))
                            .child(label(PAGES[self.page]))
                            .child(separator(480))
                            .child(self.page(cx)),
                    ),
            )
            .child(
                div()
                    .h(px(26.))
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(rgb(GRAY_7))
                    .px(px(10.))
                    .child(text_in(
                        format!("{} / {}", self.page + 1, PAGES.len()),
                        Font::Small,
                        BLACK,
                    )),
            );
        if self.alert {
            root =
                root.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .occlude()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .w(px(360.))
                                .border_2()
                                .border_color(rgb(BLACK))
                                .bg(rgb(GRAY_2))
                                .p(px(20.))
                                .flex()
                                .flex_col()
                                .gap(px(20.))
                                .child(label("This is the gallery of the Platinum controls."))
                                .child(row().justify_end().child(
                                    button("alert-ok", "OK").default_button().on_click(
                                        cx.listener(|this, _, _, cx| {
                                            this.alert = false;
                                            cx.notify();
                                        }),
                                    ),
                                )),
                        ),
                );
        }
        root
    }
}

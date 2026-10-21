use std::sync::Arc;

use gpui::{
    AnyElement, Context, Div, Entity, FocusHandle, KeyDownEvent, RenderImage, ScrollHandle, Window,
    actions, div, prelude::*, px, rgb,
};
use gpui_platinum::{
    Font, Menu, MenuItem, TextStyle,
    bitmap_icons::*,
    models::{Cell, Date, FileTree, NodeKind, Sheet, TreeNode},
    paint::{Pen, drawing},
    theme::*,
    *,
};

actions!(platinum_gallery, [NextFocus, PreviousFocus]);

const FONTS: [&str; 8] = [
    "Sans", "Serif", "Mono", "Script", "Display", "Rounded", "Slab", "Pixel",
];
const PAGES: [&str; 5] = [
    "Classic Controls",
    "More Controls",
    "Lists and Layout",
    "Forms and Office",
    "Documents and Navigation",
];
const PLANETS: [(&str, i32, i32); 8] = [
    ("Mercury", 0, 2440),
    ("Venus", 0, 6052),
    ("Earth", 1, 6371),
    ("Mars", 2, 3390),
    ("Jupiter", 95, 69911),
    ("Saturn", 146, 58232),
    ("Uranus", 28, 25362),
    ("Neptune", 16, 24622),
];
const NEW_ICON: &[&str] = &[
    "................",
    "...#######......",
    "...#wwwww##.....",
    "...#wwwww#w#....",
    "...#wwwww####...",
    "...#wwwwwwww#...",
    "...#wwwwwwww#...",
    "...#wwwwwwww#...",
    "...#wwwwwwww#...",
    "...#wwwwwwww#...",
    "...#wwwwwwww#...",
    "...#wwwwwwww#...",
    "...#wwwwwwww#...",
    "...##########...",
    "................",
    "................",
];
const LOGO: &[&str] = &[
    "....##....",
    "...#oo#...",
    "..#oooo#..",
    ".#oo++oo#.",
    "#oo+ww+oo#",
    "#oo+ww+oo#",
    ".#oo++oo#.",
    "..#oooo#..",
    "...#oo#...",
    "....##....",
];

#[derive(Clone, Copy)]
enum Command {
    Page(usize),
    Accent(usize),
    Alert,
    Open,
    Save,
}

pub struct Gallery {
    focus: FocusHandle,
    page: usize,
    capture: bool,
    theme: Theme,
    menu: Entity<MenuBar>,
    pending: Option<Command>,
    sound: bool,
    blink: bool,
    mixed: bool,
    radio: usize,
    details: bool,
    volume: f64,
    speed: f64,
    scroll: f64,
    tab: usize,
    toggle: bool,
    bold: bool,
    italic: bool,
    segment: usize,
    level: i32,
    split: f32,
    scroll_view: ScrollHandle,
    name: Entity<TextField>,
    search: Entity<TextField>,
    note: Entity<TextField>,
    sort: Entity<PopupMenu>,
    view_as: Entity<PopupMenu>,
    fonts: Entity<ListBox>,
    front: Entity<PlatinumWindow>,
    back: Entity<PlatinumWindow>,
    palette: Entity<PlatinumWindow>,
    windows: Vec<Entity<PlatinumWindow>>,
    alert: Entity<Alert>,
    open: Entity<OpenDialog>,
    save: Entity<SaveDialog>,
    style_menu: Entity<MenuBar>,
    copies: Entity<NumberField>,
    stepper: Entity<Stepper>,
    query: Entity<SearchField>,
    context: Entity<ContextMenu>,
    rich: Entity<RichTextArea>,
    docks: [Entity<PopupWindow>; 2],
    tree: Entity<TreeView>,
    icons: Entity<IconView>,
    table: Entity<Table>,
    calendar: Entity<CalendarView>,
    date: Entity<DatePicker>,
    time: Entity<TimeField>,
    combo: Entity<Combobox>,
    color: Entity<ColorWell>,
    sheet: Entity<SheetView>,
    ruler: Entity<Ruler>,
    columns: Entity<ColumnView>,
    columns_initialized: bool,
    font_picker: Entity<FontPicker>,
    path: Entity<PathPopup>,
    documents: Entity<DocumentTabs>,
}

impl Gallery {
    pub fn for_capture(page: usize, cx: &mut Context<Self>) -> Self {
        let mut gallery = Self::new(page, cx);
        gallery.capture = true;
        gallery.calendar.update(cx, |calendar, cx| {
            calendar.set_today(Date::new(1970, 1, 1).unwrap(), cx)
        });
        gallery
    }
    pub fn new(page: usize, cx: &mut Context<Self>) -> Self {
        let files = cx.new(|_| gallery_files());
        let front_content = cx.new(|_| WindowContent::AlertIcons);
        let front = cx.new(|cx| {
            PlatinumWindow::new("Window", cx)
                .position(10., 6.)
                .size(190., 104.)
                .content(front_content)
        });
        let back = cx.new(|cx| {
            PlatinumWindow::new("Behind", cx)
                .position(70., 50.)
                .size(150., 84.)
                .active(false)
        });
        let palette_content = cx.new(|_| WindowContent::Palette);
        let palette = cx.new(|cx| {
            PlatinumWindow::new("Palette", cx)
                .position(330., 14.)
                .size(100., 60.)
                .options(gpui_platinum::window::PlatinumWindowOptions {
                    utility: true,
                    ..Default::default()
                })
                .content(palette_content)
        });
        for frame in [&front, &back] {
            cx.subscribe(frame, |this, frame, event, cx| {
                match event {
                    WindowEvent::Activated => {
                        this.front
                            .update(cx, |view, cx| view.set_active(frame == this.front, cx));
                        this.back
                            .update(cx, |view, cx| view.set_active(frame == this.back, cx));
                        if let Some(index) = this.windows.iter().position(|view| *view == frame) {
                            let frame = this.windows.remove(index);
                            this.windows.push(frame);
                        }
                    }
                    WindowEvent::Closed => this.windows.retain(|view| *view != frame),
                    _ => {}
                }
                cx.notify();
            })
            .detach();
        }
        let projects_content = cx.new(|_| WindowContent::Projects);
        let mail_content = cx.new(|_| WindowContent::Empty);
        let projects = cx.new(|cx| {
            PopupWindow::new("Projects", cx)
                .position(6)
                .size(120, 80)
                .content(projects_content)
        });
        let mail = cx.new(|cx| {
            PopupWindow::new("Mail", cx)
                .position(104)
                .size(104, 60)
                .content(mail_content)
        });
        mail.update(cx, |mail, cx| {
            mail.set_open(false, cx);
            mail.set_active(false, cx);
        });
        let menu = cx.new(|cx| MenuBar::new(Vec::new(), cx));
        cx.subscribe(&menu, |this, _, event: &MenuChoice, cx| {
            let index = event.path.first().copied().unwrap_or(0);
            this.pending = match (event.menu, index) {
                (0, _) => Some(Command::Alert),
                (1, 1) => Some(Command::Open),
                (1, 2) => Some(Command::Save),
                (3, index) => Some(Command::Accent(index)),
                (4, index) => Some(Command::Page(index)),
                _ => None,
            };
            cx.notify();
        })
        .detach();
        let stepper = cx.new(|cx| Stepper::new(cx).label("Level"));
        cx.subscribe(&stepper, |this, _, event: &Step, cx| {
            this.level = (this.level + event.0).clamp(0, 10);
            cx.notify();
        })
        .detach();
        let calendar =
            cx.new(|cx| CalendarView::new(Date::new(2026, 10, 14).unwrap(), cx).label("Date"));
        let date = cx.new(|cx| DatePicker::new(Date::new(2026, 10, 14).unwrap(), cx));
        cx.subscribe(&calendar, |this, _, event: &DateChanged, cx| {
            this.date.update(cx, |date, cx| date.set_value(event.0, cx))
        })
        .detach();
        cx.subscribe(&date, |this, _, event: &DateChanged, cx| {
            this.calendar
                .update(cx, |calendar, cx| calendar.set_selection(event.0, cx))
        })
        .detach();
        let rich = cx.new(|cx| {
            let mut rich = RichTextArea::new(cx).size(452., 124.).show_size_menu(false);
            rich.set_runs(
                &[
                    Run {
                        text: "Minutes".into(),
                        style: TextStyle {
                            bold: true,
                            scale: 2,
                            ..Default::default()
                        },
                    },
                    Run {
                        text: "\nWe met at noon in the ".into(),
                        style: TextStyle::default(),
                    },
                    Run {
                        text: "east".into(),
                        style: TextStyle {
                            italic: true,
                            ..Default::default()
                        },
                    },
                    Run {
                        text: " room and agreed on the ".into(),
                        style: TextStyle::default(),
                    },
                    Run {
                        text: "budget".into(),
                        style: TextStyle {
                            bold: true,
                            ..Default::default()
                        },
                    },
                    Run {
                        text: ".\n".into(),
                        style: TextStyle::default(),
                    },
                    Run {
                        text: "Next: the 14th".into(),
                        style: TextStyle {
                            underline: true,
                            ..Default::default()
                        },
                    },
                ],
                cx,
            );
            rich
        });
        rich.read(cx).editor().update(cx, |editor, cx| {
            editor.set_alignments(&[Alignment::Center, Alignment::Left, Alignment::Right], cx);
            editor.select(0, 0, cx);
        });
        let rows = PLANETS
            .iter()
            .map(|(name, moons, radius)| {
                vec![(*name).into(), moons.to_string(), radius.to_string()]
            })
            .collect();
        let planet = bitmap_image(
            &[
                "....####....",
                "..##++++##..",
                ".#+ww+++++#.",
                ".#+w++++++#.",
                "#++++++++++#",
                "#++++++++++#",
                "#++++++++o+#",
                "#+++++++oo+#",
                ".#+++++ooo#.",
                ".#++++oooo#.",
                "..##oooo##..",
                "....####....",
            ],
            ACCENTS[0],
        );
        let table = cx.new(|cx| {
            Table::with_columns(
                vec![
                    TableColumn::new("Planet")
                        .width(150)
                        .icon(move |_| Some(planet.clone().into())),
                    TableColumn::new("Moons").width(120).right(true),
                    TableColumn::new("Radius (km)").width(160).right(true),
                ],
                rows,
                cx,
            )
            .size(456, 116)
        });
        table.update(cx, |table, cx| {
            table.sort_by(2, false, cx);
            table.set_selection(Some(2), cx);
        });
        let mut model = Sheet::default();
        for (row, values) in [
            ["Month", "Sales"],
            ["July", "120"],
            ["August", "145"],
            ["September", "98"],
            ["Total", "=SUM(B2:B4)"],
        ]
        .into_iter()
        .enumerate()
        {
            for (column, value) in values.into_iter().enumerate() {
                model.set(Cell::new(column, row), value);
            }
        }
        model.cursor = Cell::new(1, 4);
        model.anchor = Cell::new(1, 4);
        let font_picker = cx.new(|cx| FontPicker::new(cx).families(FONTS).width(444));
        font_picker.update(cx, |picker, cx| {
            picker.set_choice(
                FontChoice {
                    family: "Serif".into(),
                    size: 12.,
                    italic: true,
                    ..Default::default()
                },
                cx,
            )
        });
        let columns = cx.new(|cx| ColumnView::new(gallery_files(), cx).size(444, 96));
        columns.update(cx, |columns, cx| columns.set_selection(Some(3), cx));
        let mut this = Self {
            focus: cx.focus_handle().tab_stop(true),
            page: page.min(4),
            capture: false,
            theme: Theme::default(),
            menu,
            pending: None,
            sound: true,
            blink: false,
            mixed: false,
            radio: 1,
            details: false,
            volume: 60.,
            speed: 0.3,
            scroll: 0.,
            tab: 0,
            toggle: true,
            bold: true,
            italic: false,
            segment: 1,
            level: 4,
            split: 150.,
            scroll_view: ScrollHandle::new(),
            name: cx.new(|cx| {
                TextField::new(cx)
                    .with_text("Untitled Folder")
                    .size(200., 22.)
                    .label("Name")
            }),
            search: cx.new(|cx| {
                TextField::new(cx)
                    .placeholder("Search")
                    .size(200., 22.)
                    .label("Search")
            }),
            note: cx.new(|cx| {
                TextField::new(cx)
                    .multiline(true)
                    .with_text("Dear diary, today the controls\nall lined up on whole pixels.")
                    .size(200., 74.)
                    .label("Note")
            }),
            sort: cx
                .new(|cx| PopupMenu::new(["Name", "Date Modified", "Size", "Kind"], cx).width(116)),
            view_as: cx.new(|cx| {
                PopupMenu::new(["as Icons", "as List"], cx)
                    .width(83)
                    .disabled(true)
            }),
            fonts: cx.new(|cx| ListBox::new(FONTS, cx).size(150, 82).selected(2)),
            front: front.clone(),
            back: back.clone(),
            palette,
            windows: vec![back, front],
            alert: cx.new(|cx| {
                Alert::new(
                    AlertKind::Note,
                    "This is the gallery of the Platinum controls.",
                    "Every pixel of it is drawn on a whole-pixel grid.",
                    ["OK"],
                    cx,
                )
            }),
            open: cx.new(|cx| OpenDialog::new(files.clone(), cx)),
            save: cx.new(|cx| SaveDialog::new(files, cx).prompt("Save as:")),
            style_menu: cx.new(|cx| {
                MenuBar::context_menu(vec![MenuItem::new("Plain"), MenuItem::new("Bold")], cx)
            }),
            copies: cx.new(|cx| NumberField::new(2., 1., 99., 1., cx).label("Copies")),
            stepper,
            query: cx.new(|cx| SearchField::new(cx).width(200).label("Search")),
            context: cx.new(|cx| {
                ContextMenu::new(
                    vec![
                        MenuItem::new("Cut"),
                        MenuItem::new("Copy"),
                        MenuItem::new("Paste").disabled(true),
                    ],
                    cx,
                )
                .label("Right-click here")
                .size(206, 40)
            }),
            rich,
            docks: [projects, mail],
            tree: cx.new(|cx| {
                let mut model = gallery_files();
                model.selected = Some(4);
                TreeView::new(model, cx)
                    .size(206, 116)
                    .label("Files")
                    .default_icons(false)
            }),
            icons: cx.new(|cx| {
                IconView::new(gallery_icons(), cx)
                    .size(206, 140)
                    .selected(1)
                    .label("Icons")
            }),
            table,
            calendar,
            date,
            time: cx.new(|cx| TimeField::new(9, 41, 0, cx).step(5)),
            combo: cx.new(|cx| Combobox::new(FONTS, cx).width(160).label("Font")),
            color: cx.new(|cx| ColorWell::new(ACCENTS[0].mid, cx).label("Highlight color")),
            sheet: cx.new(|cx| {
                SheetView::new(model, cx)
                    .size(456, 160)
                    .dimensions(50, 10)
                    .label("Sales")
            }),
            ruler: cx.new(|cx| {
                Ruler::new(
                    RulerState {
                        left: 16,
                        right: 400,
                        first: 0,
                    },
                    cx,
                )
                .width(456)
            }),
            columns,
            columns_initialized: false,
            font_picker,
            path: cx.new(|cx| {
                PathPopup::new(
                    vec!["Platinum Disk".into(), "Documents".into(), "Letters".into()],
                    cx,
                )
                .width(84)
            }),
            documents: cx.new(|cx| {
                DocumentTabs::new(vec!["Letter".into(), "Notes".into()], cx)
                    .size(246, 80)
                    .content(|index, _, _| {
                        text_in(["Dear Ada,", "Buy lamp oil."][index], Font::Small, BLACK)
                            .into_any_element()
                    })
            }),
        };
        this.combo
            .update(cx, |combo, cx| combo.set_value("Mono", cx));
        this.save
            .update(cx, |save, cx| save.set_name("untitled", cx));
        this.refresh_menu(cx);
        this
    }

    fn refresh_menu(&mut self, cx: &mut Context<Self>) {
        let menus = vec![
            Menu::new("About", vec![MenuItem::new("About the Gallery…")]).icon(LOGO),
            Menu::new(
                "File",
                vec![
                    MenuItem::new("New Window").shortcut("n"),
                    MenuItem::new("Open…").shortcut("o"),
                    MenuItem::new("Save As…").shortcut("shift-s"),
                    MenuItem::new("Close").shortcut("w"),
                ],
            ),
            Menu::new("Edit", vec![MenuItem::new("Undo").shortcut("z")]),
            Menu::new(
                "Appearance",
                ACCENTS
                    .iter()
                    .enumerate()
                    .map(|(index, accent)| {
                        MenuItem::new(accent.name).checked(self.theme.accent == ACCENTS[index])
                    })
                    .collect(),
            ),
            Menu::new(
                "Page",
                PAGES
                    .iter()
                    .enumerate()
                    .map(|(index, name)| {
                        MenuItem::new(*name)
                            .shortcut((index + 1).to_string())
                            .checked(index == self.page)
                    })
                    .collect(),
            ),
        ];
        self.menu.update(cx, |menu, cx| menu.set_menus(menus, cx));
    }

    fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        macro_rules! update_themes { ($($view:expr),* $(,)?) => { $( $view.update(cx, |view, cx| view.set_theme(theme, cx)); )* }; }
        update_themes!(
            self.menu,
            self.name,
            self.search,
            self.note,
            self.sort,
            self.view_as,
            self.fonts,
            self.front,
            self.back,
            self.palette,
            self.alert,
            self.open,
            self.save,
            self.style_menu,
            self.copies,
            self.query,
            self.context,
            self.rich,
            self.tree,
            self.icons,
            self.table,
            self.calendar,
            self.date,
            self.time,
            self.combo,
            self.color,
            self.sheet,
            self.ruler,
            self.columns,
            self.font_picker,
            self.path,
            self.documents,
            self.docks[0],
            self.docks[1]
        );
        self.refresh_menu(cx);
        cx.notify();
    }

    fn page_one(&self, cx: &mut Context<Self>) -> AnyElement {
        let weak = cx.weak_entity();
        let weak_volume = weak.clone();
        let weak_speed = weak.clone();
        let weak_radio = weak.clone();
        let weak_tab = weak.clone();
        let weak_scroll = weak.clone();
        let left = column(232)
            .child(
                group_box("Push buttons", 232)
                    .child(
                        row(8)
                            .child(button("cancel", "Cancel").cancel_button().theme(self.theme))
                            .child(
                                button("ok", "OK")
                                    .default_button()
                                    .theme(self.theme)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.alert.update(cx, |alert, cx| alert.show(window, cx))
                                    })),
                            ),
                    )
                    .child(
                        row(8)
                            .child(
                                button("disabled", "Disabled")
                                    .disabled(true)
                                    .theme(self.theme),
                            )
                            .child(drawing(70., 20., |pen, _, _| {
                                draw_button(pen, "Pressed", false, true)
                            })),
                    ),
            )
            .child(
                group_box("Check boxes and radios", 232)
                    .child(
                        checkbox("sound", "Play sounds", checked(self.sound))
                            .theme(self.theme)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.sound = !this.sound;
                                cx.notify();
                            })),
                    )
                    .child(
                        checkbox("blink", "Blink the caret", checked(self.blink))
                            .theme(self.theme)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.blink = !this.blink;
                                cx.notify();
                            })),
                    )
                    .child(
                        checkbox(
                            "mixed",
                            "Some of them",
                            if self.mixed {
                                CheckState::On
                            } else {
                                CheckState::Mixed
                            },
                        )
                        .theme(self.theme)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.mixed = !this.mixed;
                            cx.notify();
                        })),
                    )
                    .child(
                        checkbox("not-now", "Not now", checked(self.blink))
                            .disabled(true)
                            .theme(self.theme),
                    )
                    .child(separator(206))
                    .child(
                        ChoiceGroup::radio(
                            "size",
                            self.radio,
                            [
                                ChoiceItem::new("Small"),
                                ChoiceItem::new("Medium"),
                                ChoiceItem::new("Large").disabled(true),
                            ],
                        )
                        .theme(self.theme)
                        .on_change(move |index, _, cx| {
                            weak_radio
                                .update(cx, |this, cx| {
                                    this.radio = index;
                                    cx.notify();
                                })
                                .ok();
                        }),
                    ),
            )
            .child(
                group_box("Disclosure", 232)
                    .child(
                        disclosure("details", "Show details", self.details)
                            .theme(self.theme)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.details = !this.details;
                                cx.notify();
                            })),
                    )
                    .when(self.details, |group| group.child(label("Here they are."))),
            )
            .child(group_box("Keyboard focus", 232).child(focus_sample(self.theme)));
        let middle = column(232)
            .child(
                group_box("Sliders and progress", 232)
                    .child(
                        Slider::new("volume", self.volume, 0., 100.)
                            .width(200)
                            .theme(self.theme)
                            .label("Volume")
                            .on_change(move |value, _, cx| {
                                weak_volume
                                    .update(cx, |this, cx| {
                                        this.volume = value;
                                        cx.notify();
                                    })
                                    .ok();
                            }),
                    )
                    .child(
                        Slider::new("speed", self.speed, 0., 1.)
                            .width(200)
                            .ticks(5)
                            .step(0.25)
                            .theme(self.theme)
                            .label("Speed")
                            .on_change(move |value, _, cx| {
                                weak_speed
                                    .update(cx, |this, cx| {
                                        this.speed = value;
                                        cx.notify();
                                    })
                                    .ok();
                            }),
                    )
                    .child(progress_bar(0.62, 200, self.theme))
                    .child(if self.capture {
                        drawing(200., 14., {
                            let accent = self.theme.accent;
                            move |pen, _, _| draw_progress_indeterminate(pen, 0, accent)
                        })
                        .into_any_element()
                    } else {
                        gpui_platinum::display::indeterminate_progress(200, self.theme)
                            .into_any_element()
                    }),
            )
            .child(
                group_box("Pop-up menus", 232)
                    .child(row(6).child(label("Sort by:")).child(self.sort.clone()))
                    .child(self.view_as.clone()),
            )
            .child(
                group_box("Text", 232)
                    .child(self.name.clone())
                    .child(self.search.clone())
                    .child(self.note.clone()),
            );
        let gap_x = 8 + ["General", "Sharing", "Memory"][..self.tab]
            .iter()
            .map(|title| Font::Bold.measure(title) + 24 - 1)
            .sum::<i32>();
        let gap_width = Font::Bold.measure(["General", "Sharing", "Memory"][self.tab]) + 24;
        let tabs = div()
            .relative()
            .w(px(232.))
            .h(px(84.))
            .flex_shrink_0()
            .child(drawing(232., 84., move |pen, _, _| {
                paint_at(pen, 0, 23, 232, 61, |pane| {
                    draw_tab_pane(pane, gap_x, gap_width)
                });
            }))
            .child(
                div().absolute().left(px(8.)).top_0().child(
                    ChoiceGroup::tabs("gallery-tabs", self.tab, ["General", "Sharing", "Memory"])
                        .theme(self.theme)
                        .on_change(move |index, _, cx| {
                            weak_tab
                                .update(cx, |this, cx| {
                                    this.tab = index;
                                    cx.notify();
                                })
                                .ok();
                        }),
                ),
            )
            .child(div().absolute().left(px(12.)).top(px(36.)).child(text_in(
                [
                    "General settings go here.",
                    "Who may connect.",
                    "How much to keep.",
                ][self.tab],
                Font::Small,
                BLACK,
            )));
        let right = column(232)
            .child(
                group_box("List and scroll bar", 232).child(
                    row(8).child(self.fonts.clone()).child(
                        ScrollBar::new("gallery-scroll", self.scroll, 20., 100.)
                            .length(82)
                            .theme(self.theme)
                            .label("Scroll")
                            .on_change(move |value, _, cx| {
                                weak_scroll
                                    .update(cx, |this, cx| {
                                        this.scroll = value;
                                        cx.notify();
                                    })
                                    .ok();
                            }),
                    ),
                ),
            )
            .child(tabs)
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(150.))
                    .bg(rgb(GRAY_5))
                    .overflow_hidden()
                    .children(self.windows.iter().cloned()),
            );
        row(12)
            .p(px(12.))
            .items_start()
            .child(left)
            .child(middle)
            .child(right)
            .into_any_element()
    }

    fn page_two(&self, cx: &mut Context<Self>) -> AnyElement {
        let weak = cx.weak_entity();
        let toolbar = Toolbar::new()
            .theme(self.theme)
            .item(bevel_button("new", "New").icon_only(NEW_ICON))
            .separator()
            .item(
                bevel_button("bold", "Bold")
                    .icon_only(ICON_BOLD)
                    .selected(self.bold)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.bold = !this.bold;
                        cx.notify();
                    })),
            )
            .item(
                bevel_button("italic", "Italic")
                    .icon_only(ICON_ITALIC)
                    .selected(self.italic)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.italic = !this.italic;
                        cx.notify();
                    })),
            )
            .separator()
            .item(bevel_button("align", "Align").icon(ICON_ALIGN_LEFT));
        let left = column(232)
            .child(
                group_box("Bevel buttons", 232)
                    .child(
                        row(6)
                            .child(bevel_button("plain", "Plain").theme(self.theme))
                            .child(
                                bevel_button("toggle", "Toggle")
                                    .selected(self.toggle)
                                    .theme(self.theme)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle = !this.toggle;
                                        cx.notify();
                                    })),
                            )
                            .child(bevel_button("off", "Off").disabled(true).theme(self.theme)),
                    )
                    .child(
                        row(6)
                            .child(
                                bevel_button("page", "Page")
                                    .icon(NEW_ICON)
                                    .theme(self.theme),
                            )
                            .child(bevel_button("style", "Style").theme(self.theme).on_click(
                                cx.listener(|this, event: &gpui::ClickEvent, window, cx| {
                                    this.style_menu.update(cx, |menu, cx| {
                                        menu.show_at(0, event.position(), window, cx)
                                    });
                                }),
                            )),
                    )
                    .child(
                        ChoiceGroup::segments("segment", self.segment, ["Left", "Center", "Right"])
                            .theme(self.theme)
                            .on_change(move |index, _, cx| {
                                weak.update(cx, |this, cx| {
                                    this.segment = index;
                                    cx.notify();
                                })
                                .ok();
                            }),
                    ),
            )
            .child(
                group_box("Badges and tags", 232).child(
                    row(6)
                        .child(badge("3", self.theme))
                        .child(badge("New", self.theme))
                        .child(tag("draft"))
                        .child(tag("letters")),
                ),
            );
        let middle = column(232)
            .child(
                group_box("Numbers and search", 232)
                    .child(
                        row(8)
                            .child(label("Copies:").height(22))
                            .child(self.copies.clone()),
                    )
                    .child(
                        row(8)
                            .child(label("Level:").height(23))
                            .child(self.stepper.clone()),
                    )
                    .child(self.query.clone()),
            )
            .child(
                group_box("Status", 232)
                    .child(
                        row(10)
                            .child(if self.capture {
                                drawing(16., 16., |pen, _, _| {
                                    gpui_platinum::display::draw_chasing_arrows(pen, 0)
                                })
                                .into_any_element()
                            } else {
                                chasing_arrows().into_any_element()
                            })
                            .child(text_in("Connecting…", Font::Small, BLACK)),
                    )
                    .child(placard("28 items, 3.2 MB available", 138)),
            );
        let right = column(232)
            .child(
                group_box("Image well and help", 232).child(
                    row(10)
                        .items_start()
                        .child(image_well(48, 48).label("Picture"))
                        .child(balloon("Drop a picture in\nthe well to use it.", 150)),
                ),
            )
            .child(
                group_box("Context menu", 232).child(div().h(px(40.)).child(self.context.clone())),
            );
        column_full()
            .child(toolbar)
            .child(
                row(12)
                    .px(px(12.))
                    .pt(px(12.))
                    .items_start()
                    .child(left)
                    .child(middle)
                    .child(right),
            )
            .child(
                row(12)
                    .p(px(12.))
                    .items_stretch()
                    .child(column(476).child(group_box("Rich text", 476).child(self.rich.clone())))
                    .child(
                        column(232).child(
                            group_box("Pop-up windows", 232).child(
                                div()
                                    .relative()
                                    .h(px(118.))
                                    .w_full()
                                    .overflow_hidden()
                                    .bg(rgb(GRAY_6))
                                    .border_1()
                                    .border_color(rgb(BLACK))
                                    .child(self.docks[1].clone())
                                    .child(self.docks[0].clone()),
                            ),
                        ),
                    ),
            )
            .into_any_element()
    }

    fn page_three(&self, cx: &mut Context<Self>) -> AnyElement {
        let weak = cx.weak_entity();
        let scroll = ScrollView::new(
            "planet-scroll",
            &self.scroll_view,
            column_full()
                .gap(px(2.))
                .children(PLANETS.iter().enumerate().map(|(index, (name, _, _))| {
                    text_in(format!("{}. {name}", index + 1), Font::Small, BLACK)
                })),
        )
        .size(144, 140)
        .theme(self.theme);
        row(12)
            .p(px(12.))
            .items_start()
            .child(
                column(232)
                    .child(group_box("Tree view", 232).child(self.tree.clone()))
                    .child(group_box("Icon view", 232).child(self.icons.clone())),
            )
            .child(
                column(482)
                    .child(group_box("Table", 482).child(self.table.clone()))
                    .child(
                        group_box("Split and scroll view", 482).child(
                            div()
                                .relative()
                                .h(px(140.))
                                .w_full()
                                .bg(rgb(GRAY_5))
                                .overflow_hidden()
                                .child(
                                    Split::new(
                                        "gallery-split",
                                        self.split,
                                        column_full().p(px(6.)).child(text_in(
                                            "A pane",
                                            Font::Small,
                                            BLACK,
                                        )),
                                        scroll,
                                    )
                                    .size(300, 140)
                                    .theme(self.theme)
                                    .on_change(
                                        move |value, _, cx| {
                                            weak.update(cx, |this, cx| {
                                                this.split = value;
                                                cx.notify();
                                            })
                                            .ok();
                                        },
                                    ),
                                )
                                .child(self.palette.clone()),
                        ),
                    ),
            )
            .into_any_element()
    }

    fn page_four(&self) -> AnyElement {
        row(12)
            .p(px(12.))
            .items_start()
            .child(
                column(232).child(
                    group_box("Dates and times", 232)
                        .child(self.calendar.clone())
                        .child(row(8).child(self.date.clone()))
                        .child(self.time.clone()),
                ),
            )
            .child(
                column(482)
                    .child(
                        group_box("Combo box and color well", 482).child(
                            row(12)
                                .child(self.combo.clone())
                                .child(label("Color:"))
                                .child(self.color.clone()),
                        ),
                    )
                    .child(group_box("Sheet", 482).child(self.sheet.clone()))
                    .child(group_box("Ruler", 482).child(self.ruler.clone())),
            )
            .into_any_element()
    }

    fn page_five(&self) -> AnyElement {
        column_full()
            .flex_1()
            .child(
                row(12)
                    .flex_1()
                    .p(px(12.))
                    .items_start()
                    .child(
                        column(470)
                            .child(group_box("Column view", 470).child(self.columns.clone()))
                            .child(group_box("Font", 470).child(self.font_picker.clone())),
                    )
                    .child(
                        column(246)
                            .child(group_box("Path", 246).child(self.path.clone()))
                            .child(self.documents.clone())
                            .child(
                                div()
                                    .w_full()
                                    .relative()
                                    .h(px(90.))
                                    .child(div().absolute().inset_0().child(drawing(
                                        246.,
                                        90.,
                                        |pen, _, _| {
                                            pen.rect(0, 0, 246, 90, GRAY_2);
                                            pen.frame(0, 0, 246, 90, BLACK);
                                            pen.bevel(1, 1, 244, 88, WHITE, GRAY_7);
                                        },
                                    )))
                                    .child(
                                        div()
                                            .absolute()
                                            .inset(px(10.))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(text_in("No messages", Font::Small, GRAY_8)),
                                    ),
                            ),
                    ),
            )
            .child(status_bar(
                ["12 items", "3 selected", "48 MB available"],
                756,
            ))
            .into_any_element()
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(command) = self.pending.take() {
            match command {
                Command::Page(page) => {
                    self.page = page;
                    self.refresh_menu(cx);
                    cx.notify();
                }
                Command::Accent(index) => {
                    let mut theme = self.theme;
                    theme.accent = ACCENTS[index];
                    self.set_theme(theme, cx);
                }
                Command::Alert => self.alert.update(cx, |alert, cx| alert.show(window, cx)),
                Command::Open => self.open.update(cx, |open, cx| open.show(window, cx)),
                Command::Save => self.save.update(cx, |save, cx| save.show(window, cx)),
            }
        }
        if self.page == 4 && !self.columns_initialized {
            self.columns_initialized = true;
            self.tree.update(cx, |tree, cx| {
                tree.model_mut().find_mut(2).unwrap().open = true;
                cx.notify();
            });
        }
        let page = match self.page {
            0 => self.page_one(cx),
            1 => self.page_two(cx),
            2 => self.page_three(cx),
            3 => self.page_four(),
            _ => self.page_five(),
        };
        let content = column_full()
            .size_full()
            .bg(rgb(self.theme.background))
            .track_focus(&self.focus)
            .key_context("Gallery")
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this
                    .menu
                    .update(cx, |menu, cx| menu.dispatch_shortcut(event, window, cx))
                {
                    cx.stop_propagation();
                }
            }))
            .on_action(|_: &NextFocus, window, cx| window.focus_next(cx))
            .on_action(|_: &PreviousFocus, window, cx| window.focus_prev(cx))
            .child(self.menu.clone())
            .child(page)
            .child(self.style_menu.clone())
            .child(self.alert.clone())
            .child(self.open.clone())
            .child(self.save.clone());
        button_scope("gallery-buttons", content)
    }
}

enum WindowContent {
    AlertIcons,
    Palette,
    Projects,
    Empty,
}
impl Render for WindowContent {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        match self {
            Self::Palette => row(4)
                .p(px(6.))
                .child(bevel_button("bold", "Bold").icon_only(ICON_BOLD))
                .child(bevel_button("italic", "Italic").icon_only(ICON_ITALIC))
                .into_any_element(),
            Self::Projects => column_full()
                .size_full()
                .p(px(4.))
                .bg(rgb(WHITE))
                .child(text_in("Platinum", Font::Small, BLACK))
                .child(text_in("Desk", Font::Small, BLACK))
                .into_any_element(),
            Self::Empty => div().bg(rgb(WHITE)).size_full().into_any_element(),
            Self::AlertIcons => row(10)
                .p(px(10.))
                .children([AlertKind::Note, AlertKind::Caution, AlertKind::Stop].map(alert_icon))
                .into_any_element(),
        }
    }
}

fn column(width: i32) -> Div {
    column_full()
        .w(px(width as f32))
        .flex_shrink_0()
        .gap(px(12.))
}
fn column_full() -> Div {
    div().flex().flex_col()
}
fn row(gap: i32) -> Div {
    div().flex().items_center().gap(px(gap as f32))
}
fn checked(value: bool) -> CheckState {
    if value {
        CheckState::On
    } else {
        CheckState::Off
    }
}

fn gallery_files() -> FileTree {
    let folder = bitmap_image(
        &[
            "###.......",
            "#..######.",
            "#.......#.",
            "#.......#.",
            "#.......#.",
            "#########.",
        ],
        ACCENTS[0],
    );
    let mut documents = TreeNode::new(1, "Documents", NodeKind::Folder).with_icon(folder.clone());
    documents.open = true;
    let letters = TreeNode::new(2, "Letters", NodeKind::Folder)
        .with_icon(folder.clone())
        .with_children(vec![TreeNode::new(3, "To Ada", NodeKind::Document)]);
    documents.children = vec![
        letters,
        TreeNode::new(4, "Recipes", NodeKind::Document),
        TreeNode::new(5, "To Do", NodeKind::Document),
    ];
    let pictures = TreeNode::new(6, "Pictures", NodeKind::Folder)
        .with_icon(folder)
        .with_children(vec![TreeNode::new(7, "Beach", NodeKind::Document)]);
    FileTree::new(vec![
        documents,
        pictures,
        TreeNode::new(8, "Read Me", NodeKind::Document),
    ])
}

fn gallery_icons() -> Vec<IconItem> {
    [
        ("Square", 0x9988dd, 0),
        ("Circle", 0xffcc44, 1),
        ("Diamond", 0x66aa77, 2),
        ("Another Square", 0xdd8888, 0),
    ]
    .map(|(title, color, shape)| {
        let inside = |x: i32, y: i32| match shape {
            0 => (5..27).contains(&x) && (5..27).contains(&y),
            1 => (x - 16).pow(2) + (y - 16).pow(2) < 144,
            _ => (x - 16).abs() + (y - 16).abs() < 13,
        };
        let image = image::RgbaImage::from_fn(32, 32, |x, y| {
            let (x, y) = (x as i32, y as i32);
            if !inside(x, y) {
                return image::Rgba([0, 0, 0, 0]);
            }
            let edge =
                !inside(x - 1, y) || !inside(x + 1, y) || !inside(x, y - 1) || !inside(x, y + 1);
            rgba(if edge { BLACK } else { color })
        });
        let mut selected = image.clone();
        for pixel in selected.pixels_mut() {
            for channel in &mut pixel.0[..3] {
                *channel = (*channel as u16 * 128 / 255) as u8;
            }
        }
        IconItem::from_image(
            title,
            Arc::new(RenderImage::new(vec![image::Frame::new(image)])),
        )
        .selected_image(Arc::new(RenderImage::new(vec![image::Frame::new(
            selected,
        )])))
    })
    .into()
}
fn bitmap_image(rows: &[&str], accent: Accent) -> Arc<RenderImage> {
    let width = rows.iter().map(|row| row.len()).max().unwrap_or(0);
    let image = image::RgbaImage::from_fn(width as u32, rows.len() as u32, |x, y| {
        match rows[y as usize].as_bytes().get(x as usize) {
            Some(b'#') => rgba(BLACK),
            Some(b'w') => rgba(WHITE),
            Some(b'+') => rgba(accent.mid),
            Some(b'o') => rgba(accent.dark),
            _ => image::Rgba([0, 0, 0, 0]),
        }
    });
    Arc::new(RenderImage::new(vec![image::Frame::new(image)]))
}
fn rgba(color: u32) -> image::Rgba<u8> {
    image::Rgba([
        (color & 255) as u8,
        ((color >> 8) & 255) as u8,
        ((color >> 16) & 255) as u8,
        255,
    ])
}

fn focus_sample(theme: Theme) -> impl IntoElement {
    drawing(206., 34., move |pen, _, _| {
        paint_at(pen, 3, 4, 58, 20, |pen| {
            draw_button(pen, "Button", false, false);
            draw_focus_ring(pen, 3, theme.accent.dark);
        });
        paint_at(pen, 76, 8, 12, 12, |pen| {
            draw_checkbox(pen, CheckState::On, false, false);
            draw_focus_ring(pen, 0, theme.accent.dark);
        });
        paint_at(pen, 102, 8, 12, 12, |pen| {
            draw_radio(pen, true, false, false);
            draw_focus_circle(pen, theme.accent.dark);
        });
        paint_at(pen, 132, 3, 70, 30, |list| {
            list.bevel(0, 0, 70, 30, GRAY_7, WHITE);
            let mut rows = Pen::new(68, 28);
            rows.rect(1, 1, 66, 26, WHITE);
            rows.frame(0, 0, 68, 28, BLACK);
            let mut content = Pen::new(68, 28);
            content.rect(1, 1, 66, 16, theme.accent.dark);
            content.text(4, 11, "Focused", Font::Small, WHITE);
            content.text(4, 27, "Row", Font::Small, BLACK);
            content.clip(gpui_platinum::paint::Rect {
                x: 1,
                y: 1,
                width: 66,
                height: 26,
            });
            rows.append(content, 0, 0);
            list.append(rows, 1, 1);
            draw_focus_ring(list, 0, theme.accent.dark);
        });
    })
}
fn paint_at(pen: &mut Pen, x: i32, y: i32, width: i32, height: i32, draw: impl FnOnce(&mut Pen)) {
    let mut child = Pen::new(width, height);
    draw(&mut child);
    pen.append(child, x, y);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn return_opens_and_dismisses_the_default_alert(cx: &mut TestAppContext) {
        cx.update(gpui_platinum::init);
        let (gallery, cx) = cx.add_window_view(|window, cx| {
            let gallery = Gallery::new(0, cx);
            window.focus(&gallery.focus, cx);
            gallery
        });
        cx.simulate_keystrokes("enter");
        cx.read(|cx| assert!(gallery.read(cx).alert.read(cx).is_open()));
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        cx.read(|cx| assert!(!gallery.read(cx).alert.read(cx).is_open()));
    }

    #[gpui::test]
    fn page_shortcuts_render_each_page_and_keep_the_text_edit(cx: &mut TestAppContext) {
        cx.update(gpui_platinum::init);
        let (gallery, cx) = cx.add_window_view(|window, cx| {
            let gallery = Gallery::new(0, cx);
            window.focus(&gallery.focus, cx);
            gallery
        });
        gallery.update(cx, |gallery, cx| {
            gallery
                .name
                .update(cx, |field, cx| field.set_value("My Folder", cx))
        });
        for page in [3, 1, 4, 2, 0] {
            cx.simulate_keystrokes(&format!("cmd-{}", page + 1));
            cx.read(|cx| assert_eq!(gallery.read(cx).page, page));
        }
        cx.read(|cx| assert_eq!(gallery.read(cx).name.read(cx).value(), "My Folder"));
        cx.read(|cx| {
            assert_eq!(
                gallery
                    .read(cx)
                    .rich
                    .read(cx)
                    .editor()
                    .read(cx)
                    .alignments(),
                [Alignment::Center, Alignment::Left, Alignment::Right]
            );
        });
    }

    #[gpui::test]
    fn calendar_and_date_picker_share_one_chosen_date(cx: &mut TestAppContext) {
        let gallery = cx.update(|cx| cx.new(|cx| Gallery::new(3, cx)));
        let chosen = Date::new(2026, 11, 5).unwrap();
        cx.update(|cx| {
            gallery.update(cx, |gallery, cx| {
                gallery
                    .calendar
                    .update(cx, |calendar, cx| calendar.select(chosen, cx))
            })
        });
        cx.read(|cx| assert_eq!(gallery.read(cx).date.read(cx).value(), chosen));
    }
}

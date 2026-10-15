use gpui::{prelude::*, *};
use gpui_platinum::{Font, InputEvent, Menu, MenuItem, theme::*, *};

actions!(controls, [NextFocus, PreviousFocus, Quit]);

const PAGES: &[&str] = &[
    "Button",
    "Checkbox",
    "Radio, RadioGroup",
    "Slider",
    "ProgressBar",
    "Disclosure",
    "GroupBox, Separator",
    "TabControl",
    "PopupMenu",
    "TextField, TextArea",
    "ListBox",
    "ScrollBar",
    "Window",
    "MenuBar",
    "Alert",
    "Dialog",
    "Label, TextIn, Paragraph",
    "BevelButton, IconButton",
    "Segmented",
    "Toolbar, ToolbarSeparator",
    "Stepper, NumberField",
    "SearchField",
    "ChasingArrows",
    "Placard, WindowHeader",
    "Badge, Tag",
    "ImageWell",
    "Balloon",
    "ContextMenu",
    "TreeView",
    "Table",
    "IconView",
    "Split, SplitVertical, ScrollView",
    "Calendar",
    "DatePicker, TimeField",
    "Combobox",
    "ColorWell",
    "Sheet, FormulaBar",
    "FindReplace",
    "Ruler",
    "ColumnView",
    "DocumentTabs",
    "FontPicker",
    "PageSetup",
    "ProgressDialog",
    "Banner",
    "Toasts",
    "EmptyState, Card",
    "PathPopup, Breadcrumbs",
    "Pagination",
    "StatusBar",
    "RichTextArea, FormatBar, SizeMenu",
    "PopupWindow, SpringOpen",
    "OpenDialog, SaveDialog",
    "ZoomRects",
];

pub struct Controls {
    focus: FocusHandle,
    balloon_focus: FocusHandle,
    page: usize,
    checks: [CheckState; 3],
    radio: usize,
    open: bool,
    tab: usize,
    toggle: bool,
    segment: usize,
    alert: Entity<Alert>,
    dialog: Entity<Dialog>,
    dialog_content: Entity<DemoDialog>,
    find: Entity<FindReplace>,
    find_target: Entity<TextField>,
    page_setup: Entity<PageSetup>,
    progress: Entity<ProgressDialog>,
    banner: Entity<Banner>,
    toasts: Entity<Toasts>,
    open_dialog: Entity<OpenDialog>,
    save_dialog: Entity<SaveDialog>,
    theme: Theme,
    language: Language,
    menu: Entity<MenuBar>,
    bevel_menu: Entity<PopupMenu>,
    context_menu: Entity<ContextMenu>,
    stepper: Entity<Stepper>,
    steps: i32,
    number: Entity<NumberField>,
    search: Entity<SearchField>,
    combo: Entity<Combobox>,
    date_picker: Entity<DatePicker>,
    time_field: Entity<TimeField>,
    color: Entity<ColorWell>,
    font_picker: Entity<FontPicker>,
    icons: Entity<IconView>,
    columns: Entity<ColumnView>,
    ruler: Entity<Ruler>,
    documents: Entity<DocumentTabs>,
    path: Entity<PathPopup>,
    breadcrumbs: Entity<Breadcrumbs>,
    pagination: Entity<Pagination>,
    split: f32,
    split_vertical: f32,
    scroll_view: ScrollHandle,
    dock: Entity<PopupWindow>,
    dock_detached: Option<Entity<PlatinumWindow>>,
    dock_content: Entity<DockContent>,
    zoom_start: Option<std::time::Instant>,
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
    sheet: Entity<SheetView>,
    rich: Entity<RichTextArea>,
}

impl Focusable for Controls {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Controls {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|cx| {
            ListBox::new(PAGES.iter().copied(), cx)
                .label("Controls")
                .size(190, 420)
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
                    if let Some(window) = this.windows.last() {
                        window.update(cx, |window, cx| window.set_active(true, cx));
                    }
                }
                cx.notify();
            })
            .detach();
        }
        let text = cx.new(|cx| {
            TextField::new(cx)
                .placeholder("Untitled")
                .label("Name")
                .size(220., 22.)
        });
        let password = cx.new(|cx| {
            TextField::new(cx)
                .password(true)
                .label("Password")
                .size(220., 22.)
        });
        cx.subscribe(&text, |this, field, event, cx| {
            if *event == InputEvent::Changed {
                let value = field.read(cx).value().to_owned();
                this.password
                    .update(cx, |field, cx| field.set_value(value, cx));
            }
        })
        .detach();
        cx.subscribe(&password, |this, field, event, cx| {
            if *event == InputEvent::Changed {
                let value = field.read(cx).value().to_owned();
                this.text.update(cx, |field, cx| field.set_value(value, cx));
            }
        })
        .detach();
        let stepper = cx.new(|cx| Stepper::new(cx).label("Value"));
        cx.subscribe(&stepper, |this, _, step: &Step, cx| {
            this.steps = (this.steps + step.0).clamp(0, 10);
            cx.notify();
        })
        .detach();
        let documents = cx.new(|cx| {
            DocumentTabs::new(vec!["Letter".into(), "Recipes".into(), "To Do".into()], cx)
                .size(300, 90)
        });
        let dialog_content = cx.new(|cx| DemoDialog {
            checked: true,
            focus: cx.focus_handle().tab_stop(true),
            language: Language::English,
            theme: Theme::default(),
        });
        let focus = dialog_content.read(cx).focus.clone();
        let dialog = cx.new(|cx| {
            Dialog::new("Window", dialog_content.clone(), cx)
                .width(260.)
                .focus_handles(vec![focus])
        });
        let area = cx.new(|cx| {
            TextField::new(cx)
                .multiline(true)
                .label("Note")
                .size(220., 70.)
        });
        let find_target = cx.new(|cx| {
            TextField::new(cx)
                .multiline(true)
                .label("Text")
                .with_text(locale::desk(Language::English, "catText"))
                .size(300., 110.)
        });
        let find = cx.new(|cx| FindReplace::new(find_target.clone(), cx));
        cx.subscribe(&find, |this, find, event, cx| {
            let note = match event {
                FindEvent::Found(_) => String::new(),
                FindEvent::NotFound => this.say("notFound"),
                FindEvent::Replaced(count) => {
                    this.say("replaced").replace("{count}", &count.to_string())
                }
            };
            find.update(cx, |find, cx| find.set_note(note, cx));
        })
        .detach();
        let files = cx.new(|_| models::FileTree::sample());
        let dock_content = cx.new(|_| DockContent(Language::English));
        let dock = cx.new(|cx| {
            PopupWindow::new("Documents", cx)
                .size(180, 80)
                .content(dock_content.clone())
        });
        cx.subscribe(&dock, |this, _, event, cx| {
            if *event == PopupWindowEvent::TornOff && this.dock_detached.is_none() {
                let frame = cx.new(|cx| {
                    PlatinumWindow::new(this.say("documents"), cx)
                        .position(8., 4.)
                        .size(180., 106.)
                        .theme(this.theme)
                        .language(this.language)
                        .content(this.dock_content.clone())
                });
                cx.subscribe(&frame, |this, _, event, cx| {
                    if *event == WindowEvent::Closed {
                        this.dock_detached = None;
                        this.dock.update(cx, |dock, cx| dock.set_open(true, cx));
                        cx.notify();
                    }
                })
                .detach();
                this.dock_detached = Some(frame);
                cx.notify();
            }
        })
        .detach();
        let mut controls = Self {
            focus: cx.focus_handle(),
            balloon_focus: cx.focus_handle().tab_stop(true),
            page: 0,
            checks: [CheckState::On, CheckState::Off, CheckState::Mixed],
            radio: 1,
            open: true,
            tab: 0,
            toggle: true,
            segment: 1,
            alert: cx.new(|cx| {
                Alert::new(
                    AlertKind::Note,
                    "Something needs your attention.",
                    "The quick brown fox jumps over the lazy dog.",
                    ["Cancel", "OK"],
                    cx,
                )
            }),
            dialog,
            dialog_content,
            find,
            find_target,
            page_setup: cx.new(|cx| PageSetup::new(PageSetupState::default(), cx)),
            progress: cx.new(|cx| {
                ProgressDialog::new("Window", "Something needs your attention.", cx)
                    .progress(0.4)
                    .detail("The quick brown fox jumps over the lazy dog.")
            }),
            banner: cx
                .new(|cx| Banner::new(AlertKind::Note, "Something needs your attention.", cx)),
            toasts: cx.new(Toasts::new),
            open_dialog: cx.new(|cx| OpenDialog::new(files.clone(), cx)),
            save_dialog: cx.new(|cx| SaveDialog::new(files.clone(), cx)),
            theme: Theme::default(),
            language: Language::English,
            bevel_menu: cx.new(|cx| PopupMenu::new(["One", "Two"], cx).width(100)),
            menu: cx.new(|cx| {
                MenuBar::new(
                    vec![
                        Menu::new(
                            "File",
                            vec![
                                MenuItem::new("Open").shortcut("o"),
                                MenuItem::separator(),
                                MenuItem::new("Save").shortcut("s").disabled(true),
                            ],
                        ),
                        Menu::new(
                            "Edit",
                            vec![
                                MenuItem::new("Copy").shortcut("c"),
                                MenuItem::new("Paste").shortcut("v"),
                            ],
                        ),
                    ],
                    cx,
                )
            }),
            context_menu: cx.new(|cx| {
                ContextMenu::new(
                    vec![
                        MenuItem::new("Cut"),
                        MenuItem::new("Copy"),
                        MenuItem::new("Paste").disabled(true),
                    ],
                    cx,
                )
                .label("Right-click or Control-click here")
            }),
            stepper,
            steps: 4,
            number: cx.new(|cx| NumberField::new(2., 1., 99., 1., cx)),
            search: cx.new(SearchField::new),
            combo: cx.new(|cx| Combobox::new(["Slow", "Medium", "Fast"], cx)),
            date_picker: cx.new(|cx| DatePicker::new(models::Date::today_utc(), cx)),
            time_field: cx.new(|cx| TimeField::new(12, 30, 0, cx)),
            color: cx.new(|cx| ColorWell::new(ACCENTS[0].mid, cx)),
            font_picker: cx.new(|cx| FontPicker::new(cx).families(["Sans", "Serif", "Mono"])),
            icons: cx.new(|cx| {
                IconView::new(
                    vec![
                        IconItem::new("Calculator", IconKind::Calculator),
                        IconItem::new("Jotter", IconKind::Jotter),
                        IconItem::new("Scribe", IconKind::Scribe),
                        IconItem::new("Puzzle", IconKind::Puzzle),
                    ],
                    cx,
                )
                .size(300, 130)
                .selected(1)
            }),
            columns: cx.new(|cx| ColumnView::new(models::FileTree::sample(), cx).size(440, 160)),
            ruler: cx.new(|cx| {
                Ruler::new(
                    RulerState {
                        left: 9,
                        right: 270,
                        first: 0,
                    },
                    cx,
                )
                .width(300)
            }),
            documents,
            path: cx.new(|cx| {
                PathPopup::new(
                    vec!["Platinum Disk".into(), "Documents".into(), "Letter".into()],
                    cx,
                )
            }),
            breadcrumbs: cx.new(|cx| {
                Breadcrumbs::new(
                    vec!["Platinum Disk".into(), "Documents".into(), "Letter".into()],
                    cx,
                )
            }),
            pagination: cx.new(|cx| Pagination::new(2, 5, cx)),
            split: 120.,
            split_vertical: 40.,
            scroll_view: ScrollHandle::new(),
            dock,
            dock_content,
            dock_detached: None,
            zoom_start: None,
            sidebar,
            slider: 40.,
            scroll: 0.,
            popup: cx.new(|cx| PopupMenu::new(["One", "Two", "Three"], cx).label("Sort by")),
            disabled_popup: cx.new(|cx| PopupMenu::new(["One", "Two", "Three"], cx).disabled(true)),
            text,
            password,
            area,
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
            sheet: cx.new(|cx| {
                SheetView::new(models::Sheet::ledger(), cx)
                    .size(440, 250)
                    .dimensions(20, 6)
            }),
            rich: cx.new(|cx| {
                RichTextArea::new(cx)
                    .with_text("One\nThe quick brown fox jumps over the lazy dog.")
                    .size(440., 220.)
            }),
        };
        controls.set_theme(Theme::default(), cx);
        controls.set_language(Language::English, cx);
        controls
    }

    fn say(&self, key: &str) -> String {
        locale::desk(self.language, key)
    }
    fn item_count(&self) -> String {
        locale::format_message(&self.say("items"), &[("count", "3".into())])
    }
    fn available(&self) -> String {
        let size = locale::format_message(&self.say("sizeMB"), &[("size", "47.7".into())]);
        locale::format_message(&self.say("available"), &[("size", size)])
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        self.sidebar
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.popup
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.disabled_popup
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.text
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.password
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.area
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.list
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.tree
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.table
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.calendar
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.sheet
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.rich
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.menu
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.bevel_menu
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.context_menu
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.stepper
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.number
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.search
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.combo
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.date_picker
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.time_field
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.color
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.font_picker
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.icons
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.columns
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.ruler
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.documents
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.path
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.breadcrumbs
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.pagination
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.dock
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        if let Some(frame) = &self.dock_detached {
            frame.update(cx, |frame, cx| frame.set_theme(theme, cx));
        }
        for window in &self.windows {
            window.update(cx, |window, cx| window.set_theme(theme, cx));
        }
        self.alert
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.dialog
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.find
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.find_target
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.page_setup
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.progress
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.banner
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.toasts
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.open_dialog
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        self.save_dialog
            .update(cx, |widget, cx| widget.set_theme(theme, cx));
        cx.notify();
    }

    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        self.text
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.password
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.area
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.tree
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.table
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.calendar
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.sheet
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.rich
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.number
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.search
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.combo
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.date_picker
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.time_field
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.color
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.font_picker
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.icons
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.columns
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.ruler
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.documents
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.path
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.breadcrumbs
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.pagination
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.alert
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.dialog
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.find
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.find_target.update(cx, |field, cx| {
            let sample = locale::LANGUAGES
                .iter()
                .any(|language| field.value() == locale::desk(*language, "catText"));
            field.set_language(language, cx);
            field.set_label(locale::desk(language, "document"), cx);
            if sample {
                field.set_value(locale::desk(language, "catText"), cx);
            }
        });
        self.page_setup
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.progress
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.banner
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.toasts
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.open_dialog
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.save_dialog
            .update(cx, |widget, cx| widget.set_language(language, cx));
        self.dialog_content.update(cx, |content, cx| {
            content.language = language;
            cx.notify();
        });
        for menu in [&self.popup, &self.disabled_popup, &self.bevel_menu] {
            menu.update(cx, |menu, cx| {
                menu.set_language(language, cx);
                menu.set_items(
                    ["catOne", "catTwo", "catThree"]
                        .map(|key| MenuItem::new(locale::desk(language, key)))
                        .into(),
                    cx,
                );
            });
        }
        self.sidebar.update(cx, |list, cx| {
            list.set_language(language, cx);
            list.set_label(locale::desk(language, "controls"), cx);
        });
        self.list.update(cx, |list, cx| {
            list.set_language(language, cx);
            list.set_items(
                ["letter", "recipes", "toDo", "readMe", "untitled"]
                    .map(|key| ListItem::from(locale::desk(language, key)))
                    .into(),
                cx,
            );
        });
        self.text.update(cx, |field, cx| {
            field.set_placeholder(locale::desk(language, "untitled"), cx)
        });
        self.combo.update(cx, |combo, cx| {
            combo.set_options(
                ["slow", "medium", "fast"]
                    .map(|key| locale::desk(language, key))
                    .into(),
                cx,
            )
        });
        self.menu.update(cx, |menu, cx| {
            menu.set_language(language, cx);
            menu.set_menus(
                vec![
                    Menu::new(
                        locale::desk(language, "menuFile"),
                        vec![
                            MenuItem::new(locale::desk(language, "open")).shortcut("o"),
                            MenuItem::separator(),
                            MenuItem::new(locale::desk(language, "save"))
                                .shortcut("s")
                                .disabled(true),
                        ],
                    ),
                    Menu::new(
                        locale::desk(language, "menuEdit"),
                        vec![
                            MenuItem::new(locale::desk(language, "copy")).shortcut("c"),
                            MenuItem::new(locale::desk(language, "paste")).shortcut("v"),
                        ],
                    ),
                ],
                cx,
            );
        });
        self.context_menu.update(cx, |menu, cx| {
            menu.set_language(language, cx);
            menu.set_label(locale::desk(language, "catContext"), cx);
            menu.set_items(
                vec![
                    MenuItem::new(locale::desk(language, "cut")),
                    MenuItem::new(locale::desk(language, "copy")),
                    MenuItem::new(locale::desk(language, "paste")).disabled(true),
                ],
                cx,
            );
        });
        self.icons.update(cx, |view, cx| {
            view.set_items(
                [
                    ("calculator", IconKind::Calculator),
                    ("jotter", IconKind::Jotter),
                    ("scribe", IconKind::Scribe),
                    ("puzzle", IconKind::Puzzle),
                ]
                .map(|(key, kind)| IconItem::new(locale::desk(language, key), kind))
                .into(),
                cx,
            )
        });
        self.path.update(cx, |path, cx| {
            path.set_path(
                ["platinumDisk", "documents", "letter"]
                    .map(|key| locale::desk(language, key))
                    .into(),
                cx,
            )
        });
        self.breadcrumbs.update(cx, |path, cx| {
            path.set_path(
                ["platinumDisk", "documents", "letter"]
                    .map(|key| locale::desk(language, key))
                    .into(),
                cx,
            )
        });
        self.documents.update(cx, |tabs, cx| {
            tabs.set_titles(
                ["letter", "recipes", "toDo"]
                    .map(|key| locale::desk(language, key))
                    .into(),
                cx,
            );
            tabs.set_content(
                move |index, _, _| {
                    text_in_language(
                        locale::desk(
                            language,
                            ["letter", "recipes", "toDo"]
                                .get(index)
                                .copied()
                                .unwrap_or("untitled"),
                        ),
                        Font::Small,
                        BLACK,
                        language,
                    )
                    .into_any_element()
                },
                cx,
            );
        });
        self.alert.update(cx, |alert, cx| {
            alert.set_message(
                locale::desk(language, "catMessage"),
                locale::desk(language, "catText"),
                cx,
            );
            alert.set_buttons(
                ["cancel", "ok"]
                    .map(|key| locale::desk(language, key).into())
                    .into(),
                cx,
            );
        });
        self.dialog.update(cx, |dialog, cx| {
            dialog.set_title(locale::desk(language, "catWindow"), cx)
        });
        self.progress.update(cx, |progress, cx| {
            progress.set_message(
                locale::desk(language, "catWindow"),
                locale::desk(language, "catMessage"),
                cx,
            );
            progress.set_progress(0.4, locale::desk(language, "catText"), cx);
        });
        self.banner.update(cx, |banner, cx| {
            banner.set_text(locale::desk(language, "catMessage"), cx)
        });
        self.dock.update(cx, |dock, cx| {
            dock.set_language(language, cx);
            dock.set_title(locale::desk(language, "documents"), cx);
        });
        if let Some(frame) = &self.dock_detached {
            frame.update(cx, |frame, cx| {
                frame.set_language(language, cx);
                frame.set_title(locale::desk(language, "documents"), cx);
            });
        }
        self.dock_content.update(cx, |content, cx| {
            content.0 = language;
            cx.notify();
        });
        if let Some(frame) = &self.dock_detached {
            frame.update(cx, |frame, cx| {
                frame.set_language(language, cx);
                frame.set_title(locale::desk(language, "documents"), cx);
            });
        }
        self.tree.update(cx, |tree, cx| {
            *tree.model_mut() = sample_tree(language);
            cx.notify();
        });
        self.columns.update(cx, |view, cx| {
            *view.model_mut() = sample_tree(language);
            cx.notify();
        });
        self.table.update(cx, |table, cx| {
            table.set_headers(
                ["item", "cost"]
                    .map(|key| locale::desk(language, key))
                    .into(),
                cx,
            );
            table.set_rows(
                [
                    ("rent", 1200),
                    ("groceries", 340),
                    ("lampOil", 18),
                    ("books", 65),
                ]
                .map(|(key, cost)| vec![locale::desk(language, key), cost.to_string()])
                .into(),
                cx,
            );
        });
        for (index, window) in self.windows.iter().enumerate() {
            window.update(cx, |window, cx| {
                window.set_language(language, cx);
                window.set_title(
                    locale::desk(language, if index == 0 { "untitled" } else { "catWindow" }),
                    cx,
                );
            });
        }
        cx.notify();
    }

    fn page(&self, cx: &mut Context<Self>) -> AnyElement {
        let column = div().flex().flex_col().gap(px(8.)).items_start();
        match PAGES[self.page] {
            "Button" => column
                .child(
                    row()
                        .child(
                            button("cancel", self.say("cancel"))
                                .cancel_button()
                                .theme(self.theme)
                                .language(self.language)
                                .on_click(|_, _, _| {}),
                        )
                        .child(
                            button("default", self.say("ok"))
                                .theme(self.theme)
                                .language(self.language)
                                .default_button()
                                .on_click(|_, _, _| {}),
                        ),
                )
                .child(
                    button("disabled", self.say("catDisabled"))
                        .theme(self.theme)
                        .language(self.language)
                        .disabled(true),
                )
                .into_any_element(),
            "Checkbox" => column
                .children(
                    [self.say("catOne"), self.say("catTwo"), self.say("catThree")]
                        .iter()
                        .enumerate()
                        .map(|(i, text)| {
                            checkbox(("check", i), text.clone(), self.checks[i])
                                .theme(self.theme)
                                .language(self.language)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.checks[i] = if this.checks[i] == CheckState::On {
                                        CheckState::Off
                                    } else {
                                        CheckState::On
                                    };
                                    cx.notify();
                                }))
                        }),
                )
                .child(
                    checkbox("disabled", self.say("catDisabled"), CheckState::Off)
                        .theme(self.theme)
                        .language(self.language)
                        .disabled(true),
                )
                .into_any_element(),
            "Radio, RadioGroup" => {
                let entity = cx.entity();
                column
                    .child(
                        ChoiceGroup::radio(
                            "radio-group",
                            self.radio,
                            [
                                ChoiceItem::new(self.say("catOne")),
                                ChoiceItem::new(self.say("catTwo")),
                                ChoiceItem::new(self.say("catDisabled")).disabled(true),
                            ],
                        )
                        .theme(self.theme)
                        .language(self.language)
                        .on_change(move |value, _, cx| {
                            entity.update(cx, |this, cx| {
                                this.radio = value;
                                cx.notify();
                            })
                        }),
                    )
                    .into_any_element()
            }
            "Disclosure" => column
                .child(
                    disclosure("details", self.say("catOne"), self.open)
                        .theme(self.theme)
                        .language(self.language)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open = !this.open;
                            cx.notify();
                        })),
                )
                .when(self.open, |el| {
                    el.child(text_in_language(
                        self.say("catText"),
                        Font::Small,
                        BLACK,
                        self.language,
                    ))
                })
                .into_any_element(),
            "GroupBox, Separator" => {
                column
                    .child(group_box_in(self.say("catOne"), 240, self.language).child(
                        paragraph_in(self.say("catText"), Font::Small, 214, self.language),
                    ))
                    .child(separator(240))
                    .into_any_element()
            }
            "TabControl" => {
                let entity = cx.entity();
                column
                    .child(
                        ChoiceGroup::tabs(
                            "tabs",
                            self.tab,
                            [self.say("catOne"), self.say("catTwo"), self.say("catThree")],
                        )
                        .width(90)
                        .theme(self.theme)
                        .language(self.language)
                        .on_change(move |value, _, cx| {
                            entity.update(cx, |this, cx| {
                                this.tab = value;
                                cx.notify();
                            })
                        }),
                    )
                    .child(text_in_language(
                        [self.say("catOne"), self.say("catTwo"), self.say("catThree")][self.tab]
                            .clone(),
                        Font::Small,
                        BLACK,
                        self.language,
                    ))
                    .into_any_element()
            }
            "BevelButton, IconButton" => column
                .child(
                    row()
                        .child(
                            bevel_button("one", self.say("catOne"))
                                .theme(self.theme)
                                .language(self.language)
                                .on_click(|_, _, _| {}),
                        )
                        .child(
                            bevel_button("two", self.say("catTwo"))
                                .theme(self.theme)
                                .language(self.language)
                                .selected(self.toggle)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle = !this.toggle;
                                    cx.notify();
                                })),
                        )
                        .child(
                            bevel_button("disabled", self.say("catDisabled"))
                                .theme(self.theme)
                                .language(self.language)
                                .disabled(true),
                        ),
                )
                .child(
                    row()
                        .child(self.bevel_menu.clone())
                        .child(
                            icon_button("bold", ICON_BOLD, self.say("bold"))
                                .theme(self.theme)
                                .language(self.language),
                        )
                        .child(
                            icon_button("italic", ICON_ITALIC, self.say("italic"))
                                .theme(self.theme)
                                .language(self.language),
                        ),
                )
                .into_any_element(),
            "Segmented" => {
                let entity = cx.entity();
                column
                    .child(
                        ChoiceGroup::segments(
                            "segments",
                            self.segment,
                            [self.say("catOne"), self.say("catTwo"), self.say("catThree")],
                        )
                        .theme(self.theme)
                        .language(self.language)
                        .on_change(move |value, _, cx| {
                            entity.update(cx, |this, cx| {
                                this.segment = value;
                                cx.notify();
                            })
                        }),
                    )
                    .into_any_element()
            }
            "ProgressBar" => column
                .child(progress_bar(0.62, 220, self.theme))
                .child(indeterminate_progress(220, self.theme))
                .into_any_element(),
            "Label, TextIn, Paragraph" => column
                .child(label_in(self.say("catText"), self.language))
                .child(text_in_language(
                    self.say("catText"),
                    Font::Small,
                    BLACK,
                    self.language,
                ))
                .child(paragraph_in(
                    format!("{} {}", self.say("catText"), self.say("catText")),
                    Font::Small,
                    240,
                    self.language,
                ))
                .into_any_element(),
            "Slider" => {
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
                            .label(self.say("catDisabled")),
                    )
                    .into_any_element()
            }
            "PopupMenu" => column
                .child(self.popup.clone())
                .child(self.disabled_popup.clone())
                .into_any_element(),
            "TextField, TextArea" => column
                .child(self.text.clone())
                .child(self.password.clone())
                .child(self.area.clone())
                .into_any_element(),
            "ListBox" => column.child(self.list.clone()).into_any_element(),
            "ScrollBar" => {
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
            "TreeView" => column.child(self.tree.clone()).into_any_element(),
            "Table" => column.child(self.table.clone()).into_any_element(),
            "Calendar" => column.child(self.calendar.clone()).into_any_element(),
            "Window" => div()
                .relative()
                .w(px(460.))
                .h(px(290.))
                .overflow_hidden()
                .bg(rgb(GRAY_5))
                .children(self.windows.iter().cloned())
                .into_any_element(),
            "Alert" => column
                .child(
                    button("show-alert", self.say("catShow"))
                        .theme(self.theme)
                        .language(self.language)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.alert.update(cx, |alert, cx| alert.show(window, cx))
                        })),
                )
                .into_any_element(),
            "Dialog" => column
                .child(
                    button("show-dialog", self.say("catShow"))
                        .theme(self.theme)
                        .language(self.language)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.dialog.update(cx, |dialog, cx| dialog.show(window, cx))
                        })),
                )
                .into_any_element(),
            "Sheet, FormulaBar" => column.child(self.sheet.clone()).into_any_element(),
            "RichTextArea, FormatBar, SizeMenu" => {
                column.child(self.rich.clone()).into_any_element()
            }
            "ChasingArrows" => column.child(chasing_arrows()).into_any_element(),
            "Placard, WindowHeader" => column
                .child(placard_in(self.item_count(), 110, self.language))
                .child(window_header_parts(
                    self.item_count(),
                    self.available(),
                    300,
                    self.language,
                ))
                .into_any_element(),
            "Badge, Tag" => column
                .child(
                    row()
                        .child(badge_in("3", self.theme, self.language))
                        .child(tag_in(self.say("catOne"), self.language))
                        .child(tag_in(self.say("catTwo"), self.language)),
                )
                .into_any_element(),
            "ImageWell" => column
                .child(paint::drawing(48., 48., |pen, _, _| {
                    pen.sunken();
                    draw_icon(pen, 8, 8, IconKind::Puzzle, false);
                }))
                .into_any_element(),
            "Balloon" => column
                .child(
                    Balloon::new(
                        "help",
                        self.say("catText"),
                        button("one", self.say("catOne"))
                            .focus_handle(&self.balloon_focus)
                            .language(self.language),
                    )
                    .anchor_focus(&self.balloon_focus)
                    .language(self.language),
                )
                .into_any_element(),
            "EmptyState, Card" => column
                .child(
                    div()
                        .w(px(260.))
                        .h(px(80.))
                        .border_1()
                        .border_color(rgb(GRAY_7))
                        .bg(rgb(WHITE))
                        .flex()
                        .justify_center()
                        .items_center()
                        .child(label_in(self.say("catEmpty"), self.language)),
                )
                .into_any_element(),
            "StatusBar" => column
                .child(status_bar_in(
                    [self.item_count(), self.available()],
                    300,
                    self.language,
                ))
                .into_any_element(),
            "MenuBar" => column
                .child(div().w(px(300.)).child(self.menu.clone()))
                .into_any_element(),
            "Toolbar, ToolbarSeparator" => column
                .child(
                    Toolbar::new()
                        .theme(self.theme)
                        .language(self.language)
                        .item(
                            icon_button("save-tool", ICON_SAVE, self.say("save"))
                                .theme(self.theme)
                                .language(self.language),
                        )
                        .separator()
                        .item(
                            icon_button("bold-tool", ICON_BOLD, self.say("bold"))
                                .theme(self.theme)
                                .language(self.language)
                                .selected(self.toggle)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle = !this.toggle;
                                    cx.notify();
                                })),
                        )
                        .item(
                            icon_button("italic-tool", ICON_ITALIC, self.say("italic"))
                                .theme(self.theme)
                                .language(self.language),
                        )
                        .separator()
                        .item(
                            bevel_button("align-tool", self.say("catThree"))
                                .icon(ICON_ALIGN_LEFT)
                                .theme(self.theme)
                                .language(self.language),
                        ),
                )
                .into_any_element(),
            "Stepper, NumberField" => column
                .child(row().child(self.stepper.clone()).child(text_in_language(
                    self.steps.to_string(),
                    Font::Small,
                    BLACK,
                    self.language,
                )))
                .child(self.number.clone())
                .into_any_element(),
            "SearchField" => column.child(self.search.clone()).into_any_element(),
            "ContextMenu" => column.child(self.context_menu.clone()).into_any_element(),
            "IconView" => column.child(self.icons.clone()).into_any_element(),
            "Split, SplitVertical, ScrollView" => {
                let entity = cx.entity();
                let vertical = cx.entity();
                let lines = div().flex().flex_col().children((1..=12).map(|i| {
                    text_in_language(
                        format!("{i}. {}", self.say("catText")),
                        Font::Small,
                        BLACK,
                        self.language,
                    )
                }));
                column
                    .child(
                        Split::new(
                            "split",
                            self.split,
                            div()
                                .p(px(6.))
                                .child(label_in(self.say("catOne"), self.language)),
                            ScrollView::new("scroll-view", &self.scroll_view, lines)
                                .size((300. - self.split - 7.) as i32, 120),
                        )
                        .size(300, 120)
                        .on_change(move |position, _, cx| {
                            entity.update(cx, |this, cx| {
                                this.split = position;
                                cx.notify();
                            })
                        }),
                    )
                    .child(
                        Split::new(
                            "split-vertical",
                            self.split_vertical,
                            label_in(self.say("catOne"), self.language),
                            label_in(self.say("catTwo"), self.language),
                        )
                        .vertical()
                        .size(300, 100)
                        .minimum(20.)
                        .on_change(move |position, _, cx| {
                            vertical.update(cx, |this, cx| {
                                this.split_vertical = position;
                                cx.notify();
                            })
                        }),
                    )
                    .into_any_element()
            }
            "DatePicker, TimeField" => column
                .child(self.date_picker.clone())
                .child(self.time_field.clone())
                .into_any_element(),
            "Combobox" => column.child(self.combo.clone()).into_any_element(),
            "ColorWell" => column.child(self.color.clone()).into_any_element(),
            "Ruler" => column.child(self.ruler.clone()).into_any_element(),
            "ColumnView" => column.child(self.columns.clone()).into_any_element(),
            "DocumentTabs" => column.child(self.documents.clone()).into_any_element(),
            "FontPicker" => column.child(self.font_picker.clone()).into_any_element(),
            "PathPopup, Breadcrumbs" => column
                .child(self.path.clone())
                .child(self.breadcrumbs.clone())
                .into_any_element(),
            "Pagination" => column.child(self.pagination.clone()).into_any_element(),
            "PopupWindow, SpringOpen" => div()
                .id("popup-demo")
                .relative()
                .w(px(300.))
                .h(px(130.))
                .bg(rgb(GRAY_5))
                .overflow_hidden()
                .on_mouse_down(MouseButton::Left, |_, _, _| {})
                .child(if let Some(frame) = &self.dock_detached {
                    frame.clone().into_any_element()
                } else {
                    self.dock.clone().into_any_element()
                })
                .into_any_element(),
            "ZoomRects" => column
                .child(
                    button("show-zoom", self.say("catShow"))
                        .theme(self.theme)
                        .language(self.language)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.zoom_start = Some(std::time::Instant::now());
                            cx.notify();
                        })),
                )
                .when_some(self.zoom_start, |column, start| {
                    column.child(zoom_rects(
                        paint::Rect {
                            x: 40,
                            y: 0,
                            width: 60,
                            height: 20,
                        },
                        paint::Rect {
                            x: 0,
                            y: 30,
                            width: 280,
                            height: 160,
                        },
                        start,
                        300,
                        220,
                    ))
                })
                .into_any_element(),
            "FindReplace" => column
                .child(
                    button("show-find", self.say("catShow"))
                        .theme(self.theme)
                        .language(self.language)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.find.update(cx, |find, cx| find.show(window, cx))
                        })),
                )
                .child(self.find_target.clone())
                .into_any_element(),
            "PageSetup" => column
                .child(
                    button("show-page", self.say("catShow"))
                        .theme(self.theme)
                        .language(self.language)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.page_setup.update(cx, |page, cx| page.show(window, cx))
                        })),
                )
                .into_any_element(),
            "ProgressDialog" => column
                .child(
                    button("show-progress", self.say("catShow"))
                        .theme(self.theme)
                        .language(self.language)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.progress
                                .update(cx, |progress, cx| progress.show(window, cx))
                        })),
                )
                .into_any_element(),
            "Banner" => column
                .child(self.banner.clone())
                .child(
                    button("show-banner", self.say("catShow"))
                        .theme(self.theme)
                        .language(self.language)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.banner.update(cx, |banner, cx| banner.show(cx))
                        })),
                )
                .into_any_element(),
            "Toasts" => column
                .child(
                    row().children(["catOne", "catTwo", "catThree"].iter().enumerate().map(
                        |(index, key)| {
                            button(("toast", index), self.say(key))
                                .theme(self.theme)
                                .language(self.language)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    let mut toast = Toast::new(this.say("catMessage"));
                                    if index == 1 {
                                        toast =
                                            toast.kind(AlertKind::Caution).action(this.say("ok"));
                                    }
                                    if index == 2 {
                                        toast = toast
                                            .kind(AlertKind::Stop)
                                            .description(this.say("catText"));
                                    }
                                    this.toasts.update(cx, |toasts, cx| {
                                        toasts.push(toast, cx);
                                    });
                                }))
                        },
                    )),
                )
                .into_any_element(),
            "OpenDialog, SaveDialog" => column
                .child(
                    row()
                        .child(
                            button("open", self.say("open"))
                                .theme(self.theme)
                                .language(self.language)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_dialog
                                        .update(cx, |dialog, cx| dialog.show(window, cx))
                                })),
                        )
                        .child(
                            button("save", self.say("saveAsMenu"))
                                .theme(self.theme)
                                .language(self.language)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    let name = this.say("untitled");
                                    this.save_dialog.update(cx, |dialog, cx| {
                                        dialog.set_name(name, cx);
                                        dialog.show(window, cx);
                                    });
                                })),
                        ),
                )
                .into_any_element(),
            _ => unreachable!("unknown control page"),
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
            .overflow_hidden()
            .bg(rgb(GRAY_2))
            .on_action(|_: &NextFocus, window, cx| window.focus_next(cx))
            .on_action(|_: &PreviousFocus, window, cx| window.focus_prev(cx))
            .on_action(|_: &Quit, _, cx| cx.quit())
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if PAGES[this.page] == "MenuBar"
                    && this
                        .menu
                        .update(cx, |menu, cx| menu.dispatch_shortcut(event, window, cx))
                {
                    cx.stop_propagation();
                }
            }))
            .child(self.sidebar.clone())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .flex_1()
                    .min_w_0()
                    .py(px(12.))
                    .px(px(14.))
                    .overflow_hidden()
                    .child(label(PAGES[self.page]))
                    .child(
                        div()
                            .w_full()
                            .h(px(2.))
                            .bg(rgb(GRAY_7))
                            .border_b_1()
                            .border_color(rgb(WHITE)),
                    )
                    .child(self.page(cx)),
            );
        root = root
            .child(self.alert.clone())
            .child(self.dialog.clone())
            .child(self.find.clone())
            .child(self.page_setup.clone())
            .child(self.progress.clone())
            .child(self.open_dialog.clone())
            .child(self.save_dialog.clone())
            .child(self.toasts.clone());
        button_scope("controls-buttons", root)
    }
}

struct DemoDialog {
    checked: bool,
    focus: FocusHandle,
    language: Language,
    theme: Theme,
}
impl Render for DemoDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        checkbox(
            "one",
            locale::desk(self.language, "catOne"),
            if self.checked {
                CheckState::On
            } else {
                CheckState::Off
            },
        )
        .focus_handle(&self.focus)
        .theme(self.theme)
        .language(self.language)
        .on_click(cx.listener(|this, _, _, cx| {
            this.checked = !this.checked;
            cx.notify();
        }))
    }
}

struct DockContent(Language);
impl Render for DockContent {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(WHITE))
            .p(px(4.))
            .child(text_in_language(
                locale::desk(self.0, "letter"),
                Font::Small,
                BLACK,
                self.0,
            ))
    }
}

fn sample_tree(language: Language) -> models::FileTree {
    fn translate(nodes: &mut [models::TreeNode], language: Language) {
        const KEYS: &[&str] = &[
            "platinumDisk",
            "applications",
            "documents",
            "systemFolder",
            "controlPanels",
            "alarmClock",
            "calculator",
            "controls",
            "jotter",
            "ledger",
            "puzzle",
            "scribe",
            "appearance",
            "dateTime",
            "language",
            "mouse",
            "sound",
            "letter",
            "recipes",
            "toDo",
            "readMe",
        ];
        for node in nodes {
            if let Some(key) = KEYS
                .iter()
                .find(|key| locale::desk(Language::English, key) == node.title)
            {
                node.title = locale::desk(language, key);
            }
            translate(&mut node.children, language);
        }
    }
    let mut model = models::FileTree::sample();
    translate(&mut model.roots, language);
    model
}

//! Menus, pop-ups, dates and times, pickers and the file dialogs, as
//! mygo-platinum's golden tests draw them.

use crate::Scene;
use gpui::{
    AnyWindowHandle, AppContext as _, HeadlessAppContext, Keystroke, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PlatformInput, div, point, prelude::*, px,
};
use gpui_platinum::{
    CalendarView, ColorWell, Combobox, DatePicker, Font, FontChoice, FontPicker, Language, Menu,
    MenuBar, MenuItem, OpenDialog, PageSetup, PageSetupState, PopupMenu, SaveDialog, TimeField,
    menu::draw_popup_button,
    models::{Date, FileTree, NodeKind, TreeNode},
    paint::drawing,
    theme::ACCENTS,
};

pub fn scenes() -> Vec<Scene> {
    vec![
        popup_states(),
        popup_open(),
        context_menu(),
        menubar_open(),
        calendar(),
        datepicker_open(),
        combobox_open(),
        timefield(),
        region("region-en-monday-24", Language::English, 1, 24),
        region(
            "region-zh-hans-sunday-12",
            Language::SimplifiedChinese,
            0,
            12,
        ),
        colorpicker(),
        pagesetup(),
        fontpicker(),
        open_dialog(),
        save_dialog("save-dialog", Language::English),
        save_dialog("save-dialog-zh-hans", Language::SimplifiedChinese),
        save_dialog_replace(),
        save_dialog_replace_zh_hans(),
    ]
}

/// Sends input to the window as the platform would, drawn first so that
/// what it reaches is where the picture has it.
fn send(app: &mut HeadlessAppContext, window: AnyWindowHandle, input: PlatformInput) {
    app.update_window(window, |_, window, cx| {
        let _ = window.draw(cx);
        window.dispatch_event(input, cx);
    })
    .unwrap();
    app.run_until_parked();
}

fn press(app: &mut HeadlessAppContext, window: AnyWindowHandle, x: f32, y: f32) {
    press_button(app, window, MouseButton::Left, x, y);
}

fn press_button(
    app: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    button: MouseButton,
    x: f32,
    y: f32,
) {
    send(
        app,
        window,
        PlatformInput::MouseDown(MouseDownEvent {
            button,
            position: point(px(x), px(y)),
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        }),
    );
}

fn release(app: &mut HeadlessAppContext, window: AnyWindowHandle, x: f32, y: f32) {
    send(
        app,
        window,
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position: point(px(x), px(y)),
            modifiers: Modifiers::default(),
            click_count: 1,
        }),
    );
}

fn move_to(app: &mut HeadlessAppContext, window: AnyWindowHandle, x: f32, y: f32, pressed: bool) {
    send(
        app,
        window,
        PlatformInput::MouseMove(MouseMoveEvent {
            position: point(px(x), px(y)),
            pressed_button: pressed.then_some(MouseButton::Left),
            modifiers: Modifiers::default(),
        }),
    );
}

fn click(app: &mut HeadlessAppContext, window: AnyWindowHandle, x: f32, y: f32) {
    move_to(app, window, x, y, false);
    press(app, window, x, y);
    release(app, window, x, y);
}

/// A click and a second one that the platform counts as a double click.
fn double_click(app: &mut HeadlessAppContext, window: AnyWindowHandle, x: f32, y: f32) {
    click(app, window, x, y);
    send(
        app,
        window,
        PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Left,
            position: point(px(x), px(y)),
            modifiers: Modifiers::default(),
            click_count: 2,
            first_mouse: false,
        }),
    );
    send(
        app,
        window,
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position: point(px(x), px(y)),
            modifiers: Modifiers::default(),
            click_count: 2,
        }),
    );
}

/// Types keys; Tab, unless what has the focus takes it, moves the focus
/// on, as the window of an application does.
fn keys(app: &mut HeadlessAppContext, window: AnyWindowHandle, keys: &[&str]) {
    for key in keys {
        app.update_window(window, |_, window, cx| {
            let _ = window.draw(cx);
            let handled = window.dispatch_keystroke(Keystroke::parse(key).unwrap(), cx);
            match *key {
                "tab" if !handled => window.focus_next(cx),
                "shift-tab" if !handled => window.focus_prev(cx),
                _ => {}
            }
        })
        .unwrap();
        app.run_until_parked();
    }
}

fn popup_states() -> Scene {
    Scene::elements("popup-states", 420., 34., |_, _| {
        div().flex().p(px(6.)).gap(px(6.)).children(
            [
                ("Name", false, false),
                ("Pressed", false, true),
                ("Disabled", true, false),
            ]
            .map(|(text, disabled, pressed)| {
                drawing(120., 20., move |pen, _, _| {
                    draw_popup_button(pen, text, disabled, pressed)
                })
            }),
        )
    })
}

/// "Sort by:" with Size chosen, opened by a click on its arrow.
fn popup_open() -> Scene {
    struct View(gpui::Entity<PopupMenu>);
    impl Render for View {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            div()
                .size_full()
                .bg(gpui::rgb(gpui_platinum::theme::GRAY_2))
                .p(px(20.))
                .child(self.0.clone())
        }
    }
    Scene::new("popup-open", 260., 160., |_, cx| {
        View(cx.new(|cx| {
            PopupMenu::new(
                [
                    MenuItem::new("Name"),
                    MenuItem::new("Date"),
                    MenuItem::separator(),
                    MenuItem::new("Size"),
                ],
                cx,
            )
            .caption("Sort by:")
            .selected(3)
        }))
    })
    .prepare(|app, window| {
        // The button's right end, 10 in and 10 down.
        let right = 20. + Font::Bold.measure("Sort by:") as f32 + 6. + popup_width() as f32;
        click(app, window, right - 10., 30.);
    })
}

fn popup_width() -> i32 {
    9 + ["Name", "Date", "Size"]
        .map(|item| Font::Bold.measure(item))
        .into_iter()
        .max()
        .unwrap()
        + 8
        + 18
}

fn context_menu() -> Scene {
    Scene::new("context-menu", 260., 160., |_, cx| {
        let menu = cx.new(|cx| {
            MenuBar::context_menu(
                vec![
                    MenuItem::new("Rename"),
                    MenuItem::new("Delete").disabled(true),
                ],
                cx,
            )
        });
        Holder(menu)
    })
    .prepare(|app, window| {
        // A right click at the middle of a row of 120 by 30, 10 in.
        press_button(app, window, MouseButton::Right, 70., 25.);
    })
}

struct Holder(gpui::Entity<MenuBar>);
impl Render for Holder {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let menu = self.0.clone();
        div()
            .size_full()
            .bg(gpui::rgb(gpui_platinum::theme::GRAY_2))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |_, event: &MouseDownEvent, window, cx| {
                    menu.update(cx, |menu, cx| menu.show_at(0, event.position, window, cx));
                }),
            )
            .child(self.0.clone())
    }
}

const APP_ICON: &[&str] = &[
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

fn menubar_open() -> Scene {
    Scene::new("menubar-open", 260., 170., |_, cx| {
        Holder(cx.new(|cx| {
            MenuBar::new(
                vec![
                    Menu::new("About", vec![MenuItem::new("About This Desk…")]).icon(APP_ICON),
                    Menu::new(
                        "File",
                        vec![
                            MenuItem::new("New").shortcut("N"),
                            MenuItem::new("Open…").shortcut("O"),
                            MenuItem::separator(),
                            MenuItem::new("Save").shortcut("S"),
                            MenuItem::new("Print…").disabled(true),
                            MenuItem::new("Recent").submenu(vec![
                                MenuItem::new("Notes.txt"),
                                MenuItem::new("Letter.txt"),
                            ]),
                        ],
                    ),
                    Menu::new(
                        "Edit",
                        vec![
                            MenuItem::new("Undo").shortcut("Z"),
                            MenuItem::separator(),
                            MenuItem::new("Wrap Lines").checked(true),
                        ],
                    ),
                ],
                cx,
            )
        }))
    })
    .prepare(|app, window| {
        // Press File and hold it down over Recent.
        press(app, window, 45., 6.);
        move_to(app, window, 45., 112., true);
    })
}

/// Views laid out as the original's ui.Column or ui.Row: padding around,
/// a gap between, on the window's Gray2.
struct Stack {
    row: bool,
    fill: bool,
    padding: f32,
    gap: f32,
    views: Vec<gpui::AnyView>,
}

impl Stack {
    fn column(padding: f32, gap: f32, views: Vec<gpui::AnyView>) -> Self {
        Self {
            row: false,
            fill: true,
            padding,
            gap,
            views,
        }
    }
    fn row(padding: f32, gap: f32, views: Vec<gpui::AnyView>) -> Self {
        Self {
            row: true,
            ..Self::column(padding, gap, views)
        }
    }
    fn inner(mut self) -> Self {
        self.fill = false;
        self
    }
}

impl Render for Stack {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .when(self.fill, |stack| {
                stack
                    .size_full()
                    .bg(gpui::rgb(gpui_platinum::theme::GRAY_2))
            })
            .p(px(self.padding))
            .flex()
            .when(!self.row, |stack| stack.flex_col())
            .items_start()
            .gap(px(self.gap))
            .children(self.views.iter().cloned())
    }
}

/// The original's tests run on a clock stopped in 1970, so no day of the
/// months they show is today.
fn stopped_today() -> Date {
    Date::new(1970, 1, 1).unwrap()
}

fn calendar() -> Scene {
    Scene::new("calendar", 200., 200., |_, cx| {
        let calendar = cx.new(|cx| {
            CalendarView::new(Date::new(2026, 10, 14).unwrap(), cx).today(stopped_today())
        });
        Stack::column(10., 0., vec![calendar.into()])
    })
    .prepare(|app, window| keys(app, window, &["tab", "right", "down", "pagedown"]))
}

fn date_and_combo(cx: &mut gpui::Context<Stack>) -> Stack {
    let picker =
        cx.new(|cx| DatePicker::new(Date::new(2026, 10, 14).unwrap(), cx).today(stopped_today()));
    let combo = cx.new(|cx| Combobox::new(["Sans", "Serif", "Slab", "Mono"], cx).label("Font"));
    Stack::column(10., 10., vec![picker.into(), combo.into()])
}

fn datepicker_open() -> Scene {
    Scene::new("datepicker-open", 260., 260., |_, cx| date_and_combo(cx)).prepare(|app, window| {
        click(app, window, 10. + date_picker_width() / 2., 20.);
    })
}

fn date_picker_width() -> f32 {
    let widest = ["Sep 30, 2000", "Dec 30, 2000"]
        .map(|text| Font::Bold.measure(text))
        .into_iter()
        .max()
        .unwrap();
    (widest + 9 + 8 + 18) as f32
}

fn combobox_open() -> Scene {
    Scene::new("combobox-open", 260., 260., |_, cx| date_and_combo(cx)).prepare(|app, window| {
        // Choose the 15th from the calendar, then type into the combo box.
        // The original tabs to its field; its text fields are no tab stops
        // of GPUI's own, so a click puts the focus there.
        click(app, window, 10. + date_picker_width() / 2., 20.);
        keys(app, window, &["right", "enter"]);
        click(app, window, 30., 50.);
        keys(app, window, &["s", "down"]);
    })
}

fn timefield() -> Scene {
    Scene::new("timefield", 200., 60., |_, cx| {
        let field = cx.new(|cx| TimeField::new(9, 41, 0, cx).step(5));
        Stack::column(10., 0., vec![field.into()])
    })
    .prepare(|app, window| keys(app, window, &["tab", "up", "right", "0", "5", "p"]))
}

fn region(name: &'static str, language: Language, first_weekday: u8, hours: u8) -> Scene {
    Scene::new(name, 400., 180., move |_, cx| {
        let day = Date::new(2026, 10, 15).unwrap();
        let calendar = cx.new(|cx| {
            CalendarView::new(day, cx)
                .language(language)
                .first_weekday(first_weekday)
                .today(stopped_today())
        });
        let picker = cx.new(|cx| {
            DatePicker::new(day, cx)
                .language(language)
                .day_first(language == Language::English)
                .first_weekday(first_weekday)
                .today(stopped_today())
        });
        let field = cx.new(|cx| {
            TimeField::new(21, 41, 0, cx)
                .language(language)
                .hour_cycle(hours)
        });
        let side = cx.new(|_| Stack::column(0., 10., vec![picker.into(), field.into()]).inner());
        Stack::row(10., 12., vec![calendar.into(), side.into()])
    })
    .east_asian()
}

/// "Highlight", Lavender's mid color, its picker open and the red of the
/// palette's third row chosen.
fn colorpicker() -> Scene {
    Scene::new("colorpicker", 340., 300., |_, cx| {
        let well = cx.new(|cx| ColorWell::new(ACCENTS[0].mid, cx).label("Highlight"));
        Stack::column(10., 0., vec![well.into()])
    })
    .prepare(|app, window| {
        click(app, window, 32., 21.);
        // The dialog, 300 wide, centered a fifth of the way down; its
        // palette 20 in from its side and 31 down, the swatch 2 + 2 × 14.
        click(app, window, 20. + 20. + 2. + 7., 60. + 31. + 30. + 7.);
    })
}

fn font_picker(cx: &mut gpui::Context<Stack>, bold: bool) -> gpui::Entity<FontPicker> {
    cx.new(|cx| {
        // As wide as the column it is in, as the original stretches it.
        let mut picker = FontPicker::new(cx)
            .families(["Sans", "Serif", "Mono"])
            .width(400);
        picker.set_choice(
            FontChoice {
                family: "Serif".into(),
                size: 12.,
                bold,
                italic: true,
                underline: true,
            },
            cx,
        );
        picker
    })
}

fn pagesetup() -> Scene {
    Scene::new("pagesetup", 420., 420., |window, cx| {
        let picker = font_picker(cx, false);
        let page = cx.new(|cx| PageSetup::new(PageSetupState::default(), cx));
        page.update(cx, |page, cx| page.show(window, cx));
        Stack::column(10., 0., vec![picker.into(), page.into()])
    })
}

/// The picker once Bold is checked, the page setup closed.
fn fontpicker() -> Scene {
    Scene::new("fontpicker", 420., 420., |_, cx| {
        let picker = font_picker(cx, true);
        Stack::column(10., 0., vec![picker.into()])
    })
}

/// The original's tree of files: a disk with Documents (Letter and
/// Recipes), an empty folder and a file, and a file beside it.
fn file_tree() -> FileTree {
    FileTree::new(vec![
        TreeNode::new(0, "Platinum Disk", NodeKind::Disk).with_children(vec![
            TreeNode::new(1, "Documents", NodeKind::Folder).with_children(vec![
                TreeNode::new(2, "Letter", NodeKind::Document),
                TreeNode::new(3, "Recipes", NodeKind::Document),
            ]),
            TreeNode::new(4, "Empty Folder", NodeKind::Folder),
            TreeNode::new(5, "Read Me", NodeKind::Document),
        ]),
        TreeNode::new(6, "Note", NodeKind::Document),
    ])
}

/// The dialog, 364 wide, centered a fifth of the way down; its list 16 in
/// from its side and under the path pop-up, 22 tall, and a gap of 8.
const FILE_LIST_LEFT: f32 = 18. + 16.;
fn file_list_top(height: f32) -> f32 {
    (height / 5.).round() + 16. + 22. + 8.
}

/// Into the disk with a double click, into Documents with Return, and
/// down to Recipes.
fn open_dialog() -> Scene {
    Scene::new("open-dialog", 400., 350., |window, cx| {
        let files = cx.new(|_| file_tree());
        let dialog = cx.new(|cx| OpenDialog::new(files, cx));
        dialog.update(cx, |dialog, cx| dialog.show(window, cx));
        Stack::column(0., 0., vec![dialog.into()])
    })
    .prepare(|app, window| {
        double_click(app, window, FILE_LIST_LEFT + 60., file_list_top(350.) + 10.);
        keys(app, window, &["enter", "down"]);
    })
}

fn save_dialog_view(
    name: &'static str,
    language: Language,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<Stack>,
) -> Stack {
    let files = cx.new(|_| file_tree());
    let dialog = cx.new(|cx| {
        let prompt = if language == Language::English {
            "Save as:"
        } else {
            "存储为："
        };
        SaveDialog::new(files, cx).language(language).prompt(prompt)
    });
    dialog.update(cx, |dialog, cx| {
        dialog.set_parent(Some(1), cx);
        dialog.set_name(name, cx);
        dialog.show(window, cx);
    });
    Stack::column(0., 0., vec![dialog.into()])
}

/// In Documents, "untitled" selected under its prompt ("Letter" in
/// Chinese, for the alert of a file of the name).
fn save_dialog(name: &'static str, language: Language) -> Scene {
    let file = if language == Language::English {
        "untitled"
    } else {
        "Letter"
    };
    let scene = Scene::new(name, 400., 390., move |window, cx| {
        save_dialog_view(file, language, window, cx)
    });
    if language == Language::English {
        scene
    } else {
        scene.east_asian()
    }
}

/// "Menu" typed over the name, a click on Recipes, dimmed, which chooses
/// nothing, then "Recipes" typed into the name, and Save.
fn save_dialog_replace() -> Scene {
    Scene::new("save-dialog-replace", 400., 390., |window, cx| {
        save_dialog_view("untitled", Language::English, window, cx)
    })
    .prepare(|app, window| {
        keys(app, window, &["shift-m", "e", "n", "u"]);
        let top = file_list_top(390.);
        click(app, window, FILE_LIST_LEFT + 60., top + 18. + 10.);
        // The name's field, under the list and the prompt.
        let field = top + 150. + 6. + 16. + 11.;
        click(app, window, FILE_LIST_LEFT + 60., field);
        keys(
            app,
            window,
            &["cmd-a", "shift-r", "e", "c", "i", "p", "e", "s"],
        );
        // Save, at the bottom of the buttons.
        click(app, window, 400. - 18. - 16. - 50., field);
    })
}

/// Return over "Letter", a file of the name.
fn save_dialog_replace_zh_hans() -> Scene {
    Scene::new("save-dialog-replace-zh-hans", 400., 390., |window, cx| {
        save_dialog_view("Letter", Language::SimplifiedChinese, window, cx)
    })
    .east_asian()
    .prepare(|app, window| {
        keys(app, window, &["enter"]);
        // The moment after Save's flash.
        app.advance_clock(std::time::Duration::from_millis(200));
        app.run_until_parked();
    })
}

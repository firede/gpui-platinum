//! Menus, pop-ups, dates and times, pickers and the file dialogs, as
//! mygo-platinum's golden tests draw them.

use crate::Scene;
use gpui::{
    AnyWindowHandle, AppContext as _, HeadlessAppContext, Keystroke, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PlatformInput, div, point, prelude::*, px,
};
use gpui_platinum::{
    CalendarView, Combobox, DatePicker, Font, Language, Menu, MenuBar, MenuItem, PopupMenu,
    TimeField, menu::draw_popup_button, models::Date, paint::drawing,
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
}

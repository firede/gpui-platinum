//! Menus, pop-ups, dates and times, pickers and the file dialogs, as
//! mygo-platinum's golden tests draw them.

use crate::Scene;
use gpui::{
    AnyWindowHandle, AppContext as _, HeadlessAppContext, Keystroke, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PlatformInput, div, point, prelude::*, px,
};
use gpui_platinum::{
    Font, Menu, MenuBar, MenuItem, PopupMenu, menu::draw_popup_button, paint::drawing,
};

pub fn scenes() -> Vec<Scene> {
    vec![popup_states(), popup_open(), context_menu(), menubar_open()]
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

fn keys(app: &mut HeadlessAppContext, window: AnyWindowHandle, keys: &[&str]) {
    for key in keys {
        app.update_window(window, |_, window, cx| {
            let _ = window.draw(cx);
            window.dispatch_keystroke(Keystroke::parse(key).unwrap(), cx);
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

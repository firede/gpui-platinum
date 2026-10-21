//! The gallery's pages and the desktop, as the original's examples draw them:
//! the desktop's scene in each language, and the states its tests picture,
//! reached by the clicks and keys the tests send.

use crate::{Scene, desktop::Desktop, gallery::Gallery};
use gpui::{
    AnyWindowHandle, HeadlessAppContext, KeyDownEvent, Keystroke, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PlatformInput, ScrollDelta, ScrollWheelEvent,
    TouchPhase, point, px,
};
use gpui_platinum::Language;

pub fn scenes() -> Vec<Scene> {
    let mut scenes: Vec<Scene> = [
        "gallery-page-1",
        "gallery-page-2",
        "gallery-page-3",
        "gallery-page-4",
        "gallery-page-5",
    ]
    .into_iter()
    .enumerate()
    .map(|(page, name)| {
        Scene::new(name, 756., 476., move |_, cx| {
            Gallery::for_capture(page, cx)
        })
    })
    .collect();
    for (name, language) in [
        ("desktop-en", Language::English),
        ("desktop-ja", Language::Japanese),
        ("desktop-ko", Language::Korean),
        ("desktop-zh-hans", Language::SimplifiedChinese),
        ("desktop-zh-hant", Language::TraditionalChinese),
    ] {
        scenes.push(
            Scene::new(name, 900., 620., move |_, cx| {
                Desktop::new(language, true, cx)
            })
            .east_asian(),
        );
    }
    // TestCatalogPages and TestCatalogIconViewScroll: the controls' first
    // page in a desk 612 by 410, and the IconView page in one 640 by 410,
    // before and after its icons are scrolled.
    for (name, language, page, width, scrolled) in [
        (
            "desktop-controls-en",
            Language::English,
            "Button",
            612.,
            false,
        ),
        (
            "desktop-controls-zh-hans",
            Language::SimplifiedChinese,
            "Button",
            612.,
            false,
        ),
        (
            "desktop-controls-iconview-en",
            Language::English,
            "IconView",
            640.,
            false,
        ),
        (
            "desktop-controls-iconview-zh-hans",
            Language::SimplifiedChinese,
            "IconView",
            640.,
            false,
        ),
        (
            "desktop-controls-iconview-scrolled-en",
            Language::English,
            "IconView",
            640.,
            true,
        ),
        (
            "desktop-controls-iconview-scrolled-zh-hans",
            Language::SimplifiedChinese,
            "IconView",
            640.,
            true,
        ),
    ] {
        let scene = Scene::new(name, width, 410., move |_, cx| {
            Desktop::with_controls(language, page, cx)
        })
        .east_asian()
        .prepare(move |app, window| {
            if scrolled {
                Steps { app, window }.scroll(260., 130., 1000.);
            }
        });
        // The original's list ends with a page for its diagrams, which
        // this port has none of, so its scroll bar's thumb is shorter; and
        // a page chosen as its tests choose one leaves its list unscrolled,
        // where this port's list scrolls to the page.
        scenes.push(scene.pending());
    }
    scenes.extend(desk_tests());
    // TestScribeAskToSave: Letter changed, then closed.
    scenes.push(
        Scene::new("desktop-scribe-ask-save", 900., 620., |_, cx| {
            Desktop::with_document(Language::English, "letter", cx)
        })
        .east_asian()
        .prepare(|app, window| {
            let mut s = Steps { app, window };
            s.click(250., 200.);
            s.keys("cmd-up");
            s.type_text("P.S. ");
            s.keys("cmd-w");
        })
        // The original shows the keyboard's ring on a button only once Tab,
        // Return or Space has moved or pressed it, not when its alert
        // gives it the focus after a key elsewhere; this port rings it.
        .pending(),
    );
    scenes
}

/// The desk at the scene's time with no window open, as the original's
/// tests start it, then driven as the test named drives it.
fn desk(name: &'static str, steps: impl FnOnce(&mut Steps) + 'static) -> Scene {
    Scene::new(name, 900., 620., |_, cx| {
        Desktop::at_scene_time(Language::English, cx)
    })
    .east_asian()
    .prepare(move |app, window| steps(&mut Steps { app, window }))
}

// Where the menus' titles are, and their items: 18 high from the bar's
// foot, a separator 9.
const DESK: f32 = 24.;
const FILE: f32 = 56.;
const VIEW: f32 = 136.;
const SPECIAL: f32 = 187.;

fn item(index: usize, separators: usize) -> f32 {
    30. + 18. * index as f32 + 9. * separators as f32
}

/// The disk's icon, at the desk's right.
const DISK: (f32, f32) = (848., 52.);

/// The Size column's header in the disk's window as a list.
const SIZE_HEADER: (f32, f32) = (240., 103.);

/// TestDesktopOffice's ledger, its B2 made 900.
fn ledger(s: &mut Steps) {
    s.choose(DESK, item(5, 1));
    s.click(185., 131.);
    s.type_text("900");
    s.keys("enter");
}

/// TestDesktopOffice's Scribe, every "the" of its text made "a".
fn replace_all(s: &mut Steps) {
    s.choose(DESK, item(7, 1));
    s.click(250., 200.);
    s.type_text("the cat and the hat");
    s.keys("cmd-f");
    s.type_text("the");
    s.keys("tab");
    s.type_text("a");
    s.click(421., 252.);
}

fn desk_tests() -> Vec<Scene> {
    vec![
        // TestDesktopClock: a click on the clock shows the date.
        desk("desktop-clock-date", |s| s.click(785., 10.)),
        // TestDesktopWastebasket: Read Me to the Wastebasket, then Empty
        // Wastebasket… from the Special menu.
        desk("desktop-empty-alert", |s| {
            s.double_click(DISK.0, DISK.1);
            s.click(240., 104.);
            s.keys("cmd-backspace");
            s.choose(SPECIAL, item(1, 0));
        }),
        // TestDesktopAlarm: the alarm clock, its alarm set.
        desk("desktop-alarm-clock", |s| {
            s.choose(DESK, item(1, 1));
            s.click(62., 152.);
        }),
        // TestDesktopOffice: the ledger, B2 made 900.
        desk("desktop-ledger", ledger),
        // TestDesktopOffice, on: Scribe's find and replace, every "the"
        // made "a".
        desk("desktop-scribe-find", |s| {
            ledger(s);
            s.keys("cmd-w");
            replace_all(s);
        }),
        // TestDesktopOffice, last: the disk's window as a list, by size,
        // over Scribe's.
        desk("desktop-cabinet-list", |s| {
            ledger(s);
            s.keys("cmd-w");
            replace_all(s);
            s.keys("escape");
            s.double_click(DISK.0, DISK.1);
            s.choose(VIEW, item(2, 0));
            s.click(SIZE_HEADER.0, SIZE_HEADER.1);
        }),
        // TestDesktopControlPanels: Date & Time an hour on, and Sound muted.
        desk("desktop-control-panels", |s| {
            s.panel(207.);
            s.click(316., 102.);
            s.drag((150., 45.), (130., 45.));
            s.panel(261.);
            s.click(94., 133.);
            s.drag((300., 67.), (658., 49.));
        }),
        // TestDesktopColumns: the disk 460 wide, as columns, Documents and
        // Letter chosen.
        desk("desktop-cabinet-columns", |s| {
            s.double_click(DISK.0, DISK.1);
            s.drag((373., 259.), (493., 259.));
            s.choose(VIEW, item(3, 0));
            s.click(90., 99.);
            s.click(220., 83.);
        }),
        // TestDesktopFindFile: "re" found.
        desk("desktop-find-file", |s| {
            s.choose(FILE, item(3, 1));
            s.type_text("re");
            s.click(428., 106.);
        }),
        // TestScribeSizeMenu: a heading made 22 point, the Size menu open.
        desk("desktop-scribe-size", |s| {
            s.choose(DESK, item(7, 1));
            s.click(250., 200.);
            s.type_text("Recipes");
            s.keys("enter");
            s.type_text("Soup of the day");
            s.keys("cmd-up");
            for _ in 0.."Recipes".len() {
                s.keys("shift-right");
            }
            s.choose(236., item(1, 0));
            s.click(236., 10.);
        }),
        // TestDesktopPopupWindow: the disk's window docked.
        desk("desktop-desk-popup", |s| {
            s.double_click(DISK.0, DISK.1);
            s.choose(VIEW, item(5, 1));
        }),
        // TestScribeSaveAndOpen: an untitled document's save dialog.
        desk("desktop-scribe-save", |s| {
            s.choose(DESK, item(7, 1));
            s.click(250., 200.);
            s.type_text("Shopping list");
            s.keys("cmd-s");
            s.type_text("Groceries");
        }),
        // TestDesktopSmoke: the desk's tour, ended in Read Me saved. The
        // calculator and the windows it opened from are closed before what
        // is pictured, so the tour starts here at the puzzle, solved; then
        // Moss and Teal Bricks are chosen in Appearance.
        desk("desktop-smoke-end", |s| {
            s.choose(DESK, item(6, 1));
            s.choose(236., item(1, 0));
            s.panel(189.);
            s.click(140., 204.);
            s.click(175., 99.);
            s.click(110., 163.);
            s.keys("cmd-w");
            s.double_click(DISK.0, DISK.1);
            s.double_click(277., 120.);
            s.click(250., 250.);
            s.keys("cmd-up");
            s.type_text("Hello! ");
            s.keys("cmd-s");
        }),
        // TestDesktopGetInfo: Read Me's info.
        desk("desktop-get-info", |s| {
            // Opened as the test opens it, the icon not chosen.
            s.double_click(DISK.0, DISK.1);
            s.click(500., 400.);
            s.drag((150., 45.), (130., 49.));
            s.drag((352., 262.), (372., 252.));
            s.click(235., 108.);
            s.keys("cmd-i");
            s.drag((120., 67.), (478., 89.));
        }),
        // TestDesktopAsButtons: the disk's window as buttons.
        desk("desktop-cabinet-buttons", |s| {
            // Opened as the test opens it, the icon not chosen.
            s.double_click(DISK.0, DISK.1);
            s.click(500., 400.);
            s.drag((150., 45.), (130., 49.));
            s.drag((352., 262.), (392., 272.));
            s.choose(VIEW, item(1, 0));
        }),
    ]
}

/// Input sent to a scene's window as the platform would, each drawn first
/// so that what it reaches is where the picture has it.
struct Steps<'a> {
    app: &'a mut HeadlessAppContext,
    window: AnyWindowHandle,
}

impl Steps<'_> {
    fn send(&mut self, input: PlatformInput) {
        self.app
            .update_window(self.window, |_, window, cx| {
                let _ = window.draw(cx);
                window.dispatch_event(input, cx);
            })
            .unwrap();
        self.app.run_until_parked();
    }
    /// Draws a frame, as the window does between inputs.
    fn frame(&mut self) {
        self.app
            .update_window(self.window, |_, window, cx| {
                let _ = window.draw(cx);
            })
            .unwrap();
        self.app.run_until_parked();
    }
    /// Waits out the blink of a menu's choice, and draws what it did.
    fn wait(&mut self) {
        self.app
            .advance_clock(std::time::Duration::from_millis(250));
        self.frame();
        self.frame();
    }
    fn move_to(&mut self, x: f32, y: f32, pressed: bool) {
        self.send(PlatformInput::MouseMove(MouseMoveEvent {
            position: point(px(x), px(y)),
            pressed_button: pressed.then_some(MouseButton::Left),
            modifiers: Modifiers::default(),
        }));
    }
    fn press(&mut self, x: f32, y: f32, clicks: usize) {
        self.send(PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Left,
            position: point(px(x), px(y)),
            modifiers: Modifiers::default(),
            click_count: clicks,
            first_mouse: false,
        }));
    }
    fn release(&mut self, x: f32, y: f32, clicks: usize) {
        self.send(PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position: point(px(x), px(y)),
            modifiers: Modifiers::default(),
            click_count: clicks,
        }));
    }
    fn click(&mut self, x: f32, y: f32) {
        self.move_to(x, y, false);
        self.press(x, y, 1);
        self.release(x, y, 1);
    }
    /// A click and a second one that the platform counts as a double click.
    fn double_click(&mut self, x: f32, y: f32) {
        self.click(x, y);
        self.press(x, y, 2);
        self.release(x, y, 2);
    }
    /// A press, a move with the button down, and the release there.
    fn drag(&mut self, from: (f32, f32), to: (f32, f32)) {
        self.move_to(from.0, from.1, false);
        self.press(from.0, from.1, 1);
        self.move_to(to.0, to.1, true);
        self.release(to.0, to.1, 1);
    }
    /// Keys, as Keystroke parses them, one after another; a menu's
    /// equivalent among them is waited out.
    fn keys(&mut self, keys: &str) {
        for key in keys.split_whitespace() {
            let keystroke = Keystroke::parse(key).unwrap();
            let command = keystroke.modifiers.platform;
            self.send(PlatformInput::KeyDown(KeyDownEvent {
                keystroke,
                is_held: false,
                prefer_character_input: false,
            }));
            if command {
                self.wait();
            }
        }
    }
    /// Chooses the item at y from the menu whose title is at x.
    fn choose(&mut self, x: f32, y: f32) {
        self.click(x, 10.);
        self.click(x + 20., y);
        self.wait();
    }
    /// Opens the control panel at y in the Desk menu's Control Panels.
    fn panel(&mut self, y: f32) {
        self.click(DESK, 10.);
        self.move_to(DESK + 20., item(8, 2), false);
        self.wait();
        self.click(190., y);
        self.wait();
    }
    /// The wheel turned at x, y, by the pixels given, toward the end.
    fn scroll(&mut self, x: f32, y: f32, pixels: f32) {
        self.move_to(x, y, false);
        self.send(PlatformInput::ScrollWheel(ScrollWheelEvent {
            position: point(px(x), px(y)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-pixels))),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        }));
        self.wait();
    }
    /// Text typed, a character at a time.
    fn type_text(&mut self, text: &str) {
        for character in text.chars() {
            let key = match character {
                ' ' => "space".to_owned(),
                character => character.to_string(),
            };
            let keystroke = Keystroke {
                key_char: Some(character.to_string()),
                ..Keystroke::parse(&key).unwrap()
            };
            self.app
                .update_window(self.window, |_, window, cx| {
                    let _ = window.draw(cx);
                    window.dispatch_keystroke(keystroke, cx);
                })
                .unwrap();
            self.app.run_until_parked();
        }
    }
}

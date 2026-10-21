//! What the controls do over time — flashes, blinks, repeats and how long
//! a click or typing waits — by the test executor's clock, which only the
//! test moves.

use gpui::{
    AnyElement, Context, Entity, FocusHandle, Focusable, Modifiers, MouseButton, Subscription,
    TestAppContext, VisualTestContext, Window, div, point, prelude::*, px,
};
use gpui_platinum::{
    ListBox, MENU_FLASH, Menu, MenuBar, MenuChoice, MenuItem, PopupMenu, PopupWindow, Step,
    Stepper, TextField, Theme, button, button_scope, chasing_arrows, indeterminate_progress,
    window::{PlatinumWindow, WindowEvent},
};
use std::time::Duration;

/// The quads of the last frame, to compare one frame with another.
fn frame(cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .map(|quad| format!("{:?} {:?}", quad.bounds, quad.background))
            .collect()
    })
}

/// Moves the clock on and draws what it changed.
fn advance(cx: &mut VisualTestContext, milliseconds: u64) {
    cx.executor()
        .advance_clock(Duration::from_millis(milliseconds));
    cx.run_until_parked();
}

struct DefaultButton {
    focus: FocusHandle,
    clicks: usize,
}

impl Render for DefaultButton {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        button_scope(
            "buttons",
            div().track_focus(&self.focus).p_2().child(
                button("ok", "OK")
                    .default_button()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.clicks += 1;
                        cx.notify();
                    })),
            ),
        )
    }
}

/// A default button chosen with Return shows pressed for 130 ms.
#[gpui::test]
fn default_button_flashes(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| DefaultButton {
        focus: cx.focus_handle(),
        clicks: 0,
    });
    cx.update(|window, cx| {
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
    });
    cx.run_until_parked();
    let normal = frame(cx);
    cx.simulate_keystrokes("enter");
    cx.read_entity(&view, |view, _| assert_eq!(view.clicks, 1));
    assert_ne!(frame(cx), normal, "the button did not flash");
    advance(cx, 129);
    assert_ne!(frame(cx), normal, "the flash ended early");
    advance(cx, 1);
    assert_eq!(
        frame(cx),
        normal,
        "the button stayed pressed after the flash"
    );
}

struct Field {
    field: Entity<TextField>,
}

impl Render for Field {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().p_2().child(self.field.clone())
    }
}

/// The caret shows for 500 ms and hides for as long, and shows at once
/// again when the user types.
#[gpui::test]
fn caret_blinks(cx: &mut TestAppContext) {
    cx.update(gpui_platinum::init);
    let (view, cx) = cx.add_window_view(|_, cx| Field {
        field: cx.new(TextField::new),
    });
    cx.update(|window, cx| {
        let focus = view.read(cx).field.focus_handle(cx);
        window.focus(&focus, cx);
    });
    cx.simulate_input("a");
    let shown = frame(cx);
    advance(cx, 500);
    let hidden = frame(cx);
    assert_ne!(shown, hidden, "the caret did not hide");
    advance(cx, 500);
    assert_eq!(frame(cx), shown, "the caret did not show again");
    advance(cx, 750);
    assert_eq!(frame(cx), hidden, "the caret did not hide again");
    cx.simulate_keystrokes("left right");
    assert_eq!(frame(cx), shown, "a key did not show the caret at once");
}

struct List {
    list: Entity<ListBox>,
}

impl Render for List {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().p_2().child(self.list.clone())
    }
}

/// Letters typed within a second of each other choose the item they begin;
/// after it, they start anew.
#[gpui::test]
fn list_type_select_waits_a_second(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| List {
        list: cx.new(|cx| ListBox::new(["Apple", "Banana", "Blueberry", "Cherry"], cx)),
    });
    let list = cx.read_entity(&view, |view, _| view.list.clone());
    cx.update(|window, cx| {
        let focus = list.focus_handle(cx);
        window.focus(&focus, cx);
    });
    for (wait, typed, want) in [
        (0, "b", 1),
        (1000, "l", 2), // "bl"
        (1001, "b", 1), // anew
        (0, "c", 1),    // "bc", which nothing begins
        (1001, "c", 3),
    ] {
        advance(cx, wait);
        cx.simulate_keystrokes(typed);
        let selection = cx.read_entity(&list, |list, _| list.selection());
        assert_eq!(selection, Some(want), "{typed:?} after {wait} ms");
    }
}

/// Delivers the frame an animation asked for, which tests have no loop for.
fn next_frame(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.simulate_next_frame(cx));
    cx.run_until_parked();
}

struct Animation {
    bar: bool,
}

impl Render for Animation {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let animation: AnyElement = if self.bar {
            indeterminate_progress(100, Theme::default()).into_any_element()
        } else {
            chasing_arrows().into_any_element()
        };
        div().p_2().child(animation)
    }
}

/// Chasing arrows and an indeterminate progress bar move on every tenth of
/// a second.
#[gpui::test]
fn status_animates(cx: &mut TestAppContext) {
    for bar in [false, true] {
        let (_, cx) = cx.add_window_view(|_, _| Animation { bar });
        cx.run_until_parked();
        let first = frame(cx);
        advance(cx, 99);
        next_frame(cx);
        assert_eq!(frame(cx), first, "bar {bar}: moved before its step");
        advance(cx, 1);
        next_frame(cx);
        assert_ne!(frame(cx), first, "bar {bar}: stood still");
    }
}

struct Spring {
    tools: Entity<PopupWindow>,
}

impl Render for Spring {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .w(px(300.))
            .h(px(140.))
            .child(self.tools.clone())
    }
}

/// A drag resting on a closed pop-up window's tab opens it after half a
/// second; one passing over does not.
#[gpui::test]
fn popup_window_springs_open_under_a_resting_drag(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| Spring {
        tools: cx.new(|cx| PopupWindow::new("Tools", cx).position(150).size(140, 80)),
    });
    let tools = cx.read_entity(&view, |view, _| view.tools.clone());
    tools.update(cx, |tools, cx| tools.set_open(false, cx));
    cx.run_until_parked();
    // The closed window is its 20-pixel tab, at the bottom, inset by 12.
    let (x0, y0) = (20., 20.);
    let (x1, y1) = (182., 130.);
    let drag = |cx: &mut VisualTestContext, x: f32, y: f32| {
        cx.simulate_mouse_move(point(px(x), px(y)), MouseButton::Left, Modifiers::none());
    };
    cx.simulate_mouse_down(point(px(x0), px(y0)), MouseButton::Left, Modifiers::none());
    for step in 1..=4 {
        let f = step as f32 / 4.;
        drag(cx, x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
    }
    advance(cx, 499);
    assert!(
        !tools.read_with(cx, |tools, _| tools.is_open()),
        "sprang open early"
    );
    // Off the tab, and back: the rest starts anew.
    drag(cx, x1 + 80., y0);
    drag(cx, x1, y1);
    advance(cx, 499);
    assert!(
        !tools.read_with(cx, |tools, _| tools.is_open()),
        "the rest went on while the drag was off the tab"
    );
    advance(cx, 1);
    assert!(
        tools.read_with(cx, |tools, _| tools.is_open()),
        "did not spring open"
    );
    cx.simulate_mouse_up(point(px(x1), px(y1)), MouseButton::Left, Modifiers::none());
}

struct Steps {
    stepper: Entity<Stepper>,
    value: i32,
    _steps: Subscription,
}

impl Render for Steps {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(self.stepper.clone())
    }
}

/// A stepper's arrow held down repeats after 300 ms, every 60 ms.
#[gpui::test]
fn stepper_repeats_while_held(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| {
        let stepper = cx.new(|cx| Stepper::new(cx).label("Count"));
        let steps = cx.subscribe(&stepper, |this: &mut Steps, _, step: &Step, _| {
            this.value += step.0
        });
        Steps {
            stepper,
            value: 0,
            _steps: steps,
        }
    });
    cx.run_until_parked();
    let up = point(px(6.), px(4.));
    cx.simulate_mouse_down(up, MouseButton::Left, Modifiers::none());
    for (held, want) in [(0, 1), (299, 1), (1, 2), (180, 5)] {
        advance(cx, held);
        let value = cx.read_entity(&view, |view, _| view.value);
        assert_eq!(value, want, "held {held} ms more");
    }
    cx.simulate_mouse_up(up, MouseButton::Left, Modifiers::none());
    advance(cx, 1000);
    let value = cx.read_entity(&view, |view, _| view.value);
    assert_eq!(value, 5, "the arrow repeated after it was let go");
}

/// Presses and lets go at x, y.
fn click(cx: &mut VisualTestContext, x: f32, y: f32, modifiers: Modifiers) {
    cx.simulate_mouse_down(point(px(x), px(y)), MouseButton::Left, modifiers);
    cx.simulate_mouse_up(point(px(x), px(y)), MouseButton::Left, modifiers);
}

/// Moves the clock on by a duration and draws what it changed.
fn advance_by(cx: &mut VisualTestContext, duration: Duration) {
    cx.executor().advance_clock(duration);
    cx.run_until_parked();
}

struct Bar {
    menu: Entity<MenuBar>,
    chosen: Vec<MenuChoice>,
    _choices: Subscription,
}

impl Render for Bar {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.menu.clone())
    }
}

/// A chosen menu item blinks for 150 ms, the menu open, before its choice
/// goes out and the menu closes.
#[gpui::test]
fn menu_flashes_before_its_action(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| {
        let items = vec![
            MenuItem::new("New"),
            MenuItem::new("Open…"),
            MenuItem::separator(),
            MenuItem::new("Save"),
        ];
        let menu = cx.new(|cx| MenuBar::new(vec![Menu::new("File", items)], cx));
        let choices = cx.subscribe(&menu, |this: &mut Bar, _, choice: &MenuChoice, _| {
            this.chosen.push(choice.clone())
        });
        Bar {
            menu,
            chosen: Vec::new(),
            _choices: choices,
        }
    });
    let menu = cx.read_entity(&view, |view, _| view.menu.clone());
    let chosen = |cx: &mut VisualTestContext| cx.read_entity(&view, |view, _| view.chosen.clone());
    // A click on the title leaves the menu open; New is its first item,
    // under the bar.
    click(cx, 20., 10., Modifiers::none());
    click(cx, 25., 30., Modifiers::none());
    let before = frame(cx);
    assert!(
        chosen(cx).is_empty(),
        "the choice went out before the blink"
    );
    advance_by(cx, MENU_FLASH / 4);
    assert_ne!(frame(cx), before, "the item did not blink");
    assert!(
        menu.read_with(cx, |menu, _| menu.is_open()) && chosen(cx).is_empty(),
        "the menu closed while the item blinked"
    );
    advance_by(cx, MENU_FLASH);
    assert_eq!(
        chosen(cx),
        vec![MenuChoice {
            menu: 0,
            path: vec![0]
        }],
        "New was not chosen once after the blink"
    );
    assert!(
        !menu.read_with(cx, |menu, _| menu.is_open()),
        "the menu stayed open after the blink"
    );
}

struct Popup {
    popup: Entity<PopupMenu>,
}

impl Render for Popup {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.popup.clone())
    }
}

/// A press on a pop-up menu let go within 400 ms leaves its menu open to
/// choose from; one held longer chooses the item under it, and closes it.
#[gpui::test]
fn popup_menu_click_hold(cx: &mut TestAppContext) {
    for held in [350, 450] {
        let (view, cx) = cx.add_window_view(|_, cx| Popup {
            popup: cx.new(|cx| PopupMenu::new(["One", "Two", "Three"], cx)),
        });
        let popup = cx.read_entity(&view, |view, _| view.popup.clone());
        let at = point(px(10.), px(10.));
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        advance(cx, held);
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
        advance_by(cx, MENU_FLASH);
        let (open, selection) =
            popup.read_with(cx, |popup, _| (popup.is_open(), popup.selection()));
        assert_eq!(
            (open, selection),
            (held < 400, 0),
            "held {held} ms: the menu open {open}, chose {selection}"
        );
    }
}

struct Frame {
    frame: Entity<PlatinumWindow>,
    chosen: usize,
    _events: Subscription,
}

impl Render for Frame {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().relative().size_full().child(self.frame.clone())
    }
}

/// A Command-press on a window's title let go within 400 ms leaves its
/// path menu open to choose from; one held longer and let go off the
/// folders closes it.
#[gpui::test]
fn window_path_menu_click_hold(cx: &mut TestAppContext) {
    for held in [350, 450] {
        let (view, cx) = cx.add_window_view(|_, cx| {
            let frame = cx.new(|cx| {
                PlatinumWindow::new("Window", cx)
                    .position(30., 30.)
                    .size(180., 120.)
                    .path(vec!["Disk".into(), "Folder".into()])
            });
            let events = cx.subscribe(&frame, |this: &mut Frame, _, event: &WindowEvent, _| {
                if let WindowEvent::PathChosen(_) = event {
                    this.chosen += 1;
                }
            });
            Frame {
                frame,
                chosen: 0,
                _events: events,
            }
        });
        let title = point(px(120.), px(38.));
        cx.simulate_mouse_down(title, MouseButton::Left, Modifiers::command());
        advance(cx, held);
        cx.simulate_mouse_up(title, MouseButton::Left, Modifiers::command());
        // Folder, the menu's second item, under the title bar.
        click(cx, 110., 80., Modifiers::none());
        advance_by(cx, MENU_FLASH);
        let chosen = cx.read_entity(&view, |view, _| view.chosen);
        assert_eq!(
            chosen,
            usize::from(held < 400),
            "held {held} ms: chose {chosen} folders"
        );
    }
}

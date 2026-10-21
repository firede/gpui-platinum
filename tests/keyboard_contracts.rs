use gpui::{
    Context, Entity, FocusHandle, Focusable, Modifiers, MouseButton, TestAppContext, Window, div,
    point, prelude::*, px,
};
use gpui_platinum::{
    Alert, AlertKind, Combobox, DateChanged, DatePicker, Dialog, FontPicker, ListBox, ListItem,
    MENU_FLASH, PopupMenu, TextField, button, button_scope,
    models::Date,
    text_input,
    window::{PlatinumWindow, PlatinumWindowOptions, WindowState},
};
use std::{cell::Cell, rc::Rc};

struct FormWidths {
    combo: Entity<Combobox>,
    font_picker: Entity<FontPicker>,
    combo_right: Rc<Cell<f32>>,
    picker_right: Rc<Cell<f32>>,
}

fn measure_left(position: Rc<Cell<f32>>) -> impl IntoElement {
    gpui::canvas(
        move |bounds, _, _| position.set(f32::from(bounds.origin.x)),
        |_, _, _, _| (),
    )
    .w(px(1.))
    .h(px(1.))
    .flex_shrink_0()
}

impl Render for FormWidths {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_start()
            .child(
                div()
                    .flex()
                    .child(self.combo.clone())
                    .child(measure_left(self.combo_right.clone())),
            )
            .child(
                div()
                    .flex()
                    .child(self.font_picker.clone())
                    .child(measure_left(self.picker_right.clone())),
            )
    }
}

#[gpui::test]
fn compound_fields_reserve_their_requested_width_after_resizing(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| FormWidths {
        combo: cx.new(|cx| Combobox::new(["One", "Two"], cx).width(140)),
        font_picker: cx.new(|cx| FontPicker::new(cx).width(444)),
        combo_right: Rc::new(Cell::new(0.)),
        picker_right: Rc::new(Cell::new(0.)),
    });
    cx.run_until_parked();
    cx.read_entity(&view, |view, _| {
        assert_eq!(view.combo_right.get(), 140.);
        assert_eq!(view.picker_right.get(), 444.);
    });
    cx.update(|_, cx| {
        let view = view.read(cx);
        let combo = view.combo.clone();
        let picker = view.font_picker.clone();
        combo.update(cx, |combo, cx| combo.set_width(200, cx));
        picker.update(cx, |picker, cx| picker.set_width(400, cx));
    });
    cx.run_until_parked();
    cx.read_entity(&view, |view, _| {
        assert_eq!(view.combo_right.get(), 200.);
        assert_eq!(view.picker_right.get(), 400.);
    });
}

struct Buttons {
    field: Entity<TextField>,
    button_focus: FocusHandle,
    accepted: usize,
    cancelled: usize,
    outer_accepted: usize,
    nested: bool,
    disabled_default: bool,
}

struct AlertHarness {
    focus: FocusHandle,
    alert: Entity<Alert>,
}

impl Render for AlertHarness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        button_scope(
            "alert-buttons",
            div()
                .track_focus(&self.focus)
                .child(
                    button("show-alert", "Show")
                        .default_button()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.alert.update(cx, |alert, cx| alert.show(window, cx))
                        })),
                )
                .child(self.alert.clone()),
        )
    }
}

#[gpui::test]
fn return_can_close_an_alert_opened_by_a_default_button(cx: &mut TestAppContext) {
    cx.update(gpui_platinum::init);
    let (view, cx) = cx.add_window_view(|_, cx| AlertHarness {
        focus: cx.focus_handle(),
        alert: cx.new(|cx| Alert::new(AlertKind::Note, "Message", "", ["OK"], cx)),
    });
    cx.update(|window, cx| {
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("enter");
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").unwrap(),
    });
    cx.read_entity(&view, |view, cx| assert!(view.alert.read(cx).is_open()));
    cx.simulate_keystrokes("enter");
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").unwrap(),
    });
    cx.read_entity(&view, |view, cx| assert!(!view.alert.read(cx).is_open()));
}

impl Buttons {
    fn new(multiline: bool, nested: bool, cx: &mut Context<Self>) -> Self {
        Self {
            field: cx.new(|cx| TextField::new(cx).multiline(multiline)),
            button_focus: cx.focus_handle(),
            accepted: 0,
            cancelled: 0,
            outer_accepted: 0,
            nested,
            disabled_default: false,
        }
    }
}

impl Render for Buttons {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let buttons = div()
            .child(self.field.clone())
            .child(
                button("accept", "OK")
                    .focus_handle(&self.button_focus)
                    .default_button()
                    .disabled(self.disabled_default)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.accepted += 1;
                        cx.notify();
                    })),
            )
            .child(
                button("cancel", "Cancel")
                    .cancel_button()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.cancelled += 1;
                        cx.notify();
                    })),
            );
        if self.nested {
            button_scope(
                "outer",
                div().child(button_scope("inner", buttons)).child(
                    button("outer-accept", "OK")
                        .default_button()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.outer_accepted += 1;
                            cx.notify();
                        })),
                ),
            )
        } else {
            button_scope("buttons", buttons)
        }
    }
}

#[gpui::test]
fn focused_default_button_activates_once_when_return_is_released(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| Buttons::new(false, false, cx));
    cx.update(|window, cx| {
        let focus = view.read(cx).button_focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("enter");
    cx.read_entity(&view, |view, _| assert_eq!(view.accepted, 0));
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").unwrap(),
    });
    cx.read_entity(&view, |view, _| assert_eq!(view.accepted, 1));
}

#[gpui::test]
fn default_and_cancel_keys_work_from_another_control(cx: &mut TestAppContext) {
    cx.update(text_input::init);
    let (view, cx) = cx.add_window_view(|_, cx| Buttons::new(false, false, cx));
    cx.update(|window, cx| {
        window.focus(&view.read(cx).field.focus_handle(cx), cx);
    });
    cx.simulate_keystrokes("enter escape cmd-.");
    cx.read_entity(&view, |view, _| {
        assert_eq!(view.accepted, 1);
        assert_eq!(view.cancelled, 2);
    });
}

#[gpui::test]
fn multiline_editor_consumes_return_before_default_button(cx: &mut TestAppContext) {
    cx.update(text_input::init);
    let (view, cx) = cx.add_window_view(|_, cx| Buttons::new(true, false, cx));
    cx.update(|window, cx| {
        window.focus(&view.read(cx).field.focus_handle(cx), cx);
    });
    cx.simulate_keystrokes("enter");
    cx.read_entity(&view, |view, cx| {
        assert_eq!(view.field.read(cx).value(), "\n");
        assert_eq!(view.accepted, 0);
    });
}

#[gpui::test]
fn nested_button_scopes_keep_default_actions_separate(cx: &mut TestAppContext) {
    cx.update(text_input::init);
    let (view, cx) = cx.add_window_view(|_, cx| Buttons::new(false, true, cx));
    cx.update(|window, cx| {
        window.focus(&view.read(cx).field.focus_handle(cx), cx);
    });
    cx.simulate_keystrokes("enter");
    cx.read_entity(&view, |view, _| {
        assert_eq!(view.accepted, 1);
        assert_eq!(view.outer_accepted, 0);
    });
}

#[gpui::test]
fn disabled_dialog_default_does_not_activate_an_outer_default(cx: &mut TestAppContext) {
    cx.update(text_input::init);
    let (view, cx) = cx.add_window_view(|_, cx| {
        let mut buttons = Buttons::new(false, true, cx);
        buttons.disabled_default = true;
        buttons
    });
    cx.update(|window, cx| window.focus(&view.read(cx).field.focus_handle(cx), cx));
    cx.simulate_keystrokes("enter");
    cx.read_entity(&view, |view, _| {
        assert_eq!(view.accepted, 0);
        assert_eq!(view.outer_accepted, 0);
    });
}

#[gpui::test]
fn first_down_selects_the_first_available_list_row(cx: &mut TestAppContext) {
    let (list, cx) = cx.add_window_view(|_, cx| ListBox::new(["One", "Two", "Three"], cx));
    cx.update(|window, cx| window.focus(&list.focus_handle(cx), cx));
    cx.simulate_keystrokes("down");
    cx.read_entity(&list, |list, _| assert_eq!(list.selection(), Some(0)));
    cx.simulate_keystrokes("down");
    cx.read_entity(&list, |list, _| assert_eq!(list.selection(), Some(1)));
}

#[gpui::test]
fn disabled_lists_and_rows_do_not_take_keyboard_selection(cx: &mut TestAppContext) {
    let (list, cx) = cx.add_window_view(|_, cx| {
        ListBox::new(
            [
                ListItem {
                    label: "One".into(),
                    disabled: true,
                },
                ListItem::from("Two"),
                ListItem::from("Three"),
            ],
            cx,
        )
        .disabled(true)
    });
    cx.update(|window, cx| window.focus(&list.focus_handle(cx), cx));
    cx.simulate_keystrokes("down");
    cx.read_entity(&list, |list, _| assert_eq!(list.selection(), None));
    cx.update(|_, cx| list.update(cx, |list, cx| list.set_disabled(false, cx)));
    cx.simulate_keystrokes("down");
    cx.read_entity(&list, |list, _| assert_eq!(list.selection(), Some(1)));
}

#[gpui::test]
fn popup_direction_keys_open_at_the_current_selection(cx: &mut TestAppContext) {
    let (menu, cx) =
        cx.add_window_view(|_, cx| PopupMenu::new(["One", "Two", "Three", "Four"], cx).selected(2));
    cx.update(|window, cx| window.focus(&menu.focus_handle(cx), cx));
    // Return chooses once the item has blinked.
    let choose = |cx: &mut gpui::VisualTestContext, keys| {
        cx.simulate_keystrokes(keys);
        cx.executor().advance_clock(MENU_FLASH);
        cx.run_until_parked();
    };
    choose(cx, "down enter");
    cx.read_entity(&menu, |menu, _| assert_eq!(menu.selection(), 2));
    choose(cx, "up enter");
    cx.read_entity(&menu, |menu, _| assert_eq!(menu.selection(), 2));
    choose(cx, "down down enter");
    cx.read_entity(&menu, |menu, _| assert_eq!(menu.selection(), 3));
}

#[gpui::test]
fn choosing_the_current_date_closes_the_picker(cx: &mut TestAppContext) {
    let (picker, cx) =
        cx.add_window_view(|_, cx| DatePicker::new(Date::new(2026, 10, 15).unwrap(), cx));
    let button = point(px(10.), px(10.));
    cx.simulate_mouse_down(button, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(button, MouseButton::Left, Modifiers::default());
    cx.read_entity(&picker, |picker, _| assert!(picker.is_open()));
    let selected_day = point(px(105.), px(100.));
    cx.simulate_mouse_down(selected_day, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(selected_day, MouseButton::Left, Modifiers::default());
    cx.read_entity(&picker, |picker, _| {
        assert!(!picker.is_open());
        assert_eq!(picker.value(), Date::new(2026, 10, 15).unwrap());
    });
}

#[gpui::test]
fn setting_the_picker_value_updates_its_calendar_without_a_change_event(cx: &mut TestAppContext) {
    let changes = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = changes.clone();
    let (picker, cx) = cx.add_window_view(|_, cx| {
        let picker = DatePicker::new(Date::new(2026, 10, 15).unwrap(), cx);
        cx.subscribe(&cx.entity(), move |_, _, _: &DateChanged, _| {
            observed.set(observed.get() + 1)
        })
        .detach();
        picker
    });
    cx.update(|_, cx| {
        picker.update(cx, |picker, cx| {
            picker.set_value(Date::new(2026, 10, 22).unwrap(), cx)
        })
    });
    let button = point(px(10.), px(10.));
    cx.simulate_mouse_down(button, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(button, MouseButton::Left, Modifiers::default());
    let selected_day = point(px(105.), px(116.));
    cx.simulate_mouse_down(selected_day, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(selected_day, MouseButton::Left, Modifiers::default());
    cx.read_entity(&picker, |picker, _| {
        assert_eq!(picker.value(), Date::new(2026, 10, 22).unwrap());
        assert!(!picker.is_open());
    });
    assert_eq!(changes.get(), 0);
}

struct DialogHarness {
    field: Entity<TextField>,
    dialog: Entity<Dialog>,
}

impl Render for DialogHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.dialog.clone()
    }
}

#[gpui::test]
fn dialog_does_not_accept_return_from_a_multiline_editor(cx: &mut TestAppContext) {
    cx.update(|cx| {
        text_input::init(cx);
        gpui_platinum::dialogs::init(cx);
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        let field = cx.new(|cx| TextField::new(cx).multiline(true));
        let focus = field.focus_handle(cx);
        let dialog = cx.new(|cx| Dialog::new("Edit", field.clone(), cx).focus_handles(vec![focus]));
        dialog.update(cx, |dialog, cx| dialog.show(window, cx));
        DialogHarness { field, dialog }
    });
    cx.simulate_keystrokes("enter");
    cx.read_entity(&view, |view, cx| {
        assert!(view.dialog.read(cx).is_open());
        assert_eq!(view.field.read(cx).value(), "\n");
    });
}

struct FrameHarness {
    frame: Entity<PlatinumWindow>,
}

impl Render for FrameHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().relative().size_full().child(self.frame.clone())
    }
}

#[gpui::test]
fn window_title_and_grow_corner_update_geometry_and_respect_minimums(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| FrameHarness {
        frame: cx.new(|cx| {
            PlatinumWindow::new("Window", cx)
                .position(30., 30.)
                .size(180., 120.)
        }),
    });
    let frame = cx.read_entity(&view, |view, _| view.frame.clone());
    cx.simulate_mouse_down(
        point(px(60.), px(38.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_move(
        point(px(80.), px(58.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_up(
        point(px(80.), px(58.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.read_entity(&frame, |frame, _| {
        assert_eq!((frame.state.x, frame.state.y), (50., 50.));
        assert_eq!(frame.content_size(), gpui::size(px(168.), px(95.)));
    });
    cx.simulate_mouse_down(
        point(px(226.), px(166.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_move(
        point(px(256.), px(186.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_up(
        point(px(256.), px(186.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.read_entity(&frame, |frame, _| {
        assert_eq!((frame.state.width, frame.state.height), (210., 140.))
    });
    cx.simulate_mouse_down(
        point(px(256.), px(186.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_move(
        point(px(70.), px(70.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_up(
        point(px(70.), px(70.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.read_entity(&frame, |frame, _| {
        assert_eq!((frame.state.width, frame.state.height), (120., 80.))
    });
}

#[gpui::test]
fn title_boxes_collapse_and_toggle_zoom_while_modal_frames_reject_them(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| FrameHarness {
        frame: cx.new(|cx| {
            PlatinumWindow::new("Window", cx)
                .position(30., 30.)
                .size(180., 120.)
                .zoom_to(WindowState {
                    x: 2.,
                    y: 2.,
                    width: 296.,
                    height: 216.,
                    collapsed: false,
                })
        }),
    });
    let frame = cx.read_entity(&view, |view, _| view.frame.clone());
    cx.simulate_click(point(px(197.), px(39.)), Modifiers::default());
    cx.read_entity(&frame, |frame, _| assert!(frame.state.collapsed));
    cx.simulate_click(point(px(197.), px(39.)), Modifiers::default());
    cx.read_entity(&frame, |frame, _| assert!(!frame.state.collapsed));
    cx.simulate_click(point(px(182.), px(39.)), Modifiers::default());
    cx.read_entity(&frame, |frame, _| {
        assert!(frame.is_zoomed());
        assert_eq!((frame.state.width, frame.state.height), (296., 216.));
    });
    cx.simulate_click(point(px(270.), px(11.)), Modifiers::default());
    cx.read_entity(&frame, |frame, _| {
        assert!(!frame.is_zoomed());
        assert_eq!((frame.state.width, frame.state.height), (180., 120.));
    });
    cx.update(|_, cx| {
        frame.update(cx, |frame, cx| {
            frame.set_options(
                PlatinumWindowOptions {
                    modal: true,
                    ..Default::default()
                },
                cx,
            )
        })
    });
    cx.simulate_click(point(px(197.), px(39.)), Modifiers::default());
    cx.read_entity(&frame, |frame, _| {
        assert!(!frame.state.collapsed);
        assert_eq!(frame.content_size(), gpui::size(px(164.), px(93.)));
    });
}

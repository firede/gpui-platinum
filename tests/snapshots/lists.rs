//! Lists, tables, sheets, text and the navigation views, as the original's
//! golden tests draw them.

use crate::Scene;
use gpui::{
    AnyWindowHandle, AppContext as _, Entity, Focusable as _, HeadlessAppContext, KeyDownEvent,
    Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PlatformInput,
    div, point, prelude::*, px, rgb,
};
use gpui_platinum::{
    ListBox, SheetView, Table, TableColumn, Theme, TreeView, draw_list_box,
    models::{Cell, FileTree, NodeKind, Sheet, TreeNode},
    paint::{Pen, drawing},
    theme::GRAY_2,
};
use std::{cell::RefCell, rc::Rc};

const FRUITS: [&str; 10] = [
    "Almond",
    "Apricot",
    "Banana",
    "Cherry",
    "Date",
    "Elderberry",
    "Fig",
    "Grape",
    "Kiwi",
    "Lemon",
];

/// Where a scene keeps the entity its preparation works on.
type Slot<T> = Rc<RefCell<Option<Entity<T>>>>;

fn slot<T>() -> Slot<T> {
    Rc::new(RefCell::new(None))
}

/// Lets the app settle and draws a frame, as a window does between inputs.
fn frame(app: &mut HeadlessAppContext, window: AnyWindowHandle) {
    app.run_until_parked();
    app.update_window(window, |_, window, cx| {
        let _ = window.draw(cx);
    })
    .unwrap();
    app.run_until_parked();
}

/// Focuses the entity in the slot.
fn focus<T: gpui::Focusable>(
    app: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    slot: &Slot<T>,
) {
    let entity = slot.borrow().clone().expect("the scene's entity");
    app.update_window(window, |_, window, cx| {
        let handle = entity.focus_handle(cx);
        window.focus(&handle, cx);
    })
    .unwrap();
    frame(app, window);
}

/// Types keys into the focused element, one keystroke at a time.
fn keys(app: &mut HeadlessAppContext, window: AnyWindowHandle, keys: &str) {
    for key in keys.split_whitespace() {
        let keystroke = Keystroke::parse(key).unwrap();
        app.update_window(window, |_, window, cx| {
            if keystroke.key.chars().count() == 1 {
                window.dispatch_keystroke(keystroke, cx);
            } else {
                // A key that types nothing, as Return, which the platform
                // turns into a command rather than text.
                window.dispatch_event(
                    PlatformInput::KeyDown(KeyDownEvent {
                        keystroke,
                        is_held: false,
                        prefer_character_input: false,
                    }),
                    cx,
                );
            }
        })
        .unwrap();
        frame(app, window);
    }
}

fn input(app: &mut HeadlessAppContext, window: AnyWindowHandle, event: PlatformInput) {
    app.update_window(window, |_, window, cx| {
        window.dispatch_event(event, cx);
    })
    .unwrap();
    frame(app, window);
}

/// Presses the button at a point of the window, moves to another, and lets
/// go there, with modifiers held.
fn drag(
    app: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    from: (f32, f32),
    to: (f32, f32),
    modifiers: Modifiers,
) {
    let at = |(x, y): (f32, f32)| point(px(x), px(y));
    input(
        app,
        window,
        PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Left,
            position: at(from),
            modifiers,
            click_count: 1,
            first_mouse: false,
        }),
    );
    if from != to {
        input(
            app,
            window,
            PlatformInput::MouseMove(MouseMoveEvent {
                position: at(to),
                pressed_button: Some(MouseButton::Left),
                modifiers,
            }),
        );
    }
    input(
        app,
        window,
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position: at(to),
            modifiers,
            click_count: 1,
        }),
    );
}

fn click(app: &mut HeadlessAppContext, window: AnyWindowHandle, x: f32, y: f32) {
    drag(app, window, (x, y), (x, y), Modifiers::default());
}

/// A view of its children on the window's Gray2, padded.
fn padded(padding: f32, child: impl IntoElement) -> impl IntoElement {
    div()
        .size_full()
        .bg(rgb(GRAY_2))
        .flex()
        .items_start()
        .p(px(padding))
        .child(child)
}

/// The view, padded, at the top left of a row, or stretched across a
/// column.
struct Host(gpui::AnyView, f32, bool);

impl Render for Host {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        if self.2 {
            div()
                .size_full()
                .bg(rgb(GRAY_2))
                .flex()
                .flex_col()
                .p(px(self.1))
                .child(self.0.clone())
                .into_any_element()
        } else {
            padded(self.1, self.0.clone()).into_any_element()
        }
    }
}

/// A scene of one entity, padded on Gray2, kept in the slot.
fn hosted<T: Render>(
    name: &'static str,
    width: f32,
    height: f32,
    padding: f32,
    slot: &Slot<T>,
    make: impl FnOnce(&mut gpui::Context<T>) -> T + 'static,
) -> Scene {
    let slot = slot.clone();
    Scene::new(name, width, height, move |_, cx| {
        let entity = cx.new(make);
        *slot.borrow_mut() = Some(entity.clone());
        Host(entity.into(), padding, false)
    })
}

/// A scene of one entity stretched across a padded column on Gray2.
fn stretched<T: Render>(
    name: &'static str,
    width: f32,
    height: f32,
    padding: f32,
    slot: &Slot<T>,
    make: impl FnOnce(&mut gpui::Context<T>) -> T + 'static,
) -> Scene {
    let slot = slot.clone();
    Scene::new(name, width, height, move |_, cx| {
        let entity = cx.new(make);
        *slot.borrow_mut() = Some(entity.clone());
        Host(entity.into(), padding, true)
    })
}

pub fn scenes() -> Vec<Scene> {
    let mut scenes = Vec::new();

    // Tab to the list, Down twice, End, "c", then a click on Date.
    let list = slot::<ListBox>();
    scenes.push(
        hosted("listbox-focused", 200., 140., 10., &list, |cx| {
            ListBox::new(FRUITS, cx).size(140, 98).selected(0)
        })
        .prepare({
            let list = list.clone();
            move |app, window| {
                focus(app, window, &list);
                keys(app, window, "down down end c");
                let entity = list.borrow().clone().unwrap();
                app.update(|cx| entity.update(cx, |list, cx| list.select(4, cx)));
            }
        }),
    );
    scenes.push(Scene::elements("listbox-states", 330., 80., |_, _| {
        let accent = Theme::default().accent;
        board(
            100.,
            66.,
            [
                Box::new(move |pen: &mut Pen| {
                    draw_list_box(pen, &FRUITS, 0, Some(1), true, false, accent)
                }) as Drawn,
                Box::new(move |pen: &mut Pen| {
                    draw_list_box(pen, &FRUITS, 2, Some(3), false, false, accent)
                }),
                Box::new(move |pen: &mut Pen| {
                    draw_list_box(pen, &FRUITS, 0, Some(1), false, true, accent)
                }),
            ],
        )
    }));

    // Sort by price twice (descending), click the first row, extend it with
    // Shift-Down, add the fifth with a Command-click, and widen Name by 20.
    let table = slot::<Table>();
    scenes.push(
        hosted("table", 270., 160., 10., &table, |cx| {
            let rows = PRODUCE
                .iter()
                .map(|(name, color, price)| {
                    vec![name.to_string(), color.to_string(), format!("{price:.2}")]
                })
                .collect();
            Table::with_columns(
                vec![
                    TableColumn::new("Name").width(80),
                    TableColumn::new("Color").width(70),
                    TableColumn::new("Price").width(60).right(true),
                ],
                rows,
                cx,
            )
            .size(240, 130)
            .label("Produce")
        })
        .prepare(|app, window| {
            // The headers are 17 high from y 11; Price starts at 10 + 2 + 150.
            click(app, window, 182., 18.);
            click(app, window, 182., 18.);
            click(app, window, 20., 33.);
            keys(app, window, "shift-down");
            let command = Modifiers {
                platform: true,
                ..Modifiers::default()
            };
            drag(app, window, (20., 33. + 64.), (20., 33. + 64.), command);
            // Name's right edge is at 10 + 2 + 80.
            drag(app, window, (91., 18.), (111., 18.), Modifiers::default());
        }),
    );

    // Down twice to Letters, Right twice into To Ada, Left twice back to
    // Letters and closed, "p" for Pictures, then a click on its triangle.
    let tree = slot::<TreeView>();
    scenes.push(
        hosted("treeview", 220., 130., 10., &tree, |cx| {
            TreeView::new(sample_tree(), cx)
                .size(180, 100)
                .label("Files")
                .default_icons(false)
        })
        .prepare({
            let tree = tree.clone();
            move |app, window| {
                focus(app, window, &tree);
                keys(app, window, "down down right right left left p enter");
                // Pictures is the fourth row: 10 + 2 + 3 * 16.
                click(app, window, 10. + 2. + 3. + 6., 10. + 2. + 48. + 8.);
            }
        }),
    );

    // A2 clicked, 30 and a sum typed in, then Up and Shift-Right.
    let sheet = slot::<SheetView>();
    scenes.push(
        stretched("sheet", 350., 200., 8., &sheet, |cx| {
            let mut model = Sheet::new();
            model.set(Cell::new(0, 0), "12");
            SheetView::new(model, cx).size(330, 170).dimensions(50, 8)
        })
        .prepare(|app, window| {
            // The sheet's ring is at 8, 38, under the formula bar.
            click(app, window, 8. + 2. + 32. + 10., 38. + 2. + 36. + 5.);
            keys(
                app,
                window,
                "3 0 enter = S U M ( A 1 : A 2 ) enter up shift-right",
            );
        }),
    );
    scenes
}

const PRODUCE: [(&str, &str, f64); 9] = [
    ("Kiwi", "green", 0.5),
    ("Apricot", "orange", 1.25),
    ("Banana", "yellow", 0.25),
    ("Cherry", "red", 4.),
    ("Fig", "purple", 2.5),
    ("Lemon", "yellow", 0.75),
    ("Grape", "green", 3.),
    ("Date", "brown", 6.),
    ("Mango", "orange", 1.5),
];

const FOLDER_MINI: [&str; 6] = [
    "###.......",
    "#..######.",
    "#.......#.",
    "#.......#.",
    "#.......#.",
    "#########.",
];

fn sample_tree() -> FileTree {
    let folder = |id, title| TreeNode::new(id, title, NodeKind::Folder).with_icon(FOLDER_MINI);
    let item = TreeNode::item;
    let mut documents = folder(1, "Documents").with_children(vec![
        folder(2, "Letters").with_children(vec![item(3, "To Ada"), item(4, "To Grace")]),
        item(5, "Recipes"),
    ]);
    documents.open = true;
    FileTree::new(vec![
        documents,
        folder(6, "Pictures").with_children(vec![item(7, "Beach")]),
        item(8, "Read Me"),
    ])
}

type Drawn = Box<dyn FnOnce(&mut Pen)>;

/// Cells of the same size in a row, padded and spaced by 6, as the
/// original's boards draw them.
fn board(width: f32, height: f32, cells: impl IntoIterator<Item = Drawn>) -> impl IntoElement {
    div().flex().p(px(6.)).gap(px(6.)).children(
        cells
            .into_iter()
            .map(move |cell| drawing(width, height, move |pen, _, _| cell(pen))),
    )
}

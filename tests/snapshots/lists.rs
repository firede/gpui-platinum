//! Lists, tables, sheets, text and the navigation views, as the original's
//! golden tests draw them.

use crate::Scene;
use gpui::{
    AnyWindowHandle, AppContext as _, Entity, Focusable as _, HeadlessAppContext, Keystroke, div,
    prelude::*, px, rgb,
};
use gpui_platinum::{
    ListBox, Theme, draw_list_box,
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
    app.run_until_parked();
}

/// Types keys into the focused element, one keystroke at a time.
fn keys(app: &mut HeadlessAppContext, window: AnyWindowHandle, keys: &str) {
    for key in keys.split_whitespace() {
        app.update_window(window, |_, window, cx| {
            window.dispatch_keystroke(Keystroke::parse(key).unwrap(), cx);
        })
        .unwrap();
        app.run_until_parked();
    }
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

struct Host(gpui::AnyView, f32);

impl Render for Host {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        padded(self.1, self.0.clone())
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
        Host(entity.into(), padding)
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
                }) as Cell,
                Box::new(move |pen: &mut Pen| {
                    draw_list_box(pen, &FRUITS, 2, Some(3), false, false, accent)
                }),
                Box::new(move |pen: &mut Pen| {
                    draw_list_box(pen, &FRUITS, 0, Some(1), false, true, accent)
                }),
            ],
        )
    }));
    scenes
}

type Cell = Box<dyn FnOnce(&mut Pen)>;

/// Cells of the same size in a row, padded and spaced by 6, as the
/// original's boards draw them.
fn board(width: f32, height: f32, cells: impl IntoIterator<Item = Cell>) -> impl IntoElement {
    div().flex().p(px(6.)).gap(px(6.)).children(
        cells
            .into_iter()
            .map(move |cell| drawing(width, height, move |pen, _, _| cell(pen))),
    )
}

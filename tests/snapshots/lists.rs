//! Lists, tables, sheets, text and the navigation views, as the original's
//! golden tests draw them.

use crate::Scene;
use gpui::{
    AnyWindowHandle, AppContext as _, Entity, Focusable as _, HeadlessAppContext, KeyDownEvent,
    Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PlatformInput,
    div, point, prelude::*, px, rgb,
};
use gpui_platinum::{
    Breadcrumbs, ColumnView, DocumentTabs, Font, ListBox, Pagination, PathPopup, RichTextArea,
    Ruler, RulerState, ScrollView, SheetView, Split, Table, TableColumn, TextField, Theme,
    TreeView, draw_list_box,
    models::{Cell, FileTree, NodeKind, Sheet, TreeNode},
    paint::{Pen, drawing},
    theme::{BLACK, GRAY_2},
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
            if keystroke.key.chars().count() == 1 || keystroke.key == "space" {
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

/// Views in a padded row on Gray2, as the original's rows lay them.
struct Row {
    views: Vec<gpui::AnyView>,
    padding: f32,
    gap: f32,
}

impl Render for Row {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(GRAY_2))
            .flex()
            .items_start()
            .p(px(self.padding))
            .gap(px(self.gap))
            .children(self.views.clone())
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

    // Typed into, cut, undone, all replaced by "Ada", submitted, and the
    // last letter taken back.
    let field = slot::<TextField>();
    scenes.push(
        hosted("textfield-focused", 200., 60., 10., &field, |cx| {
            TextField::new(cx)
                .with_text("Hello")
                .size(160., 22.)
                .label("Name")
        })
        .prepare({
            let field = field.clone();
            move |app, window| {
                focus(app, window, &field);
                keys(app, window, "end , space w o r l d");
                keys(
                    app,
                    window,
                    "alt-shift-left cmd-x cmd-z cmd-a A d a enter backspace",
                );
            }
        }),
    );

    scenes.push(Scene::new("textfield-states", 360., 40., |_, cx| {
        let field = |cx: &mut gpui::App, text: &str| {
            let text = text.to_owned();
            cx.new(|cx| TextField::new(cx).with_text(text).size(100., 22.))
        };
        Row {
            views: vec![
                field(cx, "Untitled Disk").into(),
                cx.new(|cx| TextField::new(cx).placeholder("Search").size(100., 22.))
                    .into(),
                cx.new(|cx| {
                    TextField::new(cx)
                        .with_text("Read only")
                        .size(100., 22.)
                        .disabled(true)
                })
                .into(),
            ],
            padding: 8.,
            gap: 10.,
        }
    }));

    // To the end, a new line typed, then Up and Shift-Down.
    let area = slot::<TextField>();
    scenes.push(
        hosted("textarea-focused", 180., 100., 10., &area, |cx| {
            TextField::new(cx)
                .multiline(true)
                .with_text("The quick brown fox jumps over the lazy dog, again and again.")
                .size(150., 76.)
                .label("Note")
        })
        .prepare({
            let area = area.clone();
            move |app, window| {
                focus(app, window, &area);
                keys(
                    app,
                    window,
                    "cmd-down enter L i n e space t w o up shift-down",
                );
            }
        }),
    );
    // Bold and underlined text typed at the end, then "Plain" made italic
    // and its line centered from the format bar.
    let rich = slot::<RichTextArea>();
    scenes.push(
        stretched("richtext", 280., 150., 8., &rich, |cx| {
            RichTextArea::new(cx)
                .with_text("Plain ")
                .size(260., 122.)
                .show_size_menu(false)
        })
        .prepare({
            move |app, window| {
                click(app, window, 242., 48.);
                keys(
                    app,
                    window,
                    "cmd-down cmd-b b o l d cmd-b space a n d space cmd-u u n d e r l i n e d",
                );
                keys(
                    app,
                    window,
                    "cmd-up shift-right shift-right shift-right shift-right shift-right",
                );
                click(app, window, 50., 23.); // Italic
                click(app, window, 138., 23.); // Align Center
            }
        }),
    );

    // The title centered, and the last paragraph aligned right.
    let rich = slot::<RichTextArea>();
    scenes.push(
        stretched("richtext-paragraphs", 220., 150., 8., &rich, |cx| {
            RichTextArea::new(cx)
                .with_text("Title\nThe first paragraph runs on, wrapping.\nThe end")
                .size(200., 122.)
                .show_size_menu(false)
        })
        .prepare(|app, window| {
            click(app, window, 28., 48.); // in the title
            click(app, window, 138., 23.); // Align Center
            click(app, window, 28., 48.);
            keys(app, window, "cmd-down");
            click(app, window, 164., 23.); // Align Right
        }),
    );

    // A bold heading twice the size, and a line under it with italic.
    let rich = slot::<RichTextArea>();
    scenes.push(
        stretched("richtext-sizes", 240., 130., 8., &rich, |cx| {
            RichTextArea::new(cx)
                .size(220., 110.)
                .show_format_bar(false)
                .show_size_menu(false)
        })
        .prepare(|app, window| {
            click(app, window, 28., 16.);
            keys(
                app,
                window,
                "cmd-shift-. cmd-b M e n u cmd-shift-, cmd-b space f o r space t o d a y",
            );
            keys(app, window, "enter S o u p , space cmd-i b r e a d");
            // On the second line, under the taller first.
            click(app, window, 12., 54.);
        }),
    );

    // The first line of each paragraph further in, wrapping sooner.
    let area = slot::<TextField>();
    scenes.push(hosted("first-line-indent", 180., 90., 8., &area, |cx| {
        TextField::new(cx)
            .multiline(true)
            .with_text("A first paragraph long enough to wrap.\nA second one.")
            .indent(24.)
            .size(160., 70.)
            .label("Text")
    }));

    // The left margin dragged to 36, and the first-line indent to 18.
    let ruler = slot::<Ruler>();
    scenes.push(
        hosted("ruler", 300., 50., 8., &ruler, |cx| {
            let state = RulerState {
                left: 16,
                right: 240,
                first: 0,
            };
            Ruler::new(state, cx).width(284)
        })
        .prepare(|app, window| {
            let none = Modifiers::default();
            drag(app, window, (24., 25.), (48., 25.), none);
            drag(app, window, (45., 11.), (68., 11.), none);
        }),
    );

    // The path as a pop-up and as breadcrumbs, the third of five pages, a
    // card, an empty state and a status bar.
    scenes.push(Scene::new("paths-pages", 340., 300., |_, cx| {
        let path: Vec<String> = ["Platinum Disk", "Documents", "Letters"]
            .map(String::from)
            .into();
        let popup = cx.new(|cx| PathPopup::new(path.clone(), cx));
        let crumbs = cx.new(|cx| Breadcrumbs::new(path, cx));
        let pages = cx.new(|cx| Pagination::new(3, 5, cx));
        Column {
            views: vec![popup.into(), crumbs.into(), pages.into()],
        }
    }));

    // A click on Documents, then Right into Letters and Right into To Ada.
    let columns = slot::<ColumnView>();
    scenes.push(
        hosted("columnview", 450., 130., 10., &columns, |cx| {
            ColumnView::new(sample_tree(), cx).size(420, 100)
        })
        .prepare(|app, window| {
            // Documents, the first row: 10 + 2 + 8.
            click(app, window, 80., 20.);
            keys(app, window, "right right");
        }),
    );

    // Notes chosen of three documents, its pane showing its text.
    scenes.push(Scene::new("document-tabs", 340., 120., |_, cx| {
        let tabs = cx.new(|cx| {
            let docs = ["Letter", "Notes", "Budget"];
            DocumentTabs::new(docs.map(String::from).into(), cx)
                .size(320, 90)
                .selected(1)
                .content(move |index, _, _| {
                    gpui_platinum::text_in(
                        format!("The text of {}", docs[index]),
                        Font::Small,
                        BLACK,
                    )
                    .into_any_element()
                })
        });
        Host(tabs.into(), 10., false)
    }));

    // Panes stacked, the first 40 tall.
    scenes.push(Scene::new("split-vertical", 180., 140., |_, _| {
        Elements(Box::new(|| {
            padded(
                10.,
                Split::new("split", 40., text_in("Top pane"), text_in("Bottom pane"))
                    .size(160, 120)
                    .vertical(),
            )
            .into_any_element()
        }))
    }));

    // Panes stacked, the first taking 30% of the room, at least 40.
    scenes.push(Scene::new("split-percent", 180., 147., |_, _| {
        Elements(Box::new(|| {
            padded(
                10.,
                Split::new("split", 30., text_in("30%"), text_in("70%"))
                    .size(160, 127)
                    .vertical()
                    .percent(),
            )
            .into_any_element()
        }))
    }));

    // The divider dragged 30 right and moved back 8 with Left, and the
    // lines at the right scrolled down by 40.
    scenes.push(
        Scene::new("split-scrollview", 290., 130., |_, _| SplitScroll {
            position: Rc::new(std::cell::Cell::new(80.)),
            handle: gpui::ScrollHandle::new(),
        })
        .prepare(|app, window| {
            drag(app, window, (93., 30.), (123., 30.), Modifiers::default());
            keys(app, window, "left");
            input(
                app,
                window,
                PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position: point(px(200.), px(60.)),
                    delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-40.))),
                    modifiers: Modifiers::default(),
                    touch_phase: gpui::TouchPhase::Moved,
                }),
            );
        }),
    );

    scenes
}

/// A pane of text beside one of lines that scroll, the divider between
/// them where it was last put.
struct SplitScroll {
    position: Rc<std::cell::Cell<f32>>,
    handle: gpui::ScrollHandle,
}

impl Render for SplitScroll {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        let position = self.position.get();
        let lines = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .children((1..=20).map(|n| {
                let text = format!("Line {n}");
                let font = Font::Small;
                drawing(
                    font.measure(&text) as f32,
                    font.line_height() as f32,
                    move |pen, _, _| pen.text(0, font.ascent(), &text, font, BLACK),
                )
            }));
        let setter = self.position.clone();
        padded(
            10.,
            Split::new(
                "split",
                position,
                text_in("Left pane"),
                ScrollView::new("scroll", &self.handle, lines).size(260 - position as i32 - 7, 100),
            )
            .size(260, 100)
            .on_change(move |position, _, _| setter.set(position)),
        )
    }
}

/// A line of text in the small face, sized to it, as the original's TextIn.
fn text_in(text: &'static str) -> impl IntoElement {
    let font = Font::Small;
    drawing(
        font.measure(text) as f32,
        font.line_height() as f32,
        move |pen, _, _| pen.text(0, font.ascent(), text, font, BLACK),
    )
}

/// Elements made anew each frame, as a stateless scene draws them.
struct Elements(Box<dyn Fn() -> gpui::AnyElement>);

impl Render for Elements {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        (self.0)()
    }
}

/// Views in a column, padded and 10 apart, then a card, an empty state and
/// a status bar, as the original's paths-and-pages scene lays them.
struct Column {
    views: Vec<gpui::AnyView>,
}

impl Render for Column {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(GRAY_2))
            .flex()
            .flex_col()
            .items_start()
            .p(px(10.))
            .gap(px(10.))
            .children(self.views.clone())
            .child(gpui_platinum::card().child(text_in("A card of controls")))
            .child(
                div()
                    .w(px(300.))
                    .h(px(70.))
                    .child(gpui_platinum::empty_state("No documents")),
            )
            .child(gpui_platinum::status_bar(
                ["3 items", "1.2 MB available"],
                300,
            ))
    }
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

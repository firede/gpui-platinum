//! Shows every type of Mermaid diagram limu reads, drawn in the Platinum
//! look by the diagram module: a list of the types, a pop-up menu of each
//! type's examples, and the example's text beside its drawing, to edit. The
//! item under the pointer is named in the status bar. With --src, it shows a
//! file of Mermaid instead of an example. With --png, it renders the window
//! to a PNG without showing it, and with --frame too, only the drawing, in a
//! window of the look with --frame's title, as large as the drawing:
//!
//!     cargo run --example diagrams
//!     cargo run --example diagrams -- --png diagrams.png --type Sequence --example 2
//!     cargo run --example diagrams -- --src review.mmd --png review.png --frame Review --scale 2

use gpui::{prelude::*, *};
use gpui_platinum::diagram::{self, limu, limu::scene::Scene};
use gpui_platinum::window::{FRAME_WIDTH, TITLE_BAR_HEIGHT};
use gpui_platinum::{
    InputEvent, Language, ListBox, ListEvent, MenuEvent, MenuItem, PlatinumWindow,
    PlatinumWindowOptions, PopupMenu, ScrollView, Split, TextField, status_bar, theme::*,
};
use std::sync::Arc;

#[path = "support/capture.rs"]
mod capture;
#[path = "support/diagram_samples.rs"]
mod diagram_samples;

use diagram_samples::{SampleType, samples};

/// The size of the window.
const WIDTH: f32 = 1040.;
const HEIGHT: f32 = 660.;
/// The room about the controls, and between them.
const GAP: f32 = 8.;
/// The width of the column of the list of types, and of the list in it.
const LIST_WIDTH: f32 = 170.;
const LIST_BOX_WIDTH: i32 = 160;
/// The heights of the row of the pop-up menu and of the status bar.
const MENU_HEIGHT: f32 = 20.;
const STATUS_HEIGHT: f32 = 16.;
/// The width of a split's divider.
const DIVIDER: f32 = 7.;

struct Diagrams {
    types: Vec<SampleType>,
    /// The type chosen, and its example.
    typ: usize,
    example: usize,
    list: Entity<ListBox>,
    menu: Entity<PopupMenu>,
    source: Entity<TextField>,
    /// The drawing of the text, kept while text that does not read is typed.
    scene: Option<Arc<Scene>>,
    /// Why the text does not read.
    error: Option<String>,
    split: f32,
    scroll: ScrollHandle,
    /// The item under the pointer, by its role and id.
    over: Option<String>,
    /// The size the source's field was given last.
    source_size: (f32, f32),
}

impl Diagrams {
    fn new(cx: &mut Context<Self>) -> Self {
        let types = samples();
        let list = cx.new(|cx| {
            ListBox::new(types.iter().map(|t| t.title.as_str()), cx)
                .label("Types")
                .size(LIST_BOX_WIDTH, 400)
                .fill_height()
        });
        cx.subscribe(&list, |this, _, event, cx| {
            if let ListEvent::Selected(typ) = event {
                this.choose(*typ, 0, cx);
            }
        })
        .detach();
        let menu = cx.new(|cx| PopupMenu::new(["–"], cx).caption("Example:"));
        cx.subscribe(&menu, |this, _, MenuEvent(example), cx| {
            this.choose(this.typ, *example, cx);
        })
        .detach();
        let source = cx.new(|cx| TextField::new(cx).multiline(true).size(320., 400.));
        cx.subscribe(&source, |this, _, event, cx| {
            if *event == InputEvent::Changed {
                this.layout(cx);
            }
        })
        .detach();
        let mut this = Self {
            types,
            typ: 0,
            example: 0,
            list,
            menu,
            source,
            scene: None,
            error: None,
            split: 320.,
            scroll: ScrollHandle::new(),
            over: None,
            source_size: (0., 0.),
        };
        let flowchart = this.find("Flowchart").unwrap_or(0);
        this.choose(flowchart, 0, cx);
        this
    }

    /// The index of the type of a title.
    fn find(&self, title: &str) -> Option<usize> {
        self.types.iter().position(|t| t.title == title)
    }

    /// Shows an example of a type, and lays it out.
    fn choose(&mut self, typ: usize, example: usize, cx: &mut Context<Self>) {
        (self.typ, self.example) = (typ, example);
        let t = &self.types[typ];
        let (title, src) = (t.title.clone(), t.examples[example].src.clone());
        let names: Vec<MenuItem> = t
            .examples
            .iter()
            .map(|e| MenuItem::new(e.title.clone()))
            .collect();
        self.list
            .update(cx, |list, cx| list.set_selection(Some(typ), cx));
        self.menu.update(cx, |menu, cx| {
            menu.set_items(names, cx);
            menu.set_selection(example, cx);
        });
        self.source.update(cx, |field, cx| {
            field.set_label(title, cx);
            field.set_value(src, cx);
        });
        self.scene = None;
        self.scroll = ScrollHandle::new();
        self.layout(cx);
    }

    /// Lays the text out again; text that does not read keeps the last
    /// drawing, and says why.
    fn layout(&mut self, cx: &mut Context<Self>) {
        let src = self.source.read(cx).value().to_string();
        match limu::layout(&src, &diagram::layout_options(Language::English)) {
            Ok(scene) => (self.scene, self.error) = (Some(Arc::new(scene)), None),
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn status(&self) -> String {
        let t = &self.types[self.typ];
        if let Some(error) = &self.error {
            return error.clone();
        }
        if let Some(over) = &self.over {
            return over.clone();
        }
        format!("{}, {} of {}", t.title, self.example + 1, t.examples.len())
    }
}

impl Render for Diagrams {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        let (width, height) = (
            f32::from(viewport.width).round().max(600.),
            f32::from(viewport.height).round().max(400.),
        );
        let body = height - 2. * GAP - STATUS_HEIGHT;
        let right = width - 3. * GAP - LIST_WIDTH;
        let split_height = body - MENU_HEIGHT - GAP;
        self.split = self.split.clamp(120., right - 120.);
        let source_size = (self.split, split_height);
        if self.source_size != source_size {
            self.source_size = source_size;
            self.source.update(cx, |field, cx| {
                field.set_size(source_size.0, source_size.1, cx)
            });
        }
        let this = cx.entity().downgrade();
        let drawing =
            div()
                .bg(rgb(WHITE))
                .p(px(GAP))
                .when_some(self.scene.clone(), |paper, scene| {
                    let hovered = scene.clone();
                    paper.child(diagram::view("diagram", scene).on_hover(move |at, _, cx| {
                        let over = at.map(|i| {
                            let it = &hovered.items[i];
                            match &it.id {
                                Some(id) => format!("{} {id}", it.role),
                                None => it.role.to_string(),
                            }
                        });
                        this.update(cx, |this, cx| {
                            this.over = over;
                            cx.notify();
                        })
                        .ok();
                    }))
                });
        let entity = cx.entity().downgrade();
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(GRAY_2))
            .child(
                div()
                    .flex()
                    .p(px(GAP))
                    .gap(px(GAP))
                    .h(px(body + 2. * GAP))
                    .child(div().w(px(LIST_WIDTH)).h(px(body)).child(self.list.clone()))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(GAP))
                            .child(div().h(px(MENU_HEIGHT)).child(self.menu.clone()))
                            .child(
                                Split::new(
                                    "split",
                                    self.split,
                                    self.source.clone(),
                                    ScrollView::new("scroll", &self.scroll, drawing).size(
                                        (right - self.split - DIVIDER) as i32,
                                        split_height as i32,
                                    ),
                                )
                                .size(right as i32, split_height as i32)
                                .minimum(120.)
                                .on_change(
                                    move |position, _, cx| {
                                        entity
                                            .update(cx, |this, cx| {
                                                this.split = position;
                                                cx.notify();
                                            })
                                            .ok();
                                    },
                                ),
                            ),
                    ),
            )
            .child(status_bar([self.status()], width as i32))
    }
}

/// The scene alone, on white paper in a window of the look with a title, on
/// the desk's gray.
struct Framed {
    frame: Entity<PlatinumWindow>,
}

/// The room about the window, and about the drawing on its paper.
const MARGIN: f32 = 12.;
const PAPER: f32 = 12.;

impl Framed {
    /// The view, and the size it takes.
    fn size(scene: &Scene) -> (f32, f32) {
        let frame = FRAME_WIDTH as f32;
        let w = scene.width as f32 + 2. * PAPER + 2. * frame;
        let h = TITLE_BAR_HEIGHT as f32 + scene.height as f32 + 2. * PAPER + frame;
        (w + 2. * MARGIN, h + 2. * MARGIN)
    }

    fn new(scene: Arc<Scene>, title: String, cx: &mut Context<Self>) -> Self {
        let (width, height) = Self::size(&scene);
        let paper = cx.new(|_| Paper(scene));
        let frame = cx.new(|cx| {
            PlatinumWindow::new(title, cx)
                .position(MARGIN, MARGIN)
                .size(width - 2. * MARGIN, height - 2. * MARGIN)
                .options(PlatinumWindowOptions {
                    no_grow: true,
                    ..Default::default()
                })
                .content(paper)
        });
        Self { frame }
    }
}

impl Render for Framed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .bg(rgb(GRAY_5))
            .child(self.frame.clone())
    }
}

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

struct Paper(Arc<Scene>);

impl Render for Paper {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(WHITE))
            .p(px(PAPER))
            .child(diagram::view("diagram", self.0.clone()))
    }
}

fn value<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    capture::argument(arguments, &format!("--{name}"))
        .or_else(|| capture::argument(arguments, &format!("-{name}")))
}

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("{message}");
    std::process::exit(1)
}

fn main() {
    let arguments: Vec<_> = std::env::args().collect();
    let types = samples();
    let typ_title = value(&arguments, "type").unwrap_or("Flowchart");
    let Some(typ) = types.iter().position(|t| t.title == typ_title) else {
        fail(format!("no type {typ_title:?}"));
    };
    let count = types[typ].examples.len();
    let example = match value(&arguments, "example").map(str::parse::<usize>) {
        None => 1,
        Some(Ok(n)) if (1..=count).contains(&n) => n,
        Some(_) => fail(format!("{typ_title} has examples 1 to {count}")),
    } - 1;
    let src: Option<String> = value(&arguments, "src").map(|path| {
        std::fs::read_to_string(path).unwrap_or_else(|error| fail(format!("{path}: {error}")))
    });
    let given = src.clone();
    let prepare = move |this: &mut Diagrams, cx: &mut Context<Diagrams>| {
        this.choose(typ, example, cx);
        if let Some(src) = &given {
            this.source
                .update(cx, |field, cx| field.set_value(src.clone(), cx));
            this.layout(cx);
            if let Some(error) = &this.error {
                fail(error);
            }
        }
    };
    if let Some(path) = value(&arguments, "png") {
        let scale = value(&arguments, "scale")
            .and_then(|value| value.parse().ok())
            .unwrap_or(1.);
        let result = if let Some(title) = value(&arguments, "frame") {
            // The scene is laid out in an app, as the window lays it out,
            // with the platform's text for what the bitmap faces lack.
            let src = match &src {
                Some(src) => src.clone(),
                None => types[typ].examples[example].src.clone(),
            };
            let mut scene = None;
            let _ = capture::render(1., 1., |_, cx| {
                scene = Some(
                    limu::layout(&src, &diagram::layout_options(Language::English))
                        .unwrap_or_else(|error| fail(error)),
                );
                cx.new(|_| Blank)
            });
            let scene = Arc::new(scene.unwrap_or_else(|| fail("no drawing")));
            let (width, height) = Framed::size(&scene);
            let title = title.to_string();
            capture::render_png(path, width, height, scale, move |_, cx| {
                cx.new(|cx| Framed::new(scene, title, cx))
            })
        } else {
            capture::render_png(path, WIDTH, HEIGHT, scale, move |_, cx| {
                cx.new(|cx| {
                    let mut this = Diagrams::new(cx);
                    prepare(&mut this, cx);
                    this
                })
            })
        };
        result.unwrap_or_else(|error| fail(format!("render the diagrams PNG: {error}")));
        return;
    }
    gpui_platform::application().run(move |cx: &mut App| {
        gpui_platinum::init(cx);
        let bounds = Bounds::centered(None, size(px(WIDTH), px(HEIGHT)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Diagrams".into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(600.), px(400.))),
                ..Default::default()
            },
            move |_, cx| {
                cx.new(|cx| {
                    let mut this = Diagrams::new(cx);
                    prepare(&mut this, cx);
                    this
                })
            },
        )
        .expect("open the diagrams window");
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}

use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::{Datelike, Local};
use gpui::{
    AnyView, App, Bounds, Context, DispatchPhase, Entity, FocusHandle, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, RenderImage, Role, Window,
    actions, canvas, div, img, point, prelude::*, px,
};
use gpui_platinum::locale::desk;
use gpui_platinum::{
    Font, IconKind, Language, Menu, MenuBar, MenuChoice, MenuItem, PlatinumWindow, PopupWindow,
    PopupWindowEvent, Theme, WindowEvent, WindowState, button, label_in,
    models::{FileTree, NodeKind, TreeNode},
    paint::drawing,
    theme::*,
};

use super::{
    accessories::{
        Accessories, AppKind, ClockChanged, LanguageChanged, PATTERNS, PatternChanged, ThemeChanged,
    },
    catalog::Controls,
    documents::{DocumentKind, Documents, DocumentsEvent},
    file_apps::{About, Cabinet, FileAppEvent, FindFile, GetInfo, count, message},
};

actions!(platinum_desktop, [NextFocus, PreviousFocus]);
const DISK: usize = 0;
const TRASH: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Launch {
    Folder(usize),
    Accessory(AppKind),
    Document(DocumentKind),
    Controls,
    Find,
    Info(usize),
    About,
}
impl Launch {
    fn title_key(self) -> &'static str {
        match self {
            Self::Folder(_) | Self::Info(_) => "desk",
            Self::Controls => "controls",
            Self::Find => "findFile",
            Self::About => "aboutThisDesk",
            Self::Document(DocumentKind::Jotter) => "jotter",
            Self::Document(DocumentKind::Scribe) => "scribe",
            Self::Document(DocumentKind::Ledger) => "ledger",
            Self::Accessory(kind) => match kind {
                AppKind::Calculator => "calculator",
                AppKind::Puzzle => "puzzle",
                AppKind::AlarmClock => "alarmClock",
                AppKind::Appearance => "appearance",
                AppKind::DateTime => "dateTime",
                AppKind::Sound => "sound",
                AppKind::Mouse => "mouse",
                AppKind::Language => "language",
            },
        }
    }
    fn size(self) -> (i32, i32) {
        match self {
            Self::Folder(_) => (340, 230),
            Self::Controls => (820, 500),
            Self::Find => (492, 292),
            Self::Info(_) => (262, 338),
            Self::About => (332, 232),
            Self::Document(DocumentKind::Jotter) => (286, 280),
            Self::Document(DocumentKind::Scribe) => (570, 430),
            Self::Document(DocumentKind::Ledger) => (600, 350),
            Self::Accessory(kind) => {
                let (width, height) = match kind {
                    AppKind::Calculator => (156, 196),
                    AppKind::Puzzle => (168, 194),
                    AppKind::AlarmClock => (236, 170),
                    AppKind::Appearance => (300, 250),
                    AppKind::DateTime => (390, 240),
                    AppKind::Sound => (300, 220),
                    AppKind::Mouse => (300, 200),
                    AppKind::Language => (300, 280),
                };
                (width + 12, height + 32)
            }
        }
    }
}

#[derive(Clone)]
enum Content {
    Cabinet(Entity<Cabinet>),
    Accessory(Entity<Accessories>),
    Document(Entity<Documents>),
    Controls(Entity<Controls>),
    Find(Entity<FindFile>),
    Info(Entity<GetInfo>),
    About(Entity<About>),
}
impl Content {
    fn view(&self) -> AnyView {
        match self {
            Self::Cabinet(view) => view.clone().into(),
            Self::Accessory(view) => view.clone().into(),
            Self::Document(view) => view.clone().into(),
            Self::Controls(view) => view.clone().into(),
            Self::Find(view) => view.clone().into(),
            Self::Info(view) => view.clone().into(),
            Self::About(view) => view.clone().into(),
        }
    }
    fn set_theme(&self, theme: Theme, cx: &mut App) {
        match self {
            Self::Cabinet(view) => view.update(cx, |view, cx| view.set_theme(theme, cx)),
            Self::Accessory(view) => view.update(cx, |view, cx| view.set_theme(theme, cx)),
            Self::Document(view) => view.update(cx, |view, cx| view.set_theme(theme, cx)),
            Self::Controls(view) => view.update(cx, |view, cx| view.set_theme(theme, cx)),
            Self::Find(view) => view.update(cx, |view, cx| view.set_theme(theme, cx)),
            Self::Info(view) => view.update(cx, |view, cx| view.set_theme(theme, cx)),
            Self::About(view) => view.update(cx, |view, cx| view.set_theme(theme, cx)),
        }
    }
    fn set_language(&self, language: Language, cx: &mut App) {
        match self {
            Self::Cabinet(view) => view.update(cx, |view, cx| view.set_language(language, cx)),
            Self::Accessory(view) => view.update(cx, |view, cx| view.set_language(language, cx)),
            Self::Document(view) => view.update(cx, |view, cx| view.set_language(language, cx)),
            Self::Controls(view) => view.update(cx, |view, cx| view.set_language(language, cx)),
            Self::Find(view) => view.update(cx, |view, cx| view.set_language(language, cx)),
            Self::Info(view) => view.update(cx, |view, cx| view.set_language(language, cx)),
            Self::About(view) => view.update(cx, |view, cx| view.set_language(language, cx)),
        }
    }
    fn set_size(&self, width: i32, height: i32, cx: &mut App) {
        match self {
            Self::Cabinet(view) => view.update(cx, |view, cx| view.set_size(width, height, cx)),
            Self::Find(view) => view.update(cx, |view, cx| view.set_size(width, height, cx)),
            Self::Document(view) => view.update(cx, |view, cx| view.set_size(width, height, cx)),
            Self::About(view) => view.update(cx, |view, cx| view.set_size(width, height, cx)),
            _ => {}
        }
    }
}

struct DeskWindow {
    id: usize,
    kind: Launch,
    content: Content,
    frame: Entity<PlatinumWindow>,
    popup: Option<Entity<PopupWindow>>,
}

struct BackgroundCache {
    key: (u32, u32, u32, usize),
    image: Arc<RenderImage>,
}

#[derive(Clone)]
enum Command {
    Launch(Launch),
    OpenNode(usize),
    Raise(usize),
    NewFolder,
    Open,
    Close,
    CloseId(usize),
    Trash,
    TrashNode(usize),
    PutAway,
    Info,
    Duplicate,
    Quit,
    Edit(&'static str),
    Document(&'static str),
    Mode(usize),
    Sort(usize),
    Dock,
    Cleanup,
    EmptyTrash,
    ConfirmEmpty,
    Balloons,
    HideCurrent,
    HideOthers,
    ShowAll,
    Accessory(&'static str),
}

pub struct Desktop {
    focus: FocusHandle,
    files: Entity<FileTree>,
    comments: Entity<HashMap<usize, String>>,
    app_nodes: HashMap<usize, Launch>,
    node_labels: HashMap<usize, &'static str>,
    trash_parents: HashMap<usize, usize>,
    windows: Vec<DeskWindow>,
    active: Option<usize>,
    next_id: usize,
    menu: Entity<MenuBar>,
    application_menu: Entity<MenuBar>,
    commands: HashMap<(usize, Vec<usize>), Command>,
    app_commands: Vec<Command>,
    pending: Option<Command>,
    quitting: bool,
    theme: Theme,
    language: Language,
    pattern: usize,
    clock_offset: i64,
    show_date: bool,
    balloons: bool,
    emptying: bool,
    selected_icon: Option<usize>,
    icon_positions: [Option<Point<i32>>; 2],
    icon_drag: Option<(usize, Point<Pixels>, Point<i32>)>,
    desk_bounds: Bounds<Pixels>,
    background: Option<BackgroundCache>,
}

impl Desktop {
    pub fn new(language: Language, scene: bool, cx: &mut Context<Self>) -> Self {
        let mut model = FileTree::sample();
        model.roots.push(TreeNode::new(
            TRASH,
            desk(language, "wastebasket"),
            NodeKind::Folder,
        ));
        model = FileTree::new(model.roots);
        let mut app_nodes = HashMap::new();
        let mut node_labels = HashMap::new();
        fn scan(
            nodes: &mut [TreeNode],
            language: Language,
            apps: &mut HashMap<usize, Launch>,
            labels: &mut HashMap<usize, &'static str>,
        ) {
            for node in nodes {
                let launch = match node.title.as_str() {
                    "Alarm Clock" => Some(Launch::Accessory(AppKind::AlarmClock)),
                    "Calculator" => Some(Launch::Accessory(AppKind::Calculator)),
                    "Controls" => Some(Launch::Controls),
                    "Jotter" => Some(Launch::Document(DocumentKind::Jotter)),
                    "Ledger" => Some(Launch::Document(DocumentKind::Ledger)),
                    "Puzzle" => Some(Launch::Accessory(AppKind::Puzzle)),
                    "Scribe" => Some(Launch::Document(DocumentKind::Scribe)),
                    "Appearance" => Some(Launch::Accessory(AppKind::Appearance)),
                    "Date & Time" => Some(Launch::Accessory(AppKind::DateTime)),
                    "Language" => Some(Launch::Accessory(AppKind::Language)),
                    "Mouse" => Some(Launch::Accessory(AppKind::Mouse)),
                    "Sound" => Some(Launch::Accessory(AppKind::Sound)),
                    _ => None,
                };
                if let Some(launch) = launch {
                    apps.insert(node.id, launch);
                    labels.insert(node.id, launch.title_key());
                    node.title = desk(language, launch.title_key());
                } else {
                    let key = match node.title.as_str() {
                        "Platinum Disk" => Some("platinumDisk"),
                        "Applications" => Some("applications"),
                        "Documents" => Some("documents"),
                        "System Folder" => Some("systemFolder"),
                        "Control Panels" => Some("controlPanels"),
                        _ => None,
                    };
                    if let Some(key) = key {
                        labels.insert(node.id, key);
                        node.title = desk(language, key);
                    }
                    let document = match node.title.as_str() {
                        "Letter" => Some(("letter", "letterText")),
                        "Recipes" => Some(("recipes", "recipesText")),
                        "To Do" => Some(("toDo", "toDoText")),
                        "Read Me" => Some(("readMe", "readMeText")),
                        _ => None,
                    };
                    if let Some((title, text)) = document {
                        node.title = desk(language, title);
                        node.text = desk(language, text);
                    }
                }
                scan(&mut node.children, language, apps, labels);
            }
        }
        scan(&mut model.roots, language, &mut app_nodes, &mut node_labels);
        node_labels.insert(TRASH, "wastebasket");
        let files = cx.new(|_| model);
        let comments = cx.new(|_| HashMap::new());
        let menu = cx.new(|cx| MenuBar::new(Vec::new(), cx));
        let application_menu = cx.new(|cx| MenuBar::new(Vec::new(), cx));
        cx.subscribe(&menu, |this, _, event: &MenuChoice, cx| {
            this.pending = this
                .commands
                .get(&(event.menu, event.path.clone()))
                .cloned();
            cx.notify();
        })
        .detach();
        cx.subscribe(&application_menu, |this, _, event: &MenuChoice, cx| {
            this.pending = event
                .path
                .first()
                .and_then(|index| this.app_commands.get(*index))
                .cloned();
            cx.notify();
        })
        .detach();
        cx.observe(&files, |this, _, cx| {
            this.refresh_titles(cx);
            this.refresh_menus(cx);
            cx.notify();
        })
        .detach();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        })
        .detach();
        let mut this = Self {
            focus: cx.focus_handle().tab_stop(true),
            files,
            comments,
            app_nodes,
            node_labels,
            trash_parents: HashMap::new(),
            windows: Vec::new(),
            active: None,
            next_id: 1,
            menu,
            application_menu,
            commands: HashMap::new(),
            app_commands: Vec::new(),
            pending: None,
            quitting: false,
            theme: Theme::default(),
            language,
            pattern: 0,
            clock_offset: 0,
            show_date: false,
            balloons: false,
            emptying: false,
            selected_icon: None,
            icon_positions: [None, None],
            icon_drag: None,
            desk_bounds: Bounds::default(),
            background: None,
        };
        this.refresh_menus(cx);
        if scene {
            for (kind, x, y, width, height) in [
                (Launch::Folder(DISK), 24., 20., 360, 220),
                (Launch::Document(DocumentKind::Jotter), 420., 250., 260, 250),
                (Launch::Accessory(AppKind::Calculator), 620., 40., 168, 228),
                (Launch::Accessory(AppKind::Puzzle), 120., 290., 180, 226),
            ] {
                let id = this.launch(kind, cx);
                if let Some(entry) = this.windows.iter().find(|entry| entry.id == id) {
                    entry.frame.update(cx, |frame, cx| {
                        frame.state.x = x;
                        frame.state.y = y;
                        frame.state.width = width as f32;
                        frame.state.height = height as f32;
                        cx.notify();
                    });
                    entry.content.set_size(width - 12, height - 32, cx);
                }
            }
        }
        this
    }
    fn front(&self) -> Option<&DeskWindow> {
        self.active
            .and_then(|id| self.windows.iter().find(|window| window.id == id))
    }
    fn cabinet(&self) -> Option<Entity<Cabinet>> {
        match &self.front()?.content {
            Content::Cabinet(view) => Some(view.clone()),
            _ => None,
        }
    }
    fn selected_node(&self, cx: &App) -> Option<usize> {
        self.cabinet()
            .and_then(|view| view.read(cx).selection())
            .or_else(|| {
                self.selected_icon
                    .map(|index| if index == 0 { DISK } else { TRASH })
            })
    }
    fn title(&self, kind: Launch, cx: &App) -> String {
        match kind {
            Launch::Folder(id) => self
                .files
                .read(cx)
                .find(id)
                .map(|node| node.title.clone())
                .unwrap_or_default(),
            Launch::Info(id) => message(
                self.language,
                "infoTitle",
                &[(
                    "name",
                    self.files
                        .read(cx)
                        .find(id)
                        .map(|node| node.title.clone())
                        .unwrap_or_default(),
                )],
            ),
            _ => desk(self.language, kind.title_key()),
        }
    }
    fn refresh_titles(&mut self, cx: &mut Context<Self>) {
        for entry in &self.windows {
            let title = if let Content::Document(view) = &entry.content {
                view.read(cx).title().to_owned()
            } else {
                self.title(entry.kind, cx)
            };
            entry
                .frame
                .update(cx, |frame, cx| frame.set_title(title.clone(), cx));
            if let Some(popup) = &entry.popup {
                popup.update(cx, |popup, cx| popup.set_title(title, cx));
            }
        }
        self.refresh_about(cx);
    }
    fn refresh_about(&mut self, cx: &mut Context<Self>) {
        let applications: Vec<_> = self
            .windows
            .iter()
            .filter(|entry| !matches!(entry.kind, Launch::Folder(_) | Launch::About))
            .map(|entry| {
                if let Content::Document(view) = &entry.content {
                    view.read(cx).title().to_owned()
                } else {
                    self.title(entry.kind, cx)
                }
            })
            .collect();
        let height = (178 + applications.len() as i32 * 22).clamp(232, 420);
        for entry in &self.windows {
            if let Content::About(view) = &entry.content {
                view.update(cx, |view, cx| {
                    view.set_applications(applications.clone(), cx)
                });
                let state = entry.frame.read(cx).state;
                if state.height as i32 != height {
                    entry.frame.update(cx, |frame, cx| {
                        frame.state.height = height as f32;
                        cx.notify();
                    });
                    view.update(cx, |view, cx| {
                        view.set_size(state.width as i32 - 12, height - 32, cx)
                    });
                }
            }
        }
    }
    fn raise(&mut self, id: usize, cx: &mut Context<Self>) {
        if let Some(index) = self.windows.iter().position(|entry| entry.id == id) {
            let entry = self.windows.remove(index);
            entry.frame.update(cx, |frame, cx| {
                frame.state.collapsed = false;
                frame.set_active(true, cx);
            });
            self.windows.push(entry);
            self.active = Some(id);
            for entry in &self.windows {
                if entry.id != id {
                    entry
                        .frame
                        .update(cx, |frame, cx| frame.set_active(false, cx));
                }
            }
            self.selected_icon = None;
            self.refresh_menus(cx);
            self.refresh_about(cx);
            cx.notify();
        }
    }
    fn remove_window(&mut self, id: usize, cx: &mut Context<Self>) {
        self.windows.retain(|entry| entry.id != id);
        if self.active == Some(id) {
            self.active = self.windows.last().map(|entry| entry.id);
            if let Some(id) = self.active {
                self.raise(id, cx);
            }
        }
        self.refresh_menus(cx);
        self.refresh_about(cx);
        if self.quitting {
            self.pending = Some(Command::Quit);
        }
        cx.notify();
    }
    fn file_event(&mut self, event: &FileAppEvent, cx: &mut Context<Self>) {
        match *event {
            FileAppEvent::Open(id) => self.pending = Some(Command::OpenNode(id)),
            FileAppEvent::Info(id) => self.pending = Some(Command::Launch(Launch::Info(id))),
            FileAppEvent::Trash(id) => self.pending = Some(Command::TrashNode(id)),
            FileAppEvent::Selection(_) => self.refresh_menus(cx),
        }
        cx.notify();
    }
    fn launch(&mut self, kind: Launch, cx: &mut Context<Self>) -> usize {
        if kind != Launch::Document(DocumentKind::Scribe)
            && let Some(id) = self
                .windows
                .iter()
                .find(|entry| entry.kind == kind)
                .map(|entry| entry.id)
        {
            self.raise(id, cx);
            return id;
        }
        let id = self.next_id;
        self.next_id += 1;
        let content = match kind {
            Launch::Folder(folder) => {
                let files = self.files.clone();
                let view = cx.new(|cx| Cabinet::new(files, folder, cx));
                cx.subscribe(&view, |this, _, event, cx| this.file_event(event, cx))
                    .detach();
                Content::Cabinet(view)
            }
            Launch::Find => {
                let files = self.files.clone();
                let view = cx.new(|cx| FindFile::new(files, cx));
                cx.subscribe(&view, |this, _, event, cx| this.file_event(event, cx))
                    .detach();
                Content::Find(view)
            }
            Launch::Info(node) => {
                let files = self.files.clone();
                let comments = self.comments.clone();
                Content::Info(cx.new(|cx| GetInfo::new(files, node, comments, cx)))
            }
            Launch::About => Content::About(cx.new(About::new)),
            Launch::Controls => Content::Controls(cx.new(Controls::new)),
            Launch::Accessory(app) => {
                let view = cx.new(|cx| Accessories::new(app, cx));
                cx.subscribe(&view, |this, _, event: &ThemeChanged, cx| {
                    this.set_theme(event.0, cx)
                })
                .detach();
                cx.subscribe(&view, |this, _, event: &LanguageChanged, cx| {
                    this.set_language(event.0, cx)
                })
                .detach();
                cx.subscribe(&view, |this, _, event: &PatternChanged, cx| {
                    this.pattern = event.0.min(PATTERNS.len() - 1);
                    this.background = None;
                    for entry in &this.windows {
                        if let Content::Accessory(view) = &entry.content {
                            view.update(cx, |view, cx| view.set_pattern(this.pattern, cx));
                        }
                    }
                    cx.notify();
                })
                .detach();
                cx.subscribe(&view, |this, _, event: &ClockChanged, cx| {
                    this.clock_offset = event.0;
                    for entry in &this.windows {
                        if let Content::Accessory(view) = &entry.content {
                            view.update(cx, |view, cx| view.set_clock_offset(event.0, cx));
                        }
                    }
                    cx.notify();
                })
                .detach();
                view.update(cx, |view, cx| {
                    view.set_pattern(self.pattern, cx);
                    view.set_clock_offset(self.clock_offset, cx);
                });
                Content::Accessory(view)
            }
            Launch::Document(app) => {
                let files = self.files.clone();
                let view = cx.new(|cx| Documents::new(app, cx).files(files));
                cx.subscribe(&view, move |this, _, event, cx| match event {
                    DocumentsEvent::Closed => this.remove_window(id, cx),
                    DocumentsEvent::CloseCancelled => {
                        this.quitting = false;
                        cx.notify();
                    }
                    DocumentsEvent::TitleChanged(_) => {
                        this.refresh_titles(cx);
                        this.refresh_menus(cx);
                    }
                })
                .detach();
                Content::Document(view)
            }
        };
        content.set_theme(self.theme, cx);
        content.set_language(self.language, cx);
        let (width, height) = kind.size();
        content.set_size(width - 12, height - 32, cx);
        let title = if let Content::Document(view) = &content {
            view.read(cx).title().to_owned()
        } else {
            self.title(kind, cx)
        };
        let cascade = (self.windows.len() % 6) as f32;
        let view = content.view();
        let theme = self.theme;
        let frame = cx.new(|cx| {
            PlatinumWindow::new(title, cx)
                .position(24. + cascade * 28., 20. + cascade * 24.)
                .size(width as f32, height as f32)
                .theme(theme)
                .language(self.language)
                .content(view)
                .zoom_to(WindowState {
                    x: 8.,
                    y: 8.,
                    width: 800.,
                    height: 520.,
                    collapsed: false,
                })
        });
        cx.subscribe(&frame, move |this, frame, event, cx| match event {
            WindowEvent::Activated => this.raise(id, cx),
            WindowEvent::Closed => {
                this.pending = Some(Command::CloseId(id));
                cx.notify();
            }
            WindowEvent::Resized => {
                if let Some(entry) = this.windows.iter().find(|entry| entry.id == id) {
                    let state = frame.read(cx).state;
                    entry
                        .content
                        .set_size(state.width as i32 - 12, state.height as i32 - 32, cx);
                }
                cx.notify();
            }
            WindowEvent::Moved => cx.notify(),
        })
        .detach();
        self.windows.push(DeskWindow {
            id,
            kind,
            content,
            frame,
            popup: None,
        });
        self.raise(id, cx);
        id
    }
    fn open_node(&mut self, id: usize, cx: &mut Context<Self>) {
        let Some(node) = self.files.read(cx).find(id) else {
            return;
        };
        if node.kind.is_container() {
            self.launch(Launch::Folder(id), cx);
        } else if let Some(app) = self.app_nodes.get(&id).copied() {
            self.launch(app, cx);
        } else if node.kind == NodeKind::Document {
            let window_id = self.launch(Launch::Document(DocumentKind::Scribe), cx);
            if let Some(DeskWindow {
                content: Content::Document(view),
                ..
            }) = self.windows.iter().find(|entry| entry.id == window_id)
            {
                view.update(cx, |view, cx| view.open_node(id, cx));
            }
            self.refresh_titles(cx);
        }
    }
    fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        self.menu.update(cx, |menu, cx| menu.set_theme(theme, cx));
        self.application_menu
            .update(cx, |menu, cx| menu.set_theme(theme, cx));
        for entry in &self.windows {
            entry
                .frame
                .update(cx, |frame, cx| frame.set_theme(theme, cx));
            entry.content.set_theme(theme, cx);
            if let Some(popup) = &entry.popup {
                popup.update(cx, |popup, cx| popup.set_theme(theme, cx));
            }
        }
        cx.notify();
    }
    fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        if self.language == language {
            return;
        }
        self.language = language;
        self.files.update(cx, |files, cx| {
            for (id, key) in &self.node_labels {
                if let Some(node) = files.find_mut(*id) {
                    node.title = desk(language, key);
                }
            }
            cx.notify();
        });
        for entry in &self.windows {
            entry
                .frame
                .update(cx, |frame, cx| frame.set_language(language, cx));
            if let Some(popup) = &entry.popup {
                popup.update(cx, |popup, cx| popup.set_language(language, cx));
            }
            entry.content.set_language(language, cx);
        }
        self.refresh_titles(cx);
        self.refresh_menus(cx);
        cx.notify();
    }
    fn move_to_trash(&mut self, id: usize, cx: &mut Context<Self>) {
        let files = self.files.read(cx);
        let Some(node) = files.find(id) else {
            return;
        };
        if !matches!(node.kind, NodeKind::Document | NodeKind::Folder) || id == TRASH {
            return;
        }
        let path = files.path(id);
        if path.contains(&TRASH) {
            return;
        }
        let Some(parent) = path.iter().rev().nth(1).copied() else {
            return;
        };
        let moved = self.files.update(cx, |files, cx| {
            let moved = files.move_node(id, TRASH);
            cx.notify();
            moved
        });
        if moved {
            self.trash_parents.insert(id, parent);
        }
        self.refresh_menus(cx);
    }
    fn close(&mut self, id: usize, window: &mut Window, cx: &mut Context<Self>) {
        let content = self
            .windows
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.content.clone());
        if let Some(Content::Document(view)) = content {
            view.update(cx, |view, cx| view.request_close(window, cx));
        } else {
            self.remove_window(id, cx);
        }
    }
    fn execute(&mut self, command: Command, window: &mut Window, cx: &mut Context<Self>) {
        match command {
            Command::Launch(kind) => {
                self.launch(kind, cx);
            }
            Command::OpenNode(id) => self.open_node(id, cx),
            Command::Raise(id) => self.raise(id, cx),
            Command::NewFolder => {
                if let Some(view) = self.cabinet() {
                    view.update(cx, |view, cx| view.new_folder(cx));
                }
            }
            Command::Open => {
                if let Some(Content::Document(view)) =
                    self.front().map(|entry| entry.content.clone())
                {
                    view.update(cx, |view, cx| view.command("open", window, cx));
                } else if let Some(id) = self.selected_node(cx) {
                    self.open_node(id, cx);
                }
            }
            Command::Close => {
                if let Some(id) = self.active {
                    self.close(id, window, cx);
                }
            }
            Command::CloseId(id) => self.close(id, window, cx),
            Command::Trash => {
                if let Some(id) = self.selected_node(cx) {
                    self.move_to_trash(id, cx);
                }
            }
            Command::TrashNode(id) => self.move_to_trash(id, cx),
            Command::PutAway => {
                if let Some(id) = self.selected_node(cx) {
                    let parent = self
                        .trash_parents
                        .get(&id)
                        .copied()
                        .filter(|parent| self.files.read(cx).find(*parent).is_some())
                        .unwrap_or(DISK);
                    self.files.update(cx, |files, cx| {
                        files.move_node(id, parent);
                        cx.notify();
                    });
                    self.trash_parents.remove(&id);
                }
            }
            Command::Info => {
                if let Some(id) = self.selected_node(cx) {
                    self.launch(Launch::Info(id), cx);
                }
            }
            Command::Duplicate => {
                if let Some(id) = self.selected_node(cx) {
                    let files = self.files.read(cx);
                    let Some(node) = files.find(id) else {
                        return;
                    };
                    let original = node.title.clone();
                    let Some(parent) = files.path(id).iter().rev().nth(1).copied() else {
                        return;
                    };
                    let mut title =
                        message(self.language, "copyName", &[("name", original.clone())]);
                    let mut n = 2;
                    while self
                        .files
                        .read(cx)
                        .find(parent)
                        .is_some_and(|node| node.children.iter().any(|child| child.title == title))
                    {
                        title = message(
                            self.language,
                            "copyNameN",
                            &[("name", original.clone()), ("n", n.to_string())],
                        );
                        n += 1;
                    }
                    self.files.update(cx, |files, cx| {
                        files.duplicate(id, parent, title);
                        cx.notify();
                    });
                }
            }
            Command::Quit => {
                self.quitting = true;
                if let Some(id) = self
                    .windows
                    .iter()
                    .find(|entry| matches!(entry.content, Content::Document(_)))
                    .map(|entry| entry.id)
                {
                    self.raise(id, cx);
                    self.close(id, window, cx);
                } else {
                    cx.quit();
                }
            }
            Command::Edit(action) => {
                let front = self
                    .front()
                    .map(|entry| (entry.kind, entry.content.clone()));
                match front {
                    Some((Launch::Accessory(AppKind::Calculator), Content::Accessory(view)))
                        if matches!(action, "copy" | "paste") =>
                    {
                        view.update(cx, |view, cx| view.command(action, cx))
                    }
                    Some((_, Content::Document(view))) => {
                        view.update(cx, |view, cx| view.command(action, window, cx))
                    }
                    _ => {
                        use gpui_platinum::text_input;
                        let action: Box<dyn gpui::Action> = match action {
                            "undo" => Box::new(text_input::Undo),
                            "cut" => Box::new(text_input::Cut),
                            "copy" => Box::new(text_input::Copy),
                            "paste" => Box::new(text_input::Paste),
                            _ => Box::new(text_input::SelectAll),
                        };
                        window.dispatch_action(action, cx);
                    }
                }
            }
            Command::Document(action) => {
                if let Some(Content::Document(view)) =
                    self.front().map(|entry| entry.content.clone())
                {
                    view.update(cx, |view, cx| view.command(action, window, cx));
                }
            }
            Command::Accessory(action) => {
                if let Some(Content::Accessory(view)) =
                    self.front().map(|entry| entry.content.clone())
                {
                    view.update(cx, |view, cx| view.command(action, cx));
                }
            }
            Command::Mode(mode) => {
                if let Some(view) = self.cabinet() {
                    view.update(cx, |view, cx| view.set_mode(mode, cx));
                }
            }
            Command::Sort(column) => {
                if let Some(view) = self.cabinet() {
                    view.update(cx, |view, cx| view.sort(column, cx));
                }
            }
            Command::Dock => self.toggle_dock(cx),
            Command::Cleanup => self.icon_positions = [None, None],
            Command::EmptyTrash => self.emptying = true,
            Command::ConfirmEmpty => {
                self.files.update(cx, |files, cx| {
                    if let Some(trash) = files.find_mut(TRASH) {
                        trash.children.clear();
                    }
                    cx.notify();
                });
                self.trash_parents.clear();
                self.emptying = false;
            }
            Command::Balloons => self.balloons = !self.balloons,
            Command::HideCurrent | Command::HideOthers | Command::ShowAll => {
                for entry in &self.windows {
                    let hide = match command {
                        Command::HideCurrent => Some(entry.id) == self.active,
                        Command::HideOthers => Some(entry.id) != self.active,
                        _ => false,
                    };
                    entry.frame.update(cx, |frame, cx| {
                        frame.state.collapsed = hide;
                        cx.notify();
                    });
                }
            }
        }
        self.refresh_menus(cx);
        cx.notify();
    }
    fn toggle_dock(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.active else {
            return;
        };
        let Some(index) = self.windows.iter().position(|entry| entry.id == id) else {
            return;
        };
        if self.windows[index].popup.take().is_some() {
            let state = self.windows[index].frame.read(cx).state;
            self.windows[index].content.set_size(
                state.width as i32 - 12,
                state.height as i32 - 32,
                cx,
            );
            self.raise(id, cx);
            return;
        }
        if !matches!(self.windows[index].content, Content::Cabinet(_)) {
            return;
        }
        let title = self.title(self.windows[index].kind, cx);
        let content = self.windows[index].content.view();
        let state = self.windows[index].frame.read(cx).state;
        let width = state.width as i32;
        self.windows[index].content.set_size(width - 12, 150, cx);
        let popup = cx.new(|cx| {
            PopupWindow::new(title, cx)
                .position(state.x as i32)
                .size(width, 150)
                .theme(self.theme)
                .language(self.language)
                .content(content)
        });
        cx.subscribe(&popup, move |this, _, event, cx| {
            if *event == PopupWindowEvent::TornOff {
                if let Some(entry) = this.windows.iter_mut().find(|entry| entry.id == id) {
                    entry.popup = None;
                    let state = entry.frame.read(cx).state;
                    entry
                        .content
                        .set_size(state.width as i32 - 12, state.height as i32 - 32, cx);
                }
                this.raise(id, cx);
            }
        })
        .detach();
        self.windows[index].popup = Some(popup);
        cx.notify();
    }

    fn refresh_menus(&mut self, cx: &mut Context<Self>) {
        let language = self.language;
        let cabinet = self.cabinet();
        let folder = cabinet.as_ref().map(|view| view.read(cx).folder());
        let mode = cabinet.as_ref().map(|view| view.read(cx).mode());
        let selected = self.selected_node(cx);
        let in_trash = folder == Some(TRASH);
        let file_kind = selected
            .and_then(|id| self.files.read(cx).find(id))
            .map(|node| node.kind);
        let mutable_file = matches!(file_kind, Some(NodeKind::Folder | NodeKind::Document))
            && selected != Some(TRASH)
            && !in_trash;
        let front_kind = self.front().map(|entry| entry.kind);
        let document = matches!(front_kind, Some(Launch::Document(_)));
        let scribe = front_kind == Some(Launch::Document(DocumentKind::Scribe));
        let mut commands = HashMap::new();
        let mut desk_items = Vec::new();
        for (key, launch) in [
            ("aboutThisDeskMenu", Launch::About),
            ("alarmClock", Launch::Accessory(AppKind::AlarmClock)),
            ("calculator", Launch::Accessory(AppKind::Calculator)),
            ("controls", Launch::Controls),
            ("jotter", Launch::Document(DocumentKind::Jotter)),
            ("ledger", Launch::Document(DocumentKind::Ledger)),
            ("puzzle", Launch::Accessory(AppKind::Puzzle)),
            ("scribe", Launch::Document(DocumentKind::Scribe)),
        ] {
            desk_items.push(menu_item(
                language,
                key,
                Command::Launch(launch),
                0,
                desk_items.len(),
                &mut commands,
            ));
            if desk_items.len() == 1 {
                desk_items.push(MenuItem::separator());
            }
        }
        desk_items.push(MenuItem::separator());
        let panel_index = desk_items.len();
        let mut panels = Vec::new();
        for app in [
            AppKind::Appearance,
            AppKind::DateTime,
            AppKind::Language,
            AppKind::Mouse,
            AppKind::Sound,
        ] {
            let launch = Launch::Accessory(app);
            commands.insert(
                (0, vec![panel_index, panels.len()]),
                Command::Launch(launch),
            );
            panels.push(MenuItem::new(desk(language, launch.title_key())));
        }
        desk_items.push(MenuItem::new(desk(language, "controlPanels")).submenu(panels));
        let mut file = Vec::new();
        if scribe {
            file.push(
                menu_item(
                    language,
                    "save",
                    Command::Document("save"),
                    1,
                    file.len(),
                    &mut commands,
                )
                .shortcut("s"),
            );
            file.push(menu_item(
                language,
                "saveAsMenu",
                Command::Document("save-as"),
                1,
                file.len(),
                &mut commands,
            ));
            file.push(MenuItem::separator());
        }
        file.push(
            menu_item(
                language,
                "newFolder",
                Command::NewFolder,
                1,
                file.len(),
                &mut commands,
            )
            .shortcut("n")
            .disabled(cabinet.is_none() || in_trash),
        );
        file.push(
            menu_item(
                language,
                if scribe { "openMenu" } else { "open" },
                Command::Open,
                1,
                file.len(),
                &mut commands,
            )
            .shortcut("o")
            .disabled(!scribe && selected.is_none()),
        );
        file.push(
            menu_item(
                language,
                "closeWindow",
                Command::Close,
                1,
                file.len(),
                &mut commands,
            )
            .shortcut("w")
            .disabled(self.active.is_none()),
        );
        file.push(MenuItem::separator());
        file.push(menu_item(
            language,
            "findFileMenu",
            Command::Launch(Launch::Find),
            1,
            file.len(),
            &mut commands,
        ));
        file.push(MenuItem::separator());
        file.push(
            menu_item(
                language,
                "moveToWastebasket",
                Command::Trash,
                1,
                file.len(),
                &mut commands,
            )
            .shortcut("backspace")
            .disabled(!mutable_file),
        );
        file.push(
            menu_item(
                language,
                "putAway",
                Command::PutAway,
                1,
                file.len(),
                &mut commands,
            )
            .shortcut("y")
            .disabled(!in_trash || selected.is_none()),
        );
        file.push(
            menu_item(
                language,
                "getInfo",
                Command::Info,
                1,
                file.len(),
                &mut commands,
            )
            .shortcut("i")
            .disabled(selected.is_none()),
        );
        file.push(
            menu_item(
                language,
                "duplicate",
                Command::Duplicate,
                1,
                file.len(),
                &mut commands,
            )
            .shortcut("d")
            .disabled(!mutable_file),
        );
        file.push(MenuItem::separator());
        file.push(
            menu_item(
                language,
                "quit",
                Command::Quit,
                1,
                file.len(),
                &mut commands,
            )
            .shortcut("q"),
        );
        let mut edit = Vec::new();
        for (key, action, shortcut) in [
            ("undo", "undo", "z"),
            ("cut", "cut", "x"),
            ("copy", "copy", "c"),
            ("paste", "paste", "v"),
            ("selectAll", "select-all", "a"),
        ] {
            edit.push(
                menu_item(
                    language,
                    key,
                    Command::Edit(action),
                    2,
                    edit.len(),
                    &mut commands,
                )
                .shortcut(shortcut),
            );
            if edit.len() == 1 {
                edit.push(MenuItem::separator());
            }
        }
        if scribe {
            edit.push(MenuItem::separator());
            edit.push(
                menu_item(
                    language,
                    "findMenu",
                    Command::Document("find"),
                    2,
                    edit.len(),
                    &mut commands,
                )
                .shortcut("f"),
            );
        }
        let mut view = Vec::new();
        for (key, value) in [
            ("asIcons", 0),
            ("asButtons", 3),
            ("asList", 1),
            ("asColumns", 2),
        ] {
            view.push(
                menu_item(
                    language,
                    key,
                    Command::Mode(value),
                    3,
                    view.len(),
                    &mut commands,
                )
                .checked(mode == Some(value))
                .disabled(cabinet.is_none() || (in_trash && matches!(value, 2 | 3))),
            );
        }
        let sort = ["byName", "bySize", "byKind"]
            .into_iter()
            .enumerate()
            .map(|(index, key)| {
                commands.insert((3, vec![4, index]), Command::Sort(index));
                MenuItem::new(desk(language, key))
            })
            .collect();
        view.push(
            MenuItem::new(desk(language, "sortList"))
                .submenu(sort)
                .disabled(mode != Some(1)),
        );
        view.push(MenuItem::separator());
        view.push(
            menu_item(
                language,
                "asPopupWindow",
                Command::Dock,
                3,
                view.len(),
                &mut commands,
            )
            .checked(self.front().is_some_and(|entry| entry.popup.is_some()))
            .disabled(cabinet.is_none()),
        );
        let trash_count = self
            .files
            .read(cx)
            .find(TRASH)
            .map_or(0, |node| node.children.len());
        let special = vec![
            menu_item(
                language,
                "cleanUpDesktop",
                Command::Cleanup,
                4,
                0,
                &mut commands,
            ),
            menu_item(
                language,
                "emptyWastebasketMenu",
                Command::EmptyTrash,
                4,
                1,
                &mut commands,
            )
            .disabled(trash_count == 0),
            MenuItem::separator(),
            MenuItem::new(desk(language, "restart")).disabled(true),
            MenuItem::new(desk(language, "shutDown")).disabled(true),
        ];
        let mut menus = vec![
            Menu::new(desk(language, "desk"), desk_items),
            Menu::new(desk(language, "menuFile"), file),
            Menu::new(desk(language, "menuEdit"), edit),
            Menu::new(desk(language, "menuView"), view),
            Menu::new(desk(language, "menuSpecial"), special),
        ];
        if front_kind == Some(Launch::Accessory(AppKind::Puzzle)) {
            let index = menus.len();
            menus.push(Menu::new(
                desk(language, "puzzle"),
                vec![
                    menu_item(
                        language,
                        "shuffle",
                        Command::Accessory("shuffle"),
                        index,
                        0,
                        &mut commands,
                    )
                    .shortcut("r"),
                    menu_item(
                        language,
                        "solve",
                        Command::Accessory("solve"),
                        index,
                        1,
                        &mut commands,
                    ),
                ],
            ));
        }
        if front_kind == Some(Launch::Document(DocumentKind::Jotter)) {
            let index = menus.len();
            menus.push(Menu::new(
                desk(language, "menuPage"),
                vec![
                    menu_item(
                        language,
                        "nextPage",
                        Command::Document("page-next"),
                        index,
                        0,
                        &mut commands,
                    )
                    .shortcut("]"),
                    menu_item(
                        language,
                        "previousPage",
                        Command::Document("page-prev"),
                        index,
                        1,
                        &mut commands,
                    )
                    .shortcut("["),
                ],
            ));
        }
        let help_index = menus.len();
        menus.push(Menu::new(
            desk(language, "menuHelp"),
            vec![menu_item(
                language,
                if self.balloons {
                    "hideBalloons"
                } else {
                    "showBalloons"
                },
                Command::Balloons,
                help_index,
                0,
                &mut commands,
            )],
        ));
        self.commands = commands;
        self.menu.update(cx, |menu, cx| {
            menu.set_menus(menus, cx);
            menu.set_language(language, cx);
        });
        let app_name = desk(language, front_kind.map_or("desk", Launch::title_key));
        let mut apps = vec![
            MenuItem::new(message(language, "hideApp", &[("name", app_name.clone())]))
                .disabled(self.active.is_none()),
            MenuItem::new(desk(language, "hideOthers")).disabled(self.active.is_none()),
            MenuItem::new(desk(language, "showAll")),
            MenuItem::separator(),
        ];
        self.app_commands = vec![
            Command::HideCurrent,
            Command::HideOthers,
            Command::ShowAll,
            Command::ShowAll,
        ];
        for entry in self.windows.iter().rev() {
            apps.push(
                MenuItem::new(self.title(entry.kind, cx)).checked(self.active == Some(entry.id)),
            );
            self.app_commands.push(Command::Raise(entry.id));
        }
        if self.windows.is_empty() {
            apps.push(MenuItem::new(desk(language, "desk")).checked(!document));
            self.app_commands.push(Command::ShowAll);
        }
        self.application_menu.update(cx, |menu, cx| {
            menu.set_menus(vec![Menu::new(app_name, apps)], cx);
            menu.set_language(language, cx);
        });
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.emptying {
            match event.keystroke.key.as_str() {
                "escape" => {
                    self.emptying = false;
                    cx.notify();
                }
                "enter" => {
                    self.pending = Some(Command::ConfirmEmpty);
                    cx.notify();
                }
                _ => {}
            }
            cx.stop_propagation();
            return;
        }
        if let Some(DeskWindow {
            content: Content::Document(view),
            ..
        }) = self.front()
            && view.read(cx).is_modal_open(cx)
            && event.keystroke.modifiers.platform
            && !matches!(event.keystroke.key.as_str(), "a" | "c" | "x" | "v" | "z")
        {
            cx.stop_propagation();
            return;
        }
        if event.keystroke.modifiers.platform
            && event.keystroke.modifiers.shift
            && event.keystroke.key == "s"
        {
            self.pending = Some(Command::Document("save-as"));
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if self
            .menu
            .update(cx, |menu, cx| menu.dispatch_shortcut(event, window, cx))
        {
            cx.stop_propagation();
            return;
        }
        if !self.focus.is_focused(window) {
            return;
        }
        match event.keystroke.key.as_str() {
            "up" | "left" => self.selected_icon = Some(0),
            "down" | "right" => self.selected_icon = Some(1),
            "enter" | "space" => {
                if let Some(index) = self.selected_icon {
                    self.pending = Some(Command::OpenNode(if index == 0 { DISK } else { TRASH }));
                }
            }
            _ => return,
        }
        self.refresh_menus(cx);
        cx.notify();
        cx.stop_propagation();
    }
    fn icon_position(&self, index: usize) -> Point<i32> {
        self.icon_positions[index].unwrap_or_else(|| {
            point(
                (f32::from(self.desk_bounds.size.width).round() as i32 - 84).max(0),
                if index == 0 {
                    12
                } else {
                    (f32::from(self.desk_bounds.size.height).round() as i32 - 76).max(76)
                },
            )
        })
    }
    fn icon_down(
        &mut self,
        index: usize,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus, cx);
        self.active = None;
        self.selected_icon = Some(index);
        for entry in &self.windows {
            entry
                .frame
                .update(cx, |frame, cx| frame.set_active(false, cx));
        }
        if event.click_count == 2 {
            self.pending = Some(Command::OpenNode(if index == 0 { DISK } else { TRASH }));
        } else {
            self.icon_drag = Some((index, event.position, self.icon_position(index)));
        }
        self.refresh_menus(cx);
        cx.notify();
        cx.stop_propagation();
    }
    fn background(&mut self, width: f32, height: f32, scale: f32) -> Arc<RenderImage> {
        let pixel_width = (width * scale).ceil().max(1.) as u32;
        let pixel_height = (height * scale).ceil().max(1.) as u32;
        let key = (pixel_width, pixel_height, scale.to_bits(), self.pattern);
        if let Some(cached) = &self.background
            && cached.key == key
        {
            return cached.image.clone();
        }
        let pattern = PATTERNS[self.pattern];
        let buffer = image::RgbaImage::from_fn(pixel_width, pixel_height, |x, y| {
            let x = (x as f32 / scale) as usize;
            let y = (y as f32 / scale) as usize;
            let color = if pattern.bits[y % 8] & (1 << (7 - x % 8)) != 0 {
                pattern.foreground
            } else {
                pattern.background
            };
            image::Rgba([
                (color & 255) as u8,
                ((color >> 8) & 255) as u8,
                ((color >> 16) & 255) as u8,
                255,
            ])
        });
        let image = Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]));
        self.background = Some(BackgroundCache {
            key,
            image: image.clone(),
        });
        image
    }
    fn clock_title(&self) -> String {
        let now = Local::now() + chrono::Duration::seconds(self.clock_offset);
        if self.show_date {
            match self.language {
                Language::English => now.format("%a, %b %-d, %Y").to_string(),
                Language::Korean => format!("{}년 {}월 {}일", now.year(), now.month(), now.day()),
                _ => format!("{}年{}月{}日", now.year(), now.month(), now.day()),
            }
        } else {
            now.format("%H:%M").to_string()
        }
    }
}

impl Render for Desktop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(command) = self.pending.take() {
            self.execute(command, window, cx);
        }
        let viewport = window.viewport_size();
        let width = f32::from(viewport.width);
        let height = (f32::from(viewport.height) - 20.).max(1.);
        self.desk_bounds.size = gpui::size(px(width), px(height));
        let background = self.background(width, height, window.scale_factor());
        let language = self.language;
        let theme = self.theme;
        let layout = cx.entity();
        let pointer = cx.entity();
        let clock = self.clock_title();
        let clock_width = Font::Bold.measure_in(&clock, language) + 20;
        let mut body = div()
            .id("desktop-area")
            .relative()
            .w_full()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .track_focus(&self.focus)
            .key_context("Desktop")
            .tab_stop(true)
            .child(
                div()
                    .id("desktop-background")
                    .absolute()
                    .size_full()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window, cx| {
                            window.focus(&this.focus, cx);
                            this.selected_icon = None;
                            this.active = None;
                            for entry in &this.windows {
                                entry
                                    .frame
                                    .update(cx, |frame, cx| frame.set_active(false, cx));
                            }
                            this.refresh_menus(cx);
                            cx.notify();
                        }),
                    )
                    .child(
                        img(background)
                            .size_full()
                            .object_fit(gpui::ObjectFit::Fill),
                    ),
            )
            .child(
                canvas(
                    move |bounds, _, cx| {
                        layout.update(cx, |this, _| this.desk_bounds = bounds);
                    },
                    move |_, _, window, _| {
                        let movement = pointer.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                            if phase != DispatchPhase::Bubble {
                                return;
                            }
                            movement.update(cx, |this, cx| {
                                if let Some((index, start, origin)) = this.icon_drag {
                                    if !event.dragging() {
                                        this.icon_drag = None;
                                        return;
                                    }
                                    let x = origin.x
                                        + f32::from(event.position.x - start.x).round() as i32;
                                    let y = origin.y
                                        + f32::from(event.position.y - start.y).round() as i32;
                                    this.icon_positions[index] = Some(point(
                                        x.clamp(
                                            0,
                                            (f32::from(this.desk_bounds.size.width) as i32 - 84)
                                                .max(0),
                                        ),
                                        y.clamp(
                                            0,
                                            (f32::from(this.desk_bounds.size.height) as i32 - 64)
                                                .max(0),
                                        ),
                                    ));
                                    cx.notify();
                                    cx.stop_propagation();
                                }
                            });
                        });
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase != DispatchPhase::Bubble || event.button != MouseButton::Left {
                                return;
                            }
                            pointer.update(cx, |this, _| this.icon_drag = None);
                        });
                    },
                )
                .absolute()
                .size_full(),
            );
        for index in 0..2 {
            let position = self.icon_position(index);
            let selected = self.selected_icon == Some(index);
            let trash_full = self
                .files
                .read(cx)
                .find(TRASH)
                .is_some_and(|node| !node.children.is_empty());
            let title = desk(
                language,
                if index == 0 {
                    "platinumDisk"
                } else {
                    "wastebasket"
                },
            );
            let label_width = Font::Small.measure_in(&title, language) + 4;
            let label_x = (position.x + (84 - label_width) / 2)
                .clamp(0, (width as i32 - label_width).max(0))
                - position.x;
            let icon = div()
                .id(("desk-icon", index))
                .role(Role::Button)
                .aria_label(title.clone())
                .aria_selected(selected)
                .w(px(84.))
                .h(px(64.))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event, window, cx| {
                        this.icon_down(index, event, window, cx)
                    }),
                )
                .child(drawing(84., 64., move |pen, _, _| {
                    pen.language = language;
                    gpui_platinum::draw_icon(
                        pen,
                        26,
                        4,
                        if index == 0 {
                            IconKind::Disk
                        } else if trash_full {
                            IconKind::TrashFull
                        } else {
                            IconKind::Trash
                        },
                        selected,
                    );
                    pen.rect(
                        label_x,
                        38,
                        label_width,
                        13,
                        if selected { theme.accent.dark } else { WHITE },
                    );
                    pen.text(
                        label_x + 2,
                        48,
                        &title,
                        Font::Small,
                        if selected { WHITE } else { BLACK },
                    );
                }));
            let positioned = div()
                .absolute()
                .left(px(position.x as f32))
                .top(px(position.y as f32))
                .w(px(84.))
                .h(px(64.));
            body = if self.balloons {
                body.child(positioned.child(gpui_platinum::Balloon::new(
                    ("desk-help", index),
                    desk(
                        language,
                        if index == 0 {
                            "diskHelp"
                        } else {
                            "wastebasketHelp"
                        },
                    ),
                    icon,
                )))
            } else {
                body.child(positioned.child(icon))
            };
        }
        for entry in &self.windows {
            if let Some(popup) = &entry.popup {
                body = body.child(popup.clone());
            } else {
                body = body.child(entry.frame.clone());
            }
        }
        let mut root = div()
            .id("platinum-desktop")
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(gpui::rgb(WHITE))
            .capture_key_down(cx.listener(Self::key_down))
            .on_action(|_: &NextFocus, window, cx| window.focus_next(cx))
            .on_action(|_: &PreviousFocus, window, cx| window.focus_prev(cx))
            .child(
                div()
                    .flex()
                    .w_full()
                    .h(px(20.))
                    .flex_shrink_0()
                    .bg(gpui::rgb(WHITE))
                    .border_b_1()
                    .border_color(gpui::rgb(BLACK))
                    .child(div().flex_1().min_w_0().child(self.menu.clone()))
                    .child(
                        div()
                            .id("desk-clock")
                            .role(Role::Button)
                            .aria_label(clock.clone())
                            .w(px(clock_width as f32))
                            .flex_shrink_0()
                            .h(px(19.))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.show_date = !this.show_date;
                                    cx.notify();
                                    cx.stop_propagation();
                                }),
                            )
                            .child(drawing(clock_width as f32, 19., move |pen, _, _| {
                                pen.language = language;
                                pen.text(10, 14, &clock, Font::Bold, BLACK);
                            })),
                    )
                    .child(self.application_menu.clone()),
            )
            .child(body);
        if self.emptying {
            let amount = self
                .files
                .read(cx)
                .find(TRASH)
                .map_or(0, |node| node.children.len());
            let text = count(language, "emptyWastebasket", amount);
            root = root.child(
                div()
                    .absolute()
                    .inset_0()
                    .occlude()
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .w(px(360.))
                            .bg(gpui::rgb(GRAY_2))
                            .border_2()
                            .border_color(gpui::rgb(BLACK))
                            .p(px(18.))
                            .flex()
                            .flex_col()
                            .gap(px(16.))
                            .child(label_in(text, language))
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap(px(8.))
                                    .child(
                                        button("cancel-empty", desk(language, "cancel"))
                                            .theme(theme)
                                            .language(language)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.emptying = false;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        button("confirm-empty", desk(language, "ok"))
                                            .default_button()
                                            .theme(theme)
                                            .language(language)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.pending = Some(Command::ConfirmEmpty);
                                                cx.notify();
                                            })),
                                    ),
                            ),
                    ),
            );
        }
        root
    }
}

fn menu_item(
    language: Language,
    key: &str,
    command: Command,
    menu: usize,
    index: usize,
    commands: &mut HashMap<(usize, Vec<usize>), Command>,
) -> MenuItem {
    commands.insert((menu, vec![index]), command);
    MenuItem::new(desk(language, key))
}

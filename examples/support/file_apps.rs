use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Datelike, Local};

use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, MouseButton, Window,
    div, prelude::*, px,
};
use gpui_platinum::locale::desk;
use gpui_platinum::{
    ColumnEvent, ColumnView, Font, IconEvent, IconItem, IconKind, IconView, InputEvent, Language,
    PopupMenu, Table, TableColumn, TableEvent, TextField, Theme, button, label_in,
    models::{FileTree, NodeKind, TreeNode},
    paint::drawing,
    paragraph_in, text_in_language,
    theme::*,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileAppEvent {
    Open(usize),
    Selection(Option<usize>),
    Info(usize),
    Trash(usize),
    Drop {
        source: usize,
        target: usize,
        copying: bool,
    },
}

pub struct Cabinet {
    files: Entity<FileTree>,
    folder: usize,
    ids: Vec<usize>,
    selected: Option<usize>,
    mode: usize,
    width: i32,
    height: i32,
    icons: Entity<IconView>,
    table: Entity<Table>,
    columns: Entity<ColumnView>,
    button_focus: HashMap<usize, FocusHandle>,
    spring_task: Option<gpui::Task<()>>,
    context: bool,
    theme: Theme,
    language: Language,
}
impl EventEmitter<FileAppEvent> for Cabinet {}
impl Focusable for Cabinet {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        match self.mode {
            1 => self.table.focus_handle(cx),
            2 => self.columns.focus_handle(cx),
            3 if !self.ids.is_empty() => self.button_focus[&self.ids[0]].clone(),
            _ => self.icons.focus_handle(cx),
        }
    }
}
impl Cabinet {
    pub fn new(files: Entity<FileTree>, folder: usize, cx: &mut Context<Self>) -> Self {
        let icons = cx.new(|cx| {
            IconView::new(Vec::new(), cx)
                .size(328, 174)
                .always_scrollbar(true)
                .frame(false)
        });
        let table = cx.new(|cx| {
            Table::with_columns(
                vec![
                    TableColumn::new("Name").width(150),
                    TableColumn::new("Size").width(70).right(true),
                    TableColumn::new("Kind").width(100),
                ],
                Vec::new(),
                cx,
            )
            .size(328, 174)
            .always_scrollbar(true)
        });
        let columns = cx.new(|cx| ColumnView::new(FileTree::new(Vec::new()), cx).size(328, 174));
        cx.subscribe(&icons, |this, _, event, cx| match event {
            IconEvent::Selected(index) => this.select_index(*index, cx),
            IconEvent::Activated(index) => {
                if let Some(id) = this.ids.get(*index) {
                    cx.emit(FileAppEvent::Open(*id));
                }
            }
            IconEvent::Dropped {
                source,
                target,
                copying,
            } => {
                this.spring_task = None;
                if this.folder == 1000 {
                    return;
                }
                let target = target
                    .and_then(|index| this.ids.get(index))
                    .and_then(|id| this.files.read(cx).find(*id))
                    .filter(|node| node.kind.is_container())
                    .map(|node| node.id)
                    .unwrap_or(this.folder);
                cx.emit(FileAppEvent::Drop {
                    source: *source,
                    target,
                    copying: *copying,
                });
            }
            IconEvent::DragHovered(index) => {
                this.spring_task = None;
                if this.folder == 1000 {
                    return;
                }
                let target = index
                    .and_then(|index| this.ids.get(index))
                    .and_then(|id| this.files.read(cx).find(*id))
                    .filter(|node| node.kind.is_container())
                    .map(|node| node.id);
                if let Some(id) = target {
                    this.spring_task = Some(cx.spawn(async move |this, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(500))
                            .await;
                        this.update(cx, |_, cx| cx.emit(FileAppEvent::Open(id)))
                            .ok();
                    }));
                }
            }
        })
        .detach();
        cx.subscribe(&table, |this, _, event, cx| match event {
            TableEvent::Selected(index) => this.select_index(*index, cx),
            TableEvent::SelectionChanged(_) => {}
            TableEvent::Activated(index) => {
                if let Some(id) = this.ids.get(*index) {
                    cx.emit(FileAppEvent::Open(*id));
                }
            }
        })
        .detach();
        cx.subscribe(&columns, |this, _, event, cx| match event {
            ColumnEvent::Selected(id) => {
                this.selected = Some(*id);
                this.sync_selection(cx);
                cx.emit(FileAppEvent::Selection(Some(*id)));
            }
            ColumnEvent::Activated(id) => cx.emit(FileAppEvent::Open(*id)),
        })
        .detach();
        cx.observe(&files, |this, _, cx| this.refresh(cx)).detach();
        let mut this = Self {
            files,
            folder,
            ids: Vec::new(),
            selected: None,
            mode: 0,
            width: 328,
            height: 198,
            icons,
            table,
            columns,
            button_focus: HashMap::new(),
            spring_task: None,
            context: false,
            theme: Theme::default(),
            language: Language::English,
        };
        this.refresh(cx);
        this
    }
    pub fn folder(&self) -> usize {
        self.folder
    }
    pub fn selection(&self) -> Option<usize> {
        self.selected
    }
    pub fn mode(&self) -> usize {
        self.mode
    }
    pub fn set_mode(&mut self, mode: usize, cx: &mut Context<Self>) {
        self.mode = mode.min(3);
        self.sync_selection(cx);
        cx.emit(FileAppEvent::Selection(self.selected));
        cx.notify();
    }
    pub fn set_size(&mut self, width: i32, height: i32, cx: &mut Context<Self>) {
        self.width = width.max(140);
        self.height = height.max(88);
        let body = self.height - 18;
        self.icons
            .update(cx, |view, cx| view.set_size(self.width, body, cx));
        self.table
            .update(cx, |view, cx| view.set_size(self.width, body, cx));
        self.columns
            .update(cx, |view, cx| view.set_size(self.width, body, cx));
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        self.icons.update(cx, |view, cx| view.set_theme(theme, cx));
        self.table.update(cx, |view, cx| view.set_theme(theme, cx));
        self.columns
            .update(cx, |view, cx| view.set_theme(theme, cx));
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        self.icons
            .update(cx, |view, cx| view.set_language(language, cx));
        self.table
            .update(cx, |view, cx| view.set_language(language, cx));
        self.columns
            .update(cx, |view, cx| view.set_language(language, cx));
        self.refresh(cx);
    }
    pub fn sort(&mut self, column: usize, cx: &mut Context<Self>) {
        self.table
            .update(cx, |view, cx| view.sort_by(column, false, cx));
    }
    fn select_index(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected = self.ids.get(index).copied();
        self.sync_selection(cx);
        cx.emit(FileAppEvent::Selection(self.selected));
        cx.notify();
    }
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let items = self
            .files
            .read(cx)
            .find(self.folder)
            .map(|node| node.children.clone())
            .unwrap_or_default();
        self.ids = items.iter().map(|node| node.id).collect();
        self.button_focus.retain(|id, _| self.ids.contains(id));
        for id in &self.ids {
            self.button_focus
                .entry(*id)
                .or_insert_with(|| cx.focus_handle().tab_stop(true));
        }
        self.selected = self
            .selected
            .filter(|id| self.files.read(cx).path(*id).contains(&self.folder));
        let icons = items
            .iter()
            .map(|node| {
                IconItem::new(node.title.clone(), icon_kind(node))
                    .data(node.id)
                    .draggable(
                        self.folder != 1000
                            && matches!(node.kind, NodeKind::Folder | NodeKind::Document),
                    )
            })
            .collect();
        let rows = items
            .iter()
            .map(|node| {
                vec![
                    node.title.clone(),
                    if node.kind == NodeKind::Folder {
                        "—".into()
                    } else {
                        human_size(self.language, node.size())
                    },
                    kind_name(self.language, node.kind),
                ]
            })
            .collect();
        let headers = ["columnName", "columnSize", "columnKind"]
            .map(|key| desk(self.language, key))
            .to_vec();
        self.icons.update(cx, |view, cx| view.set_items(icons, cx));
        let sizes: Vec<_> = items
            .iter()
            .map(|node| (node.kind == NodeKind::Folder, node.size()))
            .collect();
        self.table.update(cx, |view, cx| {
            view.set_headers(headers, cx);
            view.set_rows(rows, cx);
            view.set_column_comparator(1, move |left, right| sizes[left].cmp(&sizes[right]), cx);
        });
        self.columns
            .update(cx, |view, cx| view.set_model(FileTree::new(items), cx));
        self.sync_selection(cx);
        cx.notify();
    }
    fn sync_selection(&self, cx: &mut Context<Self>) {
        let index = self
            .selected
            .and_then(|id| self.ids.iter().position(|candidate| *candidate == id));
        self.icons
            .update(cx, |view, cx| view.set_selection(index, cx));
        self.table
            .update(cx, |view, cx| view.set_selection(index, cx));
        self.columns
            .update(cx, |view, cx| view.set_selection(self.selected, cx));
    }
    pub fn new_folder(&mut self, cx: &mut Context<Self>) {
        let base = desk(self.language, "untitledFolder");
        let mut title = base.clone();
        let mut n = 2;
        while self
            .files
            .read(cx)
            .find(self.folder)
            .is_some_and(|node| node.children.iter().any(|child| child.title == title))
        {
            title = message(self.language, "untitledFolderN", &[("n", n.to_string())]);
            n += 1;
        }
        let id = self.files.update(cx, |files, cx| {
            let id = files.add_folder(self.folder, title);
            cx.notify();
            id
        });
        self.selected = id;
        self.sync_selection(cx);
        cx.emit(FileAppEvent::Selection(id));
        cx.notify();
    }
}
impl Render for Cabinet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.language;
        let width = self.width;
        let count = count(language, "items", self.ids.len());
        let used = self
            .files
            .read(cx)
            .roots
            .iter()
            .map(TreeNode::size)
            .sum::<usize>();
        let free = message(
            language,
            "available",
            &[(
                "size",
                human_size(language, (48usize << 20).saturating_sub(used)),
            )],
        );
        let content = match self.mode {
            1 => self.table.clone().into_any_element(),
            2 => self.columns.clone().into_any_element(),
            3 => div()
                .relative()
                .w_full()
                .h(px((self.height - 18) as f32))
                .overflow_hidden()
                .children(
                    self.ids
                        .iter()
                        .filter_map(|id| self.files.read(cx).find(*id))
                        .enumerate()
                        .map(|(index, node)| {
                            let id = node.id;
                            let kind = icon_kind(node);
                            let title = truncate_name(&node.title, 80, language);
                            let focus = self.button_focus[&id].clone();
                            let focused = focus.is_focused(window);
                            let columns = ((width - 16) / 84).max(1) as usize;
                            let theme = self.theme;
                            div()
                                .id(("file-button", id))
                                .role(gpui::Role::Button)
                                .aria_label(node.title.clone())
                                .track_focus(&focus)
                                .tab_stop(true)
                                .absolute()
                                .left(px((10 + index % columns * 84) as f32))
                                .top(px((8 + index / columns * 78) as f32))
                                .w(px(80.))
                                .h(px(74.))
                                .text_color(gpui::rgb(BLACK))
                                .active(|style| style.text_color(gpui::rgb(WHITE)))
                                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                    window.focus(&focus, cx)
                                })
                                .on_click(
                                    cx.listener(move |_, _, _, cx| cx.emit(FileAppEvent::Open(id))),
                                )
                                .on_key_down(cx.listener(
                                    move |_, event: &gpui::KeyDownEvent, _, cx| {
                                        if matches!(event.keystroke.key.as_str(), "enter" | "space")
                                        {
                                            cx.emit(FileAppEvent::Open(id));
                                            cx.stop_propagation();
                                        }
                                    },
                                ))
                                .child(drawing(80., 74., move |pen, window, _| {
                                    pen.language = language;
                                    let pressed =
                                        window.text_style().color == gpui::rgb(WHITE).into();
                                    let mut face = gpui_platinum::paint::Pen::new(56, 56);
                                    if pressed {
                                        face.sunken();
                                    } else {
                                        face.raised(GRAY_2);
                                    }
                                    let shift = i32::from(pressed);
                                    gpui_platinum::draw_icon(
                                        &mut face,
                                        12 + shift,
                                        12 + shift,
                                        kind,
                                        false,
                                    );
                                    if focused {
                                        gpui_platinum::draw_focus_ring(
                                            &mut face,
                                            0,
                                            theme.accent.dark,
                                        );
                                    }
                                    for fill in face.fills {
                                        pen.rect(
                                            fill.rect.x + 12,
                                            fill.rect.y,
                                            fill.rect.width,
                                            fill.rect.height,
                                            fill.color,
                                        );
                                    }
                                    let title_width = Font::Small.measure_in(&title, language);
                                    pen.text(
                                        (80 - title_width) / 2,
                                        69,
                                        &title,
                                        Font::Small,
                                        BLACK,
                                    );
                                }))
                        }),
                )
                .into_any_element(),
            _ => self.icons.clone().into_any_element(),
        };
        let mut root = div()
            .id("cabinet")
            .relative()
            .w_full()
            .h_full()
            .bg(gpui::rgb(WHITE))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, _, cx| {
                    this.context = true;
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .child(gpui_platinum::window_header_parts(
                count, free, width, language,
            ))
            .child(content);
        if self.context {
            let selected = self.selected;
            root = root.child(
                gpui::deferred(
                    gpui::anchored()
                        .anchor(gpui::Anchor::TopLeft)
                        .snap_to_window_with_margin(px(4.))
                        .child(
                            div()
                                .id("cabinet-context")
                                .w(px(180.))
                                .p(px(2.))
                                .bg(gpui::rgb(GRAY_2))
                                .border_1()
                                .border_color(gpui::rgb(BLACK))
                                .occlude()
                                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                    this.context = false;
                                    cx.notify();
                                    cx.stop_propagation();
                                }))
                                .children(
                                    ["open", "getInfo", "moveToWastebasket", "newFolder"]
                                        .into_iter()
                                        .enumerate()
                                        .map(|(index, key)| {
                                            button(index, desk(language, key))
                                                .width(174)
                                                .theme(self.theme)
                                                .language(language)
                                                .disabled(index < 3 && selected.is_none())
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.context = false;
                                                    if index == 3 {
                                                        this.new_folder(cx);
                                                    } else if let Some(id) = selected {
                                                        cx.emit(match index {
                                                            0 => FileAppEvent::Open(id),
                                                            1 => FileAppEvent::Info(id),
                                                            _ => FileAppEvent::Trash(id),
                                                        });
                                                    }
                                                    cx.notify();
                                                }))
                                        }),
                                ),
                        ),
                )
                .priority(2),
            );
        }
        root
    }
}

fn truncate_name(title: &str, width: i32, language: Language) -> String {
    if Font::Small.measure_in(title, language) <= width {
        return title.to_owned();
    }
    let mut result = String::new();
    for character in title.chars() {
        if Font::Small.measure_in(&format!("{result}{character}…"), language) > width {
            break;
        }
        result.push(character);
    }
    result.push('…');
    result
}

pub struct FindFile {
    files: Entity<FileTree>,
    query: Entity<TextField>,
    where_menu: Entity<PopupMenu>,
    what: Entity<PopupMenu>,
    how: Entity<PopupMenu>,
    table: Entity<Table>,
    found: Vec<usize>,
    searched: bool,
    width: i32,
    height: i32,
    theme: Theme,
    language: Language,
}
impl EventEmitter<FileAppEvent> for FindFile {}
impl Focusable for FindFile {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.query.focus_handle(cx)
    }
}
impl FindFile {
    pub fn new(files: Entity<FileTree>, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| TextField::new(cx).size(110., 22.).label("Search for"));
        let where_menu =
            cx.new(|cx| PopupMenu::new(["on local disks", "on “Platinum Disk”"], cx).width(180));
        let what = cx.new(|cx| PopupMenu::new(["name", "kind"], cx).width(70));
        let how = cx.new(|cx| {
            PopupMenu::new(["contains", "starts with", "ends with", "is"], cx).width(100)
        });
        let table = cx.new(|cx| {
            Table::with_columns(
                vec![
                    TableColumn::new("Name").width(150),
                    TableColumn::new("Kind").width(100),
                    TableColumn::new("Where").width(182),
                ],
                Vec::new(),
                cx,
            )
            .size(448, 132)
        });
        cx.subscribe(&query, |this, _, event, cx| {
            if *event == InputEvent::Submitted {
                this.search(cx);
            }
            cx.notify();
        })
        .detach();
        cx.subscribe(&table, |this, _, event, cx| {
            if let TableEvent::Activated(index) = event
                && let Some(id) = this.found.get(*index)
            {
                cx.emit(FileAppEvent::Open(*id));
            }
        })
        .detach();
        Self {
            files,
            query,
            where_menu,
            what,
            how,
            table,
            found: Vec::new(),
            searched: false,
            width: 458,
            height: 238,
            theme: Theme::default(),
            language: Language::English,
        }
    }
    pub fn set_size(&mut self, width: i32, height: i32, cx: &mut Context<Self>) {
        self.width = width.max(400);
        self.height = height.max(180);
        self.table.update(cx, |view, cx| {
            view.set_size(self.width - 20, self.height - 122, cx)
        });
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        for menu in [&self.where_menu, &self.what, &self.how] {
            menu.update(cx, |menu, cx| menu.set_theme(theme, cx));
        }
        self.query
            .update(cx, |field, cx| field.set_theme(theme, cx));
        self.table
            .update(cx, |table, cx| table.set_theme(theme, cx));
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        self.query
            .update(cx, |field, cx| field.set_language(language, cx));
        self.table.update(cx, |table, cx| {
            table.set_language(language, cx);
            table.set_headers(
                ["columnName", "columnKind", "columnWhere"]
                    .map(|key| desk(language, key))
                    .to_vec(),
                cx,
            );
        });
        let disk = self
            .files
            .read(cx)
            .roots
            .first()
            .map(|node| node.title.clone())
            .unwrap_or_default();
        self.where_menu.update(cx, |menu, cx| {
            menu.set_items(
                vec![
                    gpui_platinum::MenuItem::new(desk(language, "onLocalDisks")),
                    gpui_platinum::MenuItem::new(message(language, "onDisk", &[("name", disk)])),
                ],
                cx,
            )
        });
        self.what.update(cx, |menu, cx| {
            menu.set_items(
                ["findName", "findKind"]
                    .map(|key| gpui_platinum::MenuItem::new(desk(language, key)))
                    .to_vec(),
                cx,
            )
        });
        self.how.update(cx, |menu, cx| {
            menu.set_items(
                ["contains", "startsWith", "endsWith", "isExactly"]
                    .map(|key| gpui_platinum::MenuItem::new(desk(language, key)))
                    .to_vec(),
                cx,
            )
        });
        if self.searched {
            self.search(cx);
        }
        cx.notify();
    }
    fn search(&mut self, cx: &mut Context<Self>) {
        let query = self.query.read(cx).value().trim().to_lowercase();
        if query.is_empty() {
            return;
        }
        let what = self.what.read(cx).selection();
        let how = self.how.read(cx).selection();
        let language = self.language;
        fn visit(
            nodes: &[TreeNode],
            query: &str,
            what: usize,
            how: usize,
            language: Language,
            found: &mut Vec<usize>,
        ) {
            for node in nodes {
                let text = if what == 1 {
                    kind_name(language, node.kind)
                } else {
                    node.title.clone()
                }
                .to_lowercase();
                let matches = match how {
                    1 => text.starts_with(query),
                    2 => text.ends_with(query),
                    3 => text == query,
                    _ => text.contains(query),
                };
                if matches {
                    found.push(node.id);
                }
                visit(&node.children, query, what, how, language, found);
            }
        }
        self.found.clear();
        let files = self.files.read(cx);
        if let Some(root) = files.roots.first() {
            visit(&root.children, &query, what, how, language, &mut self.found);
        }
        let rows = self
            .found
            .iter()
            .filter_map(|id| files.find(*id))
            .map(|node| {
                vec![
                    node.title.clone(),
                    kind_name(language, node.kind),
                    parent_path(files, node.id),
                ]
            })
            .collect();
        self.table.update(cx, |table, cx| table.set_rows(rows, cx));
        self.searched = true;
        cx.notify();
    }
}
impl Render for FindFile {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.language;
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(10.))
            .w_full()
            .h_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(label_in(desk(language, "findItems"), language))
                    .child(self.where_menu.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(label_in(desk(language, "whose"), language))
                    .child(self.what.clone())
                    .child(self.how.clone())
                    .child(self.query.clone())
                    .child(
                        button("find", desk(language, "find"))
                            .width(60)
                            .default_button()
                            .disabled(self.query.read(cx).value().trim().is_empty())
                            .theme(self.theme)
                            .language(language)
                            .on_click(cx.listener(|this, _, _, cx| this.search(cx))),
                    ),
            )
            .child(if self.searched {
                self.table.clone().into_any_element()
            } else {
                div()
                    .w_full()
                    .h(px((self.height - 122) as f32))
                    .border_1()
                    .border_color(gpui::rgb(BLACK))
                    .bg(gpui::rgb(WHITE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(label_in(desk(language, "chooseWhatToFind"), language))
                    .into_any_element()
            })
            .child(label_in(
                if self.searched {
                    count(language, "foundCount", self.found.len())
                } else {
                    String::new()
                },
                language,
            ))
    }
}

pub struct GetInfo {
    files: Entity<FileTree>,
    id: usize,
    comments: Entity<TextField>,
    language: Language,
}
impl Focusable for GetInfo {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.comments.focus_handle(cx)
    }
}
impl GetInfo {
    pub fn new(
        files: Entity<FileTree>,
        id: usize,
        comments: Entity<HashMap<usize, String>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let text = comments.read(cx).get(&id).cloned().unwrap_or_default();
        let field = cx.new(|cx| {
            TextField::new(cx)
                .multiline(true)
                .with_text(text)
                .size(226., 52.)
                .label("Comments")
        });
        cx.subscribe(&field, move |_, field, event, cx| {
            if *event == InputEvent::Changed {
                let value = field.read(cx).value().to_owned();
                comments.update(cx, |comments, cx| {
                    comments.insert(id, value);
                    cx.notify();
                });
            }
        })
        .detach();
        cx.observe(&files, |_, _, cx| cx.notify()).detach();
        Self {
            files,
            id,
            comments: field,
            language: Language::English,
        }
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.comments
            .update(cx, |field, cx| field.set_theme(theme, cx));
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        self.comments
            .update(cx, |field, cx| field.set_language(language, cx));
        cx.notify();
    }
}
impl Render for GetInfo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.language;
        let files = self.files.read(cx);
        let node = files.find(self.id).cloned();
        let Some(node) = node else {
            return div().into_any_element();
        };
        let where_text = if files.path(self.id).len() < 2 {
            desk(language, "theDesktop")
        } else {
            parent_path(files, self.id)
        };
        let kind = icon_kind(&node);
        let bytes = node.size();
        let used = count(language, "bytes", bytes);
        let blocks = bytes.div_ceil(4096) * 4;
        let size = if blocks == 0 {
            message(language, "zeroKOnDisk", &[("used", used)])
        } else {
            message(
                language,
                "onDiskK",
                &[("k", blocks.to_string()), ("used", used)],
            )
        };
        div()
            .flex()
            .flex_col()
            .p(px(12.))
            .gap(px(8.))
            .w_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(drawing(32., 32., move |pen, _, _| {
                        gpui_platinum::draw_icon(pen, 0, 0, kind, false)
                    }))
                    .child(label_in(node.title.clone(), language)),
            )
            .child(gpui_platinum::separator(226))
            .children(
                [
                    (desk(language, "kindLabel"), kind_name(language, node.kind)),
                    (desk(language, "sizeLabel"), size),
                    (desk(language, "whereLabel"), where_text),
                    (
                        desk(language, "modifiedLabel"),
                        modified_date(node.modified, language),
                    ),
                ]
                .into_iter()
                .enumerate()
                .map(|(index, (label, value))| {
                    div()
                        .id(("info-row", index))
                        .flex()
                        .gap(px(8.))
                        .child(div().w(px(62.)).child(label_in(label, language)))
                        .child(wrapped(&value, 150, language))
                }),
            )
            .child(gpui_platinum::separator(226))
            .child(label_in(desk(language, "commentsLabel"), language))
            .child(self.comments.clone())
            .into_any_element()
    }
}

pub struct About {
    language: Language,
    theme: Theme,
    applications: Vec<String>,
    width: i32,
    height: i32,
}
impl About {
    pub fn new(_: &mut Context<Self>) -> Self {
        Self {
            language: Language::English,
            theme: Theme::default(),
            applications: Vec::new(),
            width: 320,
            height: 220,
        }
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        cx.notify();
    }
    pub fn set_size(&mut self, width: i32, height: i32, cx: &mut Context<Self>) {
        self.width = width;
        self.height = height;
        cx.notify();
    }
    pub fn set_applications(&mut self, applications: Vec<String>, cx: &mut Context<Self>) {
        if self.applications != applications {
            self.applications = applications;
            cx.notify();
        }
    }
}
impl Render for About {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let language = self.language;
        let theme = self.theme;
        let width = self.width - 24;
        let mut rows = vec![(desk(language, "desk"), 12usize << 20)];
        rows.extend(
            self.applications
                .iter()
                .map(|title| (title.clone(), application_memory(title))),
        );
        let used = rows.iter().map(|(_, size)| size).sum::<usize>();
        let memory = message(
            language,
            "memory",
            &[
                ("total", human_size(language, 64usize << 20)),
                (
                    "free",
                    human_size(language, (64usize << 20).saturating_sub(used)),
                ),
            ],
        );
        div()
            .flex()
            .flex_col()
            .w(px(self.width as f32))
            .h(px(self.height as f32))
            .gap(px(10.))
            .p(px(12.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(label_in(desk(language, "platinumDesk"), language))
                    .child(text_in_language(
                        desk(language, "version").replace("MyGo", "GPUI"),
                        Font::Small,
                        BLACK,
                        language,
                    )),
            )
            .child(gpui_platinum::separator(width))
            .child(
                div()
                    .id("application-memory")
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(
                        rows.into_iter()
                            .enumerate()
                            .map(move |(index, (title, bytes))| {
                                div()
                                    .id(("memory-row", index))
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .flex_shrink_0()
                                    .child(
                                        div().w(px(100.)).flex_shrink_0().overflow_hidden().child(
                                            text_in_language(
                                                title.clone(),
                                                Font::Small,
                                                BLACK,
                                                language,
                                            ),
                                        ),
                                    )
                                    .child(div().w(px(60.)).flex_shrink_0().child(
                                        text_in_language(
                                            human_size(language, bytes),
                                            Font::Small,
                                            BLACK,
                                            language,
                                        ),
                                    ))
                                    .child(
                                        div()
                                            .id("memory-progress")
                                            .role(gpui::Role::ProgressIndicator)
                                            .aria_label(title)
                                            .aria_numeric_value(bytes as f64)
                                            .aria_max_numeric_value((16usize << 20) as f64)
                                            .child(drawing(
                                                (width - 176).max(40) as f32,
                                                10.,
                                                move |pen, _, _| {
                                                    gpui_platinum::draw_progress(
                                                        pen,
                                                        bytes as f64 / (16usize << 20) as f64,
                                                        theme.accent,
                                                    );
                                                },
                                            )),
                                    )
                            }),
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .child(paragraph_in(memory, Font::Small, width, language)),
            )
    }
}

pub fn icon_kind(node: &TreeNode) -> IconKind {
    match node.kind {
        NodeKind::Folder => IconKind::Folder,
        NodeKind::Disk => IconKind::Disk,
        NodeKind::Document => IconKind::Document,
        _ => [
            ("calculator", IconKind::Calculator),
            ("jotter", IconKind::Jotter),
            ("scribe", IconKind::Scribe),
            ("ledger", IconKind::Ledger),
            ("puzzle", IconKind::Puzzle),
            ("alarmClock", IconKind::Clock),
            ("dateTime", IconKind::DateTime),
            ("language", IconKind::Language),
            ("sound", IconKind::Sound),
            ("mouse", IconKind::Mouse),
            ("controls", IconKind::Controls),
        ]
        .into_iter()
        .find(|(key, _)| {
            gpui_platinum::locale::LANGUAGES
                .into_iter()
                .any(|language| desk(language, key) == node.title)
        })
        .map(|(_, kind)| kind)
        .unwrap_or(IconKind::Appearance),
    }
}
pub fn kind_name(language: Language, kind: NodeKind) -> String {
    desk(
        language,
        match kind {
            NodeKind::Folder => "kindFolder",
            NodeKind::Document => "kindDocument",
            NodeKind::Application => "kindApp",
            NodeKind::Disk => "kindDisk",
            NodeKind::ControlPanel => "kindPanel",
        },
    )
}
fn parent_path(files: &FileTree, id: usize) -> String {
    let mut path = files.path(id);
    path.pop();
    path.into_iter()
        .filter_map(|id| files.find(id))
        .map(|node| node.title.as_str())
        .collect::<Vec<_>>()
        .join(":")
}
pub fn human_size(language: Language, bytes: usize) -> String {
    if bytes >= 1 << 20 {
        message(
            language,
            "sizeMB",
            &[("size", (bytes as f64 / (1 << 20) as f64).to_string())],
        )
    } else if bytes >= 1 << 10 {
        message(
            language,
            "sizeK",
            &[("size", bytes.div_ceil(1024).to_string())],
        )
    } else {
        count(language, "bytes", bytes)
    }
}
pub fn count(language: Language, key: &str, count: usize) -> String {
    message(language, key, &[("count", count.to_string())])
}
pub fn message(language: Language, key: &str, arguments: &[(&str, String)]) -> String {
    gpui_platinum::locale::format_message(&desk(language, key), arguments)
}

fn wrapped(text: &str, width: i32, language: Language) -> AnyElement {
    paragraph_in(text.to_owned(), Font::Small, width, language).into_any_element()
}

fn application_memory(title: &str) -> usize {
    (1usize << 20) + title.len() * (96usize << 10)
}

fn modified_date(time: SystemTime, language: Language) -> String {
    let date: DateTime<Local> = time.into();
    match language {
        Language::English => date.format("%a, %b %-d, %Y, %-I:%M %p").to_string(),
        Language::SimplifiedChinese | Language::TraditionalChinese => {
            let weekdays = ["日", "一", "二", "三", "四", "五", "六"];
            format!(
                "{}年{}月{}日 周{} {}",
                date.year(),
                date.month(),
                date.day(),
                weekdays[date.weekday().num_days_from_sunday() as usize],
                date.format("%H:%M")
            )
        }
        Language::Japanese => {
            let weekdays = ["日", "月", "火", "水", "木", "金", "土"];
            format!(
                "{}年{}月{}日({}) {}",
                date.year(),
                date.month(),
                date.day(),
                weekdays[date.weekday().num_days_from_sunday() as usize],
                date.format("%H:%M")
            )
        }
        Language::Korean => {
            let weekdays = ["일", "월", "화", "수", "목", "금", "토"];
            format!(
                "{}년 {}월 {}일 ({}) {}",
                date.year(),
                date.month(),
                date.day(),
                weekdays[date.weekday().num_days_from_sunday() as usize],
                date.format("%H:%M")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn dragging_a_file_onto_a_folder_reports_the_target_and_option_copy(
        cx: &mut gpui::TestAppContext,
    ) {
        let (files, source, target) = cx.update(|cx| {
            let mut model = FileTree::new(vec![TreeNode::new(0, "Disk", NodeKind::Disk)]);
            let source = model.add_document(0, "Source", "text").unwrap();
            let target = model.add_folder(0, "Target").unwrap();
            (cx.new(|_| model), source, target)
        });
        let (cabinet, cx) = cx.add_window_view(|_, cx| Cabinet::new(files, 0, cx));
        let drops = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let observed = drops.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&cabinet, move |_, event: &FileAppEvent, _| {
                if let FileAppEvent::Drop { .. } = event {
                    observed.borrow_mut().push(*event);
                }
            })
        });
        let start = gpui::point(px(42.), px(56.));
        let end = gpui::point(px(126.), px(56.));
        let modifiers = gpui::Modifiers {
            alt: true,
            ..Default::default()
        };
        cx.simulate_mouse_move(start, None, modifiers);
        cx.simulate_mouse_down(start, MouseButton::Left, modifiers);
        cx.simulate_mouse_move(
            gpui::point(px(54.), px(56.)),
            Some(MouseButton::Left),
            modifiers,
        );
        cx.simulate_mouse_move(end, Some(MouseButton::Left), modifiers);
        cx.simulate_mouse_up(end, MouseButton::Left, modifiers);
        assert_eq!(
            *drops.borrow(),
            vec![FileAppEvent::Drop {
                source,
                target,
                copying: true
            }]
        );
    }

    #[gpui::test]
    fn cabinet_modes_keep_one_file_selection(cx: &mut gpui::TestAppContext) {
        let (cabinet, letter, recipes) = cx.update(|cx| {
            let files = cx.new(|_| FileTree::sample());
            let folder = files.read(cx).roots[0]
                .children
                .iter()
                .find(|node| node.title == "Documents")
                .unwrap();
            let letter = folder
                .children
                .iter()
                .position(|node| node.title == "Letter")
                .unwrap();
            let recipes = folder
                .children
                .iter()
                .position(|node| node.title == "Recipes")
                .unwrap();
            let folder = folder.id;
            (
                cx.new(|cx| Cabinet::new(files, folder, cx)),
                letter,
                recipes,
            )
        });
        cx.update(|cx| {
            cabinet.update(cx, |cabinet, cx| {
                cabinet
                    .icons
                    .update(cx, |icons, cx| icons.select(letter, cx))
            })
        });
        cx.update(|cx| cabinet.update(cx, |cabinet, cx| cabinet.set_mode(1, cx)));
        cx.update(|cx| {
            cabinet.update(cx, |cabinet, cx| {
                cabinet
                    .table
                    .update(cx, |table, cx| table.select(recipes, cx))
            })
        });
        cx.update(|cx| cabinet.update(cx, |cabinet, cx| cabinet.set_mode(0, cx)));
        cx.read(|cx| {
            let cabinet = cabinet.read(cx);
            assert_eq!(cabinet.selection(), Some(cabinet.ids[recipes]));
            assert_eq!(cabinet.icons.read(cx).selection(), Some(recipes));
            assert_eq!(cabinet.table.read(cx).selection(), Some(recipes));
            assert_eq!(
                cabinet.columns.read(cx).selection(),
                Some(cabinet.ids[recipes])
            );
        });
    }

    #[gpui::test]
    fn new_folder_selects_the_visible_new_item(cx: &mut gpui::TestAppContext) {
        let cabinet = cx.update(|cx| {
            let files = cx.new(|_| FileTree::sample());
            cx.new(|cx| Cabinet::new(files, 0, cx))
        });
        cx.update(|cx| {
            cabinet.update(cx, |cabinet, cx| {
                cabinet.icons.update(cx, |icons, cx| icons.select(0, cx))
            })
        });
        cx.update(|cx| cabinet.update(cx, |cabinet, cx| cabinet.new_folder(cx)));
        cx.read(|cx| {
            let cabinet = cabinet.read(cx);
            let selected = cabinet.selection().unwrap();
            let index = cabinet.icons.read(cx).selection().unwrap();
            assert_eq!(cabinet.ids[index], selected);
            assert_eq!(
                cabinet.files.read(cx).find(selected).unwrap().title,
                "untitled folder"
            );
        });
    }

    #[test]
    fn memory_rows_use_the_original_application_estimate() {
        assert_eq!(application_memory("Scribe"), (1 << 20) + 6 * (96 << 10));
        assert_eq!(
            application_memory("수첩"),
            (1 << 20) + "수첩".len() * (96 << 10)
        );
    }

    #[test]
    fn about_memory_keeps_the_numeric_value_on_one_line() {
        let memory = message(
            Language::English,
            "memory",
            &[
                ("total", human_size(Language::English, 64 << 20)),
                ("free", human_size(Language::English, 48 << 20)),
            ],
        );
        let lines = gpui_platinum::wrap_in(&memory, Font::Small, 296, Language::English);
        let free = human_size(Language::English, 48 << 20);
        assert!(lines.iter().any(|line| line.contains(&free)), "{lines:?}");
    }
}

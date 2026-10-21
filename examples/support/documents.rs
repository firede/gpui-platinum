use gpui::{
    AnyElement, App, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable,
    MouseButton, Render, Role, Subscription, Window, div, prelude::*, px, rgb,
};
use gpui_platinum::{
    Alert, AlertEvent, AlertKind, Alignment, FileDialogEvent, FindEvent, FindReplace, Font,
    InputEvent, Language, OpenDialog, PageSetup, PageSetupState, RichTextArea, Ruler, RulerChanged,
    RulerState, Run, SaveDialog, SaveTarget, SheetEvent, SheetView, StyleAttribute, TextField,
    TextStyle, Theme,
    locale::{desk, format_message},
    models::{Cell, FileTree, Sheet},
    paint::drawing,
    text_in_language, text_input,
    theme::*,
    window::GROW_BOX_SIZE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentKind {
    Jotter,
    Scribe,
    Ledger,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentsEvent {
    Closed,
    CloseCancelled,
    TitleChanged(String),
}
#[derive(Clone, Copy)]
enum Pending {
    Close,
    New,
    Open,
}

enum Content {
    Jotter {
        pages: Vec<Entity<TextField>>,
        page: usize,
        corner: FocusHandle,
    },
    Scribe {
        area: Entity<RichTextArea>,
        ruler: Entity<Ruler>,
        show_ruler: bool,
    },
    Ledger(Entity<SheetView>),
}

pub struct Documents {
    kind: DocumentKind,
    content: Content,
    files: Entity<FileTree>,
    file: Option<usize>,
    title: String,
    saved: String,
    theme: Theme,
    language: Language,
    width: i32,
    height: i32,
    initialized: bool,
    files_dirty: bool,
    open: Option<Entity<OpenDialog>>,
    save: Option<Entity<SaveDialog>>,
    find: Option<Entity<FindReplace>>,
    page_setup: Entity<PageSetup>,
    alert: Entity<Alert>,
    alert_choice: Option<usize>,
    pending: Option<Pending>,
    ready: Option<Pending>,
    toolbar_focus: [FocusHandle; 3],
    _subscriptions: Vec<Subscription>,
    file_subscriptions: Vec<Subscription>,
}
impl EventEmitter<DocumentsEvent> for Documents {}
impl Focusable for Documents {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        match &self.content {
            Content::Jotter { pages, page, .. } => pages[*page].focus_handle(cx),
            Content::Scribe { area, .. } => area.focus_handle(cx),
            Content::Ledger(sheet) => sheet.focus_handle(cx),
        }
    }
}
impl Documents {
    pub fn file(&self) -> Option<usize> {
        self.file
    }
    #[cfg(test)]
    pub(super) fn text(&self, cx: &App) -> String {
        self.editor(cx)
            .map(|editor| editor.read(cx).value().to_owned())
            .unwrap_or_default()
    }
    pub fn new(kind: DocumentKind, cx: &mut Context<Self>) -> Self {
        let mut subscriptions = Vec::new();
        let content = match kind {
            DocumentKind::Jotter => {
                let pages: Vec<_> = (0..8)
                    .map(|page| {
                        cx.new(|cx| {
                            TextField::new(cx).multiline(true).label("Page").with_text(
                                if page == 0 {
                                    desk(Language::English, "jotterNotes")
                                } else {
                                    String::new()
                                },
                            )
                        })
                    })
                    .collect();
                for page in &pages {
                    subscriptions.push(cx.subscribe(page, |_, _, _: &InputEvent, cx| cx.notify()));
                }
                Content::Jotter {
                    pages,
                    page: 0,
                    corner: cx.focus_handle(),
                }
            }
            DocumentKind::Scribe => {
                let area = cx.new(RichTextArea::new);
                // The document fills the window, its scroll bar over the
                // window's line, down to the row of the resize corner.
                area.read(cx)
                    .editor()
                    .update(cx, |editor, cx| editor.set_document(true, cx));
                let ruler = cx.new(|cx| {
                    Ruler::new(
                        RulerState {
                            left: 8,
                            right: 340,
                            first: 0,
                        },
                        cx,
                    )
                    .width(380)
                });
                subscriptions.push(cx.subscribe(&area, |_, _, _: &InputEvent, cx| cx.notify()));
                subscriptions.push(
                    cx.subscribe(&ruler, |this: &mut Self, _, _: &RulerChanged, cx| {
                        this.apply_ruler(cx)
                    }),
                );
                Content::Scribe {
                    area,
                    ruler,
                    show_ruler: true,
                }
            }
            DocumentKind::Ledger => {
                let sheet = cx.new(|cx| SheetView::new(Sheet::ledger(), cx));
                subscriptions.push(cx.subscribe(&sheet, |_, _, _: &SheetEvent, cx| cx.notify()));
                Content::Ledger(sheet)
            }
        };
        let editor = match &content {
            Content::Jotter { pages, page, .. } => Some(pages[*page].clone()),
            Content::Scribe { area, .. } => Some(area.read(cx).editor()),
            Content::Ledger(_) => None,
        };
        let find = editor.map(|editor| cx.new(|cx| FindReplace::new(editor, cx)));
        if let Some(find) = &find {
            subscriptions.push(cx.subscribe(find, |this: &mut Self, find, event, cx| {
                let note = match event {
                    FindEvent::NotFound => desk(this.language, "notFound"),
                    FindEvent::Replaced(count) => {
                        desk(this.language, "replaced").replace("{count}", &count.to_string())
                    }
                    FindEvent::Found(_) => String::new(),
                };
                find.update(cx, |find, cx| find.set_note(note, cx));
            }));
        }
        let alert = cx.new(|cx| {
            Alert::new(
                AlertKind::Caution,
                "",
                "",
                [
                    desk(Language::English, "dontSave"),
                    desk(Language::English, "cancel"),
                    desk(Language::English, "save"),
                ],
                cx,
            )
        });
        subscriptions.push(cx.subscribe(&alert, |this: &mut Self, _, event, cx| {
            let AlertEvent::Chosen(choice) = event;
            this.alert_choice = Some(*choice);
            cx.notify();
        }));
        let title = desk(
            Language::English,
            match kind {
                DocumentKind::Jotter => "jotter",
                DocumentKind::Scribe => "untitled",
                DocumentKind::Ledger => "ledger",
            },
        );
        let mut this = Self {
            kind,
            content,
            files: cx.new(|_| FileTree::sample()),
            file: None,
            title,
            saved: String::new(),
            theme: Theme::default(),
            language: Language::English,
            width: 400,
            height: 265,
            initialized: false,
            files_dirty: true,
            open: None,
            save: None,
            find,
            page_setup: cx.new(|cx| PageSetup::new(PageSetupState::default(), cx)),
            alert,
            alert_choice: None,
            pending: None,
            ready: None,
            toolbar_focus: std::array::from_fn(|_| cx.focus_handle()),
            _subscriptions: subscriptions,
            file_subscriptions: Vec::new(),
        };
        this.saved = this.fingerprint(cx);
        this.resize(cx);
        this
    }
    pub fn files(mut self, files: Entity<FileTree>) -> Self {
        self.files = files;
        self.files_dirty = true;
        self
    }
    pub fn is_modal_open(&self, cx: &App) -> bool {
        self.alert.read(cx).is_open()
            || self.page_setup.read(cx).is_open()
            || self
                .find
                .as_ref()
                .is_some_and(|find| find.read(cx).is_open())
            || self
                .open
                .as_ref()
                .is_some_and(|open| open.read(cx).is_open())
            || self
                .save
                .as_ref()
                .is_some_and(|save| save.read(cx).is_open())
    }

    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        match &self.content {
            Content::Jotter { pages, .. } => {
                for page in pages {
                    page.update(cx, |field, cx| field.set_theme(theme, cx));
                }
            }
            Content::Scribe { area, ruler, .. } => {
                area.update(cx, |area, cx| area.set_theme(theme, cx));
                ruler.update(cx, |ruler, cx| ruler.set_theme(theme, cx));
            }
            Content::Ledger(sheet) => sheet.update(cx, |sheet, cx| sheet.set_theme(theme, cx)),
        }
        if let Some(find) = &self.find {
            find.update(cx, |find, cx| find.set_theme(theme, cx));
        }
        if let Some(open) = &self.open {
            open.update(cx, |open, cx| open.set_theme(theme, cx));
        }
        if let Some(save) = &self.save {
            save.update(cx, |save, cx| save.set_theme(theme, cx));
        }
        self.alert
            .update(cx, |alert, cx| alert.set_theme(theme, cx));
        self.page_setup
            .update(cx, |setup, cx| setup.set_theme(theme, cx));
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        match &self.content {
            Content::Jotter { pages, .. } => {
                for page in pages {
                    page.update(cx, |field, cx| {
                        field.set_language(language, cx);
                        field.set_label(desk(language, "page"), cx);
                    });
                }
                if !self.initialized {
                    pages[0].update(cx, |field, cx| {
                        field.set_value(desk(language, "jotterNotes"), cx)
                    });
                }
            }
            Content::Scribe { area, ruler, .. } => {
                area.update(cx, |area, cx| area.set_language(language, cx));
                ruler.update(cx, |ruler, cx| ruler.set_language(language, cx));
            }
            Content::Ledger(sheet) => {
                sheet.update(cx, |sheet, cx| {
                    sheet.set_language(language, cx);
                    if !self.initialized {
                        for (row, key) in [
                            (0, "item"),
                            (1, "rent"),
                            (2, "groceries"),
                            (3, "lampOil"),
                            (4, "books"),
                            (6, "total"),
                            (7, "average"),
                        ] {
                            sheet
                                .model_mut()
                                .set(Cell::new(0, row), desk(language, key));
                        }
                        sheet
                            .model_mut()
                            .set(Cell::new(1, 0), desk(language, "cost"));
                    }
                });
            }
        }
        if self.file.is_none() {
            self.title = desk(
                language,
                match self.kind {
                    DocumentKind::Jotter => "jotter",
                    DocumentKind::Scribe => "untitled",
                    DocumentKind::Ledger => "ledger",
                },
            );
            cx.emit(DocumentsEvent::TitleChanged(self.title.clone()));
        }
        if let Some(find) = &self.find {
            find.update(cx, |find, cx| find.set_language(language, cx));
        }
        if let Some(open) = &self.open {
            open.update(cx, |open, cx| open.set_language(language, cx));
        }
        if let Some(save) = &self.save {
            save.update(cx, |save, cx| {
                save.set_language(language, cx);
                save.set_prompt(desk(language, "saveAsLabel"), cx);
            });
        }
        self.alert.update(cx, |alert, cx| {
            alert.set_language(language, cx);
            alert.set_buttons(
                ["dontSave", "cancel", "save"]
                    .map(|key| desk(language, key).into())
                    .to_vec(),
                cx,
            );
        });
        self.page_setup
            .update(cx, |setup, cx| setup.set_language(language, cx));
        cx.notify();
    }
    pub fn set_size(&mut self, width: i32, height: i32, cx: &mut Context<Self>) {
        self.width = width.max(120);
        self.height = height.max(90);
        self.resize(cx);
    }
    fn resize(&mut self, cx: &mut Context<Self>) {
        match &self.content {
            Content::Jotter { pages, .. } => {
                for page in pages {
                    page.update(cx, |field, cx| {
                        field.set_size(self.width as f32, (self.height - 20).max(60) as f32, cx)
                    });
                }
            }
            Content::Scribe {
                area,
                ruler,
                show_ruler,
            } => {
                // Under the toolbar and the ruler, over the foot.
                let height = self.height - 28 - GROW_BOX_SIZE - if *show_ruler { 22 } else { 0 };
                area.update(cx, |area, cx| {
                    area.set_size(self.width as f32, height.max(80) as f32, cx)
                });
                ruler.update(cx, |ruler, cx| ruler.set_width(self.width, cx));
            }
            Content::Ledger(sheet) => {
                sheet.update(cx, |sheet, cx| sheet.set_size(self.width, self.height, cx))
            }
        }
        self.apply_ruler(cx);
        cx.notify();
    }
    fn apply_ruler(&mut self, cx: &mut Context<Self>) {
        if let Content::Scribe {
            area,
            ruler,
            show_ruler,
        } = &self.content
        {
            let state = ruler.read(cx).state();
            let editor = area.read(cx).editor();
            editor.update(cx, |editor, cx| {
                if *show_ruler {
                    editor.set_document_layout(
                        state.left as f32,
                        (state.right - state.left) as f32,
                        state.first as f32,
                        cx,
                    );
                } else {
                    editor.set_document_layout(8., 0., 0., cx);
                }
            });
        }
    }
    fn editor(&self, cx: &App) -> Option<Entity<TextField>> {
        match &self.content {
            Content::Jotter { pages, page, .. } => Some(pages[*page].clone()),
            Content::Scribe { area, .. } => Some(area.read(cx).editor()),
            Content::Ledger(_) => None,
        }
    }
    fn fingerprint(&self, cx: &App) -> String {
        self.editor(cx)
            .map(|editor| {
                let editor = editor.read(cx);
                format!("{:?}{:?}", editor.runs(), editor.alignments())
            })
            .unwrap_or_default()
    }
    fn changed(&self, cx: &App) -> bool {
        self.kind == DocumentKind::Scribe && self.saved != self.fingerprint(cx)
    }
    fn configure_files(&mut self, cx: &mut Context<Self>) {
        if !self.files_dirty {
            return;
        }
        self.files_dirty = false;
        self.file_subscriptions.clear();
        let open = cx.new(|cx| {
            OpenDialog::new(self.files.clone(), cx)
                .theme(self.theme)
                .language(self.language)
        });
        let save = cx.new(|cx| {
            SaveDialog::new(self.files.clone(), cx)
                .prompt(desk(self.language, "saveAsLabel"))
                .theme(self.theme)
                .language(self.language)
        });
        self.file_subscriptions
            .push(cx.subscribe(&open, |this: &mut Self, _, event, cx| {
                if let FileDialogEvent::Opened(id) = event {
                    this.open_node(*id, cx);
                }
            }));
        self.file_subscriptions.push(cx.subscribe(
            &save,
            |this: &mut Self, _, event, cx| match event {
                FileDialogEvent::Saved(target) => {
                    if this.write(target.clone(), cx) {
                        this.ready = this.pending.take();
                        cx.notify();
                    }
                }
                FileDialogEvent::Cancelled => {
                    if this.pending.take().is_some() {
                        cx.emit(DocumentsEvent::CloseCancelled);
                    }
                }
                FileDialogEvent::Opened(_) => {}
            },
        ));
        self.open = Some(open);
        self.save = Some(save);
    }
    pub fn open_node(&mut self, id: usize, cx: &mut Context<Self>) {
        let Some(node) = self.files.read(cx).find(id).cloned() else {
            return;
        };
        if node.kind.is_container() {
            return;
        }
        if let Content::Scribe { area, .. } = &self.content {
            area.update(cx, |area, cx| {
                if let Some(runs) = &node.runs {
                    area.set_runs(runs, cx);
                } else {
                    area.set_runs(
                        &[Run {
                            text: node.text,
                            style: TextStyle::default(),
                        }],
                        cx,
                    );
                }
            });
            let editor = area.read(cx).editor();
            editor.update(cx, |editor, cx| {
                editor.set_alignments(node.alignments.as_deref().unwrap_or_default(), cx);
                editor.select(0, 0, cx);
            });
            self.file = Some(id);
            self.title = node.title;
            self.saved = self.fingerprint(cx);
            cx.emit(DocumentsEvent::TitleChanged(self.title.clone()));
            cx.notify();
        }
    }
    fn write(&mut self, target: SaveTarget, cx: &mut Context<Self>) -> bool {
        let Some(editor) = self.editor(cx) else {
            return false;
        };
        let editor = editor.read(cx);
        let text = editor.value().to_owned();
        let runs = editor.runs();
        let alignments = editor.alignments();
        let name = target.name;
        let id = self.files.update(cx, |files, cx| {
            let parent = target.parent.or_else(|| {
                files
                    .roots
                    .iter()
                    .find(|node| node.kind.is_container())
                    .map(|node| node.id)
            })?;
            let id = if let Some(id) = target.replace.filter(|id| files.find(*id).is_some()) {
                id
            } else {
                files.add_document(parent, &name, &text)?
            };
            let node = files.find_mut(id)?;
            node.write_document(text, runs, alignments);
            cx.notify();
            Some(id)
        });
        if let Some(id) = id {
            self.file = Some(id);
            self.title = name;
            self.saved = self.fingerprint(cx);
            cx.emit(DocumentsEvent::TitleChanged(self.title.clone()));
            cx.notify();
            true
        } else {
            false
        }
    }
    fn save(&mut self, save_as: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.configure_files(cx);
        let existing = self.file.and_then(|id| {
            self.files
                .read(cx)
                .find(id)
                .map(|node| (id, node.title.clone()))
        });
        if !save_as && let Some((id, name)) = &existing {
            let path = self.files.read(cx).path(*id);
            let parent = path.iter().rev().nth(1).copied();
            if self.write(
                SaveTarget {
                    parent,
                    name: name.clone(),
                    replace: Some(*id),
                },
                cx,
            ) {
                self.ready = self.pending.take();
            }
            return;
        }
        let tree = self.files.read(cx);
        let parent = existing
            .as_ref()
            .and_then(|(id, _)| tree.path(*id).iter().rev().nth(1).copied())
            .or_else(|| {
                tree.roots
                    .iter()
                    .flat_map(|node| &node.children)
                    .find(|node| {
                        node.kind.is_container()
                            && matches!(node.title.as_str(), "Documents" | "文稿" | "文档")
                    })
                    .map(|node| node.id)
            })
            .or_else(|| tree.roots.first().map(|node| node.id));
        let name = existing
            .map(|(_, name)| name)
            .unwrap_or_else(|| self.title.clone());
        if let Some(save) = &self.save {
            save.update(cx, |save, cx| {
                save.set_parent(parent, cx);
                save.set_name(name, cx);
                save.show(window, cx);
            });
        }
    }
    fn ask(&mut self, pending: Pending, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss_dialogs(window, cx);
        if self.changed(cx) {
            self.pending = Some(pending);
            self.alert.update(cx, |alert, cx| {
                alert.set_message(
                    desk(self.language, "saveChanges").replace("{name}", &self.title),
                    "",
                    cx,
                );
                alert.show(window, cx);
            });
        } else {
            self.perform(pending, window, cx);
        }
    }
    fn dismiss_dialogs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(find) = &self.find
            && find.read(cx).is_open()
        {
            find.update(cx, |find, cx| find.close(window, cx));
        }
        if let Some(open) = &self.open
            && open.read(cx).is_open()
        {
            open.update(cx, |open, cx| open.close(window, cx));
        }
        if let Some(save) = &self.save
            && save.read(cx).is_open()
        {
            save.update(cx, |save, cx| save.close(window, cx));
        }
        if self.page_setup.read(cx).is_open() {
            self.page_setup
                .update(cx, |setup, cx| setup.close(window, cx));
        }
    }
    fn perform(&mut self, pending: Pending, window: &mut Window, cx: &mut Context<Self>) {
        match pending {
            Pending::Close => cx.emit(DocumentsEvent::Closed),
            Pending::New => {
                if let Content::Scribe { area, .. } = &self.content {
                    area.update(cx, |area, cx| area.set_runs(&[], cx));
                    self.file = None;
                    self.title = desk(self.language, "untitled");
                    self.saved = self.fingerprint(cx);
                    cx.emit(DocumentsEvent::TitleChanged(self.title.clone()));
                }
            }
            Pending::Open => {
                self.configure_files(cx);
                if let Some(open) = &self.open {
                    open.update(cx, |open, cx| open.show(window, cx));
                }
            }
        }
        cx.notify();
    }
    pub fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ask(Pending::Close, window, cx);
    }
    pub fn command(&mut self, command: &str, window: &mut Window, cx: &mut Context<Self>) {
        match command {
            "new" => self.ask(Pending::New, window, cx),
            "open" => self.ask(Pending::Open, window, cx),
            "save" => self.save(false, window, cx),
            "save-as" => self.save(true, window, cx),
            "find" => {
                if let Some(find) = &self.find {
                    find.update(cx, |find, cx| find.show(window, cx));
                }
            }
            "page-setup" => self
                .page_setup
                .update(cx, |setup, cx| setup.show(window, cx)),
            "page-next" | "page-prev" => {
                self.turn_page(if command == "page-next" { 1 } else { -1 }, cx)
            }
            "ruler" => {
                if let Content::Scribe { show_ruler, .. } = &mut self.content {
                    *show_ruler = !*show_ruler;
                }
                self.resize(cx);
            }
            "bold" | "italic" | "underline" => {
                if let Some(editor) = self.editor(cx) {
                    editor.update(cx, |editor, cx| {
                        editor.toggle_style(
                            match command {
                                "bold" => StyleAttribute::Bold,
                                "italic" => StyleAttribute::Italic,
                                _ => StyleAttribute::Underline,
                            },
                            cx,
                        )
                    });
                }
            }
            "align-left" | "align-center" | "align-right" => {
                if let Some(editor) = self.editor(cx) {
                    editor.update(cx, |editor, cx| {
                        editor.set_alignment(
                            match command {
                                "align-left" => Alignment::Left,
                                "align-center" => Alignment::Center,
                                _ => Alignment::Right,
                            },
                            cx,
                        )
                    });
                }
            }
            "size-11" | "size-22" | "size-33" | "size-44" => {
                let scale = match command {
                    "size-11" => 1,
                    "size-22" => 2,
                    "size-33" => 3,
                    _ => 4,
                };
                if let Some(editor) = self.editor(cx) {
                    editor.update(cx, |editor, cx| editor.set_scale(scale, cx));
                }
            }
            _ => self.edit_command(command, window, cx),
        }
    }
    fn edit_command(&mut self, command: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Content::Ledger(sheet) = &self.content {
            sheet.update(cx, |sheet, cx| {
                sheet.command(
                    if command == "select-all" {
                        "selectAll"
                    } else {
                        command
                    },
                    window,
                    cx,
                )
            });
            return;
        }
        let Some(editor) = self.editor(cx) else {
            return;
        };
        if !self.is_modal_open(cx) {
            window.focus(&editor.focus_handle(cx), cx);
        }
        match command {
            "copy" => window.dispatch_action(Box::new(text_input::Copy), cx),
            "cut" => window.dispatch_action(Box::new(text_input::Cut), cx),
            "paste" => window.dispatch_action(Box::new(text_input::Paste), cx),
            "undo" => window.dispatch_action(Box::new(text_input::Undo), cx),
            "redo" => window.dispatch_action(Box::new(text_input::Redo), cx),
            "select-all" => window.dispatch_action(Box::new(text_input::SelectAll), cx),
            "clear" => window.dispatch_action(Box::new(text_input::Delete), cx),
            _ => {}
        }
    }
    fn turn_page(&mut self, delta: i32, cx: &mut Context<Self>) {
        if let Content::Jotter { pages, page, .. } = &mut self.content {
            *page = (*page as i32 + delta).rem_euclid(pages.len() as i32) as usize;
            if let Some(find) = &self.find {
                find.update(cx, |find, cx| find.set_target(pages[*page].clone(), cx));
            }
            cx.notify();
        }
    }
}
impl Render for Documents {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.initialized = true;
        self.configure_files(cx);
        if let Some(choice) = self.alert_choice.take() {
            match choice {
                0 => {
                    if let Some(pending) = self.pending.take() {
                        self.perform(pending, window, cx);
                    }
                }
                2 => self.save(false, window, cx),
                _ => {
                    self.pending = None;
                    cx.emit(DocumentsEvent::CloseCancelled);
                }
            }
        }
        if let Some(pending) = self.ready.take() {
            self.perform(pending, window, cx);
        }
        let lang = self.language;
        let content: AnyElement = match &self.content {
            Content::Jotter {
                pages,
                page,
                corner,
            } => {
                let number = desk(lang, "pageOf")
                    .replace("{page}", &(page + 1).to_string())
                    .replace("{pages}", &pages.len().to_string());
                div()
                    .flex()
                    .flex_col()
                    .bg(rgb(0xfff6c8))
                    .child(pages[*page].clone())
                    .child(
                        div()
                            .flex()
                            .h(px(20.))
                            .items_center()
                            .gap(px(8.))
                            .px(px(4.))
                            .child(
                                div()
                                    .id("turn-page")
                                    .role(Role::Button)
                                    .aria_label(desk(lang, "turnPage"))
                                    .track_focus(corner)
                                    .tab_stop(true)
                                    .size(px(16.))
                                    .on_mouse_down(MouseButton::Left, {
                                        let corner = corner.clone();
                                        move |_, window, cx| window.focus(&corner, cx)
                                    })
                                    .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                        this.turn_page(
                                            if event.modifiers().shift { -1 } else { 1 },
                                            cx,
                                        )
                                    }))
                                    .on_key_down(cx.listener(
                                        |this, event: &gpui::KeyDownEvent, _, cx| {
                                            if matches!(
                                                event.keystroke.key.as_str(),
                                                "space" | "enter"
                                            ) {
                                                this.turn_page(
                                                    if event.keystroke.modifiers.shift {
                                                        -1
                                                    } else {
                                                        1
                                                    },
                                                    cx,
                                                );
                                                cx.stop_propagation();
                                            }
                                        },
                                    ))
                                    .child(drawing(16., 16., |pen, _, _| {
                                        for y in 0..16 {
                                            pen.hline(0, y, y + 1, BLACK);
                                            if y > 1 {
                                                pen.hline(1, y, y - 1, 0xeedd99);
                                            }
                                        }
                                        pen.hline(0, 15, 16, BLACK);
                                    })),
                            )
                            .child(text_in_language(number, Font::Small, BLACK, lang)),
                    )
                    .into_any_element()
            }
            Content::Scribe {
                area,
                ruler,
                show_ruler,
            } => {
                let editor = area.read(cx).editor();
                let count = word_count(editor.read(cx).value(), lang);
                let toolbar = div()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .h(px(28.))
                    .bg(rgb(self.theme.background))
                    .child(
                        tool_button(
                            "save",
                            desk(lang, "save"),
                            SAVE_ICON,
                            false,
                            &self.toolbar_focus[0],
                            self.theme,
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.command("save", window, cx)),
                        )
                        .on_key_down(cx.listener(
                            |this, event: &gpui::KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "space" | "enter") {
                                    this.command("save", window, cx);
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    )
                    .child(
                        tool_button(
                            "find",
                            desk(lang, "find"),
                            FIND_ICON,
                            false,
                            &self.toolbar_focus[1],
                            self.theme,
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.command("find", window, cx)),
                        )
                        .on_key_down(cx.listener(
                            |this, event: &gpui::KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "space" | "enter") {
                                    this.command("find", window, cx);
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    )
                    .child(
                        tool_button(
                            "ruler",
                            desk(lang, "ruler"),
                            RULER_ICON,
                            *show_ruler,
                            &self.toolbar_focus[2],
                            self.theme,
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.command("ruler", window, cx)),
                        )
                        .on_key_down(cx.listener(
                            |this, event: &gpui::KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "space" | "enter") {
                                    this.command("ruler", window, cx);
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    );
                div()
                    .flex()
                    .flex_col()
                    .child(toolbar)
                    .when(*show_ruler, |el| el.child(ruler.clone()))
                    .child(area.clone())
                    .child({
                        // The word count in a placard beside the resize corner.
                        let width = self.width - GROW_BOX_SIZE;
                        div()
                            .relative()
                            .w(px(width as f32))
                            .h(px(GROW_BOX_SIZE as f32))
                            .pl(px(6.))
                            .child(div().absolute().left_0().top_0().child(drawing(
                                width as f32,
                                GROW_BOX_SIZE as f32,
                                |pen, _, _| {
                                    pen.rect(0, 0, pen.width, pen.height, GRAY_2);
                                    pen.bevel(0, 0, pen.width, pen.height, WHITE, GRAY_7);
                                },
                            )))
                            .child(
                                text_in_language(count, Font::Small, BLACK, lang)
                                    .height(GROW_BOX_SIZE),
                            )
                    })
                    .into_any_element()
            }
            Content::Ledger(sheet) => sheet.clone().into_any_element(),
        };
        div()
            .relative()
            .size_full()
            .child(content)
            .children(self.find.clone())
            .children(self.open.clone())
            .children(self.save.clone())
            .child(self.page_setup.clone())
            .child(self.alert.clone())
    }
}

fn tool_button(
    id: &'static str,
    label: String,
    icon: &'static [&'static str],
    selected: bool,
    focus: &FocusHandle,
    theme: Theme,
) -> gpui::Stateful<gpui::Div> {
    let handle = focus.clone();
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .track_focus(focus)
        .tab_stop(true)
        .size(px(24.))
        .text_color(rgb(BLACK))
        .active(|style| style.text_color(rgb(WHITE)))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            window.focus(&handle, cx)
        })
        .child(drawing(24., 24., move |pen, window, _| {
            let pressed = window.text_style().color == rgb(WHITE).into();
            if pressed || selected {
                pen.sunken();
                pen.rect(2, 2, 20, 20, theme.accent.light);
            } else {
                pen.raised(theme.background);
            }
            for (y, row) in icon.iter().enumerate() {
                for (x, pixel) in row.chars().enumerate() {
                    if pixel == '#' {
                        pen.dot(x as i32 + 4, y as i32 + 4, BLACK);
                    } else if pixel == 'w' {
                        pen.dot(x as i32 + 4, y as i32 + 4, WHITE);
                    }
                }
            }
        }))
}
const SAVE_ICON: &[&str] = &[
    ".##############.",
    ".#www######www#.",
    ".#www#wwww#www#.",
    ".#www#wwww#www#.",
    ".#www#wwww#www#.",
    ".#www######www#.",
    ".#wwwwwwwwwwww#.",
    ".#wwwwwwwwwwww#.",
    ".#ww########ww#.",
    ".#ww#wwwwww#ww#.",
    ".#ww#wwwwww#ww#.",
    ".#ww#wwwwww#ww#.",
    ".#ww#wwwwww#ww#.",
    ".#ww########ww#.",
    ".##############.",
    "................",
];
const FIND_ICON: &[&str] = &[
    "................",
    ".....####.......",
    "...##wwww##.....",
    "..#ww....ww#....",
    "..#w......w#....",
    ".#w........w#...",
    ".#w........w#...",
    ".#w........w#...",
    "..#w......w#....",
    "..#ww....ww#....",
    "...##wwww###....",
    ".....####.###...",
    "...........###..",
    "............###.",
    ".............##.",
    "................",
];
const RULER_ICON: &[&str] = &[
    "................",
    "................",
    "................",
    "................",
    "................",
    "################",
    "#ww#ww#ww#ww#ww#",
    "#ww#ww#ww#ww#ww#",
    "#wwwww#wwwww#ww#",
    "#wwwwwwwwwwwwww#",
    "################",
    "................",
    "................",
    "................",
    "................",
    "................",
];

fn word_count(value: &str, language: Language) -> String {
    let mut words = 0;
    let mut lines = 1;
    let mut inside_word = false;
    for character in value.chars() {
        if character == '\n' {
            lines += 1;
        }
        if matches!(character as u32, 0x3040..=0x30ff | 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0x20000..=0x3134f)
        {
            inside_word = false;
            words += 1;
        } else if character.is_whitespace()
            || matches!(character as u32, 0x3000..=0x303f | 0xff00..=0xff65)
        {
            inside_word = false;
        } else if !inside_word {
            inside_word = true;
            words += 1;
        }
    }
    format_message(
        &desk(language, "wordsLines"),
        &[("words", words.to_string()), ("lines", lines.to_string())],
    )
}
#[cfg(test)]
mod tests {
    use super::word_count;
    use gpui_platinum::Language;
    #[test]
    fn office_word_counts_treat_han_and_kana_as_separate_words() {
        assert_eq!(
            word_count("Hello world\n中文、かな", Language::English),
            "6 words, 2 lines"
        );
        assert_eq!(word_count("", Language::SimplifiedChinese), "0 个字，1 行");
    }
}

use super::{
    AlertKind,
    alert::icon,
    modal::{ModalState, ModalView, draw_frame, modal, paragraph, text},
};
use crate::{
    Font, Language, ListBox, ListEvent, ListItem, MenuEvent, MenuItem, PopupMenu, TextField, Theme,
    bitmap_icons::draw_bitmap_icon,
    button,
    models::{FileTree, NodeKind},
    paint::{Pen, drawing},
    theme::GRAY_4,
    tr,
};
use gpui::{
    AnyElement, Context, Entity, EventEmitter, FocusHandle, Focusable, Render, SharedString,
    Subscription, Window, canvas, div, prelude::*, px,
};

/// The size of a file dialog's list, and the width of its buttons.
const LIST_WIDTH: i32 = 220;
const LIST_HEIGHT: i32 = 150;
const BUTTON_WIDTH: i32 = 100;
/// The dialog: the list, the buttons 12 from it, the padding and the frame.
const DIALOG_WIDTH: i32 = LIST_WIDTH + 12 + BUTTON_WIDTH + 2 * 16;
/// How far a default button's ring reaches out of its face.
const DEFAULT_RING: f32 = 4.;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveTarget {
    pub parent: Option<usize>,
    pub name: String,
    pub replace: Option<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileDialogEvent {
    Opened(usize),
    Saved(SaveTarget),
    Cancelled,
}

pub type OpenDialog = FileDialog<false>;
pub type SaveDialog = FileDialog<true>;

/// A file picker for an application-owned in-memory disk. Saving emits a target;
/// the application decides which document contents to put there.
pub struct FileDialog<const SAVE: bool> {
    modal: ModalState,
    files: Entity<FileTree>,
    parent: Option<usize>,
    rows: Vec<usize>,
    list: Entity<ListBox>,
    path: Entity<PopupMenu>,
    path_ids: Vec<Option<usize>>,
    name: Entity<TextField>,
    prompt: SharedString,
    extension: String,
    new_folders: bool,
    replacing: Option<SaveTarget>,
    activated: bool,
    dirty: bool,
    focus: [FocusHandle; 6],
    _subscriptions: Vec<Subscription>,
}
impl<const SAVE: bool> EventEmitter<FileDialogEvent> for FileDialog<SAVE> {}
impl<const SAVE: bool> FileDialog<SAVE> {
    pub fn new(files: Entity<FileTree>, cx: &mut Context<Self>) -> Self {
        let list = cx.new(|cx| {
            ListBox::new(Vec::<ListItem>::new(), cx)
                .size(LIST_WIDTH, LIST_HEIGHT)
                .scroll_bar_always()
                .label("Files")
        });
        let path = cx.new(|cx| {
            PopupMenu::new(["Desktop"], cx)
                .bevel(DESKTOP_ICON)
                .label("Path")
        });
        let name = cx.new(|cx| {
            TextField::new(cx)
                .size(LIST_WIDTH as f32, 22.)
                .label("Save File")
        });
        let list_subscription = cx.subscribe(&list, |this: &mut Self, _, event, cx| {
            if matches!(event, ListEvent::Activated(_)) {
                this.activated = true;
            }
            cx.notify();
        });
        let path_subscription = cx.subscribe(&path, |this: &mut Self, _, event: &MenuEvent, cx| {
            if let Some(parent) = this.path_ids.get(event.0).copied()
                && parent != this.parent
            {
                this.parent = parent;
                this.refresh(cx);
            }
        });
        let file_subscription = cx.observe(&files, |this: &mut Self, _, cx| {
            if this.modal.open {
                this.refresh(cx);
            }
        });
        let name_subscription = cx.observe(&name, |_, _, cx| cx.notify());
        Self {
            modal: ModalState::new(DIALOG_WIDTH as f32, cx),
            files,
            parent: None,
            rows: Vec::new(),
            list,
            path,
            path_ids: vec![None],
            name,
            prompt: "".into(),
            extension: String::new(),
            new_folders: false,
            replacing: None,
            activated: false,
            dirty: true,
            focus: std::array::from_fn(|_| cx.focus_handle()),
            _subscriptions: vec![
                list_subscription,
                path_subscription,
                file_subscription,
                name_subscription,
            ],
        }
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.modal.theme = theme;
        self.dirty = true;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.modal.language = language;
        self.dirty = true;
        self
    }
    pub fn prompt(mut self, prompt: impl Into<SharedString>) -> Self {
        self.prompt = prompt.into();
        self
    }
    pub fn extension(mut self, extension: impl Into<String>) -> Self {
        self.extension = extension.into().trim_start_matches('.').to_owned();
        self
    }
    /// Lets New Folder make a folder in the one shown, and go into it; it
    /// is otherwise dimmed, as the original's when the app makes none.
    pub fn new_folders(mut self, enabled: bool) -> Self {
        self.new_folders = enabled;
        self
    }
    pub fn set_prompt(&mut self, prompt: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.prompt = prompt.into();
        self.dirty = true;
        cx.notify();
    }
    pub fn is_open(&self) -> bool {
        self.modal.open
    }
    pub fn set_name(&mut self, name: impl Into<String>, cx: &mut Context<Self>) {
        self.name.update(cx, |field, cx| field.set_value(name, cx));
    }
    pub fn set_parent(&mut self, parent: Option<usize>, cx: &mut Context<Self>) {
        self.parent = parent;
        self.refresh(cx);
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.modal.theme = theme;
        self.dirty = true;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.modal.language = language;
        self.dirty = true;
        self.refresh(cx);
    }
    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.replacing = None;
        self.refresh(cx);
        self.modal.stops = vec![self.path.focus_handle(cx), self.list.focus_handle(cx)];
        if SAVE {
            self.modal.stops.push(self.name.focus_handle(cx));
        }
        self.modal.stops.extend(self.focus[..4].iter().cloned());
        let focus = if SAVE {
            self.name.focus_handle(cx)
        } else {
            self.list.focus_handle(cx)
        };
        if SAVE {
            self.name
                .update(cx, |field, cx| field.select(0, field.value().len(), cx));
        }
        self.modal.show(Some(&focus), window, cx);
        cx.notify();
    }
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.replacing = None;
        self.modal.close(window, cx);
        cx.notify();
    }
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let tree = self.files.read(cx);
        if self
            .parent
            .is_some_and(|id| tree.find(id).is_none_or(|node| !node.kind.is_container()))
        {
            self.parent = None;
        }
        let nodes = self
            .parent
            .and_then(|id| tree.find(id))
            .map(|node| &node.children)
            .unwrap_or(&tree.roots);
        self.rows = nodes.iter().map(|node| node.id).collect();
        let at_desktop = self.parent.is_none();
        let items: Vec<ListItem> = nodes
            .iter()
            .map(|node| ListItem {
                label: node.title.clone().into(),
                // A save dialog shows its files, not to choose.
                disabled: SAVE && !node.kind.is_container(),
            })
            .collect();
        let icons: Vec<Vec<String>> = nodes
            .iter()
            .map(|node| {
                let icon = match node.kind.is_container() {
                    true if at_desktop => DISK_ICON,
                    true => FOLDER_ICON,
                    false => DOCUMENT_ICON,
                };
                icon.iter().map(|row| (*row).to_owned()).collect()
            })
            .collect();
        // The first item is chosen, as it opens, unless it is a file to
        // save over.
        let first = items.first().filter(|item| !item.disabled).map(|_| 0);
        // The folders from the one shown out to the desktop, which a path
        // pop-up shows by the folder's own icon, or a disk's or the
        // desktop's.
        let ids = self.parent.map(|id| tree.path(id)).unwrap_or_default();
        let icon = match ids.len() {
            0 => DESKTOP_ICON,
            1 => DISK_ICON,
            _ => FOLDER_ICON,
        };
        let mut path_items: Vec<MenuItem> = ids
            .iter()
            .rev()
            .filter_map(|id| tree.find(*id))
            .map(|node| MenuItem::new(node.title.clone()))
            .collect();
        path_items.push(MenuItem::new(tr(self.modal.language, "desktop")));
        self.path_ids = ids
            .into_iter()
            .rev()
            .map(Some)
            .chain(std::iter::once(None))
            .collect();
        self.path.update(cx, |path, cx| {
            path.set_items(path_items, cx);
            path.set_selection(0, cx);
            path.set_bevel(Some(icon), cx);
        });
        self.list.update(cx, |list, cx| {
            list.set_items(items, cx);
            list.set_icons(icons, cx);
            list.set_selection(first, cx);
        });
        cx.notify();
    }
    fn selection(&self, cx: &gpui::App) -> Option<usize> {
        self.list
            .read(cx)
            .selection()
            .and_then(|index| self.rows.get(index).copied())
    }
    fn enter_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.selection(cx) else {
            return;
        };
        let Some(node) = self.files.read(cx).find(id) else {
            return;
        };
        if node.kind.is_container() {
            self.parent = Some(id);
            self.refresh(cx);
        } else if !SAVE {
            self.close(window, cx);
            cx.emit(FileDialogEvent::Opened(id));
        }
    }
    fn save_target(&self, cx: &gpui::App) -> Option<SaveTarget> {
        let name = saved_name(self.name.read(cx).value(), &self.extension)?;
        let tree = self.files.read(cx);
        let nodes = self
            .parent
            .and_then(|id| tree.find(id))
            .map(|node| &node.children)
            .unwrap_or(&tree.roots);
        let existing = nodes.iter().find(|node| node.title == name);
        if existing.is_some_and(|node| node.kind != NodeKind::Document) {
            return None;
        }
        Some(SaveTarget {
            parent: self.parent,
            name,
            replace: existing.map(|node| node.id),
        })
    }
    fn accept(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.path.read(cx).is_open() || self.path.focus_handle(cx).is_focused(window) {
            cx.propagate();
            return;
        }
        if let Some(target) = self.replacing.take() {
            self.close(window, cx);
            cx.emit(FileDialogEvent::Saved(target));
            return;
        }
        if !SAVE {
            self.enter_selection(window, cx);
            return;
        }
        if self.list.focus_handle(cx).is_focused(window) {
            self.enter_selection(window, cx);
            return;
        }
        let Some(target) = self.save_target(cx) else {
            return;
        };
        if target.replace.is_some() {
            self.replacing = Some(target);
            self.modal.stops = vec![self.focus[5].clone(), self.focus[4].clone()];
            // The alert keeps the keys, Return choosing Replace as its
            // default button, but shows no ring until Tab moves to a
            // button, as the original's.
            window.focus(&self.modal.focus, cx);
            cx.notify();
        } else {
            self.close(window, cx);
            cx.emit(FileDialogEvent::Saved(target));
        }
    }
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.path.read(cx).is_open() {
            cx.propagate();
            return;
        }
        if self.replacing.take().is_some() {
            self.modal.stops = vec![
                self.path.focus_handle(cx),
                self.list.focus_handle(cx),
                self.name.focus_handle(cx),
            ];
            self.modal.stops.extend(self.focus[..4].iter().cloned());
            window.focus(&self.name.focus_handle(cx), cx);
            cx.notify();
        } else {
            self.close(window, cx);
            cx.emit(FileDialogEvent::Cancelled);
        }
    }
    /// The alert asking whether to replace a file of the name, over the
    /// dialog in the middle of the window, as the original's Alert.
    fn replace_alert(&self, name: &str, cx: &mut Context<Self>) -> AnyElement {
        let lang = self.modal.language;
        let theme = self.modal.theme;
        let message = tr(lang, "replaceExisting").replace("{name}", name);
        let frame = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let mut pen = Pen::new(
                    f32::from(bounds.size.width).round() as i32,
                    f32::from(bounds.size.height).round() as i32,
                );
                draw_frame(&mut pen, theme);
                pen.paint(bounds.origin, window);
            },
        )
        .absolute()
        .inset_0()
        .size_full();
        div()
            .id("replace-alert")
            .role(gpui::Role::AlertDialog)
            .aria_label(message.clone())
            .relative()
            .w(px(340.))
            .p(px(15.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(frame)
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(16.))
                    .child(icon(AlertKind::Caution, false))
                    .child(paragraph(message, Font::Bold, 262, lang)),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        button("replace-cancel", tr(lang, "cancel"))
                            .focus_handle(&self.focus[5])
                            .theme(theme)
                            .language(lang)
                            .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                    )
                    .child(
                        button("replace", tr(lang, "replace"))
                            .focus_handle(&self.focus[4])
                            .default_button()
                            .theme(theme)
                            .language(lang)
                            .on_click(cx.listener(|this, _, window, cx| this.accept(window, cx))),
                    ),
            )
            .into_any_element()
    }
    fn new_folder(&mut self, cx: &mut Context<Self>) {
        let Some(parent) = self.parent else {
            return;
        };
        let base = tr(self.modal.language, "newFolder");
        let name = unique_folder_name(self.files.read(cx), parent, &base);
        let id = self.files.update(cx, |files, cx| {
            let id = files.add_folder(parent, name);
            cx.notify();
            id
        });
        if let Some(id) = id {
            self.parent = Some(id);
            self.refresh(cx);
        }
    }
}
impl<const SAVE: bool> Render for FileDialog<SAVE> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.modal.open {
            return div().into_any_element();
        }
        if self.activated {
            self.activated = false;
            self.enter_selection(window, cx);
        }
        let lang = self.modal.language;
        let theme = self.modal.theme;
        if self.dirty {
            self.dirty = false;
            self.list.update(cx, |list, cx| {
                list.set_theme(theme, cx);
                list.set_language(lang, cx);
                list.set_label(tr(lang, "files"), cx);
            });
            self.path.update(cx, |path, cx| {
                path.set_theme(theme, cx);
                path.set_language(lang, cx);
                path.set_label(tr(lang, "path"), cx);
            });
            self.name.update(cx, |name, cx| {
                name.set_theme(theme, cx);
                name.set_language(lang, cx);
                name.set_label(
                    if self.prompt.is_empty() {
                        tr(lang, "saveFile").into()
                    } else {
                        self.prompt.clone()
                    },
                    cx,
                );
            });
        }
        if self.replacing.is_none() {
            self.modal.stops = vec![self.path.focus_handle(cx), self.list.focus_handle(cx)];
            if SAVE {
                self.modal.stops.push(self.name.focus_handle(cx));
            }
            if self.parent.is_some() {
                self.modal.stops.push(self.focus[0].clone());
                if SAVE && self.new_folders {
                    self.modal.stops.push(self.focus[1].clone());
                }
            }
            self.modal.stops.push(self.focus[2].clone());
            if (SAVE && self.save_target(cx).is_some()) || (!SAVE && self.selection(cx).is_some()) {
                self.modal.stops.push(self.focus[3].clone());
            }
        }
        let title = tr(lang, if SAVE { "saveFile" } else { "openFile" });
        let tree = self.files.read(cx);
        let disk = self
            .parent
            .and_then(|id| tree.path(id).first().copied())
            .and_then(|id| tree.find(id))
            .map(|node| node.title.clone())
            .unwrap_or_else(|| tr(lang, "desktop"));
        let mut left = div().flex().flex_col().gap(px(6.)).child(self.list.clone());
        if SAVE {
            left = left
                .child(text(self.prompt.clone(), Font::Bold, lang))
                .child(self.name.clone());
        }
        let accent = theme.accent;
        let disk = Font::Small.truncate_in(&disk, BUTTON_WIDTH - 20, lang);
        let mut right = div()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(8.))
            .w(px(BUTTON_WIDTH as f32))
            // The disk the folder is on, and its name.
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .h(px(16.))
                    .child(drawing(16., 16., move |pen, _, _| {
                        draw_bitmap_icon(pen, DISK_ICON, 0, 0, accent, false, false);
                    }))
                    .child(text(disk, Font::Small, lang)),
            )
            .child(
                // Nothing here ejects.
                button("eject", tr(lang, "eject"))
                    .width(BUTTON_WIDTH)
                    .disabled(true)
                    .theme(theme)
                    .language(lang),
            )
            .child(
                button("desktop", tr(lang, "desktop"))
                    .width(BUTTON_WIDTH)
                    .disabled(self.parent.is_none())
                    .focus_handle(&self.focus[0])
                    .theme(theme)
                    .language(lang)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.parent = None;
                        this.refresh(cx);
                    })),
            );
        if SAVE {
            right = right.child(
                button("new-folder", tr(lang, "newFolder"))
                    .width(BUTTON_WIDTH)
                    .disabled(!self.new_folders || self.parent.is_none())
                    .focus_handle(&self.focus[1])
                    .theme(theme)
                    .language(lang)
                    .on_click(cx.listener(|this, _, _, cx| this.new_folder(cx))),
            );
        }
        let accept_disabled = if SAVE {
            self.save_target(cx).is_none()
        } else {
            self.selection(cx).is_none()
        };
        right = right
            // The line between the buttons.
            .child(drawing(BUTTON_WIDTH as f32, 1., |pen, _, _| {
                pen.hline(0, 0, pen.width, GRAY_4);
            }))
            .child(div().flex_grow(1.))
            .child(
                button("cancel", tr(lang, "cancel"))
                    .width(BUTTON_WIDTH)
                    .focus_handle(&self.focus[2])
                    .theme(theme)
                    .language(lang)
                    .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
            )
            .child(
                // Its ring outside the column, its face under the others.
                div().ml(px(-DEFAULT_RING)).child(
                    button("accept", tr(lang, if SAVE { "save" } else { "open" }))
                        .width(BUTTON_WIDTH)
                        .default_button()
                        .disabled(accept_disabled)
                        .focus_handle(&self.focus[3])
                        .theme(theme)
                        .language(lang)
                        .on_click(cx.listener(|this, _, window, cx| this.accept(window, cx))),
                ),
            );
        let content = div()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(8.))
            .child(self.path.clone())
            .child(div().flex().gap(px(12.)).child(left).child(right));
        let alert = self
            .replacing
            .as_ref()
            .map(|target| self.replace_alert(&target.name, cx));
        modal(
            &mut self.modal,
            ModalView::new(title, false, content)
                .placed()
                .spacing(16., 8.)
                .overlay(alert),
            window,
            cx,
            |this| &mut this.modal,
            Self::accept,
            Self::cancel,
        )
    }
}

fn saved_name(value: &str, extension: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || matches!(value, "." | "..") || value.contains(['/', '\\', ':', '\0']) {
        return None;
    }
    let mut name = value.to_owned();
    if !extension.is_empty() && !value.contains('.') {
        name.push('.');
        name.push_str(extension);
    }
    Some(name)
}
fn unique_folder_name(files: &FileTree, parent: usize, base: &str) -> String {
    let Some(node) = files.find(parent) else {
        return base.to_owned();
    };
    let mut name = base.to_owned();
    let mut number = 2;
    while node.children.iter().any(|child| child.title == name) {
        name = format!("{base} {number}");
        number += 1;
    }
    name
}
// The 16×16 icons of the file dialogs.
const FOLDER_ICON: &[&str] = &[
    "................",
    "................",
    ".####...........",
    "#wwww#..........",
    "###############.",
    "#wwwwwwwwwwwww#.",
    "#w++++++++++++#.",
    "#w++++++++++++#.",
    "#w++++++++++++#.",
    "#w++++++++++++#.",
    "#w++++++++++++#.",
    "#w++++++++++++#.",
    "#+++++++++++++#.",
    "###############.",
    "................",
    "................",
];
const DOCUMENT_ICON: &[&str] = &[
    "..########......",
    "..#wwwwww##.....",
    "..#wwwwww#w#....",
    "..#wwwwww####...",
    "..#wwwwwwwww#...",
    "..#w::::::ww#...",
    "..#wwwwwwwww#...",
    "..#w:::::::w#...",
    "..#wwwwwwwww#...",
    "..#w::::::ww#...",
    "..#wwwwwwwww#...",
    "..#w:::::::w#...",
    "..#wwwwwwwww#...",
    "..#wwwwwwwww#...",
    "..###########...",
    "................",
];
const DISK_ICON: &[&str] = &[
    "................",
    "................",
    "................",
    "................",
    "................",
    "################",
    "#wwwwwwwwwwwwww#",
    "#w::::::::::::w#",
    "#wwwwwwwwwwwwww#",
    "#w#####wwwwww+w#",
    "#wwwwwwwwwwwwww#",
    "################",
    ".##..........##.",
    "................",
    "................",
    "................",
];
const DESKTOP_ICON: &[&str] = &[
    "................",
    "..############..",
    "..#wwwwwwwwww#..",
    "..#w++++++++w#..",
    "..#w++++++++w#..",
    "..#w++++++++w#..",
    "..#w++++++++w#..",
    "..#wwwwwwwwww#..",
    "..############..",
    "......####......",
    "....########....",
    "................",
    "................",
    "................",
    "................",
    "................",
];
#[cfg(test)]
mod tests {
    use super::{saved_name, unique_folder_name};
    use crate::models::FileTree;
    #[test]
    fn save_names_preserve_chosen_extensions_and_reject_paths() {
        assert_eq!(saved_name(" notes ", "txt"), Some("notes.txt".into()));
        assert_eq!(saved_name("notes.md", "txt"), Some("notes.md".into()));
        for name in ["", " ", "..", "a/b", "a\\b", "disk:name", "a\0b"] {
            assert_eq!(saved_name(name, "txt"), None);
        }
    }
    #[test]
    fn new_folder_names_do_not_replace_existing_items() {
        let mut files = FileTree::sample();
        files.add_folder(0, "New Folder");
        files.add_document(0, "New Folder 2", "keep");
        assert_eq!(unique_folder_name(&files, 0, "New Folder"), "New Folder 3");
    }
}

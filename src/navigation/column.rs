use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, KeyDownEvent, MouseButton, Role, Window,
    div, prelude::*, px,
};

use crate::{
    LIST_ROW_HEIGHT, Language, ListItem, ScrollBar, Theme,
    list::draw_list_frame,
    models::{FileTree, TreeNode},
    paint::drawing,
    theme::*,
};

/// The width of each column, its list box's.
const COLUMN_WIDTH: i32 = 140;
const SCROLL_BAR_WIDTH: i32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnEvent {
    Selected(usize),
    Activated(usize),
}

/// A browser of a tree in columns: a list box of the roots and, right of
/// it, one of the children of the item chosen in each column, each 140
/// wide and lying over the next one's edge, with a horizontal scroll bar
/// under them. With the keyboard focus, one ring runs round all the
/// columns, inside them, and the chosen row of the column the keys move in
/// is in the accent.
pub struct ColumnView {
    focus: FocusHandle,
    model: FileTree,
    path: Vec<usize>,
    active_column: usize,
    /// The first row in view in each column; the wheel moves it by
    /// fractions.
    tops: Vec<f64>,
    horizontal: i32,
    width: i32,
    height: i32,
    always_scrollbar: bool,
    scroll_bar_room: i32,
    theme: Theme,
    language: Language,
}
impl EventEmitter<ColumnEvent> for ColumnView {}
impl Focusable for ColumnView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl ColumnView {
    pub fn from_nodes(roots: Vec<TreeNode>, cx: &mut Context<Self>) -> Self {
        Self::new(FileTree::new(roots), cx)
    }
    pub fn new(model: FileTree, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            model,
            path: Vec::new(),
            active_column: 0,
            tops: vec![0.],
            horizontal: 0,
            width: 420,
            height: 160,
            always_scrollbar: false,
            scroll_bar_room: 0,
            theme: Theme::default(),
            language: Language::English,
        }
    }
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width.max(140);
        self.height = height.max(64);
        self.clamp_scroll_offsets();
        self
    }
    /// Keeps vertical scroll bars visible even when a column's rows fit.
    pub fn always_scrollbar(mut self, always: bool) -> Self {
        self.always_scrollbar = always;
        self
    }
    /// Leaves `room` pixels at the right of the horizontal scroll bar, as for
    /// the resize corner of the window the view fills, and lays the bar over
    /// that window's lines at its left and bottom.
    pub fn scroll_bar_room(mut self, room: i32) -> Self {
        self.scroll_bar_room = room.max(0);
        self
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }
    pub fn path(&self) -> &[usize] {
        &self.path
    }
    pub fn model(&self) -> &FileTree {
        &self.model
    }
    pub fn model_mut(&mut self) -> &mut FileTree {
        &mut self.model
    }
    pub fn selection(&self) -> Option<usize> {
        self.path.last().copied()
    }
    /// Updates the path and reveals the selected item without emitting an event.
    pub fn set_selection(&mut self, selected: Option<usize>, cx: &mut Context<Self>) {
        self.path = selected.map(|id| self.model.path(id)).unwrap_or_default();
        self.model.selected = self.path.last().copied();
        self.active_column = self.path.len().saturating_sub(1);
        self.clamp_scroll_offsets();
        for column in 0..self.path.len() {
            self.reveal(column);
        }
        self.horizontal = self.max_horizontal();
        cx.notify();
    }
    pub fn set_size(&mut self, width: i32, height: i32, cx: &mut Context<Self>) {
        self.width = width.max(140);
        self.height = height.max(64);
        self.clamp_scroll_offsets();
        cx.notify();
    }
    pub fn set_model(&mut self, model: FileTree, cx: &mut Context<Self>) {
        self.model = model;
        let valid = self
            .path
            .iter()
            .take_while(|id| self.model.find(**id).is_some())
            .count();
        self.path.truncate(valid);
        self.active_column = self.active_column.min(self.columns().saturating_sub(1));
        self.clamp_scroll_offsets();
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        cx.notify();
    }
    /// The height of the lists, the horizontal scroll bar lying over their
    /// bottom line, or, filling a window, over the window's too.
    fn lists_height(&self) -> i32 {
        self.height - if self.scroll_bar_room > 0 { 14 } else { 15 }
    }
    /// The rows that show whole in a column's list.
    fn page(&self) -> usize {
        ((self.lists_height() - 4) / LIST_ROW_HEIGHT).max(1) as usize
    }
    fn rows(&self, column: usize) -> &[TreeNode] {
        if column == 0 {
            &self.model.roots
        } else {
            self.path
                .get(column - 1)
                .and_then(|id| self.model.find(*id))
                .map(|node| node.children.as_slice())
                .unwrap_or_default()
        }
    }
    fn columns(&self) -> usize {
        self.path.len()
            + usize::from(
                self.path
                    .last()
                    .and_then(|id| self.model.find(*id))
                    .is_none_or(|node| !node.children.is_empty()),
            )
    }
    fn max_horizontal(&self) -> i32 {
        (self.content_width() - self.width).max(0)
    }
    fn content_width(&self) -> i32 {
        self.columns() as i32 * (COLUMN_WIDTH - 1) + 1
    }
    fn max_top(&self, column: usize) -> f64 {
        self.rows(column).len().saturating_sub(self.page()) as f64
    }
    fn clamp_scroll_offsets(&mut self) {
        self.tops.resize(self.columns(), 0.);
        for column in 0..self.tops.len() {
            self.tops[column] = self.tops[column].clamp(0., self.max_top(column));
        }
        self.horizontal = self.horizontal.clamp(0, self.max_horizontal());
    }
    /// Scrolls the fewest rows that bring the column's chosen row into view.
    fn reveal(&mut self, column: usize) {
        let Some(index) = self
            .path
            .get(column)
            .and_then(|id| self.rows(column).iter().position(|node| node.id == *id))
        else {
            return;
        };
        let page = self.page();
        let top = &mut self.tops[column];
        if (index as f64) < *top {
            *top = index as f64;
        } else if index as f64 >= *top + page as f64 {
            *top = (index + 1 - page) as f64;
        }
    }
    fn select(&mut self, column: usize, id: usize, cx: &mut Context<Self>) {
        if !self.rows(column).iter().any(|node| node.id == id) {
            return;
        }
        self.path.truncate(column);
        self.path.push(id);
        self.active_column = column;
        self.model.selected = Some(id);
        self.clamp_scroll_offsets();
        self.horizontal = self.max_horizontal();
        self.reveal(column);
        cx.emit(ColumnEvent::Selected(id));
        cx.notify();
    }
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let column = self.active_column.min(self.columns().saturating_sub(1));
        let rows = self.rows(column);
        let current = self
            .path
            .get(column)
            .and_then(|id| rows.iter().position(|node| node.id == *id));
        match event.keystroke.key.as_str() {
            "up" | "down" | "home" | "end" | "pageup" | "pagedown" => {
                if rows.is_empty() {
                    return;
                }
                let page = self.page().saturating_sub(1).max(1);
                let at = current.unwrap_or(0);
                let next = match event.keystroke.key.as_str() {
                    "up" => at.saturating_sub(1),
                    "down" => current.map_or(0, |i| i + 1).min(rows.len() - 1),
                    "home" => 0,
                    "end" => rows.len() - 1,
                    "pageup" => at.saturating_sub(page),
                    _ => (at + page).min(rows.len() - 1),
                };
                let id = rows[next].id;
                self.select(column, id, cx);
            }
            "right" => {
                if let Some(id) = self.rows(column + 1).first().map(|node| node.id) {
                    self.select(column + 1, id, cx);
                }
            }
            "left" => {
                if column > 0 {
                    let id = self.path[column - 1];
                    self.select(column - 1, id, cx);
                }
            }
            "enter" => {
                if let Some(id) = self.path.get(column) {
                    cx.emit(ColumnEvent::Activated(*id));
                }
            }
            _ => return,
        }
        cx.stop_propagation();
    }
    /// A column: a list box of its rows, the row chosen in it highlighted
    /// in the accent while the keys move in it, in gray otherwise.
    fn column(
        &self,
        column: usize,
        keys: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let height = self.lists_height();
        let page = self.page();
        let (theme, language) = (self.theme, self.language);
        let rows = self.rows(column);
        let count = rows.len();
        let scrolls = count > page;
        let with_bar = scrolls || self.always_scrollbar;
        let top = self.tops.get(column).copied().unwrap_or(0.);
        let first = top as usize;
        let selected = self.path.get(column).copied();
        let chosen = selected.and_then(|id| rows.iter().position(|node| node.id == id));
        let items: Vec<ListItem> = rows
            .iter()
            .map(|node| ListItem::from(node.title.clone()))
            .collect();
        let arrows: Vec<bool> = rows.iter().map(|node| !node.children.is_empty()).collect();
        let list_width = COLUMN_WIDTH - 2 - if with_bar { SCROLL_BAR_WIDTH - 1 } else { 0 };
        let scroll_entity = cx.entity();
        div()
            .id(column)
            .relative()
            .w(px(COLUMN_WIDTH as f32))
            .h(px(height as f32))
            .flex_shrink_0()
            .mr(px(-1.))
            .on_scroll_wheel(
                cx.listener(move |this, event: &gpui::ScrollWheelEvent, _, cx| {
                    let delta = f32::from(event.delta.pixel_delta(px(16.)).y) as f64;
                    let max = this.max_top(column);
                    if let Some(top) = this.tops.get_mut(column) {
                        *top = (*top - delta / LIST_ROW_HEIGHT as f64).clamp(0., max);
                    }
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .child(drawing(
                COLUMN_WIDTH as f32,
                height as f32,
                move |pen, _, _| {
                    pen.language = language;
                    let keys = keys && !crate::window::content_is_inactive();
                    draw_list_frame(
                        pen,
                        &items,
                        &[],
                        &arrows,
                        first,
                        chosen,
                        keys,
                        false,
                        theme.accent,
                        with_bar,
                    );
                },
            ))
            .children(
                rows.iter()
                    .enumerate()
                    .skip(first)
                    .take(page)
                    .map(|(index, node)| {
                        let id = node.id;
                        div()
                            .id(("row", id))
                            .role(Role::ListBoxOption)
                            .aria_label(node.title.clone())
                            .aria_selected(selected == Some(id))
                            .absolute()
                            .left(px(1.))
                            .top(px((2 + (index - first) as i32 * LIST_ROW_HEIGHT) as f32))
                            .w(px((list_width - 2) as f32))
                            .h(px(LIST_ROW_HEIGHT as f32))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(
                                    move |this, event: &gpui::MouseDownEvent, window, cx| {
                                        window.focus(&this.focus, cx);
                                        this.select(column, id, cx);
                                        if event.click_count == 2 {
                                            cx.emit(ColumnEvent::Activated(id));
                                        }
                                        cx.stop_propagation();
                                    },
                                ),
                            )
                    }),
            )
            .when(with_bar, |element| {
                element.child(
                    div()
                        .absolute()
                        .top(px(1.))
                        .left(px((COLUMN_WIDTH - 1 - SCROLL_BAR_WIDTH) as f32))
                        .child(
                            ScrollBar::new(
                                ("column-scroll", column),
                                top,
                                page as f64,
                                count.max(page) as f64,
                            )
                            .tab_stop(false)
                            .step(1.)
                            .length(height - 2)
                            .theme(theme)
                            .inactive(!scrolls)
                            .on_change(move |value, _, cx| {
                                scroll_entity.update(cx, |this, cx| {
                                    if let Some(top) = this.tops.get_mut(column) {
                                        *top = value;
                                        cx.notify();
                                    }
                                });
                            }),
                        ),
                )
            })
    }
}
impl Render for ColumnView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.clamp_scroll_offsets();
        let height = self.lists_height();
        let (width, theme, language) = (self.width, self.theme, self.language);
        let focused = self.focus.is_focused(window);
        let horizontal_entity = cx.entity();
        let columns: Vec<_> = (0..self.columns())
            .map(|column| self.column(column, focused && column == self.active_column, cx))
            .collect();
        div()
            .id("columns")
            .role(Role::ListBox)
            .aria_label(crate::tr(language, "files"))
            .track_focus(&self.focus)
            .tab_stop(true)
            .w(px(width as f32))
            .h(px(self.height as f32))
            .flex_shrink_0()
            .relative()
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| window.focus(&this.focus, cx)),
            )
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(height as f32))
                    .overflow_hidden()
                    .bg(gpui::rgb(WHITE))
                    .child(
                        div()
                            .absolute()
                            .left(px(-self.horizontal as f32))
                            .top_0()
                            .flex()
                            .children(columns),
                    )
                    .when(focused, |lists| {
                        // One ring round all the columns, inside them.
                        lists.child(div().absolute().top_0().left_0().child(drawing(
                            width as f32,
                            height as f32,
                            move |pen, _, _| {
                                if crate::window::content_is_inactive() {
                                    return;
                                }
                                let (w, h) = (pen.width, pen.height);
                                pen.frame(0, 0, w, h, theme.accent.dark);
                                pen.frame(1, 1, w - 2, h - 2, theme.accent.dark);
                            },
                        )))
                    }),
            )
            .child(
                div()
                    .when(self.scroll_bar_room > 0, |bar| {
                        bar.absolute().left(px(-1.)).top(px((height - 1) as f32))
                    })
                    .when(self.scroll_bar_room == 0, |bar| bar.mt(px(-1.)))
                    .child(
                        ScrollBar::new(
                            "columns-horizontal",
                            self.horizontal as f64,
                            width as f64,
                            self.content_width() as f64,
                        )
                        .tab_stop(false)
                        .horizontal()
                        .length(if self.scroll_bar_room > 0 {
                            width + 1 - self.scroll_bar_room
                        } else {
                            width
                        })
                        .step(COLUMN_WIDTH as f64 / 2.)
                        .theme(theme)
                        .on_change(move |value, _, cx| {
                            horizontal_entity.update(cx, |this, cx| {
                                this.horizontal = value.round() as i32;
                                cx.notify();
                            });
                        }),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn branch(id: usize, child_count: usize) -> TreeNode {
        TreeNode::item(id, format!("Folder {id}")).with_children(
            (0..child_count)
                .map(|index| TreeNode::item(id * 100 + index, format!("Item {index}")))
                .collect(),
        )
    }

    #[gpui::test]
    fn changing_to_a_shorter_directory_keeps_its_children_visible(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let view = cx.new(|cx| {
                ColumnView::from_nodes(vec![branch(1, 20), branch(2, 2)], cx).size(420, 160)
            });
            view.update(cx, |view, cx| {
                for silent in [false, true] {
                    view.select(0, 1, cx);
                    view.tops[1] = view.max_top(1);
                    assert!(view.tops[1] > 0.);

                    if silent {
                        view.set_selection(Some(2), cx);
                    } else {
                        view.select(0, 2, cx);
                    }

                    assert_eq!(view.selection(), Some(2));
                    assert_eq!(view.rows(1).len(), 2);
                    assert_eq!(view.tops[1], 0.);
                }
            });
        });
    }

    #[gpui::test]
    fn replacing_the_model_clamps_root_and_child_scroll_positions(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let view = cx.new(|cx| {
                ColumnView::from_nodes((1..=20).map(|id| branch(id, 20)).collect(), cx)
                    .size(420, 160)
            });
            view.update(cx, |view, cx| {
                view.set_selection(Some(20), cx);
                view.tops[1] = view.max_top(1);
                assert!(view.tops.iter().all(|top| *top > 0.));

                view.set_model(FileTree::new(vec![branch(20, 2), branch(21, 2)]), cx);

                assert_eq!(view.path(), &[20]);
                assert_eq!(view.tops, vec![0., 0.]);
                view.set_selection(None, cx);
                assert_eq!(view.tops, vec![0.]);
            });
        });
    }
}

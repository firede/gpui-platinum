use gpui::{
    Anchor, AnyElement, App, Context, EventEmitter, FocusHandle, Focusable, KeyDownEvent,
    MouseButton, Role, Window, anchored, deferred, div, prelude::*, px,
};
use std::rc::Rc;

use crate::{
    Font, Language, Theme,
    paint::{Pen, drawing},
    theme::*,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentTabEvent {
    Selected(usize),
    Closed(usize),
}

type ContentRenderer = Rc<dyn Fn(usize, &mut Window, &mut App) -> AnyElement>;

pub struct DocumentTabs {
    focus: FocusHandle,
    titles: Vec<String>,
    selected: usize,
    width: i32,
    height: i32,
    offset: i32,
    context_tab: Option<usize>,
    theme: Theme,
    language: Language,
    content: Option<ContentRenderer>,
}
impl EventEmitter<DocumentTabEvent> for DocumentTabs {}
impl Focusable for DocumentTabs {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl DocumentTabs {
    pub fn new(titles: Vec<String>, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            titles,
            selected: 0,
            width: 320,
            height: 120,
            offset: 0,
            context_tab: None,
            theme: Theme::default(),
            language: Language::English,
            content: None,
        }
    }
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width.max(80);
        self.height = height.max(28);
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
    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected.min(self.titles.len().saturating_sub(1));
        self
    }
    pub fn content(
        mut self,
        render: impl Fn(usize, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.content = Some(Rc::new(render));
        self
    }
    pub fn selection(&self) -> usize {
        self.selected
    }
    pub fn set_content(
        &mut self,
        render: impl Fn(usize, &mut Window, &mut App) -> AnyElement + 'static,
        cx: &mut Context<Self>,
    ) {
        self.content = Some(Rc::new(render));
        cx.notify();
    }
    pub fn titles(&self) -> &[String] {
        &self.titles
    }
    pub fn set_titles(&mut self, titles: Vec<String>, cx: &mut Context<Self>) {
        self.titles = titles;
        self.selected = self.selected.min(self.titles.len().saturating_sub(1));
        self.context_tab = None;
        self.reveal();
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        self.reveal();
        cx.notify();
    }
    pub fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.titles.len() {
            if self.selected != index {
                self.selected = index;
                cx.emit(DocumentTabEvent::Selected(index));
            }
            self.reveal();
            cx.notify();
        }
    }
    pub fn close(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.titles.len() {
            return;
        }
        self.titles.remove(index);
        if index < self.selected {
            self.selected -= 1;
        }
        self.selected = self.selected.min(self.titles.len().saturating_sub(1));
        self.context_tab = None;
        self.reveal();
        cx.emit(DocumentTabEvent::Closed(index));
        cx.notify();
    }
    fn tab_width(&self, index: usize) -> i32 {
        Font::Bold.measure_in(&self.titles[index], self.language) + 24
    }
    fn reveal(&mut self) {
        if self.titles.is_empty() {
            self.offset = 0;
            return;
        }
        let left = (0..self.selected)
            .map(|index| self.tab_width(index))
            .sum::<i32>();
        let right = left + self.tab_width(self.selected);
        if left < self.offset {
            self.offset = left;
        }
        if right > self.offset + self.width {
            self.offset = right - self.width;
        }
        let total = (0..self.titles.len())
            .map(|index| self.tab_width(index))
            .sum::<i32>();
        self.offset = self.offset.clamp(0, (total - self.width).max(0));
    }
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.context_tab.is_some() {
            match event.keystroke.key.as_str() {
                "escape" => self.context_tab = None,
                "enter" | "space" => self.close(self.context_tab.unwrap(), cx),
                _ => return,
            }
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if (event.keystroke.modifiers.platform || event.keystroke.modifiers.control)
            && event.keystroke.key == "w"
        {
            self.close(self.selected, cx);
            cx.stop_propagation();
            return;
        }
        if self.titles.is_empty() {
            return;
        }
        let next = match event.keystroke.key.as_str() {
            "left" | "up" => self.selected.saturating_sub(1),
            "right" | "down" => (self.selected + 1).min(self.titles.len() - 1),
            "home" => 0,
            "end" => self.titles.len() - 1,
            "f10" if event.keystroke.modifiers.shift => {
                self.context_tab = Some(self.selected);
                cx.notify();
                cx.stop_propagation();
                return;
            }
            _ => return,
        };
        self.select(next, cx);
        cx.stop_propagation();
    }
}
impl Render for DocumentTabs {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let language = self.language;
        let focused = self.focus.is_focused(window) && window.last_input_was_keyboard();
        let width = self.width;
        let height = self.height;
        let mut left = -self.offset;
        let mut tabs = Vec::new();
        let mut selected_bounds = (0, 0);
        for (index, title) in self.titles.iter().enumerate() {
            let title = title.clone();
            let tab_width = self.tab_width(index);
            let selected = index == self.selected;
            let x = left;
            left += tab_width;
            if selected {
                selected_bounds = (x, tab_width);
            }
            tabs.push(
                div()
                    .id(index)
                    .role(Role::Tab)
                    .aria_label(title.clone())
                    .aria_selected(selected)
                    .absolute()
                    .left(px(x as f32))
                    .top(px(if selected { 0. } else { 2. }))
                    .w(px(tab_width as f32))
                    .h(px(if selected { 25. } else { 22. }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            window.focus(&this.focus, cx);
                            this.select(index, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, _, window, cx| {
                            window.focus(&this.focus, cx);
                            this.context_tab = Some(index);
                            cx.notify();
                            cx.stop_propagation();
                        }),
                    )
                    .child(drawing(
                        tab_width as f32,
                        if selected { 25. } else { 22. },
                        move |pen, _, _| {
                            pen.language = language;
                            draw_tab(pen, &title, selected);
                            if focused && selected {
                                pen.hline(9, 19, tab_width - 18, theme.accent.dark);
                            }
                        },
                    )),
            );
        }
        let mut body = div()
            .absolute()
            .left(px(4.))
            .top(px(28.))
            .w(px((width - 8) as f32))
            .h(px((height - 32).max(0) as f32))
            .overflow_hidden();
        if !self.titles.is_empty()
            && let Some(render) = self.content.clone()
        {
            body = body.child(render(self.selected, window, cx));
        }
        let empty = self.titles.is_empty();
        let mut root = div()
            .id("document-tabs")
            .role(Role::TabList)
            .track_focus(&self.focus)
            .tab_stop(true)
            .relative()
            .w(px(width as f32))
            .h(px(height as f32))
            .flex_shrink_0()
            .on_key_down(cx.listener(Self::key_down))
            .child(drawing(width as f32, height as f32, move |pen, _, _| {
                pen.language = language;
                pen.rect(0, 24, width, height - 24, GRAY_2);
                pen.frame(0, 24, width, height - 24, BLACK);
                pen.bevel(1, 25, width - 2, height - 26, WHITE, GRAY_7);
                if !empty {
                    let (left, tab_width) = selected_bounds;
                    pen.hline(
                        left.max(1),
                        24,
                        (left + tab_width - 1).min(width - 1) - left.max(1),
                        GRAY_2,
                    );
                }
                if empty {
                    let title = crate::tr(language, "noDocuments");
                    pen.text(
                        (width - Font::Bold.measure_in(&title, language)) / 2,
                        (height + 24) / 2 + 5,
                        &title,
                        Font::Bold,
                        GRAY_8,
                    );
                }
            }))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .w_full()
                    .h(px(25.))
                    .overflow_hidden()
                    .children(tabs),
            )
            .child(body);
        if let Some(index) = self.context_tab {
            let title = crate::tr(language, "close");
            root = root.child(
                deferred(
                    anchored()
                        .anchor(Anchor::TopLeft)
                        .snap_to_window_with_margin(px(4.))
                        .child(
                            div()
                                .id("tab-context-menu")
                                .role(Role::Menu)
                                .w(px(96.))
                                .h(px(22.))
                                .occlude()
                                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                    this.context_tab = None;
                                    cx.notify();
                                    cx.stop_propagation();
                                }))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .on_mouse_up(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        this.close(index, cx);
                                        cx.stop_propagation();
                                    }),
                                )
                                .child(drawing(96., 22., move |pen, _, _| {
                                    pen.language = language;
                                    pen.raised(GRAY_1);
                                    pen.text(10, 15, &title, Font::Bold, BLACK);
                                })),
                        ),
                )
                .priority(2),
            );
        }
        root
    }
}
fn draw_tab(pen: &mut Pen, title: &str, selected: bool) {
    let width = pen.width;
    let height = pen.height;
    let face = if selected { GRAY_2 } else { GRAY_4 };
    pen.rect(1, 3, width - 2, height - 3, face);
    pen.rect(3, 1, width - 6, 2, face);
    pen.hline(3, 0, width - 6, BLACK);
    pen.hline(2, 1, 1, BLACK);
    pen.hline(width - 3, 1, 1, BLACK);
    pen.dot(1, 2, BLACK);
    pen.dot(width - 2, 2, BLACK);
    pen.vline(0, 3, height - 3, BLACK);
    pen.vline(width - 1, 3, height - 3, BLACK);
    pen.hline(3, 1, width - 6, WHITE);
    pen.vline(1, 3, height - 3, WHITE);
    pen.vline(width - 2, 3, height - 3, GRAY_7);
    pen.text(
        (width - Font::Bold.measure_in(title, pen.language)) / 2,
        16,
        title,
        Font::Bold,
        BLACK,
    );
}

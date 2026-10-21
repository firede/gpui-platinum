use gpui::{
    Anchor, App, Context, EventEmitter, FocusHandle, Focusable, KeyDownEvent, MouseButton, Role,
    Window, anchored, deferred, div, prelude::*, px,
};

use crate::{Font, Language, Theme, paint::drawing, theme::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathEvent(pub usize);

pub struct Breadcrumbs {
    focus: FocusHandle,
    path: Vec<String>,
    active: usize,
    theme: Theme,
    language: Language,
}
impl EventEmitter<PathEvent> for Breadcrumbs {}
impl Focusable for Breadcrumbs {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Breadcrumbs {
    pub fn new(path: Vec<String>, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            active: path.len().saturating_sub(1),
            path,
            theme: Theme::default(),
            language: Language::English,
        }
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }
    pub fn path(&self) -> &[String] {
        &self.path
    }
    pub fn set_path(&mut self, path: Vec<String>, cx: &mut Context<Self>) {
        self.path = path;
        self.active = self.path.len().saturating_sub(1);
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
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.path.is_empty() {
            return;
        }
        match event.keystroke.key.as_str() {
            "left" => self.active = self.active.saturating_sub(1),
            "right" => self.active = (self.active + 1).min(self.path.len() - 1),
            "home" => self.active = 0,
            "end" => self.active = self.path.len() - 1,
            "enter" | "space" => cx.emit(PathEvent(self.active)),
            _ => return,
        }
        cx.notify();
        cx.stop_propagation();
    }
}
impl Render for Breadcrumbs {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.language;
        let theme = self.theme;
        let focused = self.focus.is_focused(window) && window.last_input_was_keyboard();
        div()
            .id("breadcrumbs")
            .role(Role::Group)
            .aria_label(crate::tr(language, "path"))
            .track_focus(&self.focus)
            .tab_stop(true)
            .flex()
            .h(px(22.))
            .flex_shrink_0()
            .on_key_down(cx.listener(Self::key_down))
            .children(self.path.iter().enumerate().map(|(index, title)| {
                let title = title.clone();
                let last = index + 1 == self.path.len();
                let active = index == self.active;
                let text_width = Font::Bold.measure_in(&title, language);
                let width = text_width + if last { 8 } else { 22 };
                div()
                    .id(index)
                    .role(Role::Button)
                    .aria_label(title.clone())
                    .w(px(width as f32))
                    .h_full()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            window.focus(&this.focus, cx);
                            this.active = index;
                            cx.emit(PathEvent(index));
                            cx.notify();
                            cx.stop_propagation();
                        }),
                    )
                    .child(drawing(width as f32, 22., move |pen, _, _| {
                        pen.language = language;
                        pen.text(4, 15, &title, Font::Bold, if last { BLACK } else { GRAY_9 });
                        if focused && active {
                            pen.rect(3, 18, text_width + 2, 2, theme.accent.dark);
                        }
                        if !last {
                            for row in 0..5 {
                                pen.hline(text_width + 12, 8 + row, 3 - (row - 2i32).abs(), GRAY_7);
                            }
                        }
                    }))
            }))
    }
}

pub struct PathPopup {
    focus: FocusHandle,
    path: Vec<String>,
    open: bool,
    highlighted: usize,
    width: i32,
    theme: Theme,
    language: Language,
}
impl EventEmitter<PathEvent> for PathPopup {}
impl Focusable for PathPopup {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl PathPopup {
    pub fn new(path: Vec<String>, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            highlighted: path.len().saturating_sub(1),
            path,
            open: false,
            width: 180,
            theme: Theme::default(),
            language: Language::English,
        }
    }
    pub fn width(mut self, width: i32) -> Self {
        self.width = width.max(60);
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
    pub fn path(&self) -> &[String] {
        &self.path
    }
    pub fn set_path(&mut self, path: Vec<String>, cx: &mut Context<Self>) {
        self.path = path;
        self.highlighted = self.path.len().saturating_sub(1);
        self.open = false;
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
    fn choose(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.path.len() {
            self.open = false;
            cx.emit(PathEvent(index));
            cx.notify();
        }
    }
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.path.is_empty() {
            return;
        }
        match event.keystroke.key.as_str() {
            "escape" => self.open = false,
            "enter" | "space" => {
                if self.open {
                    self.choose(self.highlighted, cx);
                } else {
                    self.open = true;
                    self.highlighted = self.path.len() - 1;
                }
            }
            "down" => {
                if self.open {
                    self.highlighted = self.highlighted.saturating_sub(1);
                } else {
                    self.open = true;
                    self.highlighted = self.path.len() - 1;
                }
            }
            "up" => {
                if self.open {
                    self.highlighted = (self.highlighted + 1).min(self.path.len() - 1);
                } else {
                    self.open = true;
                    self.highlighted = self.path.len() - 1;
                }
            }
            "home" => {
                self.open = true;
                self.highlighted = self.path.len() - 1;
            }
            "end" => {
                self.open = true;
                self.highlighted = 0;
            }
            _ => return,
        }
        cx.notify();
        cx.stop_propagation();
    }
}
impl Render for PathPopup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = self.width;
        let language = self.language;
        let theme = self.theme;
        let focused = self.focus.is_focused(window) && window.last_input_was_keyboard();
        let title = self.path.last().cloned().unwrap_or_default();
        let mut root = div()
            .id("path-popup")
            .role(Role::ComboBox)
            .aria_label(crate::tr(language, "path"))
            .aria_value(title.clone())
            .aria_expanded(self.open)
            .track_focus(&self.focus)
            .tab_stop(true)
            .relative()
            .w(px(width as f32))
            .h(px(22.))
            .flex_shrink_0()
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    window.focus(&this.focus, cx);
                    if !this.path.is_empty() {
                        this.open = !this.open;
                        this.highlighted = this.path.len() - 1;
                        cx.notify();
                    }
                    cx.stop_propagation();
                }),
            )
            .child(drawing(width as f32, 22., move |pen, _, _| {
                pen.language = language;
                pen.raised(GRAY_2);
                pen.text(
                    21,
                    15,
                    &super::icons::fit_title(&title, width - 41, language),
                    Font::Small,
                    BLACK,
                );
                for (row, line) in [
                    "###.......",
                    "#..######.",
                    "#.......#.",
                    "#.......#.",
                    "#.......#.",
                    "#########.",
                ]
                .iter()
                .enumerate()
                {
                    for (col, ch) in line.chars().enumerate() {
                        if ch == '#' {
                            pen.dot(7 + col as i32, 8 + row as i32, BLACK);
                        }
                    }
                }
                for row in 0..4 {
                    pen.hline(width - 14 + row, 9 + row, 7 - row * 2, BLACK);
                }
                if focused {
                    pen.frame(2, 2, width - 4, 18, theme.accent.dark);
                }
            }));
        if self.open {
            let current = self.path.len().saturating_sub(1);
            let menu = div()
                .id("path-menu")
                .role(Role::Menu)
                .w(px(width as f32))
                .p(px(1.))
                .bg(gpui::rgb(GRAY_1))
                .border_1()
                .border_color(gpui::rgb(BLACK))
                .occlude()
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.open = false;
                    cx.notify();
                    cx.stop_propagation();
                }))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .children(self.path.iter().enumerate().rev().map(|(index, title)| {
                    let title = title.clone();
                    let highlighted = index == self.highlighted;
                    div()
                        .id(index)
                        .role(Role::MenuItem)
                        .aria_label(title.clone())
                        .aria_selected(index == current)
                        .h(px(20.))
                        .w_full()
                        .on_mouse_move(cx.listener(move |this, _, _, cx| {
                            if this.highlighted != index {
                                this.highlighted = index;
                                cx.notify();
                            }
                        }))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.choose(index, cx);
                                cx.stop_propagation();
                            }),
                        )
                        .child(drawing((width - 4) as f32, 20., move |pen, _, _| {
                            pen.language = language;
                            pen.rect(
                                0,
                                0,
                                pen.width,
                                20,
                                if highlighted {
                                    theme.accent.dark
                                } else {
                                    GRAY_1
                                },
                            );
                            let ink = if highlighted { WHITE } else { BLACK };
                            if current == index {
                                pen.text(3, 14, "✓", Font::Small, ink);
                            }
                            pen.text(18, 14, &title, Font::Bold, ink);
                        }))
                }));
            root = root.child(
                deferred(
                    anchored()
                        .anchor(Anchor::TopLeft)
                        .snap_to_window_with_margin(px(4.))
                        .child(menu),
                )
                .priority(2),
            );
        }
        root
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageChanged(pub usize);

pub struct Pagination {
    focus: FocusHandle,
    page: usize,
    pages: usize,
    theme: Theme,
    language: Language,
}
impl EventEmitter<PageChanged> for Pagination {}
impl Focusable for Pagination {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Pagination {
    pub fn new(page: usize, pages: usize, cx: &mut Context<Self>) -> Self {
        let pages = pages.max(1);
        Self {
            focus: cx.focus_handle().tab_stop(true),
            page: page.clamp(1, pages),
            pages,
            theme: Theme::default(),
            language: Language::English,
        }
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }
    pub fn selection(&self) -> usize {
        self.page
    }
    pub fn pages(&self) -> usize {
        self.pages
    }
    pub fn select(&mut self, page: usize, cx: &mut Context<Self>) {
        let page = page.clamp(1, self.pages);
        if self.page != page {
            self.page = page;
            cx.emit(PageChanged(page));
            cx.notify();
        }
    }
    pub fn set_pages(&mut self, pages: usize, cx: &mut Context<Self>) {
        self.pages = pages.max(1);
        self.select(self.page, cx);
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
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let page = match event.keystroke.key.as_str() {
            "left" | "up" | "pageup" => self.page.saturating_sub(1),
            "right" | "down" | "pagedown" => self.page + 1,
            "home" => 1,
            "end" => self.pages,
            _ => return,
        };
        self.select(page, cx);
        cx.stop_propagation();
    }
}
impl Render for Pagination {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.language;
        let theme = self.theme;
        let focused = self.focus.is_focused(window) && window.last_input_was_keyboard();
        let title = crate::tr(language, "pageOf")
            .replace("{page}", &self.page.to_string())
            .replace("{pages}", &self.pages.to_string());
        let width = Font::Bold.measure_in(&title, language);
        div()
            .id("pagination")
            .role(Role::Group)
            .aria_label(crate::tr(language, "pages"))
            .track_focus(&self.focus)
            .tab_stop(true)
            .flex()
            .items_center()
            .gap(px(8.))
            .h(px(22.))
            .flex_shrink_0()
            .on_key_down(cx.listener(Self::key_down))
            .child(
                page_arrow("previous", false, self.page > 1, theme, language).on_click(
                    cx.listener(|this, _, window, cx| {
                        window.focus(&this.focus, cx);
                        this.select(this.page.saturating_sub(1), cx);
                    }),
                ),
            )
            .child(drawing(width as f32, 22., move |pen, _, _| {
                pen.language = language;
                pen.text_centered(&title, Font::Bold, BLACK);
                if focused {
                    pen.hline(0, 20, width, theme.accent.dark);
                }
            }))
            .child(
                page_arrow("next", true, self.page < self.pages, theme, language).on_click(
                    cx.listener(|this, _, window, cx| {
                        window.focus(&this.focus, cx);
                        this.select(this.page + 1, cx);
                    }),
                ),
            )
    }
}

/// The month arrows of the calendar, on the bevel buttons that turn pages.
const PAGE_BACK: [&str; 7] = ["...#", "..##", ".###", "####", ".###", "..##", "...#"];
const PAGE_NEXT: [&str; 7] = ["#...", "##..", "###.", "####", "###.", "##..", "#..."];

/// A square bevel button turning to the previous page, or the next, which
/// leaves the keyboard to the pagination around it.
fn page_arrow(
    id: &'static str,
    next: bool,
    enabled: bool,
    theme: Theme,
    language: Language,
) -> crate::Control {
    let label = if next { "nextPage" } else { "previousPage" };
    crate::bevel_button(id, crate::tr(language, label))
        .icon_only(if next { &PAGE_NEXT } else { &PAGE_BACK })
        .width(22)
        .height(22)
        .disabled(!enabled)
        .tab_stop(false)
        .theme(theme)
        .language(language)
}

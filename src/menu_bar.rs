//! Menu bars and context menus with shared keyboard navigation.

use crate::{
    Font, Language, MenuItem, Theme,
    menu::{MENU_FLASH, MenuPress, flash_menu},
    paint::drawing,
    theme::*,
};
use gpui::{prelude::*, *};

#[derive(Clone, Debug)]
pub struct Menu {
    pub title: SharedString,
    pub items: Vec<MenuItem>,
    pub icon: Option<&'static [&'static str]>,
    pub icon_title: bool,
}

impl Menu {
    pub fn new(title: impl Into<SharedString>, items: Vec<MenuItem>) -> Self {
        Self {
            title: title.into(),
            items,
            icon: None,
            icon_title: false,
        }
    }
    pub fn icon(mut self, icon: &'static [&'static str]) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn icon_title(mut self, show_title: bool) -> Self {
        self.icon_title = show_title;
        self
    }
    fn title_width(&self, language: Language) -> i32 {
        let text = Font::Bold.measure_in(&self.title, language);
        let width = self.icon.map_or(text, |icon| {
            let width = icon.iter().map(|row| row.len()).max().unwrap_or(0) as i32;
            width + if self.icon_title { text + 4 } else { 0 }
        });
        width + 18
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuChoice {
    pub menu: usize,
    pub path: Vec<usize>,
}

pub struct MenuBar {
    menus: Vec<Menu>,
    open: Option<usize>,
    path: Vec<usize>,
    focus: FocusHandle,
    previous_focus: Option<FocusHandle>,
    theme: Theme,
    language: Language,
    context: bool,
    position: Option<Point<Pixels>>,
    bounds: Option<Bounds<Pixels>>,
    closed_submenu: Option<Vec<usize>>,
    press: Option<MenuPress>,
    flash: Option<Task<()>>,
    blink_off: bool,
    title_flash: Option<(usize, Task<()>)>,
}

impl EventEmitter<MenuChoice> for MenuBar {}
impl Focusable for MenuBar {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl MenuBar {
    pub fn new(menus: Vec<Menu>, cx: &mut Context<Self>) -> Self {
        Self {
            menus,
            open: None,
            path: Vec::new(),
            focus: cx.focus_handle().tab_stop(true),
            previous_focus: None,
            theme: Theme::default(),
            language: Language::English,
            context: false,
            position: None,
            bounds: None,
            closed_submenu: None,
            press: None,
            flash: None,
            blink_off: false,
            title_flash: None,
        }
    }
    pub fn context_menu(items: Vec<MenuItem>, cx: &mut Context<Self>) -> Self {
        let mut menu = Self::new(vec![Menu::new("", items)], cx);
        menu.context = true;
        menu
    }
    pub fn show_at(
        &mut self,
        index: usize,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.position = Some(position);
        self.show(index, window, cx);
    }
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        cx.notify();
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_menus(&mut self, menus: Vec<Menu>, cx: &mut Context<Self>) {
        self.menus = menus;
        self.open = None;
        self.path.clear();
        cx.notify();
    }
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = None;
        self.path.clear();
        self.closed_submenu = None;
        self.press = None;
        self.flash = None;
        self.blink_off = false;
        if let Some(focus) = self.previous_focus.take() {
            window.focus(&focus, cx);
        }
        cx.notify();
    }
    fn show(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .menus
            .get(index)
            .is_none_or(|menu| menu.items.is_empty())
        {
            return;
        }
        if self.open.is_none() && self.previous_focus.is_none() {
            self.previous_focus = window.focused(cx);
        }
        self.open = Some(index);
        self.closed_submenu = None;
        self.press = None;
        self.path = first_enabled(&self.menus[index].items)
            .into_iter()
            .collect();
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn choose(&mut self, path: Vec<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.open.filter(|_| self.flash.is_none()) else {
            return;
        };
        let Some(item) = item_at(&self.menus[menu].items, &path) else {
            return;
        };
        if item.disabled || item.separator {
            return;
        }
        if !item.submenu.is_empty() {
            self.closed_submenu = None;
            self.path = path;
            if let Some(first) = first_enabled(&item.submenu) {
                self.path.push(first);
            }
            cx.notify();
            return;
        }
        // The item blinks, the menu open, before the menu closes and the
        // choice goes out.
        self.path = path.clone();
        self.blink_off = true;
        self.flash = Some(flash_menu(
            window,
            cx,
            |this| &mut this.blink_off,
            move |this, window, cx| {
                this.close(window, cx);
                cx.emit(MenuChoice { menu, path });
            },
        ));
        cx.notify();
    }
    /// Records the press that opened the open menu, which, held and let go
    /// off its items and title, closes it.
    pub(crate) fn press(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        self.press = Some(MenuPress::new(position, cx));
    }
    /// Ends a press at position, over the item at path if any: the item is
    /// chosen; a release on the open menu's title, or a quick click, leaves
    /// the menu open to choose from; one held and let go elsewhere closes
    /// it. A release after the menu was left open takes only an item.
    fn release(
        &mut self,
        path: Option<Vec<usize>>,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let press = self.press.take();
        if self.flash.is_some() {
            return;
        }
        let Some(press) = press else {
            if let Some(path) = path {
                self.choose(path, window, cx);
            }
            return;
        };
        if let Some(path) = path
            && let Some(menu) = self.open
            && item_at(&self.menus[menu].items, &path)
                .is_some_and(|item| !item.disabled && !item.separator && item.submenu.is_empty())
        {
            self.choose(path, window, cx);
            return;
        }
        if self.open.is_some() && self.title_at(position) == self.open
            || press.quick_click(position, cx)
        {
            return;
        }
        self.close(window, cx);
    }
    /// The menu whose title on the bar is at position.
    fn title_at(&self, position: Point<Pixels>) -> Option<usize> {
        let bounds = self.bounds.filter(|_| !self.context)?;
        let x = f32::from(position.x - bounds.origin.x) as i32;
        let y = f32::from(position.y - bounds.origin.y);
        if !(0. ..19.).contains(&y) {
            return None;
        }
        let mut left = 10;
        for (index, menu) in self.menus.iter().enumerate() {
            let right = left + menu.title_width(self.language);
            if (left..right).contains(&x) {
                return Some(index);
            }
            left = right;
        }
        None
    }
    /// Highlights a menu's title for MENU_FLASH, as a keyboard equivalent
    /// of one of its items does.
    fn flash_title(&mut self, menu: usize, cx: &mut Context<Self>) {
        if self.context {
            return;
        }
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(MENU_FLASH).await;
            this.update(cx, |this, cx| {
                this.title_flash = None;
                cx.notify();
            })
            .ok();
        });
        self.title_flash = Some((menu, task));
        cx.notify();
    }
    /// Dispatch a keyboard equivalent from the containing window's key handler.
    pub fn dispatch_shortcut(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !event.keystroke.modifiers.platform
            || event.keystroke.modifiers.alt
            || event.keystroke.modifiers.shift
        {
            return false;
        }
        for (menu, entry) in self.menus.iter().enumerate() {
            if let Some(path) = shortcut_path(&entry.items, &event.keystroke.key) {
                self.close(window, cx);
                self.flash_title(menu, cx);
                cx.emit(MenuChoice { menu, path });
                return true;
            }
        }
        false
    }
    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // The menu waits out the blink of a choice.
        if self.flash.is_some() {
            cx.stop_propagation();
            return;
        }
        if self.dispatch_shortcut(event, window, cx) {
            cx.stop_propagation();
            return;
        }
        let key = event.keystroke.key.as_str();
        let Some(menu) = self.open else {
            if matches!(key, "down" | "space" | "enter") {
                self.show(0, window, cx);
                cx.stop_propagation();
            }
            return;
        };
        match key {
            "escape" => self.close(window, cx),
            "enter" | "space" => self.choose(self.path.clone(), window, cx),
            "left" => {
                if self.path.len() > 1 {
                    self.path.pop();
                    self.closed_submenu = Some(self.path.clone());
                } else if !self.context && !self.menus.is_empty() {
                    self.show((menu + self.menus.len() - 1) % self.menus.len(), window, cx);
                }
            }
            "right" => {
                if item_at(&self.menus[menu].items, &self.path)
                    .is_some_and(|item| !item.submenu.is_empty())
                {
                    self.choose(self.path.clone(), window, cx);
                } else if !self.context && !self.menus.is_empty() {
                    self.show((menu + 1) % self.menus.len(), window, cx);
                }
            }
            "up" | "down" | "home" | "end" => {
                let level = self.path.len().saturating_sub(1);
                let items = items_at(&self.menus[menu].items, &self.path[..level]);
                if let Some(next) = next_enabled(items, self.path.last().copied(), key) {
                    self.path.truncate(level);
                    self.path.push(next);
                    self.closed_submenu = None;
                }
            }
            "tab" => {
                self.close(window, cx);
                return;
            }
            _ => {
                if event.keystroke.modifiers.platform || key.chars().count() != 1 {
                    return;
                }
                let level = self.path.len().saturating_sub(1);
                let items = items_at(&self.menus[menu].items, &self.path[..level]);
                if let Some(index) = items.iter().position(|item| {
                    !item.disabled
                        && !item.separator
                        && item.title.to_lowercase().starts_with(&key.to_lowercase())
                }) {
                    self.path.truncate(level);
                    self.path.push(index);
                } else {
                    return;
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }
    fn panel(&self, items: &[MenuItem], prefix: Vec<usize>, cx: &mut Context<Self>) -> AnyElement {
        let width = menu_width(items, self.language);
        let level = prefix.len();
        let selected = self.path.get(level).copied();
        let blink_off = self.blink_off && level + 1 == self.path.len();
        let accent = self.theme.accent;
        let language = self.language;
        let mut panel = div()
            .id(SharedString::from(format!("menu-panel-{prefix:?}")))
            .relative()
            .w(px(width as f32))
            .p(px(2.))
            .child(crate::menu::menu_frame())
            .occlude();
        for (index, item) in items.iter().enumerate() {
            let mut path = prefix.clone();
            path.push(index);
            let choose_path = path.clone();
            let accessible_path = path.clone();
            let accessible = cx.entity();
            let item = item.clone();
            let highlighted = selected == Some(index) && !item.disabled && !blink_off;
            let height = if item.separator { 8 } else { 18 };
            panel = panel.child(
                div()
                    .id(index)
                    .role(Role::MenuItem)
                    .aria_label(item.title.clone())
                    .aria_selected(highlighted)
                    .w_full()
                    .h(px(height as f32))
                    .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                        if let Some(press) = &mut this.press {
                            press.track(event.position);
                        }
                        if this.flash.is_some() {
                            return;
                        }
                        if this.path != path || this.closed_submenu.is_some() {
                            this.path = path.clone();
                            this.closed_submenu = None;
                            cx.notify();
                        }
                    }))
                    .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                        accessible.update(cx, |this, cx| {
                            this.choose(accessible_path.clone(), window, cx)
                        });
                    })
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseUpEvent, window, cx| {
                            this.release(Some(choose_path.clone()), event.position, window, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .child(drawing(
                        (width - 4) as f32,
                        height as f32,
                        move |pen, _, _| {
                            pen.language = language;
                            crate::menu::draw_menu_item(pen, &item, highlighted, accent);
                        },
                    )),
            );
        }
        let mut chain = div().flex().items_start().child(panel);
        if let Some(index) = selected
            && let Some(item) = items.get(index)
            && !item.disabled
            && !item.submenu.is_empty()
        {
            let top: i32 = items[..index]
                .iter()
                .map(|item| if item.separator { 8 } else { 18 })
                .sum();
            let mut child_prefix = prefix;
            child_prefix.push(index);
            if self.closed_submenu.as_ref() != Some(&child_prefix) {
                chain = chain.child(div().mt(px(top as f32)).ml(px(-3.)).child(self.panel(
                    &item.submenu,
                    child_prefix,
                    cx,
                )));
            }
        }
        chain.into_any_element()
    }
}

impl Render for MenuBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.open.is_none()
            && let Some(focus) = self.previous_focus.take()
        {
            window.focus(&focus, cx);
        }
        let layout = cx.entity();
        let mut root = div()
            .id("menu-bar")
            .role(Role::MenuBar)
            .track_focus(&self.focus)
            .key_context("MenuBar")
            .relative()
            .flex()
            .flex_shrink_0()
            .on_key_down(cx.listener(Self::key_down));
        if !self.context {
            root = root.child(
                canvas(
                    move |bounds, _, cx| {
                        layout.update(cx, |this, _| this.bounds = Some(bounds));
                    },
                    |bounds, _, window, _| {
                        let mut pen = crate::paint::Pen::new(
                            f32::from(bounds.size.width).round() as i32,
                            f32::from(bounds.size.height).round() as i32,
                        );
                        pen.rect(0, 0, pen.width, pen.height, GRAY_2);
                        pen.hline(0, 0, pen.width, WHITE);
                        pen.hline(0, pen.height - 1, pen.width, BLACK);
                        pen.paint(bounds.origin, window);
                    },
                )
                .absolute()
                .size_full(),
            );
            root = root.h(px(20.)).pl(px(10.)).bg(rgb(GRAY_2)).tab_stop(true);
            for (index, menu) in self.menus.iter().enumerate() {
                let title = menu.title.clone();
                let width = menu.title_width(self.language);
                let icon = menu.icon;
                let icon_title = menu.icon_title;
                let selected = self
                    .open
                    .or(self.title_flash.as_ref().map(|(menu, _)| *menu))
                    == Some(index);
                let theme = self.theme;
                let language = self.language;
                let accessible = cx.entity();
                root = root.child(
                    div()
                        .id(index)
                        .role(Role::MenuItem)
                        .aria_label(title.clone())
                        .aria_expanded(selected)
                        .w(px(width as f32))
                        .flex_shrink_0()
                        .h(px(19.))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                                cx.stop_propagation();
                                if this.flash.is_some() {
                                    return;
                                }
                                if this.open == Some(index) {
                                    this.close(window, cx);
                                    return;
                                }
                                // A press into a menu left open goes on
                                // as a click.
                                let opening = this.open.is_none();
                                this.show(index, window, cx);
                                if opening {
                                    this.press(event.position, cx);
                                }
                            }),
                        )
                        .on_mouse_move(cx.listener(
                            move |this, event: &MouseMoveEvent, window, cx| {
                                if this.flash.is_some() {
                                    return;
                                }
                                if this.open.is_some() && this.open != Some(index) {
                                    // The press goes on into the menu it moved to.
                                    let press = this.press.take();
                                    this.show(index, window, cx);
                                    this.press = press;
                                }
                                if let Some(press) = &mut this.press {
                                    press.track(event.position);
                                }
                            },
                        ))
                        .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                            accessible.update(cx, |this, cx| {
                                if this.open == Some(index) {
                                    this.close(window, cx);
                                } else {
                                    this.show(index, window, cx);
                                }
                            });
                        })
                        .child(drawing(width as f32, 19., move |pen, _, _| {
                            pen.language = language;
                            if selected {
                                pen.rect(0, 0, width, 19, theme.accent.dark);
                            }
                            let mut text_left = 9;
                            if let Some(rows) = icon {
                                let top = rows
                                    .iter()
                                    .position(|row| {
                                        row.bytes().any(|pixel| b"#wo+:".contains(&pixel))
                                    })
                                    .unwrap_or(0);
                                let bottom = rows
                                    .iter()
                                    .rposition(|row| {
                                        row.bytes().any(|pixel| b"#wo+:".contains(&pixel))
                                    })
                                    .map_or(rows.len(), |row| row + 1);
                                crate::bitmap_icons::draw_bitmap_icon(
                                    pen,
                                    rows,
                                    9,
                                    (19 - (bottom - top) as i32) / 2 - top as i32,
                                    theme.accent,
                                    selected,
                                    false,
                                );
                                if !icon_title {
                                    return;
                                }
                                text_left +=
                                    rows.iter().map(|row| row.len()).max().unwrap_or(0) as i32 + 4;
                            }
                            pen.text(
                                text_left,
                                14,
                                &title,
                                Font::Bold,
                                if selected { WHITE } else { BLACK },
                            );
                        })),
                );
            }
        }
        if let Some(index) = self.open {
            let panel = div()
                .id("menu-panels")
                .role(Role::Menu)
                .occlude()
                .on_mouse_down_out(cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    if this.flash.is_some() {
                        return;
                    }
                    if this.context
                        || this
                            .bounds
                            .is_none_or(|bounds| !bounds.contains(&event.position))
                    {
                        this.close(window, cx);
                    }
                }))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, event: &MouseUpEvent, window, cx| {
                        this.release(None, event.position, window, cx)
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|this, event: &MouseUpEvent, window, cx| {
                        this.release(None, event.position, window, cx)
                    }),
                )
                .child(self.panel(&self.menus[index].items, Vec::new(), cx));
            let mut anchor = anchored()
                .anchor(Anchor::TopLeft)
                .snap_to_window_with_margin(px(2.));
            if let Some(position) = self.position {
                anchor = anchor.position(position);
            } else {
                let x: i32 = self.menus[..index]
                    .iter()
                    .map(|menu| menu.title_width(self.language))
                    .sum();
                anchor = anchor.offset(point(px((10 + x) as f32), px(19.)));
            }
            root = root.child(
                div()
                    .absolute()
                    .left(px(0.))
                    .top(px(0.))
                    .child(deferred(anchor.child(panel)).priority(20)),
            );
        }
        root
    }
}

pub struct ContextMenu {
    menu: Entity<MenuBar>,
    focus: FocusHandle,
    label: SharedString,
    width: i32,
    height: i32,
    language: Language,
}

impl EventEmitter<MenuChoice> for ContextMenu {}
impl Focusable for ContextMenu {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl ContextMenu {
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        self.menu
            .update(cx, |menu, cx| menu.set_language(language, cx));
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.menu.update(cx, |menu, cx| menu.set_theme(theme, cx));
    }
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.label = label.into();
        cx.notify();
    }
    pub fn set_items(&mut self, items: Vec<MenuItem>, cx: &mut Context<Self>) {
        self.menu.update(cx, |menu, cx| {
            menu.set_menus(vec![Menu::new("", items)], cx)
        });
    }
    pub fn new(items: Vec<MenuItem>, cx: &mut Context<Self>) -> Self {
        let menu = cx.new(|cx| MenuBar::context_menu(items, cx));
        cx.subscribe(&menu, |_, _, choice: &MenuChoice, cx| {
            cx.emit(choice.clone())
        })
        .detach();
        Self {
            menu,
            focus: cx.focus_handle().tab_stop(true),
            label: "".into(),
            width: 260,
            height: 50,
            language: Language::English,
        }
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    fn show(
        &mut self,
        position: Option<Point<Pixels>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu.update(cx, |menu, cx| {
            menu.position = position;
            menu.show(0, window, cx);
        });
    }
}
impl Render for ContextMenu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text = self.label.clone();
        let language = self.language;
        div()
            .id("context-target")
            .role(Role::Group)
            .aria_label(text.clone())
            .track_focus(&self.focus)
            .tab_stop(true)
            .w(px(self.width as f32))
            .h(px(self.height as f32))
            .relative()
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.show(Some(event.position), window, cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    if event.modifiers.control {
                        this.show(Some(event.position), window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "menu"
                    || (event.keystroke.key == "f10" && event.keystroke.modifiers.shift)
                {
                    this.show(None, window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(drawing(
                self.width as f32,
                self.height as f32,
                move |pen, _, _| {
                    pen.language = language;
                    pen.sunken();
                    pen.text_centered(&text, Font::Small, GRAY_8);
                },
            ))
            .child(div().absolute().inset_0().child(self.menu.clone()))
    }
}

fn first_enabled(items: &[MenuItem]) -> Option<usize> {
    items
        .iter()
        .position(|item| !item.disabled && !item.separator)
}
fn items_at<'a>(mut items: &'a [MenuItem], path: &[usize]) -> &'a [MenuItem] {
    for index in path {
        let Some(item) = items.get(*index) else {
            return &[];
        };
        items = &item.submenu;
    }
    items
}
fn item_at<'a>(items: &'a [MenuItem], path: &[usize]) -> Option<&'a MenuItem> {
    let (last, parents) = path.split_last()?;
    items_at(items, parents).get(*last)
}
fn shortcut_path(items: &[MenuItem], key: &str) -> Option<Vec<usize>> {
    for (index, item) in items.iter().enumerate() {
        if item.disabled || item.separator {
            continue;
        }
        if item
            .key
            .as_ref()
            .is_some_and(|shortcut| shortcut.eq_ignore_ascii_case(key))
        {
            return Some(vec![index]);
        }
        if let Some(mut path) = shortcut_path(&item.submenu, key) {
            path.insert(0, index);
            return Some(path);
        }
    }
    None
}
fn menu_width(items: &[MenuItem], language: Language) -> i32 {
    items
        .iter()
        .map(|item| {
            30 + Font::Bold.measure_in(&item.title, language)
                + item
                    .key
                    .as_ref()
                    .map_or(if item.submenu.is_empty() { 0 } else { 20 }, |key| {
                        20 + Font::Bold.measure_in(&format!("⌘{}", key.to_uppercase()), language)
                    })
        })
        .max()
        .unwrap_or(80)
        + 4
}

fn next_enabled(items: &[MenuItem], current: Option<usize>, key: &str) -> Option<usize> {
    let enabled: Vec<_> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.disabled && !item.separator)
        .map(|(index, _)| index)
        .collect();
    match key {
        "home" => enabled.first().copied(),
        "end" => enabled.last().copied(),
        "up" => current
            .and_then(|current| {
                enabled
                    .iter()
                    .rev()
                    .find(|index| **index < current)
                    .copied()
            })
            .or_else(|| enabled.last().copied()),
        _ => current
            .and_then(|current| enabled.iter().find(|index| **index > current).copied())
            .or_else(|| enabled.first().copied()),
    }
}

#[cfg(test)]
mod tests {
    use super::{MenuItem, next_enabled, shortcut_path};
    #[test]
    fn arrows_skip_disabled_items_from_the_actual_selected_index() {
        let items = vec![
            MenuItem::new("One"),
            MenuItem::new("Two").disabled(true),
            MenuItem::separator(),
            MenuItem::new("Three"),
        ];
        assert_eq!(next_enabled(&items, Some(3), "up"), Some(0));
        assert_eq!(next_enabled(&items, Some(0), "down"), Some(3));
        assert_eq!(next_enabled(&items, Some(3), "down"), Some(0));
        assert_eq!(next_enabled(&items, Some(1), "down"), Some(3));
    }
    #[test]
    fn nested_menu_paths_keep_each_level_and_disabled_sibling_separate() {
        let items = vec![MenuItem::new("Desk").submenu(vec![
            MenuItem::new("Control Panels").submenu(vec![
                MenuItem::new("Appearance"),
                MenuItem::new("Unavailable").disabled(true),
                MenuItem::new("Sound"),
            ]),
            MenuItem::new("Calculator"),
        ])];
        let panels = super::items_at(&items, &[0, 0]);
        assert_eq!(next_enabled(panels, Some(0), "down"), Some(2));
        assert_eq!(
            super::item_at(&items, &[0, 0, 2]).unwrap().title.as_ref(),
            "Sound"
        );
        assert_eq!(
            super::item_at(&items, &[0, 1]).unwrap().title.as_ref(),
            "Calculator"
        );
        assert!(super::item_at(&items, &[0, 0, 3]).is_none());
    }
    #[test]
    fn shortcuts_skip_disabled_branches_and_find_nested_items() {
        let items = vec![
            MenuItem::new("Save").shortcut("s").disabled(true),
            MenuItem::new("Export").submenu(vec![MenuItem::new("Text").shortcut("t")]),
            MenuItem::new("Hidden")
                .disabled(true)
                .submenu(vec![MenuItem::new("Print").shortcut("p")]),
        ];
        assert_eq!(shortcut_path(&items, "s"), None);
        assert_eq!(shortcut_path(&items, "T"), Some(vec![1, 0]));
        assert_eq!(shortcut_path(&items, "p"), None);
    }
}

//! A single keyboard focus group with an overflow menu for narrow toolbars.

use std::{cell::RefCell, rc::Rc};

use crate::{Control, Font, Language, Theme, bevel_button, paint::drawing, theme::*};
use gpui::{prelude::*, *};

const GAP: i32 = 4;
const PADDING: i32 = 6;
const OVERFLOW_WIDTH: i32 = 22;

#[derive(Clone)]
enum Item {
    Control(Box<Control>),
    Separator,
}
impl Item {
    fn width(&self) -> i32 {
        match self {
            Self::Control(control) => control.dimensions().0,
            Self::Separator => 6,
        }
    }
    fn enabled(&self) -> bool {
        matches!(self, Self::Control(control) if !control.disabled)
    }
}

pub struct Toolbar {
    id: ElementId,
    items: Vec<Item>,
    theme: Theme,
    language: Language,
}
impl Toolbar {
    pub fn new() -> Self {
        Self {
            id: "toolbar".into(),
            items: Vec::new(),
            theme: Theme::default(),
            language: Language::English,
        }
    }
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }
    pub fn item(mut self, control: Control) -> Self {
        self.items.push(Item::Control(Box::new(control)));
        self
    }
    pub fn separator(mut self) -> Self {
        self.items.push(Item::Separator);
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
}
impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}
impl IntoElement for Toolbar {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

struct ToolbarState {
    handles: Vec<FocusHandle>,
    current: usize,
    open: bool,
    highlighted: Option<usize>,
}
type SharedState = Rc<RefCell<ToolbarState>>;

fn visible_count(items: &[Item], width: i32) -> usize {
    let room = (width - PADDING * 2).max(0);
    let total: i32 = items.iter().map(|item| item.width() + GAP).sum();
    if total.saturating_sub(GAP) <= room {
        return items.len();
    }
    let mut used = 0;
    items
        .iter()
        .take_while(|item| {
            used += item.width() + GAP;
            used <= room - OVERFLOW_WIDTH - GAP
        })
        .count()
}

fn choose(state: &SharedState, item: &Item, event: &ClickEvent, window: &mut Window, cx: &mut App) {
    let Item::Control(control) = item else {
        return;
    };
    if control.disabled {
        return;
    }
    state.borrow_mut().open = false;
    if let Some(action) = &control.click {
        action(event, window, cx);
    }
    window.refresh();
}

impl Element for Toolbar {
    type RequestLayoutState = ();
    type PrepaintState = AnyElement;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = px(32.).into();
        style.flex_shrink = 0.;
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let shown = visible_count(&self.items, f32::from(bounds.size.width) as i32);
        let overflow = shown < self.items.len();
        let mut enabled: Vec<_> = self.items[..shown]
            .iter()
            .enumerate()
            .filter_map(|(index, item)| item.enabled().then_some(index))
            .collect();
        if overflow {
            enabled.push(shown);
        }
        let state = window.with_element_state(
            id.expect("toolbar id"),
            |previous: Option<SharedState>, _| {
                let state = previous.unwrap_or_else(|| {
                    Rc::new(RefCell::new(ToolbarState {
                        handles: Vec::new(),
                        current: 0,
                        open: false,
                        highlighted: None,
                    }))
                });
                {
                    let mut current = state.borrow_mut();
                    while current.handles.len() <= self.items.len() {
                        current.handles.push(cx.focus_handle());
                    }
                    if !enabled.contains(&current.current) {
                        current.current = enabled.first().copied().unwrap_or(0);
                    }
                    if !overflow {
                        current.open = false;
                    }
                }
                (state.clone(), state)
            },
        );
        let keyboard = state.clone();
        let items = self.items.clone();
        let language = self.language;
        let theme = self.theme;
        let mut root = div()
            .id("toolbar-input")
            .role(Role::Toolbar)
            .relative()
            .flex()
            .items_center()
            .gap(px(GAP as f32))
            .px(px(PADDING as f32))
            .w(bounds.size.width)
            .h(bounds.size.height)
            .child(div().absolute().inset_0().child(drawing(
                f32::from(bounds.size.width),
                32.,
                |pen, _, _| {
                    pen.rect(0, 0, pen.width, 32, GRAY_2);
                    pen.hline(0, 0, pen.width, WHITE);
                    pen.hline(0, 30, pen.width, GRAY_7);
                    pen.hline(0, 31, pen.width, WHITE);
                },
            )))
            .capture_key_down(move |event, window, cx| {
                let key = event.keystroke.key.as_str();
                if event.keystroke.modifiers.modified() {
                    return;
                }
                if keyboard.borrow().open {
                    let choices: Vec<_> = items
                        .iter()
                        .enumerate()
                        .skip(shown)
                        .filter_map(|(index, item)| item.enabled().then_some(index))
                        .collect();
                    match key {
                        "escape" => keyboard.borrow_mut().open = false,
                        "enter" | "space" => {
                            let selected = keyboard.borrow().highlighted;
                            if let Some(index) = selected {
                                choose(
                                    &keyboard,
                                    &items[index],
                                    &ClickEvent::Keyboard(KeyboardClickEvent {
                                        button: KeyboardButton::Enter,
                                        bounds,
                                    }),
                                    window,
                                    cx,
                                );
                            }
                        }
                        "up" | "down" | "home" | "end" => {
                            let current = keyboard.borrow().highlighted;
                            let selected = match key {
                                "home" => choices.first().copied(),
                                "end" => choices.last().copied(),
                                "up" => choices
                                    .iter()
                                    .rev()
                                    .copied()
                                    .find(|index| current.is_some_and(|current| *index < current))
                                    .or_else(|| choices.last().copied()),
                                _ => choices
                                    .iter()
                                    .copied()
                                    .find(|index| current.is_some_and(|current| *index > current))
                                    .or_else(|| choices.first().copied()),
                            };
                            keyboard.borrow_mut().highlighted = selected;
                        }
                        "tab" => {
                            keyboard.borrow_mut().open = false;
                            window.refresh();
                            return;
                        }
                        _ => return,
                    }
                } else {
                    if !matches!(key, "left" | "right" | "home" | "end") || enabled.is_empty() {
                        return;
                    }
                    let current = keyboard.borrow().current;
                    let position = enabled
                        .iter()
                        .position(|index| *index == current)
                        .unwrap_or(0);
                    let next = match key {
                        "home" => 0,
                        "end" => enabled.len() - 1,
                        "left" => (position + enabled.len() - 1) % enabled.len(),
                        _ => (position + 1) % enabled.len(),
                    };
                    let index = enabled[next];
                    keyboard.borrow_mut().current = index;
                    let handle = keyboard.borrow().handles[index].clone();
                    window.focus(&handle, cx);
                }
                window.refresh();
                cx.stop_propagation();
            });
        // The background is absolute so the tools alone determine their positions.
        // The same focus handle survives reconstructing each tool on a new frame.
        for (index, item) in self.items[..shown].iter().enumerate() {
            root = match item {
                Item::Separator => root.child(toolbar_separator()),
                Item::Control(control) => {
                    let pointer = state.clone();
                    let current = state.borrow().current;
                    let focus = state.borrow().handles[index].clone();
                    root.child(
                        div()
                            .id(("tool", index))
                            .on_mouse_down(MouseButton::Left, move |_, window, _| {
                                pointer.borrow_mut().current = index;
                                window.refresh();
                            })
                            .child(
                                control
                                    .clone()
                                    .focus_handle(&focus)
                                    .tab_stop(current == index),
                            ),
                    )
                }
            };
        }
        if overflow {
            let opened = state.clone();
            let focus = state.borrow().handles[shown].clone();
            let current = state.borrow().current;
            let first = self
                .items
                .iter()
                .enumerate()
                .skip(shown)
                .find_map(|(index, item)| item.enabled().then_some(index));
            root = root.child(div().flex_1()).child(
                bevel_button("more", "»")
                    .width(OVERFLOW_WIDTH)
                    .height(22)
                    .theme(theme)
                    .language(language)
                    .focus_handle(&focus)
                    .tab_stop(current == shown)
                    .on_click(move |_, window, _| {
                        let mut state = opened.borrow_mut();
                        state.current = shown;
                        state.open = !state.open;
                        state.highlighted = first;
                        window.refresh();
                    }),
            );
            if state.borrow().open {
                let width = self.items[shown..]
                    .iter()
                    .filter_map(|item| match item {
                        Item::Control(control) => {
                            Some(Font::Bold.measure_in(&control.label, language) + 32)
                        }
                        _ => None,
                    })
                    .max()
                    .unwrap_or(80);
                let outside = state.clone();
                let mut menu = div()
                    .id("toolbar-overflow")
                    .role(Role::Menu)
                    .w(px(width as f32))
                    .p(px(1.))
                    .bg(rgb(GRAY_1))
                    .border_1()
                    .border_color(rgb(BLACK))
                    .occlude()
                    .on_mouse_down_out(move |event, window, _| {
                        if !bounds.contains(&event.position) {
                            outside.borrow_mut().open = false;
                            window.refresh();
                        }
                    });
                for (index, item) in self.items.iter().enumerate().skip(shown) {
                    let Item::Control(control) = item else {
                        menu = menu.child(drawing((width - 4) as f32, 6., |pen, _, _| {
                            pen.hline(2, 2, pen.width - 4, GRAY_7);
                            pen.hline(2, 3, pen.width - 4, WHITE);
                        }));
                        continue;
                    };
                    let title = control.label.clone();
                    let disabled = control.disabled;
                    let checked = control.selected;
                    let highlighted = state.borrow().highlighted == Some(index);
                    let hover = state.clone();
                    let clicked = state.clone();
                    let item = item.clone();
                    menu = menu.child(
                        div()
                            .id(index)
                            .role(Role::MenuItem)
                            .aria_label(title.clone())
                            .h(px(20.))
                            .w_full()
                            .on_mouse_move(move |_, window, _| {
                                if !disabled && hover.borrow().highlighted != Some(index) {
                                    hover.borrow_mut().highlighted = Some(index);
                                    window.refresh();
                                }
                            })
                            .on_click(move |event, window, cx| {
                                choose(&clicked, &item, event, window, cx);
                                cx.stop_propagation();
                            })
                            .child(drawing((width - 4) as f32, 20., move |pen, _, _| {
                                pen.language = language;
                                if highlighted && !disabled {
                                    pen.rect(0, 0, pen.width, 20, theme.accent.dark);
                                }
                                let ink = if disabled {
                                    GRAY_8
                                } else if highlighted {
                                    WHITE
                                } else {
                                    BLACK
                                };
                                if checked {
                                    pen.text(3, 14, "✓", Font::Small, ink);
                                }
                                pen.text(18, 14, &title, Font::Bold, ink);
                            })),
                    );
                }
                root = root.child(
                    deferred(
                        anchored()
                            .position(point(bounds.right() - px(width as f32), bounds.bottom()))
                            .anchor(Anchor::TopLeft)
                            .snap_to_window_with_margin(px(4.))
                            .child(menu),
                    )
                    .priority(2),
                );
            }
        }
        let mut element = root.into_any_element();
        element.prepaint_as_root(
            bounds.origin,
            size(
                AvailableSpace::Definite(bounds.size.width),
                AvailableSpace::Definite(bounds.size.height),
            ),
            window,
            cx,
        );
        element
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        element: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx);
    }
}

pub fn toolbar_separator() -> impl IntoElement {
    drawing(6., 22., |pen, _, _| {
        pen.vline(2, 0, 22, GRAY_7);
        pen.vline(3, 0, 22, WHITE);
    })
}

#[cfg(test)]
mod tests {
    use super::{Item, visible_count};
    use crate::bevel_button;

    #[test]
    fn overflow_reserves_room_and_restores_every_tool_when_widened() {
        let items = vec![
            Item::Control(Box::new(bevel_button("one", "One").width(40))),
            Item::Separator,
            Item::Control(Box::new(bevel_button("two", "Two").width(40))),
            Item::Control(Box::new(bevel_button("three", "Three").width(40))),
        ];
        assert_eq!(visible_count(&items, 300), 4);
        assert_eq!(visible_count(&items, 100), 2);
        assert_eq!(visible_count(&items, 30), 0);
        assert_eq!(visible_count(&items, 300), 4);
    }
}

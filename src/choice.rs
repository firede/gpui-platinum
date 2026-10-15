//! Radio buttons, tabs and segments as one keyboard focus stop.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, Element, ElementId, FocusHandle, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Orientation, Pixels, Role, SharedString, Style,
    Toggled, Window, div, prelude::*, px, rgb, size,
};

use crate::{
    Font, Language, Theme,
    paint::{Pen, drawing},
    theme::*,
    widgets::{ControlKind, draw_bevel_button, draw_control, draw_focus_circle},
};

/// A choice's label and availability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChoiceItem {
    pub label: SharedString,
    pub disabled: bool,
}

impl ChoiceItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl From<&str> for ChoiceItem {
    fn from(label: &str) -> Self {
        Self::new(label.to_owned())
    }
}

impl From<String> for ChoiceItem {
    fn from(label: String) -> Self {
        Self::new(label)
    }
}

impl From<SharedString> for ChoiceItem {
    fn from(label: SharedString) -> Self {
        Self::new(label)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Radio,
    Tabs,
    Segments,
}

impl Kind {
    fn group_role(self) -> Role {
        if self == Self::Tabs {
            Role::TabList
        } else {
            Role::RadioGroup
        }
    }

    fn item_role(self) -> Role {
        if self == Self::Tabs {
            Role::Tab
        } else {
            Role::RadioButton
        }
    }

    fn height(self) -> i32 {
        match self {
            Self::Radio => 16,
            Self::Tabs => 24,
            Self::Segments => 22,
        }
    }

    fn gap(self, vertical: bool) -> i32 {
        match self {
            Self::Radio if !vertical => 14,
            Self::Radio => 2,
            Self::Tabs => 0,
            Self::Segments => -1,
        }
    }
}

type ChangeHandler = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A controlled, single-choice group. Arrows select and focus an enabled item;
/// Tab enters at the selected item and then leaves the whole group.
pub struct ChoiceGroup {
    id: ElementId,
    kind: Kind,
    selected: usize,
    items: Rc<Vec<ChoiceItem>>,
    item_width: Option<i32>,
    vertical: bool,
    disabled: bool,
    label: SharedString,
    theme: Theme,
    language: Language,
    on_change: Option<ChangeHandler>,
}

impl ChoiceGroup {
    /// A column of radio buttons.
    pub fn radio(
        id: impl Into<ElementId>,
        selected: usize,
        items: impl IntoIterator<Item = impl Into<ChoiceItem>>,
    ) -> Self {
        Self::new(id.into(), Kind::Radio, selected, items)
    }

    /// A row of tabs. The caller renders the selected tab's page.
    pub fn tabs(
        id: impl Into<ElementId>,
        selected: usize,
        items: impl IntoIterator<Item = impl Into<ChoiceItem>>,
    ) -> Self {
        Self::new(id.into(), Kind::Tabs, selected, items)
    }

    /// Joined bevel buttons selecting one view or mode.
    pub fn segments(
        id: impl Into<ElementId>,
        selected: usize,
        items: impl IntoIterator<Item = impl Into<ChoiceItem>>,
    ) -> Self {
        Self::new(id.into(), Kind::Segments, selected, items)
    }

    fn new(
        id: ElementId,
        kind: Kind,
        selected: usize,
        items: impl IntoIterator<Item = impl Into<ChoiceItem>>,
    ) -> Self {
        Self {
            id,
            kind,
            selected,
            items: Rc::new(items.into_iter().map(Into::into).collect()),
            item_width: None,
            vertical: kind == Kind::Radio,
            disabled: false,
            label: SharedString::default(),
            theme: Theme::default(),
            language: Language::English,
            on_change: None,
        }
    }

    pub fn items(mut self, items: impl IntoIterator<Item = impl Into<ChoiceItem>>) -> Self {
        self.items = Rc::new(items.into_iter().map(Into::into).collect());
        self
    }

    /// Gives every item the same width.
    pub fn width(mut self, width: i32) -> Self {
        self.item_width = Some(width.max(24));
        self
    }

    pub fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
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

    pub fn on_change(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    fn widths(&self) -> Vec<i32> {
        self.items
            .iter()
            .map(|item| {
                self.item_width.unwrap_or_else(|| {
                    let padding = match self.kind {
                        Kind::Radio => 17,
                        Kind::Tabs => 24,
                        Kind::Segments => 20,
                    };
                    Font::Bold.measure_in(&item.label, self.language) + padding
                })
            })
            .collect()
    }

    fn dimensions(&self) -> (i32, i32) {
        if self.items.is_empty() {
            return (0, 0);
        }
        let widths = self.widths();
        let count = widths.len() as i32;
        if self.vertical {
            (
                *widths.iter().max().unwrap_or(&0),
                self.kind.height() * count + self.kind.gap(self.vertical) * (count - 1),
            )
        } else {
            (
                widths.iter().sum::<i32>() + self.kind.gap(self.vertical) * (count - 1),
                self.kind.height(),
            )
        }
    }
}

struct ChoiceState {
    selected: usize,
    focus: Vec<FocusHandle>,
}

impl IntoElement for ChoiceGroup {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for ChoiceGroup {
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
        let (width, height) = self.dimensions();
        let mut style = Style::default();
        style.size.width = px(width as f32).into();
        style.size.height = px(height as f32).into();
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
        let inactive = crate::window::content_is_inactive();
        let state = window.with_element_state(
            id.expect("choice group id"),
            |state: Option<Rc<RefCell<ChoiceState>>>, _| {
                let state = state.unwrap_or_else(|| {
                    Rc::new(RefCell::new(ChoiceState {
                        selected: self.selected,
                        focus: Vec::new(),
                    }))
                });
                {
                    let mut current = state.borrow_mut();
                    current.selected = self.selected;
                    current
                        .focus
                        .resize_with(self.items.len(), || cx.focus_handle());
                    let tab_stop = if self.disabled || inactive {
                        None
                    } else {
                        entry_index(&self.items, self.selected)
                    };
                    for (index, handle) in current.focus.iter_mut().enumerate() {
                        *handle = handle.clone().tab_stop(tab_stop == Some(index));
                    }
                }
                (state.clone(), state)
            },
        );
        let (width, height) = self.dimensions();
        let mut group = div()
            .id("group")
            .role(self.kind.group_role())
            .aria_label(self.label.clone())
            .aria_orientation(if self.vertical {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            })
            .w(px(width as f32))
            .h(px(height as f32))
            .flex()
            .flex_shrink_0()
            .text_color(rgb(BLACK))
            .when(self.vertical, |element| element.flex_col())
            .when(!self.vertical, |element| element.items_end());
        let widths = self.widths();
        for (index, item) in self.items.iter().enumerate() {
            let disabled = self.disabled || item.disabled;
            let selected = index == self.selected;
            let focus = state.borrow().focus[index].clone();
            let item_width = if self.vertical { width } else { widths[index] };
            let item_height = if self.kind == Kind::Tabs && !selected {
                22
            } else {
                self.kind.height()
            };
            let kind = self.kind;
            let label = item.label.clone();
            let theme = self.theme;
            let language = self.language;
            let paint_focus = focus.clone();
            let mut item_element = div()
                .id("input")
                .w(px(item_width as f32))
                .h(px(item_height as f32))
                .flex_shrink_0()
                .when(index > 0 && self.vertical, |element| {
                    element.mt(px(kind.gap(self.vertical) as f32))
                })
                .when(index > 0 && !self.vertical, |element| {
                    element.ml(px(kind.gap(self.vertical) as f32))
                });
            if !disabled && !inactive {
                let click_state = state.clone();
                let key_state = state.clone();
                let click_handler = self.on_change.clone();
                let key_handler = self.on_change.clone();
                let items = self.items.clone();
                item_element = item_element
                    .role(kind.item_role())
                    .aria_label(label.clone())
                    .track_focus(&focus)
                    .when(kind == Kind::Tabs, |element| {
                        element.aria_selected(selected)
                    })
                    .when(kind != Kind::Tabs, |element| {
                        element.aria_toggled(if selected {
                            Toggled::True
                        } else {
                            Toggled::False
                        })
                    })
                    .active(|style| style.text_color(rgb(WHITE)))
                    .on_click(move |_, window, cx| {
                        choose(&click_state, index, &click_handler, window, cx);
                    })
                    .on_key_down(move |event, window, cx| {
                        if event.keystroke.modifiers.platform
                            || event.keystroke.modifiers.control
                            || event.keystroke.modifiers.alt
                        {
                            return;
                        }
                        let next = key_target(&items, index, &event.keystroke.key);
                        if let Some(next) = next {
                            choose(&key_state, next, &key_handler, window, cx);
                            cx.stop_propagation();
                        }
                    });
            }
            item_element = item_element.child(drawing(
                item_width as f32,
                item_height as f32,
                move |pen, window, _| {
                    pen.language = language;
                    let inactive = crate::window::content_is_inactive();
                    let pressed =
                        !disabled && !inactive && window.text_style().color == rgb(WHITE).into();
                    match kind {
                        Kind::Radio => draw_control(
                            pen,
                            &label,
                            ControlKind::Radio(selected),
                            disabled,
                            pressed,
                            theme,
                        ),
                        Kind::Tabs => draw_tab(pen, &label, selected, disabled, pressed),
                        Kind::Segments => draw_segment(pen, &label, selected, disabled, pressed),
                    }
                    if !disabled
                        && !inactive
                        && paint_focus.is_focused(window)
                        && window.last_input_was_keyboard()
                    {
                        if kind == Kind::Radio {
                            let mut mark = Pen::new(12, 12);
                            draw_focus_circle(&mut mark, theme.accent.dark);
                            pen.append(mark, 0, (pen.height - 12) / 2);
                        } else {
                            pen.rect(5, pen.height - 5, pen.width - 10, 2, theme.accent.dark);
                        }
                    }
                },
            ));
            group = group.child(ChoiceCell {
                index,
                kind,
                disabled,
                selected,
                label: item.label.clone(),
                element: item_element.into_any_element(),
            });
        }
        let mut element = group.into_any_element();
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

// Disabled items have no focusable div. Their outer cell still reports the
// label, role and checked state, with AccessKit's disabled flag.
struct ChoiceCell {
    index: usize,
    kind: Kind,
    disabled: bool,
    selected: bool,
    label: SharedString,
    element: AnyElement,
}

impl IntoElement for ChoiceCell {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for ChoiceCell {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(self.index.into())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn a11y_role(&self) -> Option<Role> {
        self.disabled.then(|| self.kind.item_role())
    }
    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        node.set_label(self.label.to_string());
        node.set_disabled();
        if self.kind == Kind::Tabs {
            node.set_selected(self.selected);
        } else {
            node.set_toggled(if self.selected {
                Toggled::True
            } else {
                Toggled::False
            });
        }
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.element.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.paint(window, cx);
    }
}

fn choose(
    state: &Rc<RefCell<ChoiceState>>,
    index: usize,
    handler: &Option<ChangeHandler>,
    window: &mut Window,
    cx: &mut App,
) {
    let changed = {
        let mut state = state.borrow_mut();
        let Some(focus) = state.focus.get(index) else {
            return;
        };
        focus.focus(window, cx);
        let changed = state.selected != index;
        state.selected = index;
        changed
    };
    if changed && let Some(handler) = handler {
        handler(index, window, cx);
    }
    window.refresh();
}

fn entry_index(items: &[ChoiceItem], selected: usize) -> Option<usize> {
    if items.get(selected).is_some_and(|item| !item.disabled) {
        Some(selected)
    } else {
        items.iter().position(|item| !item.disabled)
    }
}

fn key_target(items: &[ChoiceItem], current: usize, key: &str) -> Option<usize> {
    if items.is_empty() {
        return None;
    }
    match key {
        "home" => items.iter().position(|item| !item.disabled),
        "end" => items.iter().rposition(|item| !item.disabled),
        "enter" | "space" => items
            .get(current)
            .filter(|item| !item.disabled)
            .map(|_| current),
        "left" | "up" | "right" | "down" => {
            let backwards = matches!(key, "left" | "up");
            let count = items.len();
            let start = current.min(count - 1);
            (1..=count)
                .map(|offset| {
                    if backwards {
                        (start + count - offset) % count
                    } else {
                        (start + offset) % count
                    }
                })
                .find(|&index| !items[index].disabled)
        }
        _ => None,
    }
}

fn draw_tab(pen: &mut Pen, label: &str, selected: bool, disabled: bool, pressed: bool) {
    let inactive = crate::window::content_is_inactive();
    let (width, height) = (pen.width, pen.height);
    let face = if pressed {
        GRAY_5
    } else if selected {
        GRAY_2
    } else {
        GRAY_4
    };
    let outline = if inactive { GRAY_8 } else { BLACK };
    for row in 0..height {
        let inset = match row {
            0 => 3,
            1 | 2 => 1,
            _ => 0,
        };
        pen.hline(inset, row, width - 2 * inset, outline);
        if row > 0 {
            pen.hline(inset + 1, row, width - 2 * inset - 2, face);
        }
    }
    pen.dot(2, 1, outline);
    pen.dot(width - 3, 1, outline);
    let light = if inactive { face } else { WHITE };
    let dark = if inactive { face } else { GRAY_7 };
    pen.hline(3, 1, width - 6, light);
    pen.dot(2, 2, light);
    pen.vline(1, 3, height - 3, light);
    pen.vline(width - 2, 3, height - 3, dark);
    pen.dot(width - 3, 2, dark);
    if !selected {
        pen.hline(0, height - 1, width, outline);
    }
    let raise = if selected { 2 } else { 0 };
    let baseline = raise + (height - 1 - raise - Font::Bold.cap_height()) / 2 + Font::Bold.ascent();
    let x = (width - Font::Bold.measure_in(label, pen.language)) / 2;
    pen.text(
        x,
        baseline,
        label,
        Font::Bold,
        if disabled || inactive { GRAY_8 } else { BLACK },
    );
}

fn draw_segment(pen: &mut Pen, label: &str, selected: bool, disabled: bool, pressed: bool) {
    let on = selected || pressed;
    draw_bevel_button(pen, disabled, pressed, selected);
    let x = (pen.width - Font::Bold.measure_in(label, pen.language)) / 2;
    let y = (pen.height - Font::Bold.cap_height()) / 2 + Font::Bold.ascent() + i32::from(on);
    pen.text(
        x,
        y,
        label,
        Font::Bold,
        if disabled || crate::window::content_is_inactive() {
            GRAY_8
        } else {
            BLACK
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<ChoiceItem> {
        vec![
            ChoiceItem::new("One"),
            ChoiceItem::new("Two").disabled(true),
            ChoiceItem::new("Three"),
        ]
    }

    #[test]
    fn arrows_skip_disabled_choices_and_wrap() {
        let items = items();
        assert_eq!(key_target(&items, 0, "right"), Some(2));
        assert_eq!(key_target(&items, 2, "down"), Some(0));
        assert_eq!(key_target(&items, 0, "left"), Some(2));
        assert_eq!(key_target(&items, 2, "up"), Some(0));
    }

    #[test]
    fn tab_entry_and_endpoints_find_enabled_choices() {
        let mut items = items();
        assert_eq!(entry_index(&items, 2), Some(2));
        assert_eq!(entry_index(&items, 1), Some(0));
        assert_eq!(entry_index(&items, 10), Some(0));
        items[0].disabled = true;
        assert_eq!(key_target(&items, 0, "home"), Some(2));
        assert_eq!(key_target(&items, 0, "end"), Some(2));
    }

    #[test]
    fn empty_and_disabled_groups_have_no_selection_target() {
        assert_eq!(entry_index(&[], 0), None);
        assert_eq!(key_target(&[], 0, "right"), None);
        let items = [ChoiceItem::new("Unavailable").disabled(true)];
        for key in ["left", "right", "home", "end", "space", "enter"] {
            assert_eq!(key_target(&items, 0, key), None);
        }
        assert_eq!(entry_index(&items, 0), None);
    }

    fn assert_original_crop(pen: &Pen, reference: &image::RgbaImage, x: u32, y: u32) {
        let mut actual = image::RgbaImage::from_pixel(
            pen.width as u32,
            pen.height as u32,
            image::Rgba([0xdd, 0xdd, 0xdd, 255]),
        );
        for fill in &pen.fills {
            let color = image::Rgba([
                (fill.color >> 16) as u8,
                (fill.color >> 8) as u8,
                fill.color as u8,
                255,
            ]);
            for py in fill.rect.y..fill.rect.y + fill.rect.height {
                for px in fill.rect.x..fill.rect.x + fill.rect.width {
                    if (0..pen.width).contains(&px) && (0..pen.height).contains(&py) {
                        actual.put_pixel(px as u32, py as u32, color);
                    }
                }
            }
        }
        let differences: Vec<_> = actual
            .enumerate_pixels()
            .filter(|(px, py, pixel)| reference.get_pixel(x + *px, y + *py) != *pixel)
            .map(|(px, py, _)| (px, py))
            .collect();
        assert!(
            differences.is_empty(),
            "{} pixels differ in crop ({x}, {y}); first: {:?}",
            differences.len(),
            differences.first()
        );
    }

    #[test]
    fn joined_segments_match_the_original_pixels() {
        let reference =
            image::load_from_memory(include_bytes!("../tests/reference/bevel-states.png"))
                .unwrap()
                .to_rgba8();
        let mut x = 74;
        for (index, label) in ["Left", "Center", "Right"].iter().enumerate() {
            let width = Font::Bold.measure(label) + 20;
            let mut pen = Pen::new(width, 22);
            draw_segment(&mut pen, label, index == 1, false, false);
            // The following segment replaces the shared final column.
            if index < 2 {
                pen.width -= 1;
            }
            assert_original_crop(&pen, &reference, x, 38);
            x += width as u32 - 1;
        }
    }

    #[test]
    fn raised_and_unselected_tabs_match_the_original_pixels() {
        let reference =
            image::load_from_memory(include_bytes!("../tests/reference/group-tabs.png"))
                .unwrap()
                .to_rgba8();
        let mut x = 166;
        for (index, label) in ["General", "Views", "Labels"].iter().enumerate() {
            let selected = index == 1;
            let width = Font::Bold.measure(label) + 24;
            let mut pen = Pen::new(width, if selected { 24 } else { 22 });
            draw_tab(&mut pen, label, selected, false, false);
            assert_original_crop(&pen, &reference, x, if selected { 8 } else { 10 });
            x += width as u32;
        }
    }
}

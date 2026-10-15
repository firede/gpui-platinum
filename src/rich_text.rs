//! Styled text, shared by the rich editor and its format bar.

use std::ops::Range;

use gpui::{
    App, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable, MouseButton, Role,
    Subscription, Toggled, Window, div, prelude::*, px, rgb,
};

use crate::{
    Font, Language, Menu, MenuEvent, MenuItem, PopupMenu, Theme,
    text_input::{InputEvent, TextField, text_buffer::TextBuffer},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleAttribute {
    Bold,
    Italic,
    Underline,
}

/// A bitmap style; scale 1, 2, 3 and 4 correspond to 11, 22, 33 and 44 points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub scale: u8,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            scale: 1,
        }
    }
}

impl TextStyle {
    pub fn scaled(mut self, scale: u8) -> Self {
        self.scale = scale.clamp(1, 4);
        self
    }

    pub fn font(self) -> Font {
        if self.bold {
            Font::SmallBold
        } else {
            Font::Small
        }
    }

    fn has(self, attribute: StyleAttribute) -> bool {
        match attribute {
            StyleAttribute::Bold => self.bold,
            StyleAttribute::Italic => self.italic,
            StyleAttribute::Underline => self.underline,
        }
    }

    fn set(&mut self, attribute: StyleAttribute, value: bool) {
        match attribute {
            StyleAttribute::Bold => self.bold = value,
            StyleAttribute::Italic => self.italic = value,
            StyleAttribute::Underline => self.underline = value,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub style: TextStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatState {
    pub style: TextStyle,
    /// Zero means the selection contains more than one size.
    pub scale: u8,
    pub alignment: Alignment,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RichFormat {
    pub styles: Vec<TextStyle>,
    pub alignments: Vec<Alignment>,
    pub typing: TextStyle,
}

impl RichFormat {
    pub fn new(text: &str) -> Self {
        Self {
            styles: vec![TextStyle::default(); text.chars().count()],
            alignments: vec![Alignment::Left; text.matches('\n').count() + 1],
            typing: TextStyle::default(),
        }
    }

    pub fn resize(&mut self, text: &str) {
        self.styles
            .resize(text.chars().count(), TextStyle::default());
        self.alignments
            .resize(text.matches('\n').count() + 1, Alignment::Left);
    }

    fn character_range(text: &str, range: Range<usize>) -> Range<usize> {
        let start = text[..range.start].chars().count();
        start..start + text[range].chars().count()
    }

    pub fn update_typing(&mut self, text: &str, caret: usize) {
        let index = text[..caret].chars().count().saturating_sub(1);
        self.typing = self.styles.get(index).copied().unwrap_or_default();
    }

    pub fn replace(&mut self, before: &str, range: Range<usize>, inserted: &str) {
        let characters = Self::character_range(before, range.clone());
        self.styles.splice(
            characters,
            std::iter::repeat_n(self.typing, inserted.chars().count()),
        );
        let paragraph = before[..range.start].matches('\n').count();
        let removed = before[range].matches('\n').count();
        let added = inserted.matches('\n').count();
        let alignment = self.alignments[paragraph];
        self.alignments.splice(
            paragraph + 1..paragraph + 1 + removed,
            std::iter::repeat_n(alignment, added),
        );
    }

    pub fn replacement_changes_style(&self, text: &str, range: Range<usize>) -> bool {
        self.styles[Self::character_range(text, range)]
            .iter()
            .any(|style| *style != self.typing)
    }

    pub fn state(&self, text: &str, selection: Range<usize>) -> FormatState {
        let alignment = self.alignments[text[..selection.start].matches('\n').count()];
        if selection.is_empty() {
            return FormatState {
                style: self.typing,
                scale: self.typing.scale,
                alignment,
            };
        }
        let range = Self::character_range(text, selection);
        let mut style = self.styles[range.start];
        let mut scale = style.scale;
        for next in &self.styles[range] {
            style.bold &= next.bold;
            style.italic &= next.italic;
            style.underline &= next.underline;
            if next.scale != scale {
                scale = 0;
            }
        }
        style.scale = scale.max(1);
        FormatState {
            style,
            scale,
            alignment,
        }
    }

    pub fn toggle(&mut self, text: &str, selection: Range<usize>, attribute: StyleAttribute) {
        let enabled = !self.state(text, selection.clone()).style.has(attribute);
        if selection.is_empty() {
            self.typing.set(attribute, enabled);
        } else {
            let range = Self::character_range(text, selection);
            for style in &mut self.styles[range] {
                style.set(attribute, enabled);
            }
        }
    }

    pub fn set_scale(&mut self, text: &str, selection: Range<usize>, scale: u8) {
        let scale = scale.clamp(1, 4);
        if selection.is_empty() {
            self.typing.scale = scale;
        } else {
            let range = Self::character_range(text, selection);
            for style in &mut self.styles[range] {
                style.scale = scale;
            }
        }
    }

    pub fn set_alignment(&mut self, text: &str, selection: Range<usize>, alignment: Alignment) {
        let start = text[..selection.start].matches('\n').count();
        let end = text[..selection.end].matches('\n').count();
        self.alignments[start..=end].fill(alignment);
    }

    pub fn runs(&self, text: &str) -> Vec<Run> {
        let mut runs: Vec<Run> = Vec::new();
        for (ch, style) in text.chars().zip(&self.styles) {
            if let Some(run) = runs.last_mut()
                && run.style == *style
            {
                run.text.push(ch);
            } else {
                runs.push(Run {
                    text: ch.to_string(),
                    style: *style,
                });
            }
        }
        runs
    }
}

/// A document of text, styles and paragraph alignments, with undo history.
pub struct RichText {
    buffer: TextBuffer,
}

impl RichText {
    pub fn new(text: impl Into<String>) -> Self {
        let mut buffer = TextBuffer::default();
        buffer.set(text.into());
        buffer.rich = Some(RichFormat::new(&buffer.text));
        Self { buffer }
    }

    pub fn value(&self) -> &str {
        &self.buffer.text
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        self.buffer.set(value.into());
    }
    pub fn selection(&self) -> Range<usize> {
        self.buffer.selection()
    }

    /// Select a UTF-8 byte range. A reversed range keeps the caret at its start.
    pub fn select(&mut self, anchor: usize, caret: usize) {
        self.buffer.select(anchor, false);
        self.buffer.select(caret, true);
    }

    pub fn insert(&mut self, text: &str) {
        self.buffer.replace(self.buffer.selection(), text);
    }

    pub fn runs(&self) -> Vec<Run> {
        self.buffer.rich.as_ref().unwrap().runs(&self.buffer.text)
    }
    pub fn alignments(&self) -> &[Alignment] {
        &self.buffer.rich.as_ref().unwrap().alignments
    }
    pub fn format_state(&self) -> FormatState {
        self.buffer.format_state().unwrap()
    }
    pub fn toggle(&mut self, attribute: StyleAttribute) {
        self.buffer.toggle_style(attribute);
    }
    pub fn set_scale(&mut self, scale: u8) {
        self.buffer.set_scale(scale);
    }
    pub fn set_alignment(&mut self, alignment: Alignment) {
        self.buffer.set_alignment(alignment);
    }
    pub fn undo(&mut self) -> bool {
        self.buffer.undo()
    }
    pub fn redo(&mut self) -> bool {
        self.buffer.redo()
    }

    pub fn set_runs(&mut self, runs: &[Run]) {
        self.buffer.set_runs(runs);
    }
}

/// Creates the Size menu for a document's menu bar.
pub fn size_menu(state: FormatState, language: Language) -> Menu {
    Menu::new(
        crate::tr(language, "size"),
        size_items(state.scale, language),
    )
}

fn size_items(scale: u8, language: Language) -> Vec<MenuItem> {
    (1..=4)
        .map(|item_scale| {
            MenuItem::new(
                crate::tr(language, "points").replace("{size}", &(11 * item_scale).to_string()),
            )
            .checked(scale == item_scale)
        })
        .collect()
}

/// A Size pop-up menu that applies its choice to a rich text editor.
pub struct SizeMenu {
    editor: Entity<TextField>,
    menu: Entity<PopupMenu>,
    scale: Option<u8>,
    theme: Theme,
    language: Language,
    configure: bool,
    focus_editor: bool,
    _subscriptions: Vec<Subscription>,
}

impl SizeMenu {
    pub fn new(editor: Entity<TextField>, cx: &mut Context<Self>) -> Self {
        let menu = cx.new(|cx| {
            PopupMenu::new(size_items(1, Language::English), cx)
                .label(crate::tr(Language::English, "size"))
                .width(94)
        });
        let observer = cx.observe(&editor, |_, _, cx| cx.notify());
        let events = cx.subscribe(&menu, |this, _, event: &MenuEvent, cx| {
            this.editor
                .update(cx, |editor, cx| editor.set_scale(event.0 as u8 + 1, cx));
            this.focus_editor = true;
            cx.notify();
        });
        Self {
            editor,
            menu,
            scale: None,
            theme: Theme::default(),
            language: Language::English,
            configure: true,
            focus_editor: false,
            _subscriptions: vec![observer, events],
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

    pub fn is_open(&self, cx: &App) -> bool {
        self.menu.read(cx).is_open()
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        self.menu.update(cx, |menu, cx| menu.set_theme(theme, cx));
        cx.notify();
    }

    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        self.configure = true;
        cx.notify();
    }
}

impl Focusable for SizeMenu {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.menu.read(cx).focus_handle(cx)
    }
}

impl Render for SizeMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let scale = self
            .editor
            .read(cx)
            .format_state()
            .map_or(1, |state| state.scale);
        if self.configure || self.scale != Some(scale) {
            self.menu.update(cx, |menu, cx| {
                if self.configure {
                    menu.set_items(size_items(scale, self.language), cx);
                    menu.set_label(crate::tr(self.language, "size"), cx);
                    menu.set_theme(self.theme, cx);
                    menu.set_language(self.language, cx);
                }
                if scale == 0 {
                    menu.clear_selection(cx);
                } else {
                    menu.set_selection(scale as usize - 1, cx);
                }
            });
            self.scale = Some(scale);
            self.configure = false;
        }
        if self.focus_editor {
            self.focus_editor = false;
            window.focus(&self.editor.read(cx).focus_handle(cx), cx);
        }
        self.menu.clone()
    }
}

/// A format bar and a native rich text editor.
pub struct RichTextArea {
    editor: Entity<TextField>,
    size_menu: Entity<SizeMenu>,
    value: String,
    initial_text: Option<String>,
    initial_runs: Option<Vec<Run>>,
    width: f32,
    height: f32,
    theme: Theme,
    language: Language,
    configure: bool,
    _subscriptions: Vec<Subscription>,
}

impl RichTextArea {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| TextField::new(cx).multiline(true).rich().label("Document"));
        let size_menu = cx.new(|cx| SizeMenu::new(editor.clone(), cx));
        let observer = cx.observe(&editor, |this, editor, cx| {
            this.value = editor.read(cx).value().to_owned();
            cx.notify();
        });
        let events = cx.subscribe(&editor, |this, editor, event: &InputEvent, cx| {
            this.value = editor.read(cx).value().to_owned();
            cx.emit(*event);
        });
        Self {
            editor,
            size_menu,
            value: String::new(),
            initial_text: None,
            initial_runs: None,
            width: 360.,
            height: 210.,
            theme: Theme::default(),
            language: Language::English,
            configure: true,
            _subscriptions: vec![observer, events],
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.value = text.into();
        self.initial_text = Some(self.value.clone());
        self.initial_runs = None;
        self
    }

    pub fn with_runs(mut self, runs: Vec<Run>) -> Self {
        self.value = runs.iter().map(|run| run.text.as_str()).collect();
        self.initial_runs = Some(runs);
        self.initial_text = None;
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width.max(288.);
        self.height = height.max(80.);
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

    pub fn editor(&self) -> Entity<TextField> {
        self.editor.clone()
    }
    pub fn value(&self) -> &str {
        &self.value
    }
    pub fn runs(&self, cx: &App) -> Vec<Run> {
        self.editor.read(cx).runs()
    }
    pub fn format_state(&self, cx: &App) -> FormatState {
        self.editor.read(cx).format_state().unwrap()
    }

    pub fn set_value(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        self.initial_text = None;
        self.initial_runs = None;
        self.editor
            .update(cx, |editor, cx| editor.set_value(value.into(), cx));
        self.value = self.editor.read(cx).value().to_owned();
        cx.notify();
    }

    pub fn set_runs(&mut self, runs: &[Run], cx: &mut Context<Self>) {
        self.initial_text = None;
        self.initial_runs = None;
        self.editor
            .update(cx, |editor, cx| editor.set_runs(runs, cx));
        self.value = self.editor.read(cx).value().to_owned();
        cx.notify();
    }

    pub fn toggle(&mut self, attribute: StyleAttribute, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| editor.toggle_style(attribute, cx));
    }

    pub fn set_scale(&mut self, scale: u8, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| editor.set_scale(scale, cx));
    }

    pub fn set_alignment(&mut self, alignment: Alignment, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| editor.set_alignment(alignment, cx));
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        self.editor
            .update(cx, |editor, cx| editor.set_theme(theme, cx));
        self.size_menu
            .update(cx, |menu, cx| menu.set_theme(theme, cx));
        cx.notify();
    }

    pub fn set_size(&mut self, width: f32, height: f32, cx: &mut Context<Self>) {
        self.width = width.max(288.);
        self.height = height.max(80.);
        self.configure = true;
        cx.notify();
    }

    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        self.editor.update(cx, |editor, cx| {
            editor.set_language(language, cx);
            editor.set_label(crate::locale::desk(language, "document"), cx);
        });
        self.size_menu
            .update(cx, |menu, cx| menu.set_language(language, cx));
        cx.notify();
    }

    fn focus_editor(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.editor.read(cx).focus_handle(cx), cx);
    }
}

impl EventEmitter<InputEvent> for RichTextArea {}

impl Focusable for RichTextArea {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.editor.read(cx).focus_handle(cx)
    }
}

impl Render for RichTextArea {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(text) = self.initial_text.take() {
            self.editor
                .update(cx, |editor, cx| editor.set_value(text, cx));
        }
        if let Some(runs) = self.initial_runs.take() {
            self.editor
                .update(cx, |editor, cx| editor.set_runs(&runs, cx));
        }
        if self.configure {
            self.configure = false;
            self.editor.update(cx, |editor, cx| {
                editor.set_size(self.width, self.height - 30., cx);
                editor.set_theme(self.theme, cx);
                editor.set_language(self.language, cx);
                editor.set_label(crate::locale::desk(self.language, "document"), cx);
            });
            self.size_menu.update(cx, |menu, cx| {
                menu.set_theme(self.theme, cx);
                menu.set_language(self.language, cx);
            });
        }
        let state = self.format_state(cx);
        let mut bar = div()
            .flex()
            .items_center()
            .gap(px(2.))
            .h(px(28.))
            .flex_shrink_0();
        for (attribute, label, enabled) in [
            (StyleAttribute::Bold, "bold", state.style.bold),
            (StyleAttribute::Italic, "italic", state.style.italic),
            (
                StyleAttribute::Underline,
                "underline",
                state.style.underline,
            ),
        ] {
            bar = bar.child(format_button(
                label,
                FormatIcon::Style(attribute),
                enabled,
                self.theme,
                self.language,
                cx.listener(move |this, _, window, cx| {
                    this.toggle(attribute, cx);
                    this.focus_editor(window, cx);
                }),
            ));
        }
        bar = bar.child(
            div()
                .w(px(4.))
                .h(px(20.))
                .border_l_1()
                .border_color(rgb(crate::theme::GRAY_7)),
        );
        for (alignment, label) in [
            (Alignment::Left, "alignLeft"),
            (Alignment::Center, "alignCenter"),
            (Alignment::Right, "alignRight"),
        ] {
            bar = bar.child(format_button(
                label,
                FormatIcon::Align(alignment),
                state.alignment == alignment,
                self.theme,
                self.language,
                cx.listener(move |this, _, window, cx| {
                    this.set_alignment(alignment, cx);
                    this.focus_editor(window, cx);
                }),
            ));
        }
        bar = bar.child(
            div()
                .w(px(4.))
                .h(px(20.))
                .border_l_1()
                .border_color(rgb(crate::theme::GRAY_7)),
        );
        bar = bar.child(self.size_menu.clone());
        div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .w(px(self.width))
            .h(px(self.height))
            .child(bar)
            .child(self.editor.clone())
    }
}

#[derive(Clone, Copy)]
enum FormatIcon {
    Style(StyleAttribute),
    Align(Alignment),
}

fn format_button(
    label: &'static str,
    icon: FormatIcon,
    selected: bool,
    theme: Theme,
    language: Language,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(label)
        .w(px(24.))
        .h(px(24.))
        .flex_shrink_0()
        .role(Role::Button)
        .aria_label(crate::tr(language, label))
        .focusable()
        .tab_stop(true)
        .aria_toggled(if selected {
            Toggled::True
        } else {
            Toggled::False
        })
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .on_click(on_click)
        .child(crate::paint::drawing(24., 24., move |pen, _, _| {
            use crate::theme::{BLACK, GRAY_2, GRAY_7, WHITE};
            pen.rect(
                0,
                0,
                24,
                24,
                if selected { theme.accent.light } else { GRAY_2 },
            );
            pen.frame(0, 0, 24, 24, BLACK);
            pen.bevel(
                1,
                1,
                22,
                22,
                if selected { GRAY_7 } else { WHITE },
                if selected { WHITE } else { GRAY_7 },
            );
            match icon {
                FormatIcon::Style(attribute) => {
                    let rows = match attribute {
                        StyleAttribute::Bold => &ICON_BOLD,
                        StyleAttribute::Italic => &ICON_ITALIC,
                        StyleAttribute::Underline => &ICON_UNDERLINE,
                    };
                    for (y, row) in rows.iter().enumerate() {
                        for (x, pixel) in row.bytes().enumerate() {
                            if pixel == b'#' {
                                pen.dot(x as i32 + 4, y as i32 + 4, BLACK);
                            }
                        }
                    }
                }
                FormatIcon::Align(alignment) => {
                    for row in 0..6 {
                        let width = if row % 2 == 0 { 12 } else { 8 };
                        let x = match alignment {
                            Alignment::Left => 6,
                            Alignment::Center => 6 + (12 - width) / 2,
                            Alignment::Right => 6 + 12 - width,
                        };
                        pen.hline(x, 6 + row * 2, width, BLACK);
                    }
                }
            }
        }))
}

const ICON_BOLD: [&str; 16] = [
    "................",
    "................",
    "..#########.....",
    "...###...###....",
    "...###....###...",
    "...###....###...",
    "...###...###....",
    "...########.....",
    "...###....###...",
    "...###.....###..",
    "...###.....###..",
    "...###....###...",
    "..##########....",
    "................",
    "................",
    "................",
];
const ICON_ITALIC: [&str; 16] = [
    "................",
    "................",
    "......#######...",
    ".........##.....",
    ".........##.....",
    "........##......",
    "........##......",
    ".......##.......",
    ".......##.......",
    "......##........",
    "......##........",
    ".....##.........",
    "...#######......",
    "................",
    "................",
    "................",
];
const ICON_UNDERLINE: [&str; 16] = [
    "................",
    "................",
    "..####....####..",
    "...##......##...",
    "...##......##...",
    "...##......##...",
    "...##......##...",
    "...##......##...",
    "...##......##...",
    "....##....##....",
    ".....######.....",
    "................",
    "..############..",
    "................",
    "................",
    "................",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styling_a_unicode_selection_splits_and_merges_runs() {
        let mut document = RichText::new("ab中文cd");
        document.select(2, 8);
        document.toggle(StyleAttribute::Bold);
        assert_eq!(
            document
                .runs()
                .iter()
                .map(|run| (run.text.as_str(), run.style.bold))
                .collect::<Vec<_>>(),
            vec![("ab", false), ("中文", true), ("cd", false)]
        );
        document.select(0, document.value().len());
        assert!(!document.format_state().style.bold);
        document.toggle(StyleAttribute::Bold);
        assert_eq!(document.runs().len(), 1);
        assert!(document.runs()[0].style.bold);
        document.toggle(StyleAttribute::Bold);
        assert_eq!(document.runs().len(), 1);
        assert!(!document.runs()[0].style.bold);
    }

    #[test]
    fn inserted_text_uses_the_typing_style_and_preserves_following_runs() {
        let mut document = RichText::new("abc");
        document.select(1, 2);
        document.toggle(StyleAttribute::Bold);
        document.select(1, 1);
        document.toggle(StyleAttribute::Italic);
        document.insert("é");
        let runs = document.runs();
        assert_eq!(document.value(), "aébc");
        assert_eq!(runs[1].text, "é");
        assert!(runs[1].style.italic);
        assert!(!runs[1].style.bold);
        assert_eq!(runs[2].text, "b");
        assert!(runs[2].style.bold);
        assert!(document.undo());
        assert_eq!(document.value(), "abc");
        assert_eq!(document.runs()[1].text, "b");
    }

    #[test]
    fn paragraph_edits_inherit_alignment_and_join_without_shifting_later_paragraphs() {
        let mut document = RichText::new("first\nsecond\nthird");
        document.select(0, 0);
        document.set_alignment(Alignment::Center);
        document.select(6, 6);
        document.set_alignment(Alignment::Right);
        document.select(5, 6);
        document.insert("");
        assert_eq!(document.value(), "firstsecond\nthird");
        assert_eq!(document.alignments(), [Alignment::Center, Alignment::Left]);
        document.undo();
        assert_eq!(
            document.alignments(),
            [Alignment::Center, Alignment::Right, Alignment::Left]
        );
        document.select(8, 8);
        document.insert("\n");
        assert_eq!(
            document.alignments(),
            [
                Alignment::Center,
                Alignment::Right,
                Alignment::Right,
                Alignment::Left
            ]
        );
    }

    #[test]
    fn text_and_formatting_share_one_undo_history() {
        let mut document = RichText::new("text");
        document.select(0, 4);
        document.set_scale(3);
        document.toggle(StyleAttribute::Underline);
        document.insert("large");
        assert_eq!(document.runs()[0].style.scale, 3);
        assert!(document.runs()[0].style.underline);
        assert!(document.undo());
        assert_eq!(document.value(), "text");
        assert!(document.format_state().style.underline);
        assert!(document.undo());
        assert!(!document.format_state().style.underline);
        assert_eq!(document.format_state().scale, 3);
        assert!(document.undo());
        assert_eq!(document.format_state().scale, 1);
        assert!(document.redo());
        assert_eq!(document.format_state().scale, 3);
    }

    #[test]
    fn composition_keeps_styled_neighbors_and_undo_restores_the_document() {
        let mut document = RichText::new("a中b");
        document.select(1, 4);
        document.toggle(StyleAttribute::Bold);
        let before = document.runs();
        document.buffer.compose(1..4, "か", Some(1..1));
        document.buffer.compose(1..4, "漢字", Some(2..2));
        document.buffer.replace(1..7, "漢字");
        assert_eq!(document.value(), "a漢字b");
        assert!(document.runs()[1].style.bold);
        assert_eq!(document.runs()[2].text, "b");
        document.undo();
        assert_eq!(document.runs(), before);
        assert_eq!(document.selection(), 1..4);
    }

    #[test]
    fn mixed_sizes_report_zero_and_run_import_preserves_styles() {
        let mut document = RichText::new("");
        let runs = vec![
            Run {
                text: "small".into(),
                style: TextStyle::default(),
            },
            Run {
                text: "大".into(),
                style: TextStyle {
                    bold: true,
                    ..TextStyle::default().scaled(4)
                },
            },
        ];
        document.set_runs(&runs);
        assert_eq!(document.runs(), runs);
        document.select(0, document.value().len());
        assert_eq!(document.format_state().scale, 0);
        document.set_scale(2);
        assert_eq!(document.format_state().scale, 2);
    }
}

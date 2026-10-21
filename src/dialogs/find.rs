use std::ops::Range;

use super::modal::{ModalState, ModalView, modal, text};
use crate::{CheckState, Font, Language, TextField, Theme, button, checkbox, tr};
use gpui::{
    ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable, Render, SharedString,
    Subscription, Window, div, prelude::*, px,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindOptions {
    pub match_case: bool,
    pub whole_word: bool,
}

/// UTF-8 ranges of the non-overlapping matches, suitable for TextField::select.
pub fn find_ranges(text: &str, query: &str, options: FindOptions) -> Vec<Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    let (haystack, boundaries) = folded(text, options.match_case);
    let needle = if options.match_case {
        query.to_owned()
    } else {
        folded(query, false).0
    };
    let mut matches = Vec::new();
    let mut end = 0;
    for (start, _) in haystack.match_indices(&needle) {
        let Some(original_start) = boundaries[start] else {
            continue;
        };
        let Some(original_end) = boundaries[start + needle.len()] else {
            continue;
        };
        if original_start < end {
            continue;
        }
        if options.whole_word
            && (text[..original_start]
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric)
                || text[original_end..]
                    .chars()
                    .next()
                    .is_some_and(char::is_alphanumeric))
        {
            continue;
        }
        matches.push(original_start..original_end);
        end = original_end;
    }
    matches
}

fn folded(text: &str, match_case: bool) -> (String, Vec<Option<usize>>) {
    let mut folded = String::new();
    let mut boundaries = vec![Some(0)];
    for (offset, character) in text.char_indices() {
        if match_case {
            folded.push(character);
        } else {
            folded.extend(character.to_lowercase());
        }
        boundaries.resize(folded.len() + 1, None);
        boundaries[folded.len()] = Some(offset + character.len_utf8());
    }
    (folded, boundaries)
}

pub fn find_next(
    text: &str,
    query: &str,
    from: usize,
    options: FindOptions,
) -> Option<Range<usize>> {
    let matches = find_ranges(text, query, options);
    matches
        .iter()
        .find(|range| range.start >= from.min(text.len()))
        .or(matches.first())
        .cloned()
}

pub fn replace_all(
    text: &str,
    query: &str,
    replacement: &str,
    options: FindOptions,
) -> (String, usize) {
    let matches = find_ranges(text, query, options);
    let mut result = text.to_owned();
    for range in matches.iter().rev() {
        result.replace_range(range.clone(), replacement);
    }
    (result, matches.len())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindEvent {
    Found(Range<usize>),
    Replaced(usize),
    NotFound,
}

/// A find dialog connected to a plain or rich editor's native text field.
pub struct FindReplace {
    modal: ModalState,
    target: Entity<TextField>,
    query: Entity<TextField>,
    replacement: Entity<TextField>,
    options: FindOptions,
    note: SharedString,
    dirty: bool,
    focus: [FocusHandle; 6],
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<FindEvent> for FindReplace {}
impl FindReplace {
    pub fn new(target: Entity<TextField>, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| TextField::new(cx).label("Find").size(240., 22.));
        let replacement = cx.new(|cx| TextField::new(cx).label("Replace").size(240., 22.));
        let subscriptions = vec![
            cx.observe(&query, |_, _, cx| cx.notify()),
            cx.observe(&replacement, |_, _, cx| cx.notify()),
        ];
        Self {
            modal: ModalState::new(380., cx),
            target,
            query,
            replacement,
            options: FindOptions::default(),
            note: "".into(),
            dirty: true,
            focus: std::array::from_fn(|_| cx.focus_handle()),
            _subscriptions: subscriptions,
        }
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.modal.theme = theme;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.modal.language = language;
        self
    }
    pub fn is_open(&self) -> bool {
        self.modal.open
    }
    pub fn options(&self) -> FindOptions {
        self.options
    }
    pub fn set_target(&mut self, target: Entity<TextField>, cx: &mut Context<Self>) {
        self.target = target;
        cx.notify();
    }
    pub fn set_options(&mut self, options: FindOptions, cx: &mut Context<Self>) {
        self.options = options;
        cx.notify();
    }
    pub fn set_query(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        self.query
            .update(cx, |field, cx| field.set_value(value, cx));
    }
    pub fn set_replacement(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        self.replacement
            .update(cx, |field, cx| field.set_value(value, cx));
    }
    pub fn set_note(&mut self, note: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.note = note.into();
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.modal.theme = theme;
        self.dirty = true;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.modal.language = language;
        self.dirty = true;
        cx.notify();
    }
    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.query.focus_handle(cx);
        self.modal.stops = vec![focus.clone(), self.replacement.focus_handle(cx)];
        self.modal.stops.extend(self.focus.iter().cloned());
        // The caret at the start of the query, as the original leaves it.
        self.query.update(cx, |field, cx| field.select(0, 0, cx));
        self.modal.show(Some(&focus), window, cx);
        cx.notify();
    }
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal.close(window, cx);
        cx.notify();
    }
    pub fn find(&mut self, cx: &mut Context<Self>) {
        let editor = self.target.read(cx);
        let found = find_next(
            editor.value(),
            self.query.read(cx).value(),
            editor.selection().end,
            self.options,
        );
        if let Some(range) = found {
            self.target
                .update(cx, |editor, cx| editor.select(range.start, range.end, cx));
            cx.emit(FindEvent::Found(range));
        } else {
            cx.emit(FindEvent::NotFound);
        }
    }
    pub fn replace(&mut self, cx: &mut Context<Self>) {
        let query = self.query.read(cx).value().to_owned();
        let replacement = self.replacement.read(cx).value().to_owned();
        let editor = self.target.read(cx);
        let selected = editor.selection();
        let selected_matches =
            find_ranges(editor.value(), &query, self.options).contains(&selected);
        if selected_matches {
            let count = self.target.update(cx, |editor, cx| {
                editor.replace_ranges(&[selected], &replacement, cx)
            });
            cx.emit(FindEvent::Replaced(count));
        }
        self.find(cx);
    }
    pub fn replace_every(&mut self, cx: &mut Context<Self>) {
        let ranges = find_ranges(
            self.target.read(cx).value(),
            self.query.read(cx).value(),
            self.options,
        );
        let replacement = self.replacement.read(cx).value().to_owned();
        let count = self.target.update(cx, |editor, cx| {
            editor.replace_ranges(&ranges, &replacement, cx)
        });
        cx.emit(FindEvent::Replaced(count));
    }
    /// A click on a button leaves the keyboard in the Find field, as the
    /// original's buttons take no focus from the mouse.
    fn keep_typing(&self, event: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !event.is_keyboard() {
            window.focus(&self.query.focus_handle(cx), cx);
        }
    }
    fn accept(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.find(cx);
    }
}
impl Render for FindReplace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.modal.open {
            return div().into_any_element();
        }
        let lang = self.modal.language;
        let theme = self.modal.theme;
        if self.dirty {
            self.dirty = false;
            for (field, key) in [(&self.query, "find"), (&self.replacement, "replace")] {
                field.update(cx, |field, cx| {
                    field.set_theme(theme, cx);
                    field.set_language(lang, cx);
                    field.set_label(tr(lang, key), cx);
                });
            }
        }
        let empty = self.query.read(cx).value().is_empty();
        self.modal.stops = vec![
            self.query.focus_handle(cx),
            self.replacement.focus_handle(cx),
        ];
        self.modal
            .stops
            .extend(self.focus[..if empty { 3 } else { 6 }].iter().cloned());
        let mut content = div().flex().flex_col().gap(px(8.));
        for (key, field) in [
            ("findLabel", &self.query),
            ("replaceLabel", &self.replacement),
        ] {
            content = content.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .w(px(62.))
                            .child(text(tr(lang, key), Font::Bold, lang)),
                    )
                    .child(field.clone()),
            );
        }
        content = content.child(
            div()
                .flex()
                .gap(px(14.))
                .pl(px(68.))
                .child(
                    checkbox(
                        "match-case",
                        tr(lang, "matchCase"),
                        if self.options.match_case {
                            CheckState::On
                        } else {
                            CheckState::Off
                        },
                    )
                    .focus_handle(&self.focus[0])
                    .theme(theme)
                    .language(lang)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.options.match_case = !this.options.match_case;
                        cx.notify();
                    })),
                )
                .child(
                    checkbox(
                        "whole-word",
                        tr(lang, "wholeWord"),
                        if self.options.whole_word {
                            CheckState::On
                        } else {
                            CheckState::Off
                        },
                    )
                    .focus_handle(&self.focus[1])
                    .theme(theme)
                    .language(lang)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.options.whole_word = !this.options.whole_word;
                        cx.notify();
                    })),
                ),
        );
        if !self.note.is_empty() {
            content = content.child(
                div()
                    .id("find-note")
                    .role(gpui::Role::Status)
                    .pl(px(68.))
                    .child(crate::text_in_language(
                        self.note.clone(),
                        Font::Small,
                        crate::theme::GRAY_9,
                        lang,
                    )),
            );
        }
        content = content.child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(
                    button("cancel", tr(lang, "cancel"))
                        .focus_handle(&self.focus[2])
                        .theme(theme)
                        .language(lang)
                        .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
                )
                .child(div().flex_1())
                .child(
                    button("replace-all", tr(lang, "replaceAll"))
                        .focus_handle(&self.focus[3])
                        .theme(theme)
                        .language(lang)
                        .disabled(empty)
                        .on_click(cx.listener(|this, event, window, cx| {
                            this.keep_typing(event, window, cx);
                            this.replace_every(cx)
                        })),
                )
                .child(
                    button("replace", tr(lang, "replace"))
                        .focus_handle(&self.focus[4])
                        .theme(theme)
                        .language(lang)
                        .disabled(empty)
                        .on_click(cx.listener(|this, event, window, cx| {
                            this.keep_typing(event, window, cx);
                            this.replace(cx)
                        })),
                )
                .child(
                    button("find", tr(lang, "find"))
                        .focus_handle(&self.focus[5])
                        .default_button()
                        .theme(theme)
                        .language(lang)
                        .disabled(empty)
                        .on_click(cx.listener(|this, event, window, cx| {
                            this.keep_typing(event, window, cx);
                            this.find(cx)
                        })),
                ),
        );
        modal(
            &mut self.modal,
            ModalView::new(tr(lang, "findAndReplace"), true, content),
            window,
            cx,
            |this| &mut this.modal,
            Self::accept,
            Self::close,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{FindOptions, find_next, find_ranges, replace_all};
    #[test]
    fn unicode_matches_keep_original_byte_boundaries_and_wrap() {
        assert_eq!(
            find_next("猫 École 猫", "éCOLE", 0, FindOptions::default()),
            Some(4..10)
        );
        assert_eq!(
            find_next("猫 École 猫", "猫", 13, FindOptions::default()),
            Some(0..3)
        );
        assert_eq!(find_ranges("İX", "x", FindOptions::default()), vec![2..3]);
        assert!(find_ranges("İX", "i", FindOptions::default()).is_empty());
    }
    #[test]
    fn whole_words_ignore_partial_words_and_replacement_does_not_revisit_insertions() {
        let options = FindOptions {
            whole_word: true,
            ..FindOptions::default()
        };
        assert_eq!(
            find_ranges("cat cats cat_cat", "cat", options),
            vec![0..3, 9..12, 13..16]
        );
        assert_eq!(
            replace_all("猫猫猫", "猫", "猫猫", FindOptions::default()),
            ("猫猫猫猫猫猫".into(), 3)
        );
        assert_eq!(replace_all("aaa", "", "a", options), ("aaa".into(), 0));
    }
}

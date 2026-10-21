use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::rich_text::{Alignment, FormatState, RichFormat, Run, StyleAttribute};

#[derive(Clone, Debug, Default, PartialEq)]
struct Snapshot {
    text: String,
    anchor: usize,
    caret: usize,
    rich: Option<RichFormat>,
}

/// UTF-8 storage with grapheme movement and a UTF-16 boundary for input methods.
#[derive(Default)]
pub(crate) struct TextBuffer {
    pub text: String,
    pub anchor: usize,
    pub caret: usize,
    pub marked: Option<Range<usize>>,
    pub rich: Option<RichFormat>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

impl TextBuffer {
    pub fn set(&mut self, text: String) {
        self.text = text;
        self.anchor = self.text.len();
        self.caret = self.anchor;
        self.marked = None;
        self.undo.clear();
        self.redo.clear();
        if let Some(rich) = &mut self.rich {
            rich.resize(&self.text);
            rich.update_typing(&self.text, self.caret);
        }
    }

    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }

    pub fn select(&mut self, offset: usize, extend: bool) {
        self.caret = self.floor_boundary(offset);
        if !extend {
            self.anchor = self.caret;
        }
        if let Some(rich) = &mut self.rich {
            rich.update_typing(&self.text, self.caret);
        }
    }

    pub fn previous(&self, offset: usize) -> usize {
        self.text
            .grapheme_indices(true)
            .rev()
            .find_map(|(index, _)| (index < offset).then_some(index))
            .unwrap_or(0)
    }

    pub fn next(&self, offset: usize) -> usize {
        self.text
            .grapheme_indices(true)
            .find_map(|(index, _)| (index > offset).then_some(index))
            .unwrap_or(self.text.len())
    }

    pub fn previous_word(&self) -> usize {
        self.text
            .unicode_word_indices()
            .rev()
            .find_map(|(index, _)| (index < self.caret).then_some(index))
            .unwrap_or(0)
    }

    pub fn next_word(&self) -> usize {
        self.text
            .unicode_word_indices()
            .map(|(index, word)| index + word.len())
            .find(|end| *end > self.caret)
            .unwrap_or(self.text.len())
    }

    pub fn word_at(&self, offset: usize) -> Range<usize> {
        self.text
            .split_word_bound_indices()
            .find_map(|(start, word)| {
                let end = start + word.len();
                (start <= offset && offset < end).then_some(start..end)
            })
            .unwrap_or(self.text.len()..self.text.len())
    }

    pub fn replace(&mut self, range: Range<usize>, text: &str) -> bool {
        self.replace_impl(range, text, true)
    }

    /// Replace non-overlapping matches as one undoable edit.
    pub fn replace_many(&mut self, ranges: &[Range<usize>], text: &str) -> usize {
        let mut ranges: Vec<_> = ranges
            .iter()
            .cloned()
            .map(|range| self.safe_range(range))
            .collect();
        ranges.sort_by_key(|range| range.start);
        let mut previous_end = 0;
        ranges.retain(|range| {
            if range.is_empty() || range.start < previous_end {
                return false;
            }
            previous_end = range.end;
            true
        });
        if ranges.is_empty() {
            return 0;
        }
        self.checkpoint();
        self.marked = None;
        for range in ranges.iter().rev() {
            self.replace_impl(range.clone(), text, false);
        }
        ranges.len()
    }

    fn replace_impl(&mut self, range: Range<usize>, text: &str, record: bool) -> bool {
        let range = self.safe_range(range);
        let changed = &self.text[range.clone()] != text
            || self
                .rich
                .as_ref()
                .is_some_and(|rich| rich.replacement_changes_style(&self.text, range.clone()));
        if changed && record && self.marked.is_none() {
            self.checkpoint();
        }
        if let Some(rich) = &mut self.rich {
            rich.replace(&self.text, range.clone(), text);
        }
        self.text.replace_range(range.clone(), text);
        self.caret = range.start + text.len();
        self.anchor = self.caret;
        self.marked = None;
        changed
    }

    pub fn compose(
        &mut self,
        range: Range<usize>,
        text: &str,
        selection_utf16: Option<Range<usize>>,
    ) -> bool {
        let range = self.safe_range(range);
        if self.marked.is_none() {
            self.checkpoint();
        }
        let changed = &self.text[range.clone()] != text;
        if let Some(rich) = &mut self.rich {
            rich.replace(&self.text, range.clone(), text);
        }
        self.text.replace_range(range.clone(), text);
        self.marked = (!text.is_empty()).then_some(range.start..range.start + text.len());
        let selection = selection_utf16
            .map(|range| utf16_range_to_bytes(text, range))
            .unwrap_or(text.len()..text.len());
        self.anchor = range.start + selection.start;
        self.caret = range.start + selection.end;
        changed
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.snapshot());
        self.restore(previous);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.snapshot());
        self.restore(next);
        true
    }

    pub fn format_state(&self) -> Option<FormatState> {
        self.rich
            .as_ref()
            .map(|rich| rich.state(&self.text, self.selection()))
    }

    /// Takes a document of runs, the caret at its start, as the original
    /// does: typing keeps the style it had until the caret moves.
    pub fn set_runs(&mut self, runs: &[Run]) {
        let typing = self.rich.as_ref().map(|rich| rich.typing);
        self.set(runs.iter().map(|run| run.text.as_str()).collect());
        (self.anchor, self.caret) = (0, 0);
        let mut rich = RichFormat::new(&self.text);
        rich.styles = runs
            .iter()
            .flat_map(|run| {
                std::iter::repeat_n(run.style.scaled(run.style.scale), run.text.chars().count())
            })
            .collect();
        rich.typing = typing.unwrap_or_default();
        self.rich = Some(rich);
    }

    pub fn toggle_style(&mut self, attribute: StyleAttribute) {
        if self.rich.is_none() {
            return;
        }
        let selection = self.selection();
        if !selection.is_empty() {
            self.checkpoint();
        }
        self.rich
            .as_mut()
            .unwrap()
            .toggle(&self.text, selection, attribute);
        let style = self.format_state().unwrap().style;
        self.rich.as_mut().unwrap().typing = style;
    }

    pub fn set_scale(&mut self, scale: u8) {
        if self.rich.is_none() {
            return;
        }
        let selection = self.selection();
        if !selection.is_empty() {
            self.checkpoint();
        }
        self.rich
            .as_mut()
            .unwrap()
            .set_scale(&self.text, selection, scale);
        let style = self.format_state().unwrap().style;
        self.rich.as_mut().unwrap().typing = style;
    }

    pub fn set_alignment(&mut self, alignment: Alignment) {
        if self.rich.is_none() {
            return;
        }
        self.checkpoint();
        let selection = self.selection();
        self.rich
            .as_mut()
            .unwrap()
            .set_alignment(&self.text, selection, alignment);
    }

    pub fn utf16_range(&self, range: Range<usize>) -> Range<usize> {
        self.text[..self.floor_boundary(range.start)]
            .encode_utf16()
            .count()
            ..self.text[..self.floor_boundary(range.end)]
                .encode_utf16()
                .count()
    }

    pub fn byte_range(&self, range: Range<usize>) -> Range<usize> {
        utf16_range_to_bytes(&self.text, range)
    }

    fn floor_boundary(&self, offset: usize) -> usize {
        let mut offset = offset.min(self.text.len());
        while !self.text.is_char_boundary(offset) {
            offset -= 1;
        }
        offset
    }

    fn safe_range(&self, range: Range<usize>) -> Range<usize> {
        let start = self.floor_boundary(range.start);
        let end = self.floor_boundary(range.end).max(start);
        start..end
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            anchor: self.anchor,
            caret: self.caret,
            rich: self.rich.clone(),
        }
    }

    fn checkpoint(&mut self) {
        let snapshot = self.snapshot();
        if self.undo.last() != Some(&snapshot) {
            self.undo.push(snapshot);
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.text = snapshot.text;
        self.anchor = snapshot.anchor;
        self.caret = snapshot.caret;
        self.marked = None;
        self.rich = snapshot.rich;
    }
}

fn utf16_offset_to_byte(text: &str, offset: usize, round_up: bool) -> usize {
    let mut count = 0;
    for (index, ch) in text.char_indices() {
        if count >= offset {
            return index;
        }
        count += ch.len_utf16();
        if count > offset {
            return index + if round_up { ch.len_utf8() } else { 0 };
        }
    }
    text.len()
}

fn utf16_range_to_bytes(text: &str, range: Range<usize>) -> Range<usize> {
    let start = utf16_offset_to_byte(text, range.start, false);
    let end = utf16_offset_to_byte(text, range.end, !range.is_empty());
    start..end.max(start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn importing_a_document_resets_previous_styles_and_undo_history() {
        let mut buffer = TextBuffer::default();
        buffer.set_runs(&[Run {
            text: "old".into(),
            style: crate::TextStyle {
                bold: true,
                ..Default::default()
            },
        }]);
        buffer.replace(0..3, "edited");
        buffer.set_runs(&[Run {
            text: "新 document".into(),
            style: crate::TextStyle::default(),
        }]);
        assert!(!buffer.undo());
        assert!(
            buffer
                .rich
                .as_ref()
                .unwrap()
                .styles
                .iter()
                .all(|style| !style.bold)
        );
        buffer.replace(0..3, "New");
        assert!(buffer.undo());
        assert_eq!(buffer.text, "新 document");
    }

    fn buffer(text: &str) -> TextBuffer {
        let mut buffer = TextBuffer::default();
        buffer.set(text.into());
        buffer
    }

    #[test]
    fn replacing_several_matches_is_one_undoable_edit() {
        let mut buffer = TextBuffer::default();
        buffer.set("猫 cat 猫".into());
        buffer.select(3, false);
        assert_eq!(buffer.replace_many(&[0..3, 8..11], "dog"), 2);
        assert_eq!(buffer.text, "dog cat dog");
        assert!(buffer.undo());
        assert_eq!(buffer.text, "猫 cat 猫");
        assert_eq!(buffer.selection(), 3..3);
        assert!(buffer.redo());
        assert_eq!(buffer.text, "dog cat dog");
    }

    #[test]
    fn cursor_steps_over_combining_marks_and_emoji_as_graphemes() {
        let buffer = buffer("Ae\u{301}👩‍💻中");
        assert_eq!(buffer.next(1), 4);
        assert_eq!(buffer.previous(4), 1);
        assert_eq!(buffer.next(4), 15);
        assert_eq!(buffer.previous(15), 4);
        assert_eq!(buffer.previous(0), 0);
        assert_eq!(buffer.next(buffer.text.len()), buffer.text.len());
    }

    #[test]
    fn selection_keeps_its_anchor_when_crossing_it() {
        let mut buffer = buffer("abcd");
        buffer.select(2, false);
        buffer.select(4, true);
        assert_eq!(buffer.selection(), 2..4);
        buffer.select(1, true);
        assert_eq!(buffer.selection(), 1..2);
        assert_eq!(buffer.anchor, 2);
    }

    #[test]
    fn utf16_ranges_handle_supplementary_characters_and_invalid_offsets() {
        let buffer = buffer("A😀中");
        assert_eq!(buffer.byte_range(1..3), 1..5);
        assert_eq!(buffer.utf16_range(1..5), 1..3);
        assert_eq!(buffer.byte_range(2..2), 1..1);
        assert_eq!(buffer.byte_range(2..3), 1..5);
        assert_eq!(buffer.byte_range(90..99), 8..8);
    }

    #[test]
    fn undo_restores_replaced_selection_and_redo_is_cleared_by_new_edits() {
        let mut buffer = buffer("one two");
        buffer.select(4, false);
        buffer.select(7, true);
        buffer.replace(buffer.selection(), "three");
        assert_eq!(buffer.text, "one three");
        assert!(buffer.undo());
        assert_eq!(buffer.text, "one two");
        assert_eq!(buffer.selection(), 4..7);
        assert!(buffer.redo());
        assert_eq!(buffer.text, "one three");
        buffer.undo();
        buffer.replace(buffer.selection(), "four");
        assert!(!buffer.redo());
    }

    #[test]
    fn composition_selection_is_relative_to_inserted_text_and_undo_is_atomic() {
        let mut buffer = buffer("prefix suffix");
        buffer.select(7, false);
        buffer.compose(7..7, "😀a", Some(2..3));
        assert_eq!(buffer.selection(), 11..12);
        buffer.compose(buffer.marked.clone().unwrap(), "中文", Some(1..2));
        assert_eq!(buffer.selection(), 10..13);
        buffer.replace(buffer.marked.clone().unwrap(), "中文");
        assert!(buffer.undo());
        assert_eq!(buffer.text, "prefix suffix");
        assert!(!buffer.undo());
    }
}

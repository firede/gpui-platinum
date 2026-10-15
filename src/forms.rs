//! Compound fields and pickers made from the Platinum controls.

mod color;
mod combo;
mod datetime;
mod font_picker;
mod number;
mod search;
mod stepper;

pub use color::{ColorChanged, ColorWell};
pub use combo::{ComboChanged, Combobox};
pub use datetime::{DatePicker, Time, TimeChanged, TimeField};
pub use font_picker::{FontChanged, FontChoice, FontPicker};
pub use number::{NumberChanged, NumberField};
pub use search::{SearchChanged, SearchField};
pub use stepper::{Step, Stepper, StepperState, draw_stepper};

use crate::{Language, TextField, Theme};
use gpui::{App, Entity, SharedString};

#[derive(Clone)]
struct FieldStyle {
    theme: Theme,
    language: Language,
    label: SharedString,
    width: i32,
    disabled: bool,
    dirty: bool,
}

impl FieldStyle {
    fn new(width: i32) -> Self {
        Self {
            theme: Theme::default(),
            language: Language::English,
            label: "".into(),
            width,
            disabled: false,
            dirty: true,
        }
    }

    fn apply(&mut self, field: &Entity<TextField>, width: i32, cx: &mut App) {
        if !self.dirty {
            return;
        }
        field.update(cx, |field, cx| {
            field.set_theme(self.theme, cx);
            field.set_language(self.language, cx);
            field.set_label(self.label.clone(), cx);
            field.set_disabled(self.disabled, cx);
            field.set_size(width as f32, 22., cx);
        });
        self.dirty = false;
    }
}

fn modified(event: &gpui::KeyDownEvent) -> bool {
    event.keystroke.modifiers.platform
        || event.keystroke.modifiers.control
        || event.keystroke.modifiers.alt
}

fn word(language: Language, key: &str) -> String {
    crate::tr(language, key)
}

fn form_label(text: String, language: Language) -> impl gpui::IntoElement {
    crate::text_in_language(text, crate::Font::Bold, crate::theme::BLACK, language)
}

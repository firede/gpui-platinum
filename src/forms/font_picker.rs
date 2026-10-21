use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, Role, SharedString, Subscription,
    Window, div, prelude::*, px,
};

use super::{FieldStyle, NumberChanged, NumberField};
use crate::{
    CheckState, Font, Language, ListBox, ListEvent, TextStyle, Theme, checkbox,
    paint::{Pen, Rect, drawing},
    theme::BLACK,
};

const SIZES: [f64; 7] = [9., 10., 12., 14., 18., 24., 36.];

#[derive(Clone, Debug, PartialEq)]
pub struct FontChoice {
    pub family: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for FontChoice {
    fn default() -> Self {
        Self {
            family: "Platinum Sans Small".into(),
            size: 12.,
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontChanged(pub FontChoice);

/// Family, size and style selectors with a bitmap preview of the chosen style.
pub struct FontPicker {
    choice: FontChoice,
    families: Vec<String>,
    family_list: Entity<ListBox>,
    size_list: Entity<ListBox>,
    size_field: Entity<NumberField>,
    families_dirty: bool,
    style: FieldStyle,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<FontChanged> for FontPicker {}
impl Focusable for FontPicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.family_list.focus_handle(cx)
    }
}

impl FontPicker {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let families = vec!["Platinum Sans".into(), "Platinum Sans Small".into()];
        let family_list = cx.new(|cx| ListBox::new(families.clone(), cx).size(140, 98).selected(1));
        let size_list = cx.new(|cx| {
            ListBox::new(SIZES.map(|size| size.to_string()), cx)
                .size(76, 70)
                .selected(2)
        });
        let size_field = cx.new(|cx| NumberField::new(12., 4., 288., 1., cx).digits(0));
        let family_subscription = cx.subscribe(&family_list, |this, _, event, cx| {
            if let ListEvent::Selected(index) = event {
                let Some(family) = this.families.get(*index) else {
                    return;
                };
                if this.choice.family != *family {
                    this.choice.family = family.clone();
                    this.changed(cx);
                }
            }
        });
        let size_subscription = cx.subscribe(&size_list, |this, _, event, cx| {
            if let ListEvent::Selected(index) = event {
                let size = SIZES[*index];
                this.size_field
                    .update(cx, |field, cx| field.set_value(size, cx));
                if this.choice.size != size {
                    this.choice.size = size;
                    this.changed(cx);
                }
            }
        });
        let field_subscription = cx.subscribe(&size_field, |this, _, event: &NumberChanged, cx| {
            this.choice.size = event.0;
            this.size_list.update(cx, |list, cx| {
                list.set_selection(SIZES.iter().position(|size| *size == event.0), cx)
            });
            this.changed(cx);
        });
        Self {
            choice: FontChoice::default(),
            families,
            family_list,
            size_list,
            size_field,
            families_dirty: false,
            style: FieldStyle::new(360),
            _subscriptions: vec![family_subscription, size_subscription, field_subscription],
        }
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.style.theme = theme;
        self.style.dirty = true;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.style.language = language;
        self.style.dirty = true;
        self
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.style.label = label.into();
        self
    }
    pub fn width(mut self, width: i32) -> Self {
        self.style.width = width.max(1);
        self
    }
    pub fn set_width(&mut self, width: i32, cx: &mut Context<Self>) {
        self.style.width = width.max(1);
        cx.notify();
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.style.disabled = disabled;
        self.style.dirty = true;
        self
    }
    pub fn families(mut self, families: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.families = families.into_iter().map(Into::into).collect();
        if !self.families.contains(&self.choice.family) {
            self.choice.family = self.families.first().cloned().unwrap_or_default();
        }
        self.families_dirty = true;
        self
    }
    pub fn choice(&self) -> &FontChoice {
        &self.choice
    }
    pub fn set_choice(&mut self, mut choice: FontChoice, cx: &mut Context<Self>) {
        choice.size = if choice.size.is_finite() {
            choice.size.clamp(4., 288.)
        } else {
            12.
        };
        self.choice = choice;
        self.family_list.update(cx, |list, cx| {
            list.set_selection(
                self.families
                    .iter()
                    .position(|family| *family == self.choice.family),
                cx,
            )
        });
        self.size_list.update(cx, |list, cx| {
            list.set_selection(SIZES.iter().position(|size| *size == self.choice.size), cx)
        });
        self.size_field
            .update(cx, |field, cx| field.set_value(self.choice.size, cx));
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.style.theme = theme;
        self.style.dirty = true;
        cx.notify();
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.style.disabled = disabled;
        self.style.dirty = true;
        cx.notify();
    }
    pub fn set_families(&mut self, families: Vec<String>, cx: &mut Context<Self>) {
        self.families = families;
        if !self.families.contains(&self.choice.family) {
            self.choice.family = self.families.first().cloned().unwrap_or_default();
        }
        self.families_dirty = true;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.style.language = language;
        self.style.dirty = true;
        cx.notify();
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(FontChanged(self.choice.clone()));
        cx.notify();
    }
}

impl Render for FontPicker {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.style.language;
        let theme = self.style.theme;
        if self.families_dirty {
            self.family_list.update(cx, |list, cx| {
                list.set_items(self.families.iter().cloned().map(Into::into).collect(), cx);
                list.set_selection(
                    self.families
                        .iter()
                        .position(|family| *family == self.choice.family),
                    cx,
                );
            });
            self.families_dirty = false;
        }
        if self.style.dirty {
            self.family_list.update(cx, |list, cx| {
                list.set_theme(theme, cx);
                list.set_disabled(self.style.disabled, cx);
                list.set_label(super::word(language, "families"), cx);
            });
            self.size_list.update(cx, |list, cx| {
                list.set_theme(theme, cx);
                list.set_disabled(self.style.disabled, cx);
                list.set_label(super::word(language, "sizes"), cx);
            });
            self.size_field.update(cx, |field, cx| {
                field.set_theme(theme, cx);
                field.set_language(language, cx);
                field.set_label(super::word(language, "size"), cx);
                field.set_disabled(self.style.disabled, cx);
            });
            self.style.dirty = false;
        }
        let choice = self.choice.clone();
        let mut styles = div().flex().flex_col().gap(px(2.)).child(super::form_label(
            super::word(language, "styleLabel"),
            language,
        ));
        for (index, key, selected) in [
            (0usize, "bold", choice.bold),
            (1, "italic", choice.italic),
            (2, "underline", choice.underline),
        ] {
            styles = styles.child(
                checkbox(
                    ("style", index),
                    super::word(language, key),
                    if selected {
                        CheckState::On
                    } else {
                        CheckState::Off
                    },
                )
                .language(language)
                .theme(theme)
                .disabled(self.style.disabled)
                .on_click(cx.listener(move |this, _, _, cx| {
                    match index {
                        0 => this.choice.bold = !this.choice.bold,
                        1 => this.choice.italic = !this.choice.italic,
                        _ => this.choice.underline = !this.choice.underline,
                    }
                    this.changed(cx);
                })),
            );
        }
        let title = if self.style.label.is_empty() {
            super::word(language, "font").into()
        } else {
            self.style.label.clone()
        };
        div()
            .id("font-picker")
            .role(Role::Group)
            .aria_label(title)
            .w(px(self.style.width as f32))
            .flex()
            .flex_col()
            .gap(px(8.))
            .flex_shrink_0()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(super::form_label(
                                super::word(language, "fontLabel"),
                                language,
                            ))
                            .child(self.family_list.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(super::form_label(
                                super::word(language, "sizeLabel"),
                                language,
                            ))
                            .child(self.size_field.clone())
                            .child(self.size_list.clone()),
                    )
                    .child(styles),
            )
            .child(
                div()
                    .id("sample")
                    .role(Role::Image)
                    .aria_label(super::word(language, "sample"))
                    .overflow_hidden()
                    .child(drawing(self.style.width as f32, 40., move |pen, _, _| {
                        draw_sample(pen, &choice, language)
                    })),
            )
    }
}

fn draw_sample(pen: &mut Pen, choice: &FontChoice, language: Language) {
    pen.set_dimmed(crate::window::content_is_inactive());
    pen.sunken();
    let font = Font::Small;
    let scale = ((choice.size + 6.) / 12.).floor().clamp(1., 24.) as i32;
    let name = format!("{} {}", choice.family, choice.size);
    if scale == 1 {
        // The sample sentence in the style chosen, inside the well.
        let mut sample = Pen::new(pen.width, pen.height);
        sample.language = language;
        let style = TextStyle {
            bold: choice.bold,
            italic: choice.italic,
            underline: choice.underline,
            scale: 1,
        };
        let text = format!("{} — {name}", super::word(language, "sampleText"));
        let baseline = (pen.height - font.line_height()) / 2 + font.ascent();
        sample.text_styled(8, baseline, &text, font, BLACK, style);
        sample.clip(Rect {
            x: 3,
            y: 3,
            width: pen.width - 6,
            height: pen.height - 6,
        });
        pen.append(sample, 0, 0);
        return;
    }
    // Larger, "Aa" and the size, in plain pixels as large as the size.
    let text = format!("Aa {name}");
    let baseline = (pen.height - font.line_height() * scale) / 2 + font.ascent() * scale;
    let mut x = 8;
    for character in text.chars() {
        if x >= pen.width - 3 {
            break;
        }
        let glyph = font.glyph_or_replacement(character, language);
        glyph.for_each_run(|dx, dy, width| {
            clipped_rect(
                pen,
                x + dx * scale,
                baseline + dy * scale,
                width * scale,
                scale,
            );
        });
        x += glyph.advance * scale;
    }
    if choice.underline {
        clipped_rect(pen, 8, baseline + scale, x - 8, scale);
    }
}

fn clipped_rect(pen: &mut Pen, x: i32, y: i32, width: i32, height: i32) {
    let left = x.max(3);
    let top = y.max(3);
    pen.rect(
        left,
        top,
        (x + width).min(pen.width - 3) - left,
        (y + height).min(pen.height - 3) - top,
        BLACK,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_preview_ink_stays_inside_the_well() {
        let mut pen = Pen::new(360, 40);
        let choice = FontChoice {
            size: 288.,
            bold: true,
            italic: true,
            underline: true,
            ..FontChoice::default()
        };
        let mut frame = Pen::new(360, 40);
        frame.sunken();
        draw_sample(&mut pen, &choice, Language::English);
        for fill in &pen.fills[frame.fills.len()..] {
            assert!(fill.rect.x >= 3 && fill.rect.y >= 3);
            assert!(fill.rect.x + fill.rect.width <= 357 && fill.rect.y + fill.rect.height <= 37);
        }
    }
}

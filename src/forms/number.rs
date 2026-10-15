use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, Role, SharedString, Subscription,
    Window, div, prelude::*, px,
};

use super::{FieldStyle, Step, Stepper};
use crate::{InputEvent, Language, TextField, Theme};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberChanged(pub f64);

/// An editable number and repeating arrows. Return or blur commits and clamps it.
pub struct NumberField {
    field: Entity<TextField>,
    stepper: Entity<Stepper>,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    digits: usize,
    style: FieldStyle,
    blur: Option<Subscription>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<NumberChanged> for NumberField {}
impl Focusable for NumberField {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.field.focus_handle(cx)
    }
}

impl NumberField {
    pub fn new(value: f64, min: f64, max: f64, step: f64, cx: &mut Context<Self>) -> Self {
        let min = if min.is_finite() { min } else { 0. };
        let max = if max.is_finite() { max.max(min) } else { min };
        let step = if step.is_finite() && step > 0. {
            step
        } else {
            1.
        };
        let value = clamp(value, min, max);
        let digits = decimal_places(step);
        let field = cx.new(|cx| {
            TextField::new(cx)
                .with_text(format_number(value, digits))
                .size(56., 22.)
        });
        let stepper = cx.new(|cx| Stepper::new(cx).tab_stop(false));
        let input_subscription = cx.subscribe(&field, |this, _, event, cx| match event {
            InputEvent::Submitted => this.commit(cx),
            InputEvent::Changed => {
                let parsed = parse_number(this.field.read(cx).value());
                if let Some(value) = parsed.filter(|value| *value >= this.min && *value <= this.max)
                {
                    this.change_value(value, cx);
                }
            }
        });
        let step_subscription = cx.subscribe(&stepper, |this, _, event: &Step, cx| {
            this.increment(event.0, cx)
        });
        Self {
            field,
            stepper,
            value,
            min,
            max,
            step,
            digits,
            style: FieldStyle::new(72),
            blur: None,
            _subscriptions: vec![input_subscription, step_subscription],
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
        self.style.dirty = true;
        self
    }
    pub fn width(mut self, width: i32) -> Self {
        self.style.width = width.max(48);
        self.style.dirty = true;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.style.disabled = disabled;
        self.style.dirty = true;
        self
    }
    pub fn digits(mut self, digits: usize) -> Self {
        self.digits = digits.min(8);
        self.style.dirty = true;
        self
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn text_field(&self) -> &Entity<TextField> {
        &self.field
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.style.theme = theme;
        self.style.dirty = true;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.style.language = language;
        self.style.dirty = true;
        cx.notify();
    }
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.style.label = label.into();
        self.style.dirty = true;
        cx.notify();
    }
    pub fn set_value(&mut self, value: f64, cx: &mut Context<Self>) {
        self.value = clamp(value, self.min, self.max);
        self.sync_text(cx);
    }

    pub fn commit(&mut self, cx: &mut Context<Self>) {
        if self.style.disabled {
            return;
        }
        if let Some(value) = parse_number(self.field.read(cx).value()) {
            self.change_value(clamp(value, self.min, self.max), cx);
        }
        self.sync_text(cx);
    }

    fn change_value(&mut self, value: f64, cx: &mut Context<Self>) {
        if self.value != value {
            self.value = value;
            cx.emit(NumberChanged(value));
            cx.notify();
        }
    }

    fn increment(&mut self, direction: i32, cx: &mut Context<Self>) {
        if self.style.disabled {
            return;
        }
        let current = parse_number(self.field.read(cx).value()).unwrap_or(self.value);
        let next = stepped_value(current, direction, self.min, self.max, self.step);
        self.change_value(next, cx);
        self.sync_text(cx);
    }

    fn sync_text(&mut self, cx: &mut Context<Self>) {
        let text = format_number(self.value, self.digits);
        self.field.update(cx, |field, cx| field.set_value(text, cx));
        cx.notify();
    }
}

impl Render for NumberField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.blur.is_none() {
            self.blur = Some(
                cx.on_blur(&self.field.focus_handle(cx), window, |this, _, cx| {
                    this.commit(cx)
                }),
            );
        }
        if self.style.dirty {
            self.stepper.update(cx, |stepper, cx| {
                stepper.set_theme(self.style.theme, cx);
                stepper.set_label(self.style.label.clone(), cx);
                stepper.set_disabled(self.style.disabled, cx);
            });
            self.style.apply(&self.field, self.style.width - 16, cx);
            self.sync_text(cx);
        }
        div()
            .id("number-field")
            .role(Role::Group)
            .aria_label(self.style.label.clone())
            .flex()
            .items_center()
            .gap(px(3.))
            .flex_shrink_0()
            .capture_key_down(cx.listener(|this, event, _, cx| {
                if super::modified(event) || this.style.disabled {
                    return;
                }
                let direction = match event.keystroke.key.as_str() {
                    "up" => 1,
                    "down" => -1,
                    _ => return,
                };
                this.increment(direction, cx);
                cx.stop_propagation();
            }))
            .child(self.field.clone())
            .child(self.stepper.clone())
    }
}

fn parse_number(text: &str) -> Option<f64> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}
fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        min
    }
}
fn stepped_value(value: f64, direction: i32, min: f64, max: f64, step: f64) -> f64 {
    let next = clamp(value, min, max) + direction as f64 * step;
    let snapped = min + ((next - min) / step).round() * step;
    clamp(snapped, min, max)
}
fn decimal_places(step: f64) -> usize {
    for digits in 0..=8 {
        let scaled = step * 10f64.powi(digits as i32);
        if (scaled - scaled.round()).abs() < 1e-8 {
            return digits;
        }
    }
    8
}
fn format_number(value: f64, digits: usize) -> String {
    format!("{value:.digits$}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_or_nonfinite_input_cannot_replace_a_number() {
        for text in ["", "-", ".", "NaN", "inf", "1e999"] {
            assert_eq!(parse_number(text), None);
        }
        assert_eq!(parse_number(" -2.5 "), Some(-2.5));
    }
    #[test]
    fn arrow_steps_snap_and_clamp_with_fractional_precision() {
        assert_eq!(stepped_value(9.9, 1, 0., 10., 0.25), 10.);
        assert_eq!(stepped_value(-2., -1, -2., 10., 0.5), -2.);
        assert_eq!(decimal_places(0.125), 3);
        assert_eq!(format_number(1.25, 2), "1.25");
    }
}

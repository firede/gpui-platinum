use std::time::{Duration, Instant};

use gpui::{
    Anchor, App, Context, Entity, EventEmitter, FocusHandle, Focusable, MouseButton, Role,
    SharedString, Subscription, Window, anchored, deferred, div, point, prelude::*, px,
};

use super::{FieldStyle, Step, Stepper};
use crate::{
    CalendarView, DateChanged, Font, Language, Theme,
    models::Date,
    paint::{Pen, drawing},
    theme::*,
};

/// A date button opening the shared calendar. Keyboard changes remain open
/// until Return; clicking a day closes the calendar.
pub struct DatePicker {
    focus: FocusHandle,
    calendar: Entity<CalendarView>,
    value: Date,
    open: bool,
    pressed: bool,
    auto_width: bool,
    style: FieldStyle,
    subscriptions: Vec<Subscription>,
}

impl EventEmitter<DateChanged> for DatePicker {}
impl Focusable for DatePicker {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl DatePicker {
    pub fn new(date: Date, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            calendar: cx.new(|cx| CalendarView::new(date, cx)),
            value: date,
            open: false,
            pressed: false,
            auto_width: true,
            style: FieldStyle::new(160),
            subscriptions: Vec::new(),
        }
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.style.theme = theme;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.style.language = language;
        self
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.style.label = label.into();
        self
    }
    pub fn width(mut self, width: i32) -> Self {
        self.style.width = width.max(100);
        self.auto_width = false;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.style.disabled = disabled;
        self
    }
    pub fn value(&self) -> Date {
        self.value
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn set_value(&mut self, value: Date, cx: &mut Context<Self>) {
        self.value = value;
        self.calendar
            .update(cx, |calendar, cx| calendar.set_selection(value, cx));
        cx.notify();
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.style.disabled = disabled;
        if disabled {
            self.open = false;
            self.pressed = false;
        }
        cx.notify();
    }
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.style.label = label.into();
        cx.notify();
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

    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.style.disabled {
            return;
        }
        self.open = !self.open;
        let focus = if self.open {
            self.calendar.focus_handle(cx)
        } else {
            self.focus.clone()
        };
        window.focus(&focus, cx);
        cx.notify();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }
}

impl Render for DatePicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.subscriptions.is_empty() {
            self.subscriptions.push(cx.subscribe_in(
                &self.calendar,
                window,
                |this, _, event: &DateChanged, _, cx| {
                    if this.value != event.0 {
                        this.value = event.0;
                        cx.emit(*event);
                    }
                    cx.notify();
                },
            ));
            self.subscriptions.push(cx.subscribe_in(
                &self.calendar,
                window,
                |this, _, _: &crate::calendar::DateChosen, window, cx| this.close(window, cx),
            ));
        }
        if self.style.dirty {
            self.calendar.update(cx, |calendar, cx| {
                calendar.set_theme(self.style.theme, cx);
                calendar.set_language(self.style.language, cx);
            });
            self.style.dirty = false;
        }
        let text = format_date(self.value, self.style.language);
        let width = if self.auto_width {
            date_picker_width(self.style.language)
        } else {
            self.style.width
        };
        let pressed = self.pressed;
        let disabled = self.style.disabled;
        let open = self.open;
        let focused = self.focus.is_focused(window) && window.last_input_was_keyboard();
        let accent = self.style.theme.accent.dark;
        let language = self.style.language;
        let mut root = div()
            .id("date-picker")
            .role(Role::ComboBox)
            .aria_label(if self.style.label.is_empty() {
                text.clone().into()
            } else {
                self.style.label.clone()
            })
            .aria_value(text.clone())
            .aria_expanded(open)
            .relative()
            .w(px(width as f32))
            .h(px(20.))
            .flex_shrink_0()
            .when(!disabled, |element| element.track_focus(&self.focus))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if !this.style.disabled {
                        this.pressed = true;
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.pressed = false;
                    cx.notify();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.pressed = false;
                    cx.notify();
                }),
            )
            .on_click(cx.listener(|this, _, window, cx| this.toggle(window, cx)))
            .capture_key_down(cx.listener(|this, event, window, cx| {
                if this.style.disabled || super::modified(event) {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "escape" | "enter" if this.open => this.close(window, cx),
                    "down" | "enter" | "space" if !this.open => this.toggle(window, cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .child(drawing(width as f32, 20., move |pen, _, _| {
                pen.language = language;
                draw_date_button(pen, &text, disabled, pressed);
                if focused && !crate::window::content_is_inactive() {
                    crate::draw_focus_ring(pen, 3, accent);
                }
            }));
        if open && !disabled {
            let calendar = div()
                .id("calendar-popup")
                .occlude()
                .on_mouse_down_out(cx.listener(|this, _, window, cx| this.close(window, cx)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(self.calendar.clone());
            root = root.child(
                div().absolute().left(px(0.)).top(px(0.)).child(
                    deferred(
                        anchored()
                            .anchor(Anchor::TopLeft)
                            .offset(point(px(0.), px(22.)))
                            .snap_to_window_with_margin(px(4.))
                            .child(calendar),
                    )
                    .priority(3),
                ),
            );
        }
        root
    }
}

fn date_picker_width(language: Language) -> i32 {
    [
        Date::new(2000, 9, 30).unwrap(),
        Date::new(2000, 12, 30).unwrap(),
    ]
    .into_iter()
    .map(|date| Font::Bold.measure_in(&format_date(date, language), language))
    .max()
    .unwrap_or(0)
        + 35
}

fn draw_date_button(pen: &mut Pen, text: &str, disabled: bool, pressed: bool) {
    let inactive = crate::window::content_is_inactive();
    let pressed = pressed && !inactive;
    crate::draw_button(pen, "", disabled, pressed);
    let arrow = pen.width - 18;
    let ink = if disabled || inactive {
        GRAY_8
    } else if pressed {
        WHITE
    } else {
        BLACK
    };
    let (line, highlight) = if disabled {
        (GRAY_4, GRAY_1)
    } else if inactive {
        (GRAY_2, GRAY_2)
    } else if pressed {
        (GRAY_11, GRAY_8)
    } else {
        (GRAY_7, WHITE)
    };
    pen.vline(arrow, 3, pen.height - 6, line);
    pen.vline(arrow + 1, 3, pen.height - 6, highlight);
    for row in 0..4 {
        pen.hline(
            arrow + 6 + row,
            (pen.height - 4) / 2 + row,
            7 - row * 2,
            ink,
        );
    }
    let limit = arrow - 13;
    let mut label = text.to_owned();
    if Font::Bold.measure_in(&label, pen.language) > limit {
        while !label.is_empty()
            && Font::Bold.measure_in(&(label.clone() + "…"), pen.language) > limit
        {
            label.pop();
        }
        label.push('…');
    }
    pen.text(9, (pen.height - 9) / 2 + 9, &label, Font::Bold, ink);
}

fn format_date(date: Date, language: Language) -> String {
    match language {
        Language::English => {
            let months = [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ];
            format!(
                "{} {}, {}",
                months[(date.month - 1) as usize],
                date.day,
                date.year
            )
        }
        Language::Korean => format!("{}년 {}월 {}일", date.year, date.month, date.day),
        _ => format!("{}年{}月{}日", date.year, date.month, date.day),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Time {
    pub fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self {
            hour: hour.min(23),
            minute: minute.min(59),
            second: second.min(59),
        }
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:02}:{:02}:{:02}",
            self.hour, self.minute, self.second
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeChanged(pub Time);

/// A localized time of day: hours and minutes, with AM or PM in a 12-hour cycle.
/// Seconds are retained in the value while the displayed parts are edited.
pub struct TimeField {
    focus: FocusHandle,
    stepper: Entity<Stepper>,
    value: Time,
    segment: usize,
    minute_step: u8,
    twelve_hours: Option<bool>,
    typed: String,
    typed_at: Instant,
    style: FieldStyle,
    _subscription: Subscription,
}

impl EventEmitter<TimeChanged> for TimeField {}
impl Focusable for TimeField {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TimeField {
    pub fn new(hour: u8, minute: u8, second: u8, cx: &mut Context<Self>) -> Self {
        let stepper = cx.new(|cx| Stepper::new(cx).tab_stop(false));
        let subscription = cx.subscribe(&stepper, |this, _, event: &Step, cx| {
            this.increment(event.0, cx)
        });
        Self {
            focus: cx.focus_handle().tab_stop(true),
            stepper,
            value: Time::new(hour, minute, second),
            segment: 0,
            minute_step: 1,
            twelve_hours: None,
            typed: String::new(),
            typed_at: cx.background_executor().now(),
            style: FieldStyle::new(96),
            _subscription: subscription,
        }
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.style.theme = theme;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.style.language = language;
        self
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.style.label = label.into();
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.style.disabled = disabled;
        self
    }
    /// Overrides the language's default with a cycle of 12 or 24 hours.
    pub fn hour_cycle(mut self, hours: u8) -> Self {
        self.twelve_hours = Some(hours == 12);
        self
    }
    /// The number of minutes moved by Up, Down and the little arrows.
    pub fn step(mut self, minutes: u8) -> Self {
        self.minute_step = minutes.clamp(1, 60);
        self
    }
    pub fn value(&self) -> Time {
        self.value
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
    pub fn set_value(&mut self, value: Time, cx: &mut Context<Self>) {
        self.value = Time::new(value.hour, value.minute, value.second);
        cx.notify();
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.style.disabled = disabled;
        self.style.dirty = true;
        self.typed.clear();
        cx.notify();
    }
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.style.label = label.into();
        self.style.dirty = true;
        cx.notify();
    }
    pub fn set_hour_cycle(&mut self, hours: u8, cx: &mut Context<Self>) {
        self.twelve_hours = Some(hours == 12);
        self.typed.clear();
        cx.notify();
    }

    fn layout(&self) -> TimeLayout {
        let twelve = self.twelve_hours.unwrap_or(matches!(
            self.style.language,
            Language::English | Language::TraditionalChinese | Language::Korean
        ));
        TimeLayout::new(self.style.language, twelve)
    }
    fn change(&mut self, value: Time, cx: &mut Context<Self>) {
        if self.value != value {
            self.value = value;
            cx.emit(TimeChanged(value));
        }
        cx.notify();
    }
    fn increment(&mut self, direction: i32, cx: &mut Context<Self>) {
        if self.style.disabled {
            return;
        }
        self.typed.clear();
        self.change(
            step_time(self.value, self.segment, direction, self.minute_step),
            cx,
        );
    }
    fn move_segment(&mut self, direction: i32) {
        let layout = self.layout();
        let current = layout
            .order
            .iter()
            .position(|part| *part == self.segment)
            .unwrap_or(0);
        let next = (current as i32 + direction).clamp(0, layout.order.len() as i32 - 1) as usize;
        self.segment = layout.order[next];
        self.typed.clear();
    }
    fn type_digit(&mut self, digit: char, cx: &mut Context<Self>) {
        if self.segment == 2 {
            return;
        }
        let now = cx.background_executor().now();
        if now.duration_since(self.typed_at) > Duration::from_secs(1) {
            self.typed.clear();
        }
        self.typed_at = now;
        self.typed.push(digit);
        let twelve = self.layout().twelve;
        let maximum = if self.segment == 0 {
            if twelve { 12 } else { 23 }
        } else {
            59
        };
        let mut number = self.typed.parse::<u8>().unwrap_or(maximum + 1);
        if number > maximum || (self.segment == 0 && twelve && number == 0) {
            self.typed = digit.to_string();
            number = digit as u8 - b'0';
        }
        let mut value = self.value;
        if self.segment == 0 {
            value.hour = if twelve {
                number % 12 + if value.hour >= 12 { 12 } else { 0 }
            } else {
                number
            };
        } else {
            value.minute = number;
        }
        self.change(value, cx);
        if self.typed.len() == 2 {
            self.move_segment(1);
        }
    }
}

impl Render for TimeField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.style.dirty {
            self.stepper.update(cx, |stepper, cx| {
                stepper.set_theme(self.style.theme, cx);
                stepper.set_disabled(self.style.disabled, cx);
                stepper.set_label(super::word(self.style.language, "changeTime"), cx);
            });
            self.style.dirty = false;
        }
        let layout = self.layout();
        if !layout.order.contains(&self.segment) {
            self.segment = 0;
        }
        let positions = layout.positions(self.value);
        let width = layout.width();
        let disabled = self.style.disabled;
        let focused =
            self.focus.is_focused(window) || self.stepper.focus_handle(cx).is_focused(window);
        let value = self.value;
        let segment = self.segment;
        let theme = self.style.theme;
        let label = if self.style.label.is_empty() {
            super::word(self.style.language, "time").into()
        } else {
            self.style.label.clone()
        };
        let painter = layout.clone();
        let mut field = div()
            .id("time")
            .relative()
            .role(Role::TextInput)
            .aria_label(label)
            .aria_value(layout.text(value))
            .w(px(width as f32))
            .h(px(22.))
            .flex_shrink_0()
            .when(!disabled, |element| element.track_focus(&self.focus))
            .child(drawing(width as f32, 22., move |pen, _, _| {
                painter.draw(pen, value, segment, focused, disabled, theme)
            }));
        for (position, &segment) in layout.order.iter().enumerate() {
            let left = if position == 0 {
                0
            } else {
                positions[position] - 2
            };
            let right = if position + 1 == layout.order.len() {
                width
            } else {
                positions[position + 1] - 2
            };
            field = field.child(
                div()
                    .id(segment)
                    .absolute()
                    .left(px(left as f32))
                    .top(px(0.))
                    .w(px((right - left) as f32))
                    .h(px(22.))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            if this.style.disabled {
                                return;
                            }
                            this.segment = segment;
                            this.typed.clear();
                            window.focus(&this.focus, cx);
                            cx.notify();
                            cx.stop_propagation();
                        }),
                    ),
            );
        }
        div()
            .id("time-field")
            .flex()
            .items_center()
            .gap(px(3.))
            .flex_shrink_0()
            .capture_key_down(cx.listener(|this, event, _, cx| {
                if this.style.disabled || super::modified(event) {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "left" => this.move_segment(-1),
                    "right" | ":" => this.move_segment(1),
                    "home" => {
                        this.segment = this.layout().order[0];
                        this.typed.clear();
                    }
                    "end" => {
                        this.segment = *this.layout().order.last().unwrap();
                        this.typed.clear();
                    }
                    "up" => this.increment(1, cx),
                    "down" => this.increment(-1, cx),
                    "a" | "p" if this.layout().twelve && this.segment == 2 => {
                        let mut value = this.value;
                        value.hour =
                            value.hour % 12 + if event.keystroke.key == "p" { 12 } else { 0 };
                        this.change(value, cx);
                    }
                    key if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() => {
                        this.type_digit(key.chars().next().unwrap(), cx)
                    }
                    _ => return,
                }
                cx.notify();
                cx.stop_propagation();
            }))
            .child(field)
            .child(self.stepper.clone())
    }
}

fn step_time(mut value: Time, segment: usize, direction: i32, minute_step: u8) -> Time {
    match segment {
        0 => value.hour = (i32::from(value.hour) + direction).rem_euclid(24) as u8,
        1 => {
            let step = i32::from(minute_step.clamp(1, 60));
            value.minute =
                (((i32::from(value.minute) + direction * step).rem_euclid(60) / step) * step) as u8;
        }
        _ => value.hour = (value.hour + 12) % 24,
    }
    value
}

#[derive(Clone)]
struct TimeLayout {
    language: Language,
    twelve: bool,
    order: Vec<usize>,
    separators: Vec<&'static str>,
    padded: bool,
}
impl TimeLayout {
    fn new(language: Language, twelve: bool) -> Self {
        let (order, separators) = if !twelve {
            (vec![0, 1], vec![":"])
        } else if language == Language::English {
            (vec![0, 1, 2], vec![":", " "])
        } else {
            (
                vec![2, 0, 1],
                vec![
                    if language == Language::Korean {
                        " "
                    } else {
                        ""
                    },
                    ":",
                ],
            )
        };
        Self {
            language,
            twelve,
            order,
            separators,
            padded: !twelve && matches!(language, Language::English | Language::Korean),
        }
    }
    fn periods(&self) -> [&'static str; 2] {
        match self.language {
            Language::English => ["AM", "PM"],
            Language::Japanese => ["午前", "午後"],
            Language::Korean => ["오전", "오후"],
            _ => ["上午", "下午"],
        }
    }
    fn parts(&self, value: Time) -> [String; 3] {
        let hour = if self.twelve {
            match value.hour % 12 {
                0 => 12,
                hour => hour,
            }
        } else {
            value.hour
        };
        [
            if self.padded {
                format!("{hour:02}")
            } else {
                hour.to_string()
            },
            format!("{:02}", value.minute),
            self.periods()[usize::from(value.hour >= 12)].into(),
        ]
    }
    fn join(&self, parts: &[String; 3]) -> String {
        self.order
            .iter()
            .enumerate()
            .map(|(index, part)| {
                format!(
                    "{}{}",
                    parts[*part],
                    self.separators.get(index).unwrap_or(&"")
                )
            })
            .collect()
    }
    fn text(&self, value: Time) -> String {
        self.join(&self.parts(value))
    }
    fn positions(&self, value: Time) -> Vec<i32> {
        let parts = self.parts(value);
        let mut positions = vec![5];
        for (index, part) in self.order.iter().enumerate() {
            let shown = format!(
                "{}{}",
                parts[*part],
                self.separators.get(index).unwrap_or(&"")
            );
            positions.push(
                positions.last().unwrap() + Font::Small.measure_in(&shown, self.language) + 1,
            );
        }
        positions
    }
    fn width(&self) -> i32 {
        self.periods()
            .iter()
            .map(|period| {
                Font::Small.measure_in(
                    &self.join(&["23".into(), "59".into(), (*period).into()]),
                    self.language,
                )
            })
            .max()
            .unwrap_or(0)
            + 18
    }
    fn draw(
        &self,
        pen: &mut crate::paint::Pen,
        value: Time,
        segment: usize,
        focused: bool,
        disabled: bool,
        theme: Theme,
    ) {
        let inactive = crate::window::content_is_inactive();
        let focused = focused && !inactive;
        pen.sunken();
        if inactive {
            pen.frame(1, 1, pen.width - 2, pen.height - 2, GRAY_8);
        }
        pen.language = self.language;
        if disabled {
            pen.frame(1, 1, pen.width - 2, pen.height - 2, GRAY_6);
            pen.rect(2, 2, pen.width - 4, pen.height - 4, GRAY_2);
        }
        let positions = self.positions(value);
        let parts = self.parts(value);
        for (index, &part) in self.order.iter().enumerate() {
            let x = positions[index];
            if focused && !disabled && part == segment {
                pen.rect(
                    x - 1,
                    4,
                    positions[index + 1] - x - 1,
                    pen.height - 8,
                    theme.accent.light,
                );
            }
            let text = format!(
                "{}{}",
                parts[part],
                self.separators.get(index).unwrap_or(&"")
            );
            pen.text(
                x,
                15,
                &text,
                Font::Small,
                if disabled || inactive { GRAY_8 } else { BLACK },
            );
        }
        if focused && !disabled {
            crate::draw_focus_ring(pen, 0, theme.accent.dark);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raster(pen: &Pen, image: &mut image::RgbaImage, x: i32, y: i32) {
        for fill in &pen.fills {
            let color = image::Rgba([
                (fill.color >> 16) as u8,
                (fill.color >> 8) as u8,
                fill.color as u8,
                255,
            ]);
            for row in 0..fill.rect.height {
                for column in 0..fill.rect.width {
                    let x = x + fill.rect.x + column;
                    let y = y + fill.rect.y + row;
                    if x >= 0 && y >= 0 && x < image.width() as i32 && y < image.height() as i32 {
                        image.put_pixel(x as u32, y as u32, color);
                    }
                }
            }
        }
    }

    #[test]
    fn date_button_matches_the_original_pixels() {
        let reference =
            image::load_from_memory(include_bytes!("../../tests/reference/datepicker-open.png"))
                .unwrap()
                .to_rgba8();
        let width = date_picker_width(Language::English);
        let mut pen = Pen::new(width, 20);
        draw_date_button(&mut pen, "Oct 14, 2026", false, false);
        let mut actual =
            image::RgbaImage::from_pixel(width as u32, 20, image::Rgba([0xdd, 0xdd, 0xdd, 255]));
        raster(&pen, &mut actual, 0, 0);
        let reference = image::imageops::crop_imm(&reference, 10, 10, width as u32, 20).to_image();
        // The open calendar's focus ring overlaps the button's bottom row.
        let differences: Vec<_> = actual
            .enumerate_pixels()
            .filter(|(x, y, pixel)| *y < 19 && reference.get_pixel(*x, *y) != *pixel)
            .map(|(x, y, _)| (x, y))
            .collect();
        assert!(
            differences.is_empty(),
            "{} pixels differ; first {:?}",
            differences.len(),
            differences.first()
        );
    }

    #[test]
    fn localized_time_field_matches_the_original_pixels() {
        let reference =
            image::load_from_memory(include_bytes!("../../tests/reference/timefield.png"))
                .unwrap()
                .to_rgba8();
        let mut actual =
            image::RgbaImage::from_pixel(200, 60, image::Rgba([0xdd, 0xdd, 0xdd, 255]));
        let layout = TimeLayout::new(Language::English, true);
        let mut field = Pen::new(layout.width(), 22);
        layout.draw(
            &mut field,
            Time::new(22, 5, 0),
            2,
            true,
            false,
            Theme::default(),
        );
        raster(&field, &mut actual, 10, 11);
        let mut stepper = Pen::new(13, 23);
        crate::draw_stepper(&mut stepper, 0, crate::StepperState::Normal);
        raster(&stepper, &mut actual, 10 + layout.width() + 3, 10);
        let differences: Vec<_> = actual
            .enumerate_pixels()
            .filter(|(x, y, pixel)| reference.get_pixel(*x, *y) != *pixel)
            .map(|(x, y, _)| (x, y))
            .collect();
        assert!(
            differences.is_empty(),
            "{} pixels differ; first {:?}; width {}",
            differences.len(),
            differences.first(),
            layout.width()
        );
    }

    #[test]
    fn time_layout_follows_language_and_hour_cycle() {
        let time = Time::new(21, 41, 7);
        for (language, twelve, text) in [
            (Language::English, true, "9:41 PM"),
            (Language::English, false, "21:41"),
            (Language::SimplifiedChinese, true, "下午9:41"),
            (Language::SimplifiedChinese, false, "21:41"),
            (Language::TraditionalChinese, true, "下午9:41"),
            (Language::Japanese, true, "午後9:41"),
            (Language::Korean, true, "오후 9:41"),
            (Language::Korean, false, "21:41"),
        ] {
            assert_eq!(TimeLayout::new(language, twelve).text(time), text);
        }
        assert_eq!(
            step_time(Time::new(23, 59, 7), 1, 1, 5),
            Time::new(23, 0, 7)
        );
    }
    #[test]
    fn time_segments_wrap_without_changing_other_parts() {
        assert_eq!(
            step_time(Time::new(23, 59, 59), 0, 1, 1),
            Time::new(0, 59, 59)
        );
        assert_eq!(
            step_time(Time::new(12, 0, 30), 1, -1, 1),
            Time::new(12, 59, 30)
        );
        assert_eq!(
            step_time(Time::new(12, 10, 59), 2, 1, 1),
            Time::new(0, 10, 59)
        );
        assert_eq!(Time::new(99, 99, 99).to_string(), "23:59:59");
    }
    #[test]
    fn dates_use_the_selected_languages_order() {
        let date = Date::new(2026, 10, 15).unwrap();
        assert_eq!(format_date(date, Language::English), "Oct 15, 2026");
        assert_eq!(format_date(date, Language::Japanese), "2026年10月15日");
        assert_eq!(format_date(date, Language::Korean), "2026년 10월 15일");
    }
}

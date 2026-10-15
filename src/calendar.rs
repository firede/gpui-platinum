//! A calendar with a month header and six weeks of selectable days.

use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, KeyDownEvent, MouseButton, Role,
    SharedString, Window, div, prelude::*, px,
};

use crate::{
    Font, Language, Theme,
    models::{Calendar, Date},
    paint::drawing,
    theme::*,
};

pub const CALENDAR_WIDTH: i32 = 166;
pub const CALENDAR_HEIGHT: i32 = 140;
const DAY_WIDTH: i32 = 22;
const DAY_HEIGHT: i32 = 16;
const GRID_LEFT: i32 = 6;
const GRID_TOP: i32 = 38;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateChanged(pub Date);

pub struct CalendarView {
    focus: FocusHandle,
    model: Calendar,
    today: Date,
    theme: Theme,
    language: Language,
    label: SharedString,
}

impl EventEmitter<DateChanged> for CalendarView {}

impl CalendarView {
    pub fn new(date: Date, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            model: Calendar::new(date),
            today: Date::today_utc(),
            theme: Theme::default(),
            language: Language::English,
            label: "Calendar".into(),
        }
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self.model.first_weekday = first_weekday(language);
        self
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    pub fn first_weekday(mut self, day: u8) -> Self {
        self.model.first_weekday = day % 7;
        self
    }
    pub fn today(mut self, today: Date) -> Self {
        self.today = today;
        self
    }
    pub fn selection(&self) -> Date {
        self.model.selected
    }
    pub fn model(&self) -> &Calendar {
        &self.model
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        self.model.first_weekday = first_weekday(language);
        cx.notify();
    }

    pub fn select(&mut self, date: Date, cx: &mut Context<Self>) {
        let changed = self.model.selected != date;
        self.model.select(date);
        if changed {
            cx.emit(DateChanged(date));
        }
        cx.notify();
    }

    pub fn change_month(&mut self, delta: i32, cx: &mut Context<Self>) {
        self.model.change_month(delta);
        cx.notify();
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let date = self.model.selected;
        let date = match event.keystroke.key.as_str() {
            "left" => date.add_days(-1),
            "right" => date.add_days(1),
            "up" => date.add_days(-7),
            "down" => date.add_days(7),
            "pageup" => date.add_months(-1),
            "pagedown" => date.add_months(1),
            "home" => date.first_of_month(),
            "end" => Date {
                day: Date::days_in_month(date.year, date.month),
                ..date
            },
            _ => return,
        };
        self.select(date, cx);
        cx.stop_propagation();
    }
}

impl Focusable for CalendarView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for CalendarView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.model.selected;
        let grid = self.model.grid();
        let month = self.model.month;
        let first_weekday = self.model.first_weekday;
        let today = self.today;
        let focused = self.focus.is_focused(window);
        let theme = self.theme;
        let language = self.language;
        let children = grid
            .iter()
            .copied()
            .enumerate()
            .map(|(index, date)| {
                div()
                    .id(index)
                    .role(Role::Button)
                    .aria_label(date.to_string())
                    .aria_selected(date == selected)
                    .absolute()
                    .left(px((GRID_LEFT + index as i32 % 7 * DAY_WIDTH) as f32))
                    .top(px((GRID_TOP + index as i32 / 7 * DAY_HEIGHT) as f32))
                    .w(px(DAY_WIDTH as f32))
                    .h(px(DAY_HEIGHT as f32))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        window.focus(&this.focus, cx);
                        this.select(date, cx);
                    }))
            })
            .collect::<Vec<_>>();
        let (previous_label, next_label) = month_button_labels(language);
        div()
            .id("platinum-calendar")
            .relative()
            .role(Role::Group)
            .aria_label(self.label.clone())
            .aria_value(selected.to_string())
            .track_focus(&self.focus)
            .tab_stop(true)
            .w(px(CALENDAR_WIDTH as f32))
            .h(px(CALENDAR_HEIGHT as f32))
            .flex_shrink_0()
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    window.focus(&this.focus, cx);
                    cx.notify();
                }),
            )
            .child(drawing(
                CALENDAR_WIDTH as f32,
                CALENDAR_HEIGHT as f32,
                move |pen, _, _| {
                    pen.language = language;
                    pen.rect(0, 0, pen.width, pen.height, WHITE);
                    pen.bevel(0, 0, pen.width, pen.height, GRAY_7, WHITE);
                    pen.frame(1, 1, pen.width - 2, pen.height - 2, BLACK);
                    if focused {
                        pen.frame(0, 0, pen.width, pen.height, theme.accent.dark);
                    }
                    pen.rect(2, 2, pen.width - 4, 20, GRAY_2);
                    pen.bevel(2, 2, pen.width - 4, 20, WHITE, GRAY_7);
                    pen.hline(2, 22, pen.width - 4, BLACK);
                    for col in 0..4 {
                        pen.vline(8 + col, 11 - col, 1 + col * 2, BLACK);
                        pen.vline(pen.width - 9 - col, 11 - col, 1 + col * 2, BLACK);
                    }
                    let title = month_title(month, language);
                    pen.text(
                        (pen.width - Font::Bold.measure_in(&title, language)) / 2,
                        16,
                        &title,
                        Font::Bold,
                        BLACK,
                    );
                    let weekdays = weekday_labels(language);
                    for column in 0..7 {
                        let name = weekdays[(column + usize::from(first_weekday)) % 7];
                        let x = GRID_LEFT
                            + column as i32 * DAY_WIDTH
                            + (DAY_WIDTH - Font::Small.measure_in(name, language)) / 2;
                        pen.text(x, 33, name, Font::Small, GRAY_8);
                    }
                    pen.hline(GRID_LEFT, GRID_TOP - 1, DAY_WIDTH * 7, GRAY_4);
                    for (index, date) in grid.iter().enumerate() {
                        let x = GRID_LEFT + index as i32 % 7 * DAY_WIDTH;
                        let y = GRID_TOP + index as i32 / 7 * DAY_HEIGHT;
                        let mut ink = if date.month == month.month {
                            BLACK
                        } else {
                            GRAY_6
                        };
                        if *date == selected {
                            pen.rect(
                                x + 1,
                                y,
                                DAY_WIDTH - 2,
                                DAY_HEIGHT,
                                if focused { theme.accent.dark } else { GRAY_4 },
                            );
                            if focused {
                                ink = WHITE;
                            }
                        }
                        if *date == today {
                            pen.frame(x + 1, y, DAY_WIDTH - 2, DAY_HEIGHT, BLACK);
                        }
                        let text = date.day.to_string();
                        pen.text(
                            x + (DAY_WIDTH - Font::Small.measure(&text)) / 2,
                            y + 12,
                            &text,
                            Font::Small,
                            ink,
                        );
                    }
                },
            ))
            .child(
                div()
                    .id("previous-month")
                    .role(Role::Button)
                    .aria_label(previous_label)
                    .absolute()
                    .left(px(2.))
                    .top(px(2.))
                    .w(px(22.))
                    .h(px(20.))
                    .on_click(cx.listener(|this, _, window, cx| {
                        window.focus(&this.focus, cx);
                        this.change_month(-1, cx);
                    })),
            )
            .child(
                div()
                    .id("next-month")
                    .role(Role::Button)
                    .aria_label(next_label)
                    .absolute()
                    .right(px(2.))
                    .top(px(2.))
                    .w(px(22.))
                    .h(px(20.))
                    .on_click(cx.listener(|this, _, window, cx| {
                        window.focus(&this.focus, cx);
                        this.change_month(1, cx);
                    })),
            )
            .children(children)
    }
}

fn first_weekday(language: Language) -> u8 {
    if language == Language::SimplifiedChinese {
        1
    } else {
        0
    }
}

fn weekday_labels(language: Language) -> [&'static str; 7] {
    match language {
        Language::English => ["S", "M", "T", "W", "T", "F", "S"],
        Language::Korean => ["일", "월", "화", "수", "목", "금", "토"],
        Language::Japanese => ["日", "月", "火", "水", "木", "金", "土"],
        _ => ["日", "一", "二", "三", "四", "五", "六"],
    }
}

fn month_title(date: Date, language: Language) -> String {
    match language {
        Language::English => {
            let months = [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ];
            format!("{} {}", months[usize::from(date.month - 1)], date.year)
        }
        Language::Korean => format!("{}년 {}월", date.year, date.month),
        _ => format!("{}年{}月", date.year, date.month),
    }
}

fn month_button_labels(language: Language) -> (&'static str, &'static str) {
    match language {
        Language::English => ("Previous month", "Next month"),
        Language::SimplifiedChinese => ("上个月", "下个月"),
        Language::TraditionalChinese => ("上個月", "下個月"),
        Language::Japanese => ("前の月", "次の月"),
        Language::Korean => ("이전 달", "다음 달"),
    }
}

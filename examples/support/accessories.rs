use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Timelike};
use gpui::{
    App, ClipboardItem, Context, Div, Entity, EventEmitter, FocusHandle, Focusable, KeyDownEvent,
    MouseButton, Role, Subscription, Task, Window, anchored, deferred, div, point, prelude::*, px,
    rgb,
};
use gpui_platinum::{
    CalendarView, CheckState, ChoiceGroup, DateChanged, Font, Language, ListBox, ListEvent,
    MenuEvent, MenuItem, PopupMenu, Slider, Theme, Time, TimeChanged, TimeField, button, checkbox,
    label_in,
    locale::{LANGUAGE_NAMES, LANGUAGES, desk},
    models::{Calculator, Date},
    paint::{Pen, drawing},
    separator, text_in_language,
    theme::*,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppKind {
    Calculator,
    Puzzle,
    AlarmClock,
    Appearance,
    DateTime,
    Sound,
    Mouse,
    Language,
}

#[derive(Clone, Copy, Debug)]
pub struct ThemeChanged(pub Theme);
#[derive(Clone, Copy, Debug)]
pub struct LanguageChanged(pub Language);
#[derive(Clone, Copy, Debug)]
pub struct PatternChanged(pub usize);
/// Seconds added to the computer's local clock within this desktop.
#[derive(Clone, Copy, Debug)]
pub struct ClockChanged(pub i64);

#[derive(Clone, Copy)]
pub struct Pattern {
    pub key: &'static str,
    pub bits: [u8; 8],
    pub foreground: u32,
    pub background: u32,
}

pub const PATTERNS: [Pattern; 5] = [
    Pattern {
        key: "patternLavenderWeave",
        bits: [0x88, 0x54, 0x22, 0x45, 0x88, 0x15, 0x22, 0x51],
        foreground: 0x7777aa,
        background: 0x9999cc,
    },
    Pattern {
        key: "patternGrayTweed",
        bits: [0x80, 0x10, 0x02, 0x20, 0x01, 0x08, 0x40, 0x04],
        foreground: 0x888888,
        background: 0xaaaaaa,
    },
    Pattern {
        key: "patternTealBricks",
        bits: [0xff, 0x80, 0x80, 0x80, 0xff, 0x08, 0x08, 0x08],
        foreground: 0x337777,
        background: 0x559999,
    },
    Pattern {
        key: "patternRoseWaves",
        bits: [0x00, 0x18, 0x24, 0xc3, 0x00, 0x18, 0x24, 0xc3],
        foreground: 0xaa6677,
        background: 0xcc99aa,
    },
    Pattern {
        key: "patternPlainSlate",
        bits: [0; 8],
        foreground: 0,
        background: 0x667788,
    },
];

pub struct Accessories {
    kind: AppKind,
    focus: FocusHandle,
    theme: Theme,
    language: Language,
    calculator: Calculator,
    puzzle: Puzzle,
    appearance_tab: usize,
    pattern: usize,
    patterns: Entity<ListBox>,
    clock_offset: i64,
    calendar: Entity<CalendarView>,
    time: Entity<TimeField>,
    alarm_hour: Entity<PopupMenu>,
    alarm_minute: Entity<PopupMenu>,
    alarm_armed: bool,
    alarm_ringing: bool,
    alarm_presented: bool,
    alarm_focus: FocusHandle,
    previous_focus: Option<FocusHandle>,
    volume: f64,
    muted: bool,
    sounds: Entity<ListBox>,
    tracking: f64,
    double_click: usize,
    language_choice: usize,
    _subscriptions: Vec<Subscription>,
    _clock_task: Option<Task<()>>,
}

impl EventEmitter<ThemeChanged> for Accessories {}
impl EventEmitter<LanguageChanged> for Accessories {}
impl EventEmitter<PatternChanged> for Accessories {}
impl EventEmitter<ClockChanged> for Accessories {}
impl Focusable for Accessories {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Accessories {
    pub fn new(kind: AppKind, cx: &mut Context<Self>) -> Self {
        let now = Local::now();
        let calendar = cx.new(|cx| CalendarView::new(date_part(now), cx));
        let time = cx
            .new(|cx| TimeField::new(now.hour() as u8, now.minute() as u8, now.second() as u8, cx));
        let alarm_hour = cx.new(|cx| {
            PopupMenu::new((0..24).map(|hour| MenuItem::new(format!("{hour:02}"))), cx)
                .selected(now.hour() as usize)
                .width(42)
        });
        let alarm_minute = cx.new(|cx| {
            PopupMenu::new(
                (0..12).map(|minute| MenuItem::new(format!("{:02}", minute * 5))),
                cx,
            )
            .selected(((now.minute() / 5 + 1) % 12) as usize)
            .width(42)
        });
        let patterns = cx.new(|cx| {
            ListBox::new(
                PATTERNS.map(|pattern| desk(Language::English, pattern.key)),
                cx,
            )
            .size(180, 120)
            .selected(0)
        });
        let sounds = cx.new(|cx| {
            ListBox::new(sound_names(Language::English), cx)
                .size(250, 80)
                .selected(0)
        });
        let subscriptions = vec![
            cx.subscribe(&calendar, |this, _, event: &DateChanged, cx| {
                if event.0 != date_part(this.now()) {
                    this.choose_clock(event.0, time_part(this.now()), cx);
                }
            }),
            cx.subscribe(&time, |this, _, event: &TimeChanged, cx| {
                this.choose_clock(date_part(this.now()), event.0, cx)
            }),
            cx.subscribe(&patterns, |this, _, event, cx| {
                if let ListEvent::Selected(index) = event {
                    this.pattern = *index;
                    cx.emit(PatternChanged(*index));
                    cx.notify();
                }
            }),
            cx.subscribe(&alarm_hour, |_, _, _: &MenuEvent, cx| cx.notify()),
            cx.subscribe(&alarm_minute, |_, _, _: &MenuEvent, cx| cx.notify()),
        ];
        let clock_task = matches!(kind, AppKind::AlarmClock | AppKind::DateTime).then(|| {
            cx.spawn(async |panel, cx| {
                loop {
                    cx.background_executor().timer(Duration::from_secs(1)).await;
                    if panel.update(cx, |panel, cx| panel.tick(cx)).is_err() {
                        break;
                    }
                }
            })
        });
        Self {
            kind,
            focus: cx.focus_handle().tab_stop(true),
            theme: Theme::default(),
            language: Language::English,
            calculator: Calculator::new(),
            puzzle: Puzzle::shuffled(1),
            appearance_tab: 0,
            pattern: 0,
            patterns,
            clock_offset: 0,
            calendar,
            time,
            alarm_hour,
            alarm_minute,
            alarm_armed: false,
            alarm_ringing: false,
            alarm_presented: false,
            alarm_focus: cx.focus_handle().tab_stop(true),
            previous_focus: None,
            volume: 5.,
            muted: false,
            sounds,
            tracking: 3.,
            double_click: 1,
            language_choice: 1,
            _subscriptions: subscriptions,
            _clock_task: clock_task,
        }
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        self.calendar
            .update(cx, |calendar, cx| calendar.set_theme(theme, cx));
        self.time.update(cx, |time, cx| time.set_theme(theme, cx));
        for list in [&self.patterns, &self.sounds] {
            list.update(cx, |list, cx| list.set_theme(theme, cx));
        }
        for menu in [&self.alarm_hour, &self.alarm_minute] {
            menu.update(cx, |menu, cx| menu.set_theme(theme, cx));
        }
        cx.notify();
    }

    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        if self.language_choice != 0 {
            self.language_choice = language.index() + 1;
        }
        self.calendar
            .update(cx, |calendar, cx| calendar.set_language(language, cx));
        self.time
            .update(cx, |time, cx| time.set_language(language, cx));
        self.patterns.update(cx, |list, cx| {
            list.set_items(
                PATTERNS
                    .map(|pattern| desk(language, pattern.key).into())
                    .to_vec(),
                cx,
            );
            list.set_label(desk(language, "desktopTab"), cx);
        });
        self.sounds.update(cx, |list, cx| {
            list.set_items(
                sound_names(language).into_iter().map(Into::into).collect(),
                cx,
            );
            list.set_label(desk(language, "alertSound"), cx);
        });
        cx.notify();
    }

    pub fn set_pattern(&mut self, pattern: usize, cx: &mut Context<Self>) {
        self.pattern = pattern.min(PATTERNS.len() - 1);
        self.patterns
            .update(cx, |list, cx| list.set_selection(Some(self.pattern), cx));
        cx.notify();
    }

    pub fn set_clock_offset(&mut self, seconds: i64, cx: &mut Context<Self>) {
        self.clock_offset = seconds;
        self.sync_clock_fields(cx);
        cx.notify();
    }

    pub fn command(&mut self, command: &str, cx: &mut Context<Self>) {
        match (self.kind, command) {
            (AppKind::Puzzle, "shuffle") => {
                self.puzzle = Puzzle::shuffled(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as u64,
                )
            }
            (AppKind::Puzzle, "solve") => self.puzzle = Puzzle::default(),
            (AppKind::Calculator, "copy") => {
                cx.write_to_clipboard(ClipboardItem::new_string(self.calculator.display().into()))
            }
            (AppKind::Calculator, "paste") => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.calculator.paste(&text);
                }
            }
            (AppKind::Calculator, "clear") => self.calculator.press("C"),
            _ => return,
        }
        cx.notify();
    }

    fn now(&self) -> DateTime<Local> {
        Local::now() + chrono::Duration::seconds(self.clock_offset)
    }
    fn choose_clock(&mut self, date: Date, time: Time, cx: &mut Context<Self>) {
        let Some(naive) = NaiveDate::from_ymd_opt(date.year, date.month as u32, date.day as u32)
            .and_then(|date| {
                date.and_hms_opt(time.hour as u32, time.minute as u32, time.second as u32)
            })
        else {
            return;
        };
        let Some(chosen) = Local.from_local_datetime(&naive).earliest() else {
            return;
        };
        self.clock_offset = chosen.timestamp() - Local::now().timestamp();
        cx.emit(ClockChanged(self.clock_offset));
        cx.notify();
    }
    fn sync_clock_fields(&mut self, cx: &mut Context<Self>) {
        let now = self.now();
        let date = date_part(now);
        self.calendar.update(cx, |calendar, cx| {
            if calendar.selection() != date {
                calendar.select(date, cx);
            }
        });
        self.time
            .update(cx, |time, cx| time.set_value(time_part(now), cx));
    }
    fn tick(&mut self, cx: &mut Context<Self>) {
        if self.kind == AppKind::DateTime {
            self.sync_clock_fields(cx);
        }
        let now = self.now();
        if self.alarm_armed
            && now.hour() as usize == self.alarm_hour.read(cx).selection()
            && now.minute() as usize == self.alarm_minute.read(cx).selection() * 5
        {
            self.alarm_armed = false;
            self.alarm_ringing = true;
        }
        cx.notify();
    }

    fn dismiss_alarm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.alarm_ringing = false;
        self.alarm_presented = false;
        if let Some(focus) = self.previous_focus.take() {
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    fn calculator(&self, cx: &mut Context<Self>) -> Div {
        let language = self.language;
        let shown = if self.calculator.display() == "Error" {
            desk(language, "error")
        } else {
            self.calculator.display().to_owned()
        };
        let mut keys = div().flex().flex_col().gap(px(0.));
        for (row, labels) in [
            ["C", "←", "÷", "×"],
            ["7", "8", "9", "−"],
            ["4", "5", "6", "+"],
        ]
        .into_iter()
        .enumerate()
        {
            keys =
                keys.child(div().flex().children(labels.into_iter().enumerate().map(
                    |(column, key)| self.calc_key((row * 4 + column) as u32, key, 28, 20, cx),
                )));
        }
        keys = keys.child(
            div()
                .flex()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(div().flex().children(
                            ["1", "2", "3"].into_iter().enumerate().map(|(index, key)| {
                                self.calc_key(12 + index as u32, key, 28, 20, cx)
                            }),
                        ))
                        .child(
                            div()
                                .flex()
                                .child(self.calc_key(15, "0", 62, 20, cx))
                                .child(self.calc_key(16, ".", 28, 20, cx)),
                        ),
                )
                .child(self.calc_key(17, "=", 28, 46, cx)),
        );
        div()
            .flex()
            .flex_col()
            .gap(px(5.))
            .child(
                div()
                    .id("calculator-display")
                    .role(Role::Status)
                    .aria_label(desk(language, "display"))
                    .aria_value(shown.clone())
                    .child(drawing(136., 24., move |pen, _, _| {
                        pen.bevel(0, 0, 136, 24, GRAY_7, WHITE);
                        pen.frame(1, 1, 134, 22, BLACK);
                        pen.rect(2, 2, 132, 20, 0xdde8cc);
                        pen.language = language;
                        pen.text(
                            130 - Font::Bold.measure_in(&shown, language),
                            17,
                            &shown,
                            Font::Bold,
                            BLACK,
                        );
                    })),
            )
            .child(keys)
    }

    fn calc_key(
        &self,
        id: u32,
        key: &'static str,
        width: i32,
        height: i32,
        cx: &mut Context<Self>,
    ) -> gpui_platinum::Control {
        button(("calc", id), key)
            .width(width)
            .height(height)
            .theme(self.theme)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.calculator.press(key);
                cx.notify();
            }))
    }

    fn puzzle(&self, cx: &mut Context<Self>) -> Div {
        let tiles = self.puzzle.tiles;
        let theme = self.theme;
        let language = self.language;
        let mut board = div()
            .id("puzzle-board")
            .role(Role::Group)
            .aria_label(desk(language, "tiles"))
            .relative()
            .w(px(148.))
            .h(px(148.))
            .child(drawing(148., 148., move |pen, _, _| {
                pen.sunken();
                pen.rect(2, 2, 144, 144, GRAY_5);
                for (index, value) in tiles.into_iter().enumerate() {
                    if value == 0 {
                        continue;
                    }
                    let x = 2 + index as i32 % 4 * 36;
                    let y = 2 + index as i32 / 4 * 36;
                    pen.rect(
                        x,
                        y,
                        36,
                        36,
                        if usize::from(value) == index + 1 {
                            theme.accent.light
                        } else {
                            GRAY_2
                        },
                    );
                    pen.frame(x, y, 36, 36, BLACK);
                    pen.bevel(x + 1, y + 1, 34, 34, WHITE, GRAY_7);
                    let label = value.to_string();
                    pen.text(
                        x + (36 - Font::Bold.measure(&label)) / 2,
                        y + 23,
                        &label,
                        Font::Bold,
                        BLACK,
                    );
                }
            }));
        for (index, tile) in tiles.iter().enumerate() {
            board = board.child(
                div()
                    .id(index)
                    .role(Role::Button)
                    .aria_label(tile.to_string())
                    .absolute()
                    .left(px((2 + index % 4 * 36) as f32))
                    .top(px((2 + index / 4 * 36) as f32))
                    .w(px(36.))
                    .h(px(36.))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        window.focus(&this.focus, cx);
                        if this.puzzle.slide(index) {
                            cx.notify();
                        }
                    })),
            );
        }
        let status = if self.puzzle.solved() {
            desk(language, "solved")
        } else {
            moves_label(self.puzzle.moves, language)
        };
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(6.))
            .child(board)
            .child(text_in_language(status, Font::Bold, BLACK, language))
    }

    fn clock(&self, cx: &mut Context<Self>) -> Div {
        let now = self.now();
        let time = now.format("%H:%M:%S").to_string();
        let language = self.language;
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .w(px(216.))
            .child(
                div()
                    .id("clock-display")
                    .role(Role::Status)
                    .aria_label(desk(language, "time"))
                    .aria_value(time.clone())
                    .child(drawing(216., 40., move |pen, _, _| {
                        pen.sunken();
                        pen.rect(2, 2, 212, 36, GRAY_1);
                        scaled_time(pen, &time);
                    })),
            )
            .child(div().flex().justify_center().child(text_in_language(
                date_label(now, language),
                Font::Small,
                BLACK,
                language,
            )))
            .child(separator(216))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        checkbox(
                            "armed",
                            desk(language, "alarmAt"),
                            checked(self.alarm_armed),
                        )
                        .language(language)
                        .theme(self.theme)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.alarm_armed = !this.alarm_armed;
                            cx.notify();
                        })),
                    )
                    .child(self.alarm_hour.clone())
                    .child(text_in_language(":", Font::Bold, BLACK, language))
                    .child(self.alarm_minute.clone()),
            )
    }

    fn appearance(&self, cx: &mut Context<Self>) -> Div {
        let language = self.language;
        let weak = cx.weak_entity();
        let tabs = ChoiceGroup::tabs(
            "appearance-tabs",
            self.appearance_tab,
            [desk(language, "colors"), desk(language, "desktopTab")],
        )
        .language(language)
        .theme(self.theme)
        .on_change(move |tab, _, cx| {
            weak.update(cx, |this, cx| {
                this.appearance_tab = tab;
                cx.notify();
            })
            .ok();
        });
        let mut body = div().flex().flex_col().gap(px(10.)).w(px(280.)).child(tabs);
        if self.appearance_tab == 0 {
            let weak = cx.weak_entity();
            let selected = ACCENTS
                .iter()
                .position(|accent| *accent == self.theme.accent)
                .unwrap_or(0);
            let names = [
                "accentLavender",
                "accentGraphite",
                "accentHarbor",
                "accentMoss",
                "accentEmber",
                "accentOrchid",
            ]
            .map(|key| desk(language, key));
            let swatches = div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .children(ACCENTS.into_iter().map(|accent| {
                    drawing(24., 18., move |pen, _, _| {
                        pen.frame(0, 3, 24, 12, BLACK);
                        pen.rect(1, 4, 11, 10, accent.dark);
                        pen.rect(12, 4, 11, 10, accent.light);
                    })
                }));
            body = body
                .child(label_in(desk(language, "highlightColor"), language))
                .child(
                    div().flex().gap(px(8.)).child(swatches).child(
                        ChoiceGroup::radio("accent", selected, names)
                            .language(language)
                            .theme(self.theme)
                            .on_change(move |index, _, cx| {
                                weak.update(cx, |this, cx| {
                                    this.theme.accent = ACCENTS[index];
                                    cx.emit(ThemeChanged(this.theme));
                                    cx.notify();
                                })
                                .ok();
                            }),
                    ),
                );
        } else {
            let pattern = PATTERNS[self.pattern];
            body = body.child(
                div()
                    .flex()
                    .gap(px(10.))
                    .child(self.patterns.clone())
                    .child(
                        div()
                            .id("pattern-preview")
                            .role(Role::Image)
                            .aria_label(desk(language, "preview"))
                            .child(drawing(64., 64., move |pen, _, _| {
                                pen.frame(0, 0, 64, 64, BLACK);
                                for y in 1..63 {
                                    for x in 1..63 {
                                        pen.dot(
                                            x,
                                            y,
                                            if pattern.bits[(y - 1) as usize % 8]
                                                & (1 << (7 - (x - 1) % 8))
                                                != 0
                                            {
                                                pattern.foreground
                                            } else {
                                                pattern.background
                                            },
                                        );
                                    }
                                }
                            })),
                    ),
            );
        }
        body
    }

    fn date_time(&self, cx: &mut Context<Self>) -> Div {
        let language = self.language;
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .w(px(370.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(12.))
                    .p(px(10.))
                    .border_1()
                    .border_color(rgb(GRAY_7))
                    .child(self.calendar.clone())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(label_in(desk(language, "timeLabel"), language))
                            .child(self.time.clone())
                            .child(localized_paragraph(
                                date_label(self.now(), language),
                                155,
                                language,
                            )),
                    ),
            )
            .child(
                div().flex().justify_end().child(
                    button("computer-clock", desk(language, "useComputersClock"))
                        .language(language)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.set_clock_offset(0, cx);
                            cx.emit(ClockChanged(0));
                        })),
                ),
            )
    }

    fn sound(&self, cx: &mut Context<Self>) -> Div {
        let language = self.language;
        let weak = cx.weak_entity();
        div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(label_in(desk(language, "volumeLabel"), language))
                            .child(
                                Slider::new("volume", self.volume, 0., 7.)
                                    .width(150)
                                    .ticks(8)
                                    .step(1.)
                                    .disabled(self.muted)
                                    .label(desk(language, "volume"))
                                    .theme(self.theme)
                                    .on_change(move |volume, _, cx| {
                                        weak.update(cx, |this, cx| {
                                            this.volume = volume;
                                            cx.notify();
                                        })
                                        .ok();
                                    }),
                            ),
                    )
                    .child(
                        checkbox("mute", desk(language, "mute"), checked(self.muted))
                            .language(language)
                            .theme(self.theme)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.muted = !this.muted;
                                cx.notify();
                            })),
                    ),
            )
            .child(section(desk(language, "alertSound"), 280, language).child(self.sounds.clone()))
    }

    fn mouse(&self, cx: &mut Context<Self>) -> Div {
        let language = self.language;
        let weak = cx.weak_entity();
        let weak_radio = cx.weak_entity();
        div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                section(desk(language, "trackingSpeed"), 280, language)
                    .child(
                        Slider::new("tracking", self.tracking, 0., 6.)
                            .width(240)
                            .ticks(7)
                            .step(1.)
                            .theme(self.theme)
                            .label(desk(language, "trackingSpeed"))
                            .on_change(move |tracking, _, cx| {
                                weak.update(cx, |this, cx| {
                                    this.tracking = tracking;
                                    cx.notify();
                                })
                                .ok();
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(text_in_language(
                                desk(language, "slow"),
                                Font::Small,
                                BLACK,
                                language,
                            ))
                            .child(text_in_language(
                                desk(language, "fast"),
                                Font::Small,
                                BLACK,
                                language,
                            )),
                    ),
            )
            .child(
                section(desk(language, "doubleClickSpeed"), 280, language).child(
                    ChoiceGroup::radio(
                        "double-click",
                        self.double_click,
                        [
                            desk(language, "slow"),
                            desk(language, "medium"),
                            desk(language, "fast"),
                        ],
                    )
                    .vertical(false)
                    .theme(self.theme)
                    .language(language)
                    .on_change(move |index, _, cx| {
                        weak_radio
                            .update(cx, |this, cx| {
                                this.double_click = index;
                                cx.notify();
                            })
                            .ok();
                    }),
                ),
            )
    }

    fn language(&self, cx: &mut Context<Self>) -> Div {
        let language = self.language;
        let names = std::iter::once(desk(language, "languageSystem"))
            .chain(LANGUAGE_NAMES.map(str::to_owned));
        let weak = cx.weak_entity();
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                section(desk(language, "languageLabel"), 280, language).child(
                    ChoiceGroup::radio("language", self.language_choice, names)
                        .language(language)
                        .theme(self.theme)
                        .on_change(move |index, _, cx| {
                            weak.update(cx, |this, cx| {
                                this.language_choice = index;
                                let language = if index == 0 {
                                    system_language()
                                } else {
                                    LANGUAGES[index - 1]
                                };
                                this.set_language(language, cx);
                                cx.emit(LanguageChanged(language));
                            })
                            .ok();
                        }),
                ),
            )
            .child(localized_paragraph(
                desk(language, "languageNote"),
                270,
                language,
            ))
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let modifiers = &event.keystroke.modifiers;
        if modifiers.platform || modifiers.control || modifiers.alt {
            return;
        }
        match self.kind {
            AppKind::Calculator => {
                let key = event
                    .keystroke
                    .key_char
                    .as_deref()
                    .unwrap_or(&event.keystroke.key);
                let key = match event.keystroke.key.as_str() {
                    "enter" => "=",
                    "escape" => "C",
                    "backspace" => "←",
                    _ => key,
                };
                if key.len() == 1 && key.as_bytes()[0].is_ascii_digit()
                    || [".", "+", "-", "*", "/", "=", "C", "←"].contains(&key)
                {
                    self.calculator.press(key);
                    cx.stop_propagation();
                    cx.notify();
                }
            }
            AppKind::Puzzle => {
                let (dx, dy) = match event.keystroke.key.as_str() {
                    "left" => (1, 0),
                    "right" => (-1, 0),
                    "up" => (0, 1),
                    "down" => (0, -1),
                    _ => return,
                };
                let hole = self.puzzle.hole() as i32;
                let (x, y) = (hole % 4 + dx, hole / 4 + dy);
                if (0..4).contains(&x) && (0..4).contains(&y) {
                    self.puzzle.slide((y * 4 + x) as usize);
                    cx.notify();
                }
                cx.stop_propagation();
            }
            _ => {}
        }
    }
}

impl Render for Accessories {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.kind {
            AppKind::Calculator => self.calculator(cx),
            AppKind::Puzzle => self.puzzle(cx),
            AppKind::AlarmClock => self.clock(cx),
            AppKind::Appearance => self.appearance(cx),
            AppKind::DateTime => self.date_time(cx),
            AppKind::Sound => self.sound(cx),
            AppKind::Mouse => self.mouse(cx),
            AppKind::Language => self.language(cx),
        };
        let mut root = div()
            .id("accessory")
            .track_focus(&self.focus)
            .p(px(10.))
            .flex()
            .flex_col()
            .items_start()
            .bg(rgb(GRAY_2))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    if matches!(this.kind, AppKind::Calculator | AppKind::Puzzle) {
                        window.focus(&this.focus, cx);
                    }
                }),
            )
            .on_key_down(cx.listener(Self::key_down))
            .child(body);
        if self.alarm_ringing {
            if !self.alarm_presented {
                self.previous_focus = window.focused(cx);
                window.focus(&self.alarm_focus, cx);
                self.alarm_presented = true;
            }
            let language = self.language;
            let viewport = window.viewport_size();
            let dialog = div()
                .id("alarm-alert")
                .role(Role::AlertDialog)
                .aria_label(desk(language, "itIsTime"))
                .w(px(300.))
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .border_2()
                .border_color(rgb(BLACK))
                .bg(rgb(GRAY_2))
                .occlude()
                .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "escape" | "enter" => this.dismiss_alarm(window, cx),
                        "tab" => window.focus(&this.alarm_focus, cx),
                        _ => return,
                    }
                    cx.stop_propagation();
                }))
                .child(label_in(desk(language, "itIsTime"), language))
                .child(localized_paragraph(
                    desk(language, "alarmGoneOff"),
                    264,
                    language,
                ))
                .child(
                    div().flex().justify_end().child(
                        button("dismiss-alarm", desk(language, "ok"))
                            .language(language)
                            .default_button()
                            .focus_handle(&self.alarm_focus)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.dismiss_alarm(window, cx)),
                            ),
                    ),
                );
            let overlay = div()
                .id("alarm-backdrop")
                .relative()
                .w(viewport.width)
                .h(viewport.height)
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .absolute()
                        .left(px(((f32::from(viewport.width) - 300.) / 2.).max(0.)))
                        .top(px(((f32::from(viewport.height) - 150.) / 2.).max(0.)))
                        .child(dialog),
                );
            root = root.child(
                deferred(anchored().position(point(px(0.), px(0.))).child(overlay)).priority(40),
            );
        }
        root
    }
}

#[derive(Clone, Debug)]
struct Puzzle {
    tiles: [u8; 16],
    moves: usize,
}
impl Default for Puzzle {
    fn default() -> Self {
        Self {
            tiles: std::array::from_fn(|index| ((index + 1) % 16) as u8),
            moves: 0,
        }
    }
}
impl Puzzle {
    fn hole(&self) -> usize {
        self.tiles
            .iter()
            .position(|tile| *tile == 0)
            .expect("one empty tile")
    }
    fn slide(&mut self, index: usize) -> bool {
        let hole = self.hole();
        if index >= 16 || (index % 4).abs_diff(hole % 4) + (index / 4).abs_diff(hole / 4) != 1 {
            return false;
        }
        self.tiles.swap(hole, index);
        self.moves += 1;
        true
    }
    fn solved(&self) -> bool {
        self.tiles
            .iter()
            .enumerate()
            .all(|(index, tile)| usize::from(*tile) == (index + 1) % 16)
    }
    fn shuffled(mut seed: u64) -> Self {
        let mut puzzle = Self::default();
        for _ in 0..200 {
            let hole = puzzle.hole();
            let adjacent: Vec<_> = (0usize..16)
                .filter(|index| {
                    (index % 4).abs_diff(hole % 4) + (index / 4).abs_diff(hole / 4) == 1
                })
                .collect();
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            puzzle.slide(adjacent[(seed >> 32) as usize % adjacent.len()]);
        }
        puzzle.moves = 0;
        puzzle
    }
}

fn checked(value: bool) -> CheckState {
    if value {
        CheckState::On
    } else {
        CheckState::Off
    }
}
fn date_part(now: DateTime<Local>) -> Date {
    Date::new(now.year(), now.month() as u8, now.day() as u8).expect("local calendar date")
}
fn time_part(now: DateTime<Local>) -> Time {
    Time::new(now.hour() as u8, now.minute() as u8, now.second() as u8)
}
fn date_label(now: DateTime<Local>, language: Language) -> String {
    match language {
        Language::English => now.format("%A, %B %-d, %Y").to_string(),
        Language::Korean => format!("{}년 {}월 {}일", now.year(), now.month(), now.day()),
        _ => format!("{}年{}月{}日", now.year(), now.month(), now.day()),
    }
}
fn moves_label(count: usize, language: Language) -> String {
    let message = desk(language, "moves");
    if let Some(start) = message.find(if count == 1 { "one {" } else { "other {" }) {
        let part = &message[start..];
        let part = part.split_once('{').map(|(_, part)| part).unwrap_or(part);
        return part
            .split('}')
            .next()
            .unwrap_or(part)
            .replace('#', &count.to_string());
    }
    message.replace("{count}", &count.to_string())
}
fn sound_names(language: Language) -> Vec<String> {
    [
        "soundChime",
        "soundClick",
        "soundDroplet",
        "soundGlass",
        "soundWhistle",
    ]
    .map(|key| desk(language, key))
    .to_vec()
}
#[track_caller]
fn section(title: String, width: i32, language: Language) -> Div {
    div()
        .relative()
        .w(px(width as f32))
        .flex()
        .flex_col()
        .gap(px(6.))
        .p(px(12.))
        .pt(px(18.))
        .border_1()
        .border_color(rgb(GRAY_7))
        .child(
            div()
                .absolute()
                .left(px(8.))
                .top(px(-9.))
                .px(px(4.))
                .bg(rgb(GRAY_2))
                .child(label_in(title, language)),
        )
}
fn localized_paragraph(text: String, width: i32, language: Language) -> Div {
    div().flex().flex_col().w(px(width as f32)).children(
        gpui_platinum::display::wrap(&text, Font::Small, width)
            .into_iter()
            .enumerate()
            .map(move |(index, line)| {
                div().id(("line", index)).child(text_in_language(
                    line,
                    Font::Small,
                    BLACK,
                    language,
                ))
            }),
    )
}
fn scaled_time(pen: &mut Pen, text: &str) {
    let mut x = (pen.width - Font::Bold.measure(text) * 2) / 2;
    for character in text.chars() {
        let glyph = Font::Bold.glyph_or_replacement(character, Language::English);
        glyph.for_each_run(|dx, dy, width| pen.rect(x + dx * 2, 29 + dy * 2, width * 2, 2, BLACK));
        x += glyph.advance * 2;
    }
}
fn system_language() -> Language {
    let locale = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default()
        .to_lowercase();
    if locale.starts_with("ja") {
        Language::Japanese
    } else if locale.starts_with("ko") {
        Language::Korean
    } else if locale.starts_with("zh") {
        if locale.contains("hant") || locale.contains("tw") || locale.contains("hk") {
            Language::TraditionalChinese
        } else {
            Language::SimplifiedChinese
        }
    } else {
        Language::English
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn puzzle_only_slides_a_neighbor_and_counts_actual_moves() {
        let mut puzzle = Puzzle::default();
        assert!(!puzzle.slide(0));
        assert!(!puzzle.slide(16));
        assert_eq!(puzzle.moves, 0);
        assert!(puzzle.slide(14));
        assert_eq!(puzzle.hole(), 14);
        assert_eq!(puzzle.moves, 1);
        assert!(puzzle.slide(15));
        assert!(puzzle.solved());
    }
    #[test]
    fn shuffled_boards_keep_every_tile_and_are_solvable() {
        for seed in 0..100 {
            let puzzle = Puzzle::shuffled(seed);
            let mut sorted = puzzle.tiles;
            sorted.sort();
            assert_eq!(sorted, std::array::from_fn(|index| index as u8));
            let values: Vec<_> = puzzle
                .tiles
                .into_iter()
                .filter(|value| *value != 0)
                .collect();
            let inversions: usize = values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    values[index + 1..]
                        .iter()
                        .filter(|later| *later < value)
                        .count()
                })
                .sum();
            let row_from_bottom = 4 - puzzle.hole() / 4;
            assert_eq!((inversions + row_from_bottom) % 2, 1);
            assert_eq!(puzzle.moves, 0);
        }
    }
    #[test]
    fn move_status_formats_the_original_plural_forms() {
        assert_eq!(moves_label(1, Language::English), "1 move");
        assert_eq!(moves_label(2, Language::English), "2 moves");
        for language in LANGUAGES {
            let label = moves_label(2, language);
            assert!(!label.contains('{'));
            assert!(label.contains('2'));
        }
    }
}

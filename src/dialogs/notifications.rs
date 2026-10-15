use super::{
    AlertKind,
    alert::icon,
    modal::{draw_frame, text},
};
use crate::{
    Font, Language, Theme, button,
    paint::{Pen, drawing},
    theme::*,
    tr,
};
use gpui::{
    Context, Div, ElementId, EventEmitter, FocusHandle, MouseButton, Render, Role, SharedString,
    Stateful, Window, canvas, div, prelude::*, px, rgb,
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerEvent {
    Dismissed,
    Action,
}
pub struct Banner {
    open: bool,
    kind: AlertKind,
    text: SharedString,
    action: Option<SharedString>,
    focus: [FocusHandle; 2],
    theme: Theme,
    language: Language,
}
impl EventEmitter<BannerEvent> for Banner {}
impl Banner {
    pub fn new(kind: AlertKind, text: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            open: true,
            kind,
            text: text.into(),
            action: None,
            focus: [cx.focus_handle(), cx.focus_handle()],
            theme: Theme::default(),
            language: Language::English,
        }
    }
    pub fn action(mut self, action: impl Into<SharedString>) -> Self {
        self.action = Some(action.into());
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
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.text = text.into();
        cx.notify();
    }
    pub fn show(&mut self, cx: &mut Context<Self>) {
        self.open = true;
        cx.notify();
    }
    pub fn close(&mut self, cx: &mut Context<Self>) {
        if self.open {
            self.open = false;
            cx.emit(BannerEvent::Dismissed);
            cx.notify();
        }
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        cx.notify();
    }
}
impl Render for Banner {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }
        let mut row = div()
            .id("banner")
            .role(Role::Status)
            .relative()
            .h(px(30.))
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(8.))
            .child(frame(self.theme))
            .child(icon(self.kind, true))
            .child(div().flex_1().overflow_hidden().child(text(
                self.text.clone(),
                Font::Small,
                self.language,
            )));
        if let Some(action) = &self.action {
            row = row.child(
                button("action", action.clone())
                    .focus_handle(&self.focus[0])
                    .theme(self.theme)
                    .language(self.language)
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(BannerEvent::Action))),
            );
        }
        row.child(
            close_box("close", &self.focus[1], self.language)
                .on_click(cx.listener(|this, _, _, cx| this.close(cx)))
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "space" | "enter") {
                        this.close(cx);
                        cx.stop_propagation();
                    }
                })),
        )
        .into_any_element()
    }
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub title: SharedString,
    pub description: SharedString,
    pub kind: AlertKind,
    pub action: Option<SharedString>,
    pub timeout: Duration,
}
impl Toast {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: "".into(),
            kind: AlertKind::Note,
            action: None,
            timeout: Duration::from_secs(5),
        }
    }
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }
    pub fn kind(mut self, kind: AlertKind) -> Self {
        self.kind = kind;
        self
    }
    pub fn action(mut self, action: impl Into<SharedString>) -> Self {
        self.action = Some(action.into());
        self
    }
    /// A zero timeout keeps the toast until it is dismissed.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastEvent {
    Action(u64),
    Dismissed(u64),
}
struct ToastEntry {
    id: u64,
    toast: Toast,
    focus: [FocusHandle; 2],
}
pub struct Toasts {
    entries: Vec<ToastEntry>,
    next_id: u64,
    theme: Theme,
    language: Language,
}
impl EventEmitter<ToastEvent> for Toasts {}
impl Toasts {
    pub fn new(_: &mut Context<Self>) -> Self {
        Self {
            entries: Vec::new(),
            next_id: 0,
            theme: Theme::default(),
            language: Language::English,
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
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        cx.notify();
    }
    pub fn push(&mut self, toast: Toast, cx: &mut Context<Self>) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let timeout = toast.timeout;
        self.entries.push(ToastEntry {
            id,
            toast,
            focus: [cx.focus_handle(), cx.focus_handle()],
        });
        if !timeout.is_zero() {
            cx.spawn(async move |toasts, cx| {
                cx.background_executor().timer(timeout).await;
                toasts.update(cx, |toasts, cx| toasts.dismiss(id, cx)).ok();
            })
            .detach();
        }
        cx.notify();
        id
    }
    pub fn dismiss(&mut self, id: u64, cx: &mut Context<Self>) {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        if self.entries.len() != before {
            cx.emit(ToastEvent::Dismissed(id));
            cx.notify();
        }
    }
}
impl Render for Toasts {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("toasts")
            .absolute()
            .bottom(px(12.))
            .left_0()
            .right_0()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(6.))
            .children(self.entries.iter().map(|entry| {
                let id = entry.id;
                let toast = &entry.toast;
                let words = if toast.description.is_empty() {
                    toast.title.to_string()
                } else {
                    format!("{} — {}", toast.title, toast.description)
                };
                let mut row = div()
                    .id(id)
                    .role(Role::Status)
                    .relative()
                    .h(px(30.))
                    .max_w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(8.))
                    .child(frame(self.theme))
                    .child(icon(toast.kind, true))
                    .child(
                        div()
                            .overflow_hidden()
                            .child(text(words, Font::Small, self.language)),
                    );
                if let Some(action) = &toast.action {
                    row = row.child(
                        button(("action", id), action.clone())
                            .focus_handle(&entry.focus[0])
                            .theme(self.theme)
                            .language(self.language)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.emit(ToastEvent::Action(id));
                                this.dismiss(id, cx);
                            })),
                    );
                }
                row.child(
                    close_box(("close", id), &entry.focus[1], self.language)
                        .on_click(cx.listener(move |this, _, _, cx| this.dismiss(id, cx)))
                        .on_key_down(cx.listener(
                            move |this, event: &gpui::KeyDownEvent, _, cx| {
                                if matches!(event.keystroke.key.as_str(), "space" | "enter") {
                                    this.dismiss(id, cx);
                                    cx.stop_propagation();
                                }
                            },
                        )),
                )
            }))
    }
}
fn frame(theme: Theme) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let mut pen = Pen::new(
                f32::from(bounds.size.width).round() as i32,
                f32::from(bounds.size.height).round() as i32,
            );
            draw_frame(&mut pen, theme);
            pen.paint(bounds.origin, window);
        },
    )
    .absolute()
    .inset_0()
    .size_full()
}

fn close_box(id: impl Into<ElementId>, focus: &FocusHandle, language: Language) -> Stateful<Div> {
    let handle = focus.clone();
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(tr(language, "close"))
        .track_focus(focus)
        .tab_stop(true)
        .w(px(12.))
        .h(px(12.))
        .text_color(rgb(BLACK))
        .active(|style| style.text_color(rgb(WHITE)))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            window.focus(&handle, cx)
        })
        .child(drawing(12., 12., |pen, window, _| {
            let pressed = window.text_style().color == rgb(WHITE).into();
            pen.raised(if pressed { GRAY_4 } else { GRAY_2 });
        }))
}

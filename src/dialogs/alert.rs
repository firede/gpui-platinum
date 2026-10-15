use gpui::{
    AnyElement, AnyView, Context, EventEmitter, FocusHandle, Render, SharedString, Window, div,
    prelude::*, px,
};

use super::modal::{ModalState, ModalView, modal, paragraph, text};
use crate::{
    Font, Language, Theme, button,
    paint::{Pen, Rect, drawing},
    theme::*,
    tr,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlertKind {
    #[default]
    Note,
    Caution,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertEvent {
    Chosen(usize),
}

pub struct Alert {
    modal: ModalState,
    kind: AlertKind,
    message: SharedString,
    detail: SharedString,
    buttons: Vec<SharedString>,
    focus: Vec<FocusHandle>,
}

impl EventEmitter<AlertEvent> for Alert {}

impl Alert {
    pub fn new(
        kind: AlertKind,
        message: impl Into<SharedString>,
        detail: impl Into<SharedString>,
        buttons: impl IntoIterator<Item = impl Into<SharedString>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut buttons: Vec<_> = buttons.into_iter().map(Into::into).collect();
        if buttons.is_empty() {
            buttons.push("OK".into());
        }
        let focus = buttons.iter().map(|_| cx.focus_handle()).collect();
        Self {
            modal: ModalState::new(340., cx),
            kind,
            message: message.into(),
            detail: detail.into(),
            buttons,
            focus,
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
    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal.stops = self.focus.clone();
        self.modal.show(self.focus.last(), window, cx);
        cx.notify();
    }
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal.close(window, cx);
        cx.notify();
    }
    pub fn set_message(
        &mut self,
        message: impl Into<SharedString>,
        detail: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.message = message.into();
        self.detail = detail.into();
        cx.notify();
    }
    pub fn set_buttons(&mut self, buttons: Vec<SharedString>, cx: &mut Context<Self>) {
        self.buttons = if buttons.is_empty() {
            vec!["OK".into()]
        } else {
            buttons
        };
        while self.focus.len() < self.buttons.len() {
            self.focus.push(cx.focus_handle());
        }
        self.focus.truncate(self.buttons.len());
        self.modal.stops = self.focus.clone();
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.modal.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.modal.language = language;
        cx.notify();
    }
    fn choose(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.close(window, cx);
        cx.emit(AlertEvent::Chosen(index));
    }
    fn accept(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.choose(self.buttons.len() - 1, window, cx);
    }
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cancel = tr(self.modal.language, "cancel");
        if let Some(index) = self
            .buttons
            .iter()
            .position(|label| label.as_ref() == "Cancel" || label.as_ref() == cancel)
        {
            self.choose(index, window, cx);
        }
    }
}

impl Render for Alert {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.modal.open {
            return div().into_any_element();
        }
        let language = self.modal.language;
        let theme = self.modal.theme;
        let mut words = div().flex().flex_col().gap(px(6.)).child(paragraph(
            self.message.clone(),
            Font::Bold,
            260,
            language,
        ));
        if !self.detail.is_empty() {
            words = words.child(paragraph(self.detail.clone(), Font::Small, 260, language));
        }
        let mut buttons = div().flex().justify_end().items_center().gap(px(10.));
        for (index, label) in self.buttons.iter().enumerate() {
            let mut control = button(("alert-button", index), label.clone())
                .focus_handle(&self.focus[index])
                .theme(theme)
                .language(language)
                .on_click(cx.listener(move |this, _, window, cx| this.choose(index, window, cx)));
            if index == self.buttons.len() - 1 {
                control = control.default_button();
            }
            buttons = buttons.child(control);
        }
        let content = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(16.))
                    .child(icon(self.kind, false))
                    .child(words),
            )
            .child(buttons);
        modal(
            &mut self.modal,
            ModalView::new(self.message.clone(), false, content),
            window,
            cx,
            |this| &mut this.modal,
            Self::accept,
            Self::cancel,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogEvent {
    Accepted,
    Cancelled,
}

/// A movable modal around application-owned content and its focus handles.
pub struct Dialog {
    modal: ModalState,
    title: SharedString,
    content: AnyView,
    content_focus: Vec<FocusHandle>,
    buttons: [FocusHandle; 2],
}

impl EventEmitter<DialogEvent> for Dialog {}
impl Dialog {
    pub fn new(
        title: impl Into<SharedString>,
        content: impl Into<AnyView>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            modal: ModalState::new(360., cx),
            title: title.into(),
            content: content.into(),
            content_focus: Vec::new(),
            buttons: [cx.focus_handle(), cx.focus_handle()],
        }
    }
    pub fn width(mut self, width: f32) -> Self {
        self.modal.width = width.max(180.);
        self
    }
    pub fn set_title(&mut self, title: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.title = title.into();
        cx.notify();
    }
    pub fn focus_handles(mut self, handles: Vec<FocusHandle>) -> Self {
        self.content_focus = handles;
        self
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
    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal.stops = self
            .content_focus
            .iter()
            .chain(&self.buttons)
            .cloned()
            .collect();
        let first = self.content_focus.first().unwrap_or(&self.buttons[1]);
        self.modal.show(Some(first), window, cx);
        cx.notify();
    }
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal.close(window, cx);
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.modal.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.modal.language = language;
        cx.notify();
    }
    fn accept(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close(window, cx);
        cx.emit(DialogEvent::Accepted);
    }
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close(window, cx);
        cx.emit(DialogEvent::Cancelled);
    }
}
impl Render for Dialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.modal.open {
            return div().into_any_element();
        }
        let lang = self.modal.language;
        let theme = self.modal.theme;
        let content = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(self.content.clone())
            .child(
                div()
                    .flex()
                    .justify_end()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        button("cancel", tr(lang, "cancel"))
                            .focus_handle(&self.buttons[0])
                            .theme(theme)
                            .language(lang)
                            .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                    )
                    .child(
                        button("ok", tr(lang, "ok"))
                            .focus_handle(&self.buttons[1])
                            .default_button()
                            .theme(theme)
                            .language(lang)
                            .on_click(cx.listener(|this, _, window, cx| this.accept(window, cx))),
                    ),
            );
        modal(
            &mut self.modal,
            ModalView::new(self.title.clone(), true, content),
            window,
            cx,
            |this| &mut this.modal,
            Self::accept,
            Self::cancel,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressEvent {
    Cancelled,
}

pub struct ProgressDialog {
    modal: ModalState,
    title: SharedString,
    message: SharedString,
    detail: SharedString,
    value: f64,
    cancel_focus: FocusHandle,
}
impl EventEmitter<ProgressEvent> for ProgressDialog {}
impl ProgressDialog {
    pub fn new(
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            modal: ModalState::new(320., cx),
            title: title.into(),
            message: message.into(),
            detail: "".into(),
            value: 0.,
            cancel_focus: cx.focus_handle(),
        }
    }
    pub fn progress(mut self, value: f64) -> Self {
        self.value = progress_value(value);
        self
    }
    pub fn set_message(
        &mut self,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.title = title.into();
        self.message = message.into();
        cx.notify();
    }
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
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
    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal.stops = vec![self.cancel_focus.clone()];
        self.modal.show(Some(&self.cancel_focus), window, cx);
        cx.notify();
    }
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal.close(window, cx);
        cx.notify();
    }
    pub fn set_progress(
        &mut self,
        value: f64,
        detail: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.value = progress_value(value);
        self.detail = detail.into();
        cx.notify();
    }
    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.modal.theme = theme;
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.modal.language = language;
        cx.notify();
    }
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close(window, cx);
        cx.emit(ProgressEvent::Cancelled);
    }
}
impl Render for ProgressDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.modal.open {
            return div().into_any_element();
        }
        let lang = self.modal.language;
        let content = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(text(self.message.clone(), Font::Bold, lang))
            .child(
                div()
                    .id("progress")
                    .role(gpui::Role::ProgressIndicator)
                    .aria_label(self.message.clone())
                    .aria_numeric_value(self.value * 100.)
                    .aria_min_numeric_value(0.)
                    .aria_max_numeric_value(100.)
                    .child(crate::progress_bar(self.value, 290, self.modal.theme)),
            )
            .when(!self.detail.is_empty(), |el| {
                el.child(text(self.detail.clone(), Font::Small, lang))
            })
            .child(
                div().flex().justify_end().child(
                    button("cancel", tr(lang, "cancel"))
                        .default_button()
                        .focus_handle(&self.cancel_focus)
                        .theme(self.modal.theme)
                        .language(lang)
                        .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                ),
            );
        modal(
            &mut self.modal,
            ModalView::new(self.title.clone(), true, content),
            window,
            cx,
            |this| &mut this.modal,
            Self::cancel,
            Self::cancel,
        )
    }
}

fn progress_value(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0., 1.)
    } else {
        0.
    }
}

pub(super) fn icon(kind: AlertKind, small: bool) -> AnyElement {
    let edge = if small { 16 } else { 32 };
    drawing(edge as f32, edge as f32, move |pen, _, _| {
        draw_icon(pen, kind, small)
    })
    .into_any_element()
}

fn draw_icon(pen: &mut Pen, kind: AlertKind, small: bool) {
    if small {
        let mut full = Pen::new(32, 32);
        draw_icon(&mut full, kind, false);
        for fill in full.fills {
            let r = fill.rect;
            if r.y % 2 == 0 {
                pen.rect(r.x / 2, r.y / 2, (r.width + 1) / 2, 1, fill.color);
            }
        }
        return;
    }
    match kind {
        AlertKind::Caution => {
            for y in 2..30 {
                let half = (y - 2) / 2;
                let x = 15 - half;
                pen.hline(x, y, half * 2 + 2, 0xffcc00);
                pen.dot(x, y, BLACK);
                pen.dot(x + half * 2 + 1, y, BLACK);
            }
            pen.hline(2, 30, 28, BLACK);
            pen.rect(15, 10, 2, 12, BLACK);
            pen.rect(14, 11, 4, 8, BLACK);
            pen.rect(15, 25, 2, 2, BLACK);
        }
        AlertKind::Stop => {
            for y in 0..32 {
                let cut = if y < 9 {
                    9 - y
                } else if y > 22 {
                    y - 22
                } else {
                    0
                };
                pen.hline(cut, y, 32 - 2 * cut, BLACK);
                if (1..31).contains(&y) {
                    pen.hline(cut.max(1) + 1, y, 30 - 2 * cut.max(1), WHITE);
                }
                if (3..29).contains(&y) {
                    let inset = (cut + i32::from(cut > 0)).max(3);
                    pen.hline(inset, y, 32 - 2 * inset, 0xcc2222);
                }
            }
            pen.rect(14, 7, 4, 12, WHITE);
            pen.rect(14, 22, 4, 3, WHITE);
        }
        AlertKind::Note => {
            pen.round_frame(
                Rect {
                    x: 1,
                    y: 1,
                    width: 30,
                    height: 22,
                },
                4,
                BLACK,
                Some(WHITE),
            );
            for i in 0..6 {
                pen.hline(7 + i, 22 + i, 6 - i, BLACK);
                pen.hline(8 + i, 22 + i, 4 - i, WHITE);
            }
            pen.rect(14, 5, 4, 3, BLACK);
            pen.rect(13, 10, 5, 2, BLACK);
            pen.rect(14, 12, 4, 6, BLACK);
            pen.rect(12, 17, 8, 2, BLACK);
        }
    }
}

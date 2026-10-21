use super::modal::{ModalState, ModalView, modal, text};
use crate::{
    Font, Language, MenuEvent, NumberChanged, NumberField, PopupMenu, Theme, bevel_button, button,
    paint::{Pen, drawing},
    theme::*,
    tr,
};
use gpui::{
    Context, Entity, EventEmitter, FocusHandle, Focusable, Render, Subscription, Window, div,
    prelude::*, px,
};

const PORTRAIT_ICON: &[&str] = &[
    "######..", "#....##.", "#.....#.", "#.###.#.", "#.....#.", "#.###.#.", "#.....#.", "#######.",
];
const LANDSCAPE_ICON: &[&str] = &[
    "##########",
    "#........#",
    "#.######.#",
    "#........#",
    "#.######.#",
    "##########",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paper {
    pub name: &'static str,
    pub width: f64,
    pub height: f64,
}
pub const PAPERS: [Paper; 5] = [
    Paper {
        name: "US Letter",
        width: 612.,
        height: 792.,
    },
    Paper {
        name: "US Legal",
        width: 612.,
        height: 1008.,
    },
    Paper {
        name: "A4",
        width: 595.,
        height: 842.,
    },
    Paper {
        name: "A5",
        width: 420.,
        height: 595.,
    },
    Paper {
        name: "Envelope #10",
        width: 297.,
        height: 684.,
    },
];
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageSetupState {
    pub paper: usize,
    pub landscape: bool,
    pub scale: f64,
    pub margin: f64,
}
impl Default for PageSetupState {
    fn default() -> Self {
        Self {
            paper: 0,
            landscape: false,
            scale: 100.,
            margin: 36.,
        }
    }
}
impl PageSetupState {
    pub fn normalized(self) -> Self {
        Self {
            paper: self.paper.min(PAPERS.len() - 1),
            landscape: self.landscape,
            scale: if self.scale.is_finite() {
                self.scale.clamp(25., 400.)
            } else {
                100.
            },
            margin: if self.margin.is_finite() {
                self.margin.clamp(0., 144.)
            } else {
                36.
            },
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PageSetupEvent {
    Accepted(PageSetupState),
    Cancelled,
}

pub struct PageSetup {
    modal: ModalState,
    value: PageSetupState,
    draft: PageSetupState,
    paper: Entity<PopupMenu>,
    scale: Entity<NumberField>,
    margin: Entity<NumberField>,
    focus: [FocusHandle; 4],
    dirty: bool,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<PageSetupEvent> for PageSetup {}
impl PageSetup {
    pub fn new(value: PageSetupState, cx: &mut Context<Self>) -> Self {
        let value = value.normalized();
        let paper =
            cx.new(|cx| PopupMenu::new(PAPERS.iter().map(|paper| paper.name), cx).label("Paper"));
        let scale = cx.new(|cx| NumberField::new(value.scale, 25., 400., 5., cx).label("Scale"));
        let margin = cx.new(|cx| NumberField::new(value.margin, 0., 144., 6., cx).label("Margins"));
        let subscriptions = vec![
            cx.subscribe(&paper, |this: &mut Self, _, event: &MenuEvent, cx| {
                this.draft.paper = event.0;
                cx.notify();
            }),
            cx.subscribe(&scale, |this: &mut Self, _, event: &NumberChanged, cx| {
                this.draft.scale = event.0;
                cx.notify();
            }),
            cx.subscribe(&margin, |this: &mut Self, _, event: &NumberChanged, cx| {
                this.draft.margin = event.0;
                cx.notify();
            }),
        ];
        Self {
            modal: ModalState::new(360., cx),
            value,
            draft: value,
            paper,
            scale,
            margin,
            focus: std::array::from_fn(|_| cx.focus_handle()),
            dirty: true,
            _subscriptions: subscriptions,
        }
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.modal.theme = theme;
        self.dirty = true;
        self
    }
    pub fn language(mut self, language: Language) -> Self {
        self.modal.language = language;
        self.dirty = true;
        self
    }
    pub fn value(&self) -> PageSetupState {
        self.value
    }
    pub fn is_open(&self) -> bool {
        self.modal.open
    }
    pub fn set_value(&mut self, value: PageSetupState, cx: &mut Context<Self>) {
        self.value = value.normalized();
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
        self.draft = self.value;
        self.paper
            .update(cx, |paper, cx| paper.choose(self.draft.paper, cx));
        self.scale
            .update(cx, |scale, cx| scale.set_value(self.draft.scale, cx));
        self.margin
            .update(cx, |margin, cx| margin.set_value(self.draft.margin, cx));
        let focus = self.paper.focus_handle(cx);
        self.modal.stops = vec![
            focus.clone(),
            self.focus[0].clone(),
            self.focus[1].clone(),
            self.scale.focus_handle(cx),
            self.margin.focus_handle(cx),
            self.focus[2].clone(),
            self.focus[3].clone(),
        ];
        self.modal.show(Some(&focus), window, cx);
        cx.notify();
    }
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal.close(window, cx);
        cx.notify();
    }
    fn accept(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.paper.read(cx).is_open() || self.paper.focus_handle(cx).is_focused(window) {
            cx.propagate();
            return;
        }
        self.scale.update(cx, |scale, cx| scale.commit(cx));
        self.margin.update(cx, |margin, cx| margin.commit(cx));
        self.draft.scale = self.scale.read(cx).value();
        self.draft.margin = self.margin.read(cx).value();
        self.value = self.draft.normalized();
        self.close(window, cx);
        cx.emit(PageSetupEvent::Accepted(self.value));
    }
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.paper.read(cx).is_open() {
            cx.propagate();
            return;
        }
        self.close(window, cx);
        cx.emit(PageSetupEvent::Cancelled);
    }
}
impl Render for PageSetup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.modal.open {
            return div().into_any_element();
        }
        let lang = self.modal.language;
        let theme = self.modal.theme;
        if self.dirty {
            self.dirty = false;
            self.paper.update(cx, |paper, cx| {
                paper.set_theme(theme, cx);
                paper.set_language(lang, cx);
                paper.set_label(tr(lang, "paperLabel"), cx);
                paper.set_caption(tr(lang, "paperLabel"), cx);
            });
            for (field, key) in [(&self.scale, "scale"), (&self.margin, "margins")] {
                field.update(cx, |field, cx| {
                    field.set_theme(theme, cx);
                    field.set_language(lang, cx);
                    field.set_label(tr(lang, key), cx);
                });
            }
        }
        let row = |key: &str| {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(text(tr(lang, key), Font::Bold, lang))
        };
        let draft = self.draft;
        let mut orientations = row("orientationLabel");
        // Two icon bevel buttons, the one chosen pressed in.
        for (index, key, landscape) in [(0, "portrait", false), (1, "landscape", true)] {
            orientations = orientations.child(
                bevel_button(key, tr(lang, key))
                    .icon_only(if landscape {
                        LANDSCAPE_ICON
                    } else {
                        PORTRAIT_ICON
                    })
                    .selected(draft.landscape == landscape)
                    .focus_handle(&self.focus[index])
                    .theme(theme)
                    .language(lang)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.draft.landscape = landscape;
                        cx.notify();
                    })),
            );
        }
        let content = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .gap(px(16.))
                    .items_start()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_start()
                            .gap(px(8.))
                            .child(self.paper.clone())
                            .child(orientations)
                            .child(row("scaleLabel").child(self.scale.clone()).child(text(
                                "%",
                                Font::Bold,
                                lang,
                            )))
                            .child(row("marginsLabel").child(self.margin.clone()).child(text(
                                "pt",
                                Font::Bold,
                                lang,
                            ))),
                    )
                    .child(
                        div()
                            .id("page-preview")
                            .role(gpui::Role::Image)
                            .aria_label(tr(lang, "page"))
                            .child(drawing(90., 110., move |pen, _, _| {
                                draw_preview(pen, draft)
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        button("cancel", tr(lang, "cancel"))
                            .focus_handle(&self.focus[2])
                            .theme(theme)
                            .language(lang)
                            .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                    )
                    .child(
                        button("ok", tr(lang, "ok"))
                            .focus_handle(&self.focus[3])
                            .default_button()
                            .theme(theme)
                            .language(lang)
                            .on_click(cx.listener(|this, _, window, cx| this.accept(window, cx))),
                    ),
            );
        modal(
            &mut self.modal,
            ModalView::new(tr(lang, "pageSetup"), true, content),
            window,
            cx,
            |this| &mut this.modal,
            Self::accept,
            Self::cancel,
        )
    }
}
fn draw_preview(pen: &mut Pen, state: PageSetupState) {
    let state = state.normalized();
    let paper = PAPERS[state.paper];
    let (pw, ph) = if state.landscape {
        (paper.height, paper.width)
    } else {
        (paper.width, paper.height)
    };
    let k = ((pen.width - 6) as f64 / pw).min((pen.height - 6) as f64 / ph);
    let (w, h) = ((pw * k) as i32, (ph * k) as i32);
    let (x, y) = ((pen.width - w) / 2, (pen.height - h) / 2);
    pen.rect(x + 2, y + 2, w, h, GRAY_7);
    pen.rect(x, y, w, h, WHITE);
    pen.frame(x, y, w, h, BLACK);
    let margin = (state.margin * k) as i32;
    if margin > 0 && 2 * margin < w && 2 * margin < h {
        for i in (x + margin..x + w - margin).step_by(2) {
            pen.dot(i, y + margin, GRAY_5);
            pen.dot(i, y + h - margin - 1, GRAY_5);
        }
        for j in (y + margin..y + h - margin).step_by(2) {
            pen.dot(x + margin, j, GRAY_5);
            pen.dot(x + w - margin - 1, j, GRAY_5);
        }
    }
    let line_width =
        (((w - 2 * margin - 4) as f64 * 100. / state.scale) as i32).min(w - 2 * margin - 4);
    for j in (y + margin + 3..y + h - margin - 3).step_by(3) {
        pen.hline(x + margin + 2, j, line_width, GRAY_4);
    }
}
#[cfg(test)]
mod tests {
    use super::{PageSetupState, draw_preview};
    use crate::paint::Pen;
    #[test]
    fn invalid_page_values_are_clamped_before_drawing() {
        let state = PageSetupState {
            paper: usize::MAX,
            landscape: true,
            scale: f64::NAN,
            margin: 500.,
        }
        .normalized();
        assert_eq!(state.paper, 4);
        assert_eq!(state.scale, 100.);
        assert_eq!(state.margin, 144.);
        let mut pen = Pen::new(90, 110);
        draw_preview(&mut pen, state);
        assert!(
            pen.fills
                .iter()
                .all(|fill| fill.rect.width > 0 && fill.rect.height > 0)
        );
    }
}

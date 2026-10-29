//! An element that shows a scene and tells what the pointer is over.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::{prelude::*, *};
use limu::scene::style::Look;
use limu::scene::{Icons, Scene};

use super::{Options, SLOP, paint};
use crate::{Language, Theme};

type HoverHandler = Rc<dyn Fn(Option<usize>, &mut Window, &mut App)>;
type ClickHandler = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// An element the size of a scene laid out with
/// [`layout_options`](super::layout_options) that draws it from its top left
/// corner, as [`paint`] does. The item under the pointer shows its tooltip
/// in a help balloon, and the pointer is a hand over one with a link;
/// [`on_hover`](DiagramView::on_hover) and [`on_click`](DiagramView::on_click)
/// tell which item that is, by its index in the scene, and following a link
/// is the host's to do. Assistive technology reads the diagram as an image,
/// named by the scene's title and described by its description.
pub fn view(id: impl Into<ElementId>, scene: impl Into<Arc<Scene>>) -> DiagramView {
    DiagramView {
        id: id.into(),
        scene: scene.into(),
        options: Options::default(),
        label: None,
        on_hover: None,
        on_click: None,
    }
}

#[derive(IntoElement)]
pub struct DiagramView {
    id: ElementId,
    scene: Arc<Scene>,
    options: Options<'static>,
    label: Option<SharedString>,
    on_hover: Option<HoverHandler>,
    on_click: Option<ClickHandler>,
}

impl DiagramView {
    /// Draws the diagram in a Platinum theme's colors.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.options.look = Look::new(super::theme(&theme));
        self
    }

    /// Draws the diagram in a look of limu's, such as a Platinum theme's
    /// with overrides of roles.
    pub fn look(mut self, look: Look) -> Self {
        self.options.look = look;
        self
    }

    /// Draws text in a language's letterforms, the one it was measured in.
    pub fn language(mut self, language: Language) -> Self {
        self.options.language = language;
        self
    }

    /// Draws icons, as limu-gpui draws them.
    pub fn icons(mut self, icons: Icons<'static>) -> Self {
        self.options.icons = Some(icons);
        self
    }

    /// Draws images, as limu-gpui draws them.
    pub fn images(mut self, images: limu_gpui::Images<'static>) -> Self {
        self.options.images = Some(images);
        self
    }

    /// Names the diagram for assistive technology, in place of the scene's
    /// title.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Called with the item the pointer comes to be over, or none when it
    /// leaves the items.
    pub fn on_hover(mut self, f: impl Fn(Option<usize>, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Rc::new(f));
        self
    }

    /// Called with the item a click of the primary button was on, where it
    /// went down.
    pub fn on_click(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

#[derive(Default)]
struct ViewState {
    /// Where the scene's origin was painted last, in the window.
    origin: Option<Point<Pixels>>,
    hovered: Option<usize>,
    /// The item where the primary button went down.
    pressed: Option<usize>,
}

impl ViewState {
    fn item_at(&self, scene: &Scene, position: Point<Pixels>) -> Option<usize> {
        let at = position - self.origin?;
        scene.item_at(
            limu::scene::Point::new(f64::from(at.x), f64::from(at.y)),
            SLOP,
        )
    }
}

impl RenderOnce for DiagramView {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| ViewState::default());
        let scene = self.scene;
        let hovered = state.read(cx).hovered.and_then(|i| scene.items.get(i));
        let link = hovered.is_some_and(|it| it.link.is_some());
        let tooltip = hovered.and_then(|it| it.tooltip.clone());
        let label = self
            .label
            .or_else(|| scene.title.clone().map(SharedString::from));
        let language = self.options.language;
        let on_hover = self.on_hover;
        let (moved, left, down, up) = (state.clone(), state.clone(), state.clone(), state.clone());
        let (moved_scene, down_scene) = (scene.clone(), scene.clone());
        let left_hover = on_hover.clone();
        let on_click = self.on_click;
        let options = self.options;
        let painted = state;
        let painted_scene = scene.clone();
        div()
            .id(self.id)
            .role(Role::Image)
            .when_some(label, |d, label| d.aria_label(label))
            .when_some(scene.desc.clone(), |d, desc| d.aria_description(desc))
            .w(px(scene.width as f32))
            .h(px(scene.height as f32))
            .flex_shrink_0()
            .on_mouse_move(move |event: &MouseMoveEvent, window, cx| {
                let at = moved.read(cx).item_at(&moved_scene, event.position);
                if moved.read(cx).hovered == at {
                    return;
                }
                moved.update(cx, |state, cx| {
                    state.hovered = at;
                    cx.notify();
                });
                if let Some(f) = &on_hover {
                    f(at, window, cx);
                }
            })
            .on_hover(move |inside: &bool, window, cx| {
                if *inside || left.read(cx).hovered.is_none() {
                    return;
                }
                left.update(cx, |state, cx| {
                    state.hovered = None;
                    cx.notify();
                });
                if let Some(f) = &left_hover {
                    f(None, window, cx);
                }
            })
            .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                let at = down.read(cx).item_at(&down_scene, event.position);
                down.update(cx, |state, _| state.pressed = at);
            })
            .on_click(move |event, window, cx| {
                if !event.standard_click() {
                    return;
                }
                let pressed = up.update(cx, |state, _| state.pressed.take());
                if let (Some(f), Some(at)) = (&on_click, pressed) {
                    f(at, window, cx);
                }
            })
            .when(link, |d| d.cursor_pointer())
            .when_some(tooltip, |d, tooltip| {
                d.tooltip_show_delay(Duration::from_millis(600))
                    .tooltip(move |_, cx| {
                        cx.new(|_| crate::extras::HelpBalloon(tooltip.clone().into(), language))
                            .into()
                    })
            })
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, cx| {
                        let origin = bounds.origin.map(|v| v.round());
                        painted.update(cx, |state, _| state.origin = Some(origin));
                        paint(&painted_scene, origin, &options, window, cx);
                    },
                )
                .size_full(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Arc, Language, Rc, Scene, view};
    use crate::diagram::limu;
    use gpui::{
        Context, IntoElement, Modifiers, ParentElement, Render, Styled, TestAppContext, Window,
        div, point, px,
    };
    use std::cell::RefCell;

    struct Harness {
        scene: Arc<Scene>,
        hovered: Rc<RefCell<Vec<Option<usize>>>>,
        clicked: Rc<RefCell<Vec<usize>>>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let (hovered, clicked) = (self.hovered.clone(), self.clicked.clone());
            div().p(px(10.)).child(
                view("diagram", self.scene.clone())
                    .on_hover(move |at, _, _| hovered.borrow_mut().push(at))
                    .on_click(move |at, _, _| clicked.borrow_mut().push(at)),
            )
        }
    }

    // The item under the pointer is told as it comes and goes, and a click
    // tells the item it was on.
    #[gpui::test]
    fn hover_and_click(cx: &mut TestAppContext) {
        let scene = limu::layout(
            "flowchart LR\n  a[One] --> b[Two]",
            &super::super::layout_options(Language::English),
        )
        .unwrap();
        let node = |id: &str| {
            let i = scene
                .items
                .iter()
                .position(|it| it.id.as_deref() == Some(id) && it.kind == limu::scene::Kind::Node)
                .unwrap();
            let b = scene.items[i].bounds;
            let at = point(
                px((b.x + b.w / 2.) as f32 + 10.),
                px((b.y + b.h / 2.) as f32 + 10.),
            );
            (i, at)
        };
        let ((a, at_a), (b, at_b)) = (node("a"), node("b"));
        let (hovered, clicked) = (Rc::default(), Rc::default());
        let harness = Harness {
            scene: Arc::new(scene),
            hovered: Rc::clone(&hovered),
            clicked: Rc::clone(&clicked),
        };
        let (_, cx) = cx.add_window_view(|_, _| harness);
        cx.run_until_parked();
        cx.simulate_mouse_move(at_a, None, Modifiers::default());
        cx.simulate_mouse_move(point(px(11.), px(11.)), None, Modifiers::default());
        cx.simulate_mouse_move(at_b, None, Modifiers::default());
        cx.simulate_click(at_b, Modifiers::default());
        assert_eq!(*hovered.borrow(), [Some(a), None, Some(b)]);
        assert_eq!(*clicked.borrow(), [b]);
    }
}

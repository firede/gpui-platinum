use gpui::{
    Context, Entity, Modifiers, MouseButton, TestAppContext, Window, div, point, prelude::*, px,
};
use gpui_platinum::{MenuBar, MenuItem, Slider};

struct HiddenMenu {
    menu: Entity<MenuBar>,
}

impl Render for HiddenMenu {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(div().h(px(20.)))
            .child(self.menu.clone())
    }
}

#[gpui::test]
fn closed_context_menu_does_not_paint_a_menu_bar(cx: &mut TestAppContext) {
    let (_, cx) = cx.add_window_view(|_, cx| HiddenMenu {
        menu: cx.new(|cx| MenuBar::context_menu(vec![MenuItem::new("Open")], cx)),
    });
    cx.run_until_parked();
    cx.update(|window, _| assert!(window.painted_quads().is_empty()));
}

struct SteppedSlider {
    value: f64,
}

impl Render for SteppedSlider {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        div().child(
            Slider::new("stepped", self.value, 0., 1.)
                .step(0.25)
                .on_change(move |value, _, cx| {
                    entity.update(cx, |this, cx| {
                        this.value = value;
                        cx.notify();
                    });
                }),
        )
    }
}

#[gpui::test]
fn slider_preserves_the_supplied_value_until_the_user_changes_it(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, _| SteppedSlider { value: 0.3 });
    cx.run_until_parked();
    cx.update(|window, _| {
        let scale = window.scale_factor();
        let thumb = window
            .painted_quads()
            .into_iter()
            .find(|quad| {
                quad.bounds.origin.y.0 == 0.
                    && quad.bounds.size.width.0 == 11. * scale
                    && quad.bounds.size.height.0 == scale
            })
            .expect("slider thumb's top edge");
        assert_eq!(thumb.bounds.origin.x.0, 45. * scale);
    });
    cx.simulate_mouse_down(
        point(px(50.), px(7.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_up(
        point(px(50.), px(7.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_keystrokes("right");
    cx.read_entity(&view, |view, _| assert_eq!(view.value, 0.5));
}

//! Inherited presentation state for controls in background windows.

use gpui::{prelude::*, *};
use std::cell::Cell;

thread_local! {
    static INACTIVE: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn content_is_inactive() -> bool {
    INACTIVE.with(Cell::get)
}

fn with_inactive<R>(inactive: bool, run: impl FnOnce() -> R) -> R {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            INACTIVE.with(|state| state.set(self.0));
        }
    }
    let previous = INACTIVE.with(|state| state.replace(state.get() || inactive));
    let _restore = Restore(previous);
    run()
}

/// Passes a window's presentation state through cached GPUI views and canvases.
pub(crate) struct Inactive {
    inactive: bool,
    content: AnyElement,
}

impl IntoElement for Inactive {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Inactive {
    pub(crate) fn new(inactive: bool, content: impl IntoElement) -> Self {
        Self {
            inactive,
            content: content.into_any_element(),
        }
    }
}

impl Element for Inactive {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let layout = with_inactive(self.inactive, || self.content.request_layout(window, cx));
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        with_inactive(self.inactive, || self.content.prepaint(window, cx));
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        with_inactive(self.inactive, || self.content.paint(window, cx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    #[test]
    fn nested_windows_cannot_reactivate_background_content() {
        assert!(!content_is_inactive());
        with_inactive(true, || {
            assert!(content_is_inactive());
            with_inactive(false, || assert!(content_is_inactive()));
            assert!(content_is_inactive());
        });
        assert!(!content_is_inactive());
    }

    #[test]
    fn presentation_state_is_restored_after_unwinding() {
        let _ = std::panic::catch_unwind(|| with_inactive(true, || panic!("drawing failed")));
        assert!(!content_is_inactive());
    }
}

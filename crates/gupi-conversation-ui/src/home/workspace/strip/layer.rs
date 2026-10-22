//! Lift above neighboring tabs while explicitly keeping paint and hit tests in the strip.
use super::*;
use std::cell::Cell;

pub(super) struct Lifted {
    child: Option<AnyElement>,
    viewport: Bounds<Pixels>,
    mask: Rc<Cell<Bounds<Pixels>>>,
}
pub(super) fn lifted(child: impl IntoElement, viewport: Bounds<Pixels>) -> Lifted {
    let mask = Rc::new(Cell::new(viewport));
    Lifted {
        child: Some(
            Masked {
                child: child.into_any_element(),
                mask: mask.clone(),
            }
            .into_any_element(),
        ),
        viewport,
        mask,
    }
}
// Locked GPUI only applies defer_draw's mask in paint. Prepaint needs the same
// mask explicitly so invisible tab hitboxes cannot cover adjacent window chrome.
struct Masked {
    child: AnyElement,
    mask: Rc<Cell<Bounds<Pixels>>>,
}
impl IntoElement for Masked {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Masked {
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
        (self.child.request_layout(window, cx), ())
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
        window.with_content_mask(
            Some(ContentMask {
                bounds: self.mask.get(),
            }),
            |window| {
                self.child.prepaint(window, cx);
            },
        );
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
        window.with_content_mask(
            Some(ContentMask {
                bounds: self.mask.get(),
            }),
            |window| self.child.paint(window, cx),
        );
    }
}
impl IntoElement for Lifted {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Lifted {
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
        (self.child.as_mut().unwrap().request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        _: &mut App,
    ) {
        let mask = ContentMask {
            bounds: self.viewport,
        }
        .intersect(&window.content_mask());
        self.mask.set(mask.bounds);
        window.defer_draw(
            self.child.take().unwrap(),
            window.element_offset(),
            1,
            Some(mask),
        );
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;
    use gpui_kit::component::Root;
    use std::cell::Cell;
    struct Fixture {
        hidden: Rc<Cell<usize>>,
        neighbor: Rc<Cell<usize>>,
    }
    impl Render for Fixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let hidden = self.hidden.clone();
            let neighbor = self.neighbor.clone();
            div()
                .size_full()
                .child(
                    Button::new("neighbor")
                        .absolute()
                        .left(px(220.))
                        .top(px(50.))
                        .w(px(100.))
                        .h(px(100.))
                        .on_click(move |_, _, _| neighbor.set(neighbor.get() + 1)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(100.))
                        .top(px(50.))
                        .w(px(100.))
                        .h(px(100.))
                        .overflow_hidden()
                        .child(lifted(
                            Button::new("lifted")
                                .w(px(200.))
                                .h(px(100.))
                                .on_click(move |_, _, _| hidden.set(hidden.get() + 1)),
                            Bounds::new(point(px(0.), px(0.)), size(px(500.), px(300.))),
                        )),
                )
        }
    }
    #[gpui_kit::test]
    fn clipped_lifted_hitbox_cannot_intercept_neighbor_outside_ancestor(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            app_theme::init(cx);
        });
        let hidden = Rc::new(Cell::new(0));
        let neighbor = Rc::new(Cell::new(0));
        let (_, visual) = cx.add_window_view(|window, cx| {
            Root::new(
                cx.new(|_| Fixture {
                    hidden: hidden.clone(),
                    neighbor: neighbor.clone(),
                }),
                window,
                cx,
            )
        });
        visual.simulate_resize(size(px(500.), px(300.)));
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        visual.simulate_mouse_down(
            point(px(250.), px(80.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        visual.simulate_mouse_up(
            point(px(250.), px(80.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        assert_eq!(hidden.get(), 0);
        assert_eq!(neighbor.get(), 1);
        visual.simulate_mouse_down(
            point(px(150.), px(80.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        visual.simulate_mouse_up(
            point(px(150.), px(80.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        assert_eq!(hidden.get(), 1);
    }
}

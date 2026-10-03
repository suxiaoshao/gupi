use gpui_kit::*;
use std::sync::atomic::{AtomicBool, Ordering};
static RECORDING: AtomicBool = AtomicBool::new(false);
pub fn set_recording(value: bool) {
    RECORDING.store(value, Ordering::Relaxed);
}
fn is_recording() -> bool {
    RECORDING.load(Ordering::Relaxed)
}
/// Delegate without adding layout nodes, identity, invalidation or frame requests.
pub fn measure(region: &'static str, shimmer: bool, element: impl IntoElement) -> AnyElement {
    let element = element.into_any_element();
    if !is_recording() {
        return element;
    }
    DrawProbe {
        region,
        shimmer,
        element,
    }
    .into_any_element()
}

struct DrawProbe {
    region: &'static str,
    shimmer: bool,
    element: AnyElement,
}

impl IntoElement for DrawProbe {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for DrawProbe {
    type RequestLayoutState = ();
    type PrepaintState = bool;

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
        let _span = tracing::debug_span!(target: "gupi::performance", "ui.request_layout",
            region = self.region, shimmer = self.shimmer, reduce_motion = cx.reduce_motion(),
            view = ?window.current_view())
        .entered();
        (self.element.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        // Geometric visibility only: this does not detect occluding overlays.
        let visible = !bounds
            .intersect(&window.content_mask().bounds)
            .intersect(&Bounds::new(Point::default(), window.viewport_size()))
            .is_empty();
        let _span = tracing::debug_span!(target: "gupi::performance", "ui.prepaint",
            region = self.region, shimmer = self.shimmer, visible,
            view = ?window.current_view())
        .entered();
        self.element.prepaint(window, cx);
        visible
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        visible: &mut bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let _span = tracing::debug_span!(target: "gupi::performance", "ui.paint",
            region = self.region, shimmer = self.shimmer, visible = *visible,
            view = ?window.current_view())
        .entered();
        self.element.paint(window, cx);
    }
}

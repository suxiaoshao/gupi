use gpui_kit::{Bounds, Pixels, Point, Size, point, px, size};

pub(crate) fn bounds(
    start: Point<Pixels>,
    end: Point<Pixels>,
    viewport: Size<Pixels>,
) -> Option<Bounds<Pixels>> {
    let x = f32::from(start.x.min(end.x)).clamp(0., viewport.width.into());
    let y = f32::from(start.y.min(end.y)).clamp(0., viewport.height.into());
    let right = f32::from(start.x.max(end.x)).clamp(0., viewport.width.into());
    let bottom = f32::from(start.y.max(end.y)).clamp(0., viewport.height.into());
    // Physical pointer tolerance, independent of the application's text scale.
    (right - x >= 2. && bottom - y >= 2.)
        .then(|| Bounds::new(point(px(x), px(y)), size(px(right - x), px(bottom - y))))
}

pub(crate) fn crop_rect(
    bounds: Bounds<Pixels>,
    viewport: Size<Pixels>,
    width: u32,
    height: u32,
) -> (u32, u32, u32, u32) {
    let sx = width as f32 / f32::from(viewport.width);
    let sy = height as f32 / f32::from(viewport.height);
    let left = (f32::from(bounds.origin.x) * sx).floor().max(0.) as u32;
    let top = (f32::from(bounds.origin.y) * sy).floor().max(0.) as u32;
    let right = (f32::from(bounds.right()) * sx).ceil().min(width as f32) as u32;
    let bottom = (f32::from(bounds.bottom()) * sy).ceil().min(height as f32) as u32;
    (left, top, right - left, bottom - top)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reverse_drag_clamps_to_frame_and_uses_actual_pixel_ratio() {
        let viewport = size(px(100.), px(80.));
        let rect = bounds(point(px(120.), px(70.)), point(px(10.), px(-10.)), viewport).unwrap();
        assert_eq!(crop_rect(rect, viewport, 200, 240), (20, 0, 180, 210));
        assert!(bounds(point(px(1.), px(1.)), point(px(1.), px(50.)), viewport).is_none());
    }
}

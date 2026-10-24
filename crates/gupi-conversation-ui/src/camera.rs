//! Shared world/screen transform; each canvas owns its zoom limits.
use gpui_kit::{Point, point};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Camera {
    pub zoom: f32,
    pub pan: Point<f32>,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 1.,
            pan: point(0., 0.),
        }
    }
}
impl Camera {
    pub fn screen(self, p: Point<f32>) -> Point<f32> {
        p * self.zoom + self.pan
    }
    pub fn world(self, p: Point<f32>) -> Point<f32> {
        (p - self.pan) / self.zoom
    }
    pub fn zoom_at(&mut self, anchor: Point<f32>, zoom: f32) {
        let world = self.world(anchor);
        self.zoom = zoom;
        self.pan = anchor - world * self.zoom;
    }
}

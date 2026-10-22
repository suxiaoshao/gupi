//! Domain order uses frozen, unanimated slots. Springs only place the tab visuals.
use super::*;
use std::cell::RefCell;

type Id = Option<String>;
const HYSTERESIS: Pixels = px(1.);
#[derive(Clone)]
pub(super) struct Geometry {
    pub slots: Vec<(Id, Bounds<Pixels>)>,
    pub viewport: Bounds<Pixels>,
    pub scroll: Point<Pixels>,
}
impl Geometry {
    pub fn read(ids: &[Id], scroll: &ScrollHandle) -> Option<Self> {
        let offset = scroll.offset();
        let slots = ids
            .iter()
            .enumerate()
            .map(|(ix, id)| Some((id.clone(), scroll.bounds_for_item(ix)? + offset)))
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            slots,
            viewport: scroll.bounds(),
            scroll: offset,
        })
    }
    fn same_members(&self, ids: &[Id]) -> bool {
        ids.len() == self.slots.len()
            && ids
                .iter()
                .all(|id| self.slots.iter().filter(|(key, _)| key == id).count() == 1)
    }
    pub fn slot(&self, id: &Id) -> Option<Bounds<Pixels>> {
        self.slots
            .iter()
            .find(|(key, _)| key == id)
            .map(|(_, bounds)| *bounds)
    }
    fn arrange(&mut self, ids: &[Id]) -> bool {
        if !self.same_members(ids) {
            return false;
        }
        let Some((_, first)) = self.slots.first() else {
            return false;
        };
        let mut x = first.left();
        let gap = self
            .slots
            .windows(2)
            .next()
            .map(|slots| slots[1].1.left() - slots[0].1.right())
            .unwrap_or_default();
        let slots = ids
            .iter()
            .map(|id| {
                let mut bounds = self.slot(id).unwrap();
                bounds.origin.x = x;
                x += bounds.size.width + gap;
                (id.clone(), bounds)
            })
            .collect();
        self.slots = slots;
        true
    }
    fn reorder(
        &mut self,
        ids: &mut [Id],
        id: &Id,
        left: Pixels,
        rightward: bool,
        leftward: bool,
    ) -> bool {
        if !self.slots.iter().map(|(key, _)| key).eq(ids.iter()) {
            return false;
        }
        let Some(mut ix) = ids.iter().position(|key| key == id) else {
            return false;
        };
        let width = self.slots[ix].1.size.width;
        while rightward
            && ix + 1 < ids.len()
            && left + width > self.slots[ix + 1].1.center().x + HYSTERESIS
        {
            ids.swap(ix, ix + 1);
            ix += 1;
            self.arrange(ids);
        }
        while leftward && ix > 0 && left < self.slots[ix - 1].1.center().x - HYSTERESIS {
            ids.swap(ix - 1, ix);
            ix -= 1;
            self.arrange(ids);
        }
        true
    }
    pub fn current(&self, scroll: &ScrollHandle) -> bool {
        self.scroll == scroll.offset() && self.viewport == scroll.bounds()
    }
}
struct Press {
    id: Id,
    grab: Pixels,
}
struct Gesture {
    id: Id,
    grab: Pixels,
    left: Pixels,
}
pub(super) struct Motion {
    pub geometry: Geometry,
    pub lifted: Id,
    pub epoch: u64,
    pub seeded: RefCell<HashSet<Id>>,
    pub starts: HashMap<Id, Pixels>,
    pub moving: std::cell::Cell<bool>,
}
#[derive(Default)]
pub(in crate::home) struct Interaction {
    pub(super) visuals: Rc<RefCell<HashMap<Id, Bounds<Pixels>>>>,
    measurement: Option<Geometry>,
    press: Option<Press>,
    gesture: Option<Gesture>,
    pub(super) motion: Option<Motion>,
    owned: bool,
}
impl Interaction {
    pub fn active(&self) -> bool {
        self.owned
    }
    pub(super) fn arm(&mut self, id: Id, x: Pixels) {
        self.press = self.visuals.borrow().get(&id).map(|bounds| Press {
            id,
            grab: x - bounds.left(),
        });
    }
    pub(super) fn measure(&mut self, geometry: Geometry) {
        self.visuals
            .borrow_mut()
            .retain(|id, _| geometry.slots.iter().any(|(key, _)| key == id));
        self.measurement = Some(geometry);
    }
    pub(super) fn begin(
        &mut self,
        id: &Id,
        ids: &mut [Id],
        scroll: &ScrollHandle,
        x: Pixels,
    ) -> bool {
        self.owned = true;
        let Some(press) = self.press.take().filter(|press| &press.id == id) else {
            return false;
        };
        let Some(mut geometry) = self
            .measurement
            .clone()
            .filter(|g| g.current(scroll) && g.same_members(ids))
        else {
            return false;
        };
        if !geometry.arrange(ids) {
            return false;
        }
        let Some(slot) = geometry
            .slot(id)
            .filter(|slot| slot.size.width <= geometry.viewport.size.width)
        else {
            return false;
        };
        let left = self
            .visuals
            .borrow()
            .get(id)
            .map(|b| b.left())
            .unwrap_or(slot.left());
        static NEXT_GESTURE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let epoch = NEXT_GESTURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.motion = Some(Motion {
            geometry,
            lifted: id.clone(),
            epoch,
            seeded: Default::default(),
            moving: std::cell::Cell::new(true),
            starts: self
                .visuals
                .borrow()
                .iter()
                .map(|(id, bounds)| (id.clone(), bounds.left()))
                .collect(),
        });
        self.gesture = Some(Gesture {
            id: id.clone(),
            grab: press.grab,
            left,
        });
        self.advance(ids, scroll, x)
    }
    pub(super) fn valid(&self, ids: &[Id], scroll: &ScrollHandle) -> bool {
        self.motion.as_ref().is_some_and(|motion| {
            motion.geometry.current(scroll)
                && motion
                    .geometry
                    .slots
                    .iter()
                    .map(|(id, _)| id)
                    .eq(ids.iter())
        })
    }
    pub(super) fn advance(&mut self, ids: &mut [Id], scroll: &ScrollHandle, x: Pixels) -> bool {
        if !self.valid(ids, scroll) {
            return false;
        }
        let (Some(motion), Some(gesture)) = (&mut self.motion, &mut self.gesture) else {
            return false;
        };
        let geometry = &mut motion.geometry;
        let width = geometry.slot(&gesture.id).unwrap().size.width;
        let left =
            (x - gesture.grab).clamp(geometry.viewport.left(), geometry.viewport.right() - width);
        let rightward = left > gesture.left;
        let leftward = left < gesture.left;
        gesture.left = left;
        geometry.reorder(ids, &gesture.id, left, rightward, leftward)
    }
    pub(super) fn held_left(&self, id: &Id) -> Option<Pixels> {
        self.gesture
            .as_ref()
            .filter(|gesture| &gesture.id == id)
            .map(|gesture| gesture.left)
    }
    pub fn finish(&mut self) -> bool {
        self.press = None;
        if let Some(gesture) = self.gesture.take()
            && let Some(motion) = &mut self.motion
        {
            motion.starts.insert(gesture.id.clone(), gesture.left);
            motion.seeded.borrow_mut().remove(&gesture.id);
            motion.moving.set(true);
        }
        std::mem::take(&mut self.owned)
    }
    pub(super) fn reset(&mut self) {
        self.motion = None;
        self.press = None;
        self.measurement = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;
    fn geometry(widths: &[f32]) -> (Geometry, Vec<Id>) {
        let ids: Vec<_> = (0..widths.len()).map(|i| Some(i.to_string())).collect();
        let mut x = px(100.);
        let slots = ids
            .iter()
            .zip(widths)
            .map(|(id, width)| {
                let bounds = Bounds::new(point(x, px(10.)), size(px(*width), px(28.)));
                x += px(*width + 2.);
                (id.clone(), bounds)
            })
            .collect();
        (
            Geometry {
                slots,
                viewport: Bounds::new(point(px(100.), px(0.)), size(x - px(102.), px(44.))),
                scroll: point(px(-60.), px(0.)),
            },
            ids,
        )
    }
    #[test]
    fn all_adjacent_leading_edge_swaps_preserve_widths_and_gaps() {
        for widths in [&[160., 160.][..], &[160., 160., 160.], &[160., 240., 190.]] {
            for ix in 0..widths.len() - 1 {
                for right in [true, false] {
                    let (mut geometry, mut order) = geometry(widths);
                    let original = order.clone();
                    let grabbed = if right { ix } else { ix + 1 };
                    let neighbor = if right { ix + 1 } else { ix };
                    let id = order[grabbed].clone();
                    let edge = geometry.slots[neighbor].1.center().x;
                    let width = geometry.slots[grabbed].1.size.width;
                    let left = if right {
                        edge - width + px(2.)
                    } else {
                        edge - px(2.)
                    };
                    assert!(geometry.reorder(&mut order, &id, left, right, !right));
                    let mut expected = original.clone();
                    expected.swap(ix, ix + 1);
                    assert_eq!(order, expected);
                    for (id, bounds) in &geometry.slots {
                        let index = original.iter().position(|key| key == id).unwrap();
                        assert_eq!(bounds.size, size(px(widths[index]), px(28.)));
                    }
                    for slots in geometry.slots.windows(2) {
                        assert_eq!(slots[1].1.left() - slots[0].1.right(), px(2.));
                    }
                }
            }
        }
    }
    #[test]
    fn multicross_reverse_hysteresis_and_stale_order() {
        let (mut geometry, mut order) = geometry(&[160., 240., 190.]);
        let id = order[0].clone();
        let original = order.clone();
        let left = geometry.slots[1].1.center().x - px(160.);
        geometry.reorder(&mut order, &id, left, true, false);
        assert_eq!(order, original); // equality (and its small deadband) is stable.
        geometry.reorder(
            &mut order,
            &id,
            geometry.viewport.right() - px(160.),
            true,
            false,
        );
        assert_eq!(
            order,
            vec![
                original[1].clone(),
                original[2].clone(),
                original[0].clone()
            ]
        );
        geometry.reorder(&mut order, &id, geometry.viewport.left(), false, true);
        assert_eq!(order, original);
        order.pop();
        assert!(!geometry.reorder(&mut order, &id, px(500.), true, false));
    }
}

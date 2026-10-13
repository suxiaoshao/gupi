//! Display identity and mode metadata, independent of captured raster dimensions.
use crate::Error;
use gpui_kit::{App, Bounds, Pixels};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplaySnapshot {
    pub(crate) bounds: Bounds<Pixels>,
    scale: f32,
    mode_pixels: (u32, u32),
    rotation: f32,
}
pub(crate) type Snapshot = BTreeMap<u64, DisplaySnapshot>;

pub(crate) fn read(cx: &App) -> Result<(Snapshot, Vec<u64>), Error> {
    let displays = cx.displays();
    let monitors = xcap::Monitor::all().map_err(|_| Error::DisplayChanged)?;
    let ids: Vec<_> = displays.iter().map(|d| u64::from(d.id())).collect();
    check_ids(
        &ids,
        &monitors
            .iter()
            .map(|m| m.id().map_err(|_| Error::DisplayChanged))
            .collect::<Result<Vec<_>, _>>()?,
    )?;
    let mut snapshot = Snapshot::new();
    let mut mapped = Vec::new();
    for display in displays {
        let id = u64::from(display.id());
        let monitor = monitors
            .iter()
            .find(|m| m.id().ok() == Some(id as u32))
            .ok_or(Error::DisplayChanged)?
            .clone();
        #[cfg(target_os = "macos")]
        let mode_pixels = {
            let mode = objc2_core_graphics::CGDisplayCopyDisplayMode(id as u32)
                .ok_or(Error::DisplayChanged)?;
            (
                objc2_core_graphics::CGDisplayMode::pixel_width(Some(&mode)) as u32,
                objc2_core_graphics::CGDisplayMode::pixel_height(Some(&mode)) as u32,
            )
        };
        #[cfg(target_os = "windows")]
        let mode_pixels = (
            monitor.width().map_err(|_| Error::DisplayChanged)?,
            monitor.height().map_err(|_| Error::DisplayChanged)?,
        );
        snapshot.insert(
            id,
            DisplaySnapshot {
                bounds: display.bounds(),
                scale: monitor.scale_factor().map_err(|_| Error::DisplayChanged)?,
                mode_pixels,
                rotation: monitor.rotation().map_err(|_| Error::DisplayChanged)?,
            },
        );
        mapped.push(id);
    }
    Ok((snapshot, mapped))
}
pub(crate) fn current(cx: &App) -> Result<Snapshot, Error> {
    // GPUI's test platform has synthetic display IDs. Substitute only native
    // metadata, leaving request ownership, events, crop and encoding unchanged.
    #[cfg(test)]
    if let Some(fixture) = cx.try_global::<SnapshotFixture>() {
        return Ok(fixture.0.clone());
    }
    read(cx).map(|(snapshot, _)| snapshot)
}
#[cfg(test)]
pub(crate) struct SnapshotFixture(pub(crate) Snapshot);
#[cfg(test)]
impl gpui_kit::Global for SnapshotFixture {}

fn check_ids(gpui: &[u64], xcap: &[u32]) -> Result<(), Error> {
    let full: BTreeSet<_> = gpui.iter().copied().collect();
    let truncated: BTreeSet<_> = gpui.iter().map(|id| *id as u32).collect();
    let native: BTreeSet<_> = xcap.iter().copied().collect();
    if gpui.is_empty()
        || full.len() != gpui.len()
        || truncated.len() != gpui.len()
        || native.len() != xcap.len()
        || native != truncated
    {
        return Err(Error::DisplayChanged);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{point, px, size};
    #[test]
    fn mapping_rejects_ambiguous_or_missing_native_ids() {
        assert!(check_ids(&[0x100000001, 0x200000001], &[1]).is_err());
        assert!(check_ids(&[7, 9], &[9, 7]).is_ok());
        assert!(check_ids(&[7], &[7, 9]).is_err());
        assert!(check_ids(&[7], &[7, 7]).is_err());
    }
    #[test]
    fn retina_mode_snapshot_is_keyed_and_detects_scale_and_raster_changes() {
        let display = DisplaySnapshot {
            bounds: Bounds::new(point(px(0.), px(0.)), size(px(100.), px(80.))),
            scale: 2.,
            mode_pixels: (200, 160),
            rotation: 0.,
        };
        let original = Snapshot::from([(7, display.clone()), (9, display.clone())]);
        let reordered = Snapshot::from([(9, display.clone()), (7, display.clone())]);
        assert_eq!(original, reordered);
        for changed in [
            DisplaySnapshot {
                scale: 1.,
                ..display.clone()
            },
            DisplaySnapshot {
                mode_pixels: (300, 240),
                ..display.clone()
            },
            DisplaySnapshot {
                rotation: 90.,
                ..display.clone()
            },
        ] {
            assert_ne!(
                original,
                Snapshot::from([(7, changed), (9, display.clone())])
            );
        }
        // Captured image dimensions are deliberately separate from display mode.
        let rect = crate::selection::crop_rect(
            Bounds::new(point(px(10.), px(10.)), size(px(20.), px(20.))),
            display.bounds.size,
            300,
            240,
        );
        assert_eq!(rect, (30, 30, 60, 60));
    }
}

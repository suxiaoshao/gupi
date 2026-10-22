//! Two-level allocation: workspace first, then history inside the conversation.
//! All inputs and outputs are resolved pixels; preferences remain in rem.
pub(in crate::home) const BODY_MAX_REM: f32 = 51.25;
pub(in crate::home) const BODY_MIN_REM: f32 = 24.;
pub(in crate::home) const CONVERSATION_MIN_REM: f32 = 24.;
pub(in crate::home) const WORKSPACE_MIN_REM: f32 = 20.;
pub(in crate::home) const HISTORY_MIN_REM: f32 = 15.;
pub(in crate::home) const HISTORY_GAP_REM: f32 = 1.;
// A physical border, independent of application zoom.
pub(in crate::home) const DIVIDER: f32 = 1.;

pub(in crate::home) fn clamp(value: f32, minimum: f32, maximum: f32) -> f32 {
    let maximum = maximum.max(0.);
    value.clamp(minimum.min(maximum), maximum)
}

#[derive(Clone, Copy, Debug, Default)]
pub(in crate::home) struct Workspace {
    pub conversation: f32,
    pub width: f32,
    pub ratio_width: f32,
}
impl Workspace {
    pub fn dockable(available: f32, rem: f32) -> bool {
        available >= (CONVERSATION_MIN_REM + WORKSPACE_MIN_REM) * rem + DIVIDER
    }
    pub fn docked(available: f32, ratio: f32, rem: f32) -> Self {
        let ratio_width = (available - DIVIDER).max(0.);
        let conversation = clamp(
            ratio * ratio_width,
            CONVERSATION_MIN_REM * rem,
            ratio_width - WORKSPACE_MIN_REM * rem,
        );
        Self {
            conversation,
            width: (ratio_width - conversation).max(0.),
            ratio_width,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(in crate::home) struct History {
    pub dockable: bool,
    pub width: f32,
    pub body: f32,
    pub origin: f32,
}
impl History {
    pub fn new(conversation: f32, requested: f32, pinned: bool, rem: f32) -> Self {
        let width = clamp(requested, HISTORY_MIN_REM * rem, conversation);
        let dockable = conversation - width - HISTORY_GAP_REM * rem >= BODY_MIN_REM * rem;
        let body = (BODY_MAX_REM * rem)
            .min(if pinned && dockable {
                conversation - width - HISTORY_GAP_REM * rem
            } else {
                conversation
            })
            .max(0.);
        let origin = if pinned && dockable {
            ((conversation - body) / 2.).min(conversation - width - HISTORY_GAP_REM * rem - body)
        } else {
            (conversation - body) / 2.
        };
        Self {
            dockable,
            width,
            body,
            origin,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_outer_boundary_and_inverse_ratio() {
        for rem in [12., 16., 24., 40.] {
            let edge = (CONVERSATION_MIN_REM + WORKSPACE_MIN_REM) * rem + DIVIDER;
            assert!(!Workspace::dockable(edge - 0.01, rem));
            assert!(Workspace::dockable(edge, rem));
            for available in [edge, edge + 500.] {
                for ratio in [0.01, 0.4, 0.99] {
                    let layout = Workspace::docked(available, ratio, rem);
                    assert!(layout.conversation >= CONVERSATION_MIN_REM * rem - 0.001);
                    assert!(layout.width >= WORKSPACE_MIN_REM * rem - 0.001);
                    let restored =
                        Workspace::docked(available, layout.conversation / layout.ratio_width, rem);
                    assert!((restored.conversation - layout.conversation).abs() < 0.001);
                }
            }
            for width in [0., 0.5, 10., 100.] {
                let history = History::new(width, 10000., false, rem);
                assert!(
                    [history.width, history.body, history.origin]
                        .into_iter()
                        .all(|n| n.is_finite() && n >= 0.)
                );
            }
        }
    }
    #[test]
    fn history_shift_is_continuous_and_rechecks_after_workspace_undocks() {
        let rem = 16.;
        let requested = 20. * rem;
        let boundary = BODY_MAX_REM * rem + 2. * (requested + HISTORY_GAP_REM * rem);
        let before = History::new(boundary - 0.01, requested, true, rem);
        let after = History::new(boundary + 0.01, requested, true, rem);
        assert!((before.origin - after.origin).abs() < 0.02);
        assert!(!History::new(24. * rem, requested, true, rem).dockable);
        assert!(History::new(70. * rem, requested, true, rem).dockable);
    }
}

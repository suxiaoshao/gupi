//! Two-level allocation: workspace first, then history inside the conversation.
//! All inputs and outputs are resolved pixels; preferences remain in rem.
pub(in crate::home) const BODY_MAX_REM: f32 = 51.25;
pub(in crate::home) const BODY_MIN_REM: f32 = 24.;
pub(in crate::home) const CONVERSATION_MIN_REM: f32 = 24.;
pub(in crate::home) const NAVIGATOR_MIN_REM: f32 = 15.;
pub(in crate::home) const CONTENT_MIN_REM: f32 = 30.;
pub(in crate::home) const OVERLAY_CONTENT_MIN_REM: f32 = 12.;
pub(in crate::home) const OVERLAY_STRIP_REM: f32 = 3.;
pub(in crate::home) const HISTORY_MIN_REM: f32 = 15.;
pub(in crate::home) const HISTORY_GAP_REM: f32 = 1.;
// A physical border, independent of application zoom.
pub(in crate::home) const DIVIDER: f32 = 1.;

pub(in crate::home) fn clamp(value: f32, minimum: f32, maximum: f32) -> f32 {
    let maximum = maximum.max(0.);
    value.clamp(minimum.min(maximum), maximum)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::home) struct Content {
    pub main: bool,
    pub navigator: bool,
}
impl Content {
    pub fn minimum(self, rem: f32) -> f32 {
        CONVERSATION_MIN_REM * rem + DIVIDER + self.workspace_minimum(rem)
    }
    /// The narrowest workspace beside the conversation, excluding its leading divider.
    pub fn workspace_minimum(self, rem: f32) -> f32 {
        match (self.main, self.navigator) {
            (true, true) => CONTENT_MIN_REM * rem + DIVIDER + NAVIGATOR_MIN_REM * rem,
            (true, false) => CONTENT_MIN_REM * rem,
            (false, true) => NAVIGATOR_MIN_REM * rem,
            (false, false) => 0.,
        }
    }
    pub fn dockable(self, available: f32, rem: f32) -> bool {
        available >= self.minimum(rem)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(in crate::home) struct Workspace {
    pub conversation: f32,
    pub navigator: f32,
    pub main: f32,
    pub width: f32,
    pub ratio_width: f32,
}
impl Workspace {
    /// The source split divides the whole workspace from the conversation, so
    /// showing or hiding the navigator only redistributes space inside it.
    pub fn docked(available: f32, content: Content, navigator: f32, ratio: f32, rem: f32) -> Self {
        let mut result = Self::default();
        if !content.main {
            result.navigator = clamp(
                navigator,
                NAVIGATOR_MIN_REM * rem,
                available - DIVIDER - CONVERSATION_MIN_REM * rem,
            );
            result.conversation = (available - result.navigator - DIVIDER).max(0.);
            result.width = result.navigator;
            return result;
        }
        result.ratio_width = (available - DIVIDER).max(0.);
        result.conversation = clamp(
            ratio * result.ratio_width,
            CONVERSATION_MIN_REM * rem,
            result.ratio_width - content.workspace_minimum(rem),
        );
        result.width = result.ratio_width - result.conversation;
        result.navigator = if content.navigator {
            clamp(
                navigator,
                NAVIGATOR_MIN_REM * rem,
                result.width - DIVIDER - CONTENT_MIN_REM * rem,
            )
        } else {
            0.
        };
        result.main =
            (result.width - result.navigator - if content.navigator { DIVIDER } else { 0. })
                .max(0.);
        result
    }
    pub fn overlay(available: f32, content: Content, navigator: f32, rem: f32) -> Self {
        let width = (available - (OVERLAY_STRIP_REM * rem).min(available * 0.1)).max(0.);
        let divider = if content.main && content.navigator {
            DIVIDER.min(width)
        } else {
            0.
        };
        let minimum_main = (OVERLAY_CONTENT_MIN_REM * rem).min((width - divider).max(0.) * 0.5);
        let navigator = if content.navigator {
            clamp(
                navigator,
                NAVIGATOR_MIN_REM * rem,
                width - divider - if content.main { minimum_main } else { 0. },
            )
        } else {
            0.
        };
        Self {
            conversation: available.max(0.),
            navigator,
            main: if content.main {
                (width - navigator - divider).max(0.)
            } else {
                0.
            },
            width: if content.main { width } else { navigator },
            ratio_width: 0.,
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
    fn allocation_and_inverse_drag_share_boundaries() {
        for rem in [12., 16., 24., 40.] {
            for content in [
                Content {
                    main: false,
                    navigator: true,
                },
                Content {
                    main: true,
                    navigator: true,
                },
                Content {
                    main: true,
                    navigator: false,
                },
            ] {
                let edge = content.minimum(rem);
                assert!(!content.dockable(edge - 0.01, rem));
                assert!(content.dockable(edge, rem));
                for available in [edge, edge + 500.] {
                    for requested in [0., 100000.] {
                        let layout = Workspace::docked(available, content, requested, 0.4, rem);
                        assert!(layout.conversation >= CONVERSATION_MIN_REM * rem - 0.001);
                        if content.main {
                            assert!(layout.main >= CONTENT_MIN_REM * rem - 0.001);
                            let restored = Workspace::docked(
                                available,
                                content,
                                requested,
                                layout.conversation / layout.ratio_width,
                                rem,
                            );
                            assert!((restored.conversation - layout.conversation).abs() < 0.001);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn navigator_visibility_keeps_the_source_split() {
        let rem = 16.;
        let with = Content {
            main: true,
            navigator: true,
        };
        let without = Content {
            main: true,
            navigator: false,
        };
        let available = with.minimum(rem) + 500.;
        let a = Workspace::docked(available, with, 20. * rem, 0.5, rem);
        let b = Workspace::docked(available, without, 20. * rem, 0.5, rem);
        assert_eq!(a.conversation, b.conversation);
        assert_eq!(a.width, b.width);
    }
    #[test]
    fn overlays_remain_finite_at_infeasible_sizes() {
        for width in [0., 0.5, 10., 100., 800.] {
            for navigator in [false, true] {
                let layout = Workspace::overlay(
                    width,
                    Content {
                        main: true,
                        navigator,
                    },
                    10000.,
                    40.,
                );
                let history = History::new(width, 10000., false, 40.);
                for value in [
                    layout.width,
                    layout.navigator,
                    layout.main,
                    history.width,
                    history.body,
                    history.origin,
                ] {
                    assert!(value.is_finite() && value >= 0.);
                }
                assert!(layout.width <= width);
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

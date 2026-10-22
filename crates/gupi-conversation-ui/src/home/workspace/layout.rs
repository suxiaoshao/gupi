//! Responsive master/detail allocation inside the stable workspace rectangle.
use super::super::panes::geometry::{DIVIDER, clamp};

pub(in crate::home) const MAIN_MIN_REM: f32 = 20.;
pub(in crate::home) const NAVIGATOR_MIN_REM: f32 = 15.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::home) struct Content {
    pub main: bool,
    pub navigator: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::home) struct Layout {
    pub main_width: f32,
    pub navigator_width: f32,
    pub split: bool,
    pub compact: bool,
}
impl Layout {
    pub(in crate::home) fn new(
        width: f32,
        rem: f32,
        content: Content,
        temporary_list: bool,
        preference: f32,
    ) -> Self {
        let width = width.max(0.);
        let split = content.main
            && content.navigator
            && width >= (MAIN_MIN_REM + NAVIGATOR_MIN_REM) * rem + DIVIDER;
        let compact = content.main && width < (MAIN_MIN_REM + NAVIGATOR_MIN_REM) * rem + DIVIDER;
        if !content.main || (compact && temporary_list) {
            return Self {
                navigator_width: width,
                compact,
                ..Self::default()
            };
        }
        let navigator_width = if split {
            clamp(
                preference,
                NAVIGATOR_MIN_REM * rem,
                width - DIVIDER - MAIN_MIN_REM * rem,
            )
        } else {
            0.
        };
        Self {
            main_width: (width - navigator_width - if split { DIVIDER } else { 0. }).max(0.),
            navigator_width,
            split,
            compact,
        }
    }
    pub fn navigator_visible(self) -> bool {
        self.navigator_width > 0.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn responsive_list_is_independent_of_user_hidden_preference() {
        let content = Content {
            main: true,
            navigator: true,
        };
        let wide = Layout::new(1000., 16., content, false, 320.);
        assert!(wide.split);
        let compact = Layout::new(400., 16., content, false, 320.);
        assert_eq!(compact.main_width, 400.);
        assert!(compact.compact);
        assert_eq!(
            Layout::new(400., 16., content, true, 320.).navigator_width,
            400.
        );
        assert!(
            !Layout::new(
                1000.,
                16.,
                Content {
                    navigator: false,
                    ..content
                },
                false,
                320.
            )
            .split
        );
        assert_eq!(
            Layout::new(
                400.,
                16.,
                Content {
                    main: false,
                    ..content
                },
                false,
                320.
            )
            .navigator_width,
            400.
        );
        for rem in [12., 16., 32.] {
            for width in [0., 0.5, 40., 300., 1200.] {
                for temporary in [false, true] {
                    let value = Layout::new(width, rem, content, temporary, 10000.);
                    assert!(value.main_width.is_finite() && value.navigator_width.is_finite());
                    assert!(value.main_width >= 0. && value.navigator_width >= 0.);
                    assert!(value.main_width + value.navigator_width <= width);
                }
            }
        }
    }
}

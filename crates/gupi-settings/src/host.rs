//! Required synchronous application integration. Install before creating configuration owners.
use crate::config::{AppConfig, ConfigController};
use gpui_kit::{App, Entity, Global, Subscription};

pub struct Host {
    pub(crate) prepare_shortcuts: fn(&AppConfig, &mut App) -> Result<(), String>,
    pub(crate) apply_shortcuts: fn(&AppConfig, &mut App),
    pub(crate) refresh_menus: fn(&mut App),
    pub(crate) writes_blocked: fn(&App) -> bool,
    pub(crate) skipped_version: fn(&App) -> Option<String>,
    pub(crate) observe_admission: fn(&Entity<ConfigController>, &mut App) -> Subscription,
}
impl Global for Host {}
impl Host {
    pub fn new(
        prepare_shortcuts: fn(&AppConfig, &mut App) -> Result<(), String>,
        apply_shortcuts: fn(&AppConfig, &mut App),
        refresh_menus: fn(&mut App),
        writes_blocked: fn(&App) -> bool,
        skipped_version: fn(&App) -> Option<String>,
        observe_admission: fn(&Entity<ConfigController>, &mut App) -> Subscription,
    ) -> Self {
        Self {
            prepare_shortcuts,
            apply_shortcuts,
            refresh_menus,
            writes_blocked,
            skipped_version,
            observe_admission,
        }
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn headless() -> Self {
        Self::new(
            |_, _| Ok(()),
            |_, _| {},
            |_| {},
            |_| false,
            |_| None,
            |_, _| Subscription::new(|| {}),
        )
    }
}
pub(crate) fn prepare_shortcuts(value: &AppConfig, cx: &mut App) -> Result<(), String> {
    (cx.global::<Host>().prepare_shortcuts)(value, cx)
}
pub(crate) fn apply_shortcuts(value: &AppConfig, cx: &mut App) {
    (cx.global::<Host>().apply_shortcuts)(value, cx);
}
pub(crate) fn refresh_menus(cx: &mut App) {
    (cx.global::<Host>().refresh_menus)(cx);
}
pub(crate) fn writes_blocked(cx: &App) -> bool {
    (cx.global::<Host>().writes_blocked)(cx)
}
pub(crate) fn skipped_version(cx: &App) -> Option<String> {
    (cx.global::<Host>().skipped_version)(cx)
}
pub(crate) fn observe_admission(owner: &Entity<ConfigController>, cx: &mut App) -> Subscription {
    (cx.global::<Host>().observe_admission)(owner, cx)
}

#[cfg(test)]
mod tests {
    #[gpui_kit::test]
    fn missing_host_cannot_approve_configuration(cx: &mut gpui_kit::TestAppContext) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cx.update(|cx| super::prepare_shortcuts(&Default::default(), cx))
        }));
        assert!(
            result.is_err(),
            "missing integration must not approve a write"
        );
    }
}

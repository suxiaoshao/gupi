//! Synchronous application integration for preferences and native menus.
use crate::config::AppConfig;
use gpui_kit::{App, Global};

pub struct Host {
    pub prepare_shortcuts: fn(&AppConfig, &mut App) -> Result<(), String>,
    pub apply_shortcuts: fn(&AppConfig, &mut App),
    pub refresh_menus: fn(&mut App),
}
impl Global for Host {}
pub fn prepare_shortcuts(value: &AppConfig, cx: &mut App) -> Result<(), String> {
    if let Some(host) = cx.try_global::<Host>() {
        (host.prepare_shortcuts)(value, cx)
    } else {
        Ok(())
    }
}
pub fn apply_shortcuts(value: &AppConfig, cx: &mut App) {
    if let Some(host) = cx.try_global::<Host>() {
        (host.apply_shortcuts)(value, cx);
    }
}
pub fn refresh_menus(cx: &mut App) {
    if let Some(host) = cx.try_global::<Host>() {
        (host.refresh_menus)(cx);
    }
}

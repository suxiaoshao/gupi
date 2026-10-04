//! Notification policy and transient session notices. No second session lifecycle.
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionMode {
    Off,
    #[default]
    Background,
    Always,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct Preferences {
    pub waiting: bool,
    pub failures: bool,
    pub completion: CompletionMode,
    pub plugins: bool,
    pub attention: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            waiting: true,
            failures: true,
            completion: CompletionMode::Background,
            plugins: false,
            attention: true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryKind {
    Waiting,
    Completed,
    Failed,
    Plugin,
}
impl Preferences {
    pub fn system(&self, kind: &DeliveryKind, foreground: bool, source_visible: bool) -> bool {
        if source_visible {
            return false;
        }
        match kind {
            DeliveryKind::Waiting => self.waiting && !foreground,
            DeliveryKind::Failed => self.failures && !foreground,
            DeliveryKind::Completed => match self.completion {
                CompletionMode::Off => false,
                CompletionMode::Background => !foreground,
                CompletionMode::Always => true,
            },
            DeliveryKind::Plugin => self.plugins && !foreground,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_route_by_app_and_source_visibility() {
        let p = Preferences::default();
        for kind in [
            DeliveryKind::Waiting,
            DeliveryKind::Failed,
            DeliveryKind::Completed,
        ] {
            assert!(p.system(&kind, false, false));
            assert!(!p.system(&kind, true, false));
            assert!(!p.system(&kind, true, true));
        }
        assert!(!p.system(&DeliveryKind::Plugin, false, false));
        let p = Preferences {
            completion: CompletionMode::Always,
            plugins: true,
            ..p
        };
        assert!(p.system(&DeliveryKind::Completed, true, false));
        assert!(!p.system(&DeliveryKind::Completed, true, true));
        assert!(p.system(&DeliveryKind::Plugin, false, false));
    }
    #[test]
    fn old_config_gets_defaults() {
        let p: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(p, Preferences::default());
    }
}

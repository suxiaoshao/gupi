//! Semantic conversation attention events; delivery policy belongs to the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}
impl Severity {
    pub fn from_pi(value: Option<&str>) -> Self {
        match value {
            Some("warning") => Self::Warning,
            Some("error") => Self::Error,
            _ => Self::Info,
        }
    }
}
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct NoticeContent {
    pub id: Option<String>,
    pub message: String,
    pub severity: Severity,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Waiting(String),
    Completed,
    Failed,
    Plugin,
}
impl Kind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Waiting(_) => "notification-waiting",
            Self::Completed => "notification-completed",
            Self::Failed => "notification-failed",
            Self::Plugin => "notification-plugin",
        }
    }
}
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Notice {
    pub key: String,
    pub binding: u64,
    pub kind: Kind,
    pub message: Option<NoticeContent>,
}

impl NoticeContent {
    pub fn new(id: Option<String>, message: String, severity: Severity) -> Self {
        Self {
            id,
            message,
            severity,
        }
    }
}
impl Notice {
    pub fn new(key: String, binding: u64, kind: Kind, message: Option<NoticeContent>) -> Self {
        Self {
            key,
            binding,
            kind,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Severity;
    #[test]
    fn unknown_pi_severity_is_info() {
        assert_eq!(Severity::from_pi(Some("warning")), Severity::Warning);
        assert_eq!(Severity::from_pi(Some("custom")), Severity::Info);
    }
}

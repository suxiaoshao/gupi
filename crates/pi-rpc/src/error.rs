#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    #[error("Pi I/O failed: {0}")]
    Io(String),
    #[error("invalid Pi protocol: {0}")]
    Protocol(String),
    #[error("Pi rejected {command}: {message}")]
    Rejected { command: String, message: String },
    #[error("Pi connection closed")]
    Closed,
    #[error("Pi startup did not become ready before its deadline")]
    StartupTimeout,
    #[error("Pi did not finish graceful shutdown before its deadline")]
    ShutdownTimeout,
    #[error("Pi exited before startup became ready")]
    NotReady,
    #[error("Pi {0} capacity exceeded")]
    Capacity(&'static str),
    #[error("invalid client options: {0}")]
    Options(&'static str),
}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Protocol(error.to_string())
    }
}

impl Error {
    /// Stable diagnostic metadata; never includes protocol or operating-system text.
    pub fn category(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Protocol(_) => "protocol",
            Self::Rejected { .. } => "rejected",
            Self::Closed => "closed",
            Self::StartupTimeout => "startup_timeout",
            Self::ShutdownTimeout => "shutdown_timeout",
            Self::NotReady => "not_ready",
            Self::Capacity(_) => "capacity",
            Self::Options(_) => "options",
        }
    }
}

use std::env;
use std::io;
use std::path::Path;
use std::path::PathBuf;

fn directory(key: &str, fallback: Option<PathBuf>) -> io::Result<PathBuf> {
    let path = env::var_os(key)
        .filter(|v| !v.to_string_lossy().trim().is_empty())
        .map(PathBuf::from)
        .or(fallback)
        .ok_or_else(|| io::Error::other("system directory unavailable"))?;
    std::path::absolute(path)
}
pub fn config_dir() -> io::Result<PathBuf> {
    directory(
        "GUPI_CONFIG_DIR",
        dirs_next::config_dir().map(|p| p.join("gupi")),
    )
}
pub fn log_dir() -> io::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    let fallback = dirs_next::home_dir().map(|p| p.join("Library/Logs/gupi"));
    #[cfg(not(target_os = "macos"))]
    let fallback = dirs_next::data_local_dir().map(|p| p.join("gupi/logs"));
    directory("GUPI_LOG_DIR", fallback)
}

pub fn temporary_dir() -> io::Result<PathBuf> {
    directory(
        "GUPI_DATA_DIR",
        dirs_next::data_local_dir().map(|p| p.join("gupi")),
    )
    .map(|root| root.join("temporary-workspaces"))
}

#[derive(Clone, Debug)]
pub struct Discovery {
    pub home: PathBuf,
    pub agent: PathBuf,
    pub current: PathBuf,
    pub session_override: Option<PathBuf>,
}
impl Discovery {
    pub fn environment() -> io::Result<Self> {
        let home =
            dirs_next::home_dir().ok_or_else(|| io::Error::other("home directory unavailable"))?;
        let current = std::env::current_dir()?;
        let agent = std::env::var("PI_CODING_AGENT_DIR")
            .ok()
            .filter(|v| !v.is_empty())
            .map(|v| expand(&v, &home, &current))
            .unwrap_or_else(|| home.join(".pi/agent"));
        let session_override = std::env::var("PI_CODING_AGENT_SESSION_DIR")
            .ok()
            .filter(|v| !v.is_empty())
            .map(|v| expand(&v, &home, &current));
        Ok(Self {
            home,
            agent,
            current,
            session_override,
        })
    }
}
pub fn expand(value: &str, home: &Path, cwd: &Path) -> PathBuf {
    let path = if value == "~" {
        home.to_path_buf()
    } else if let Some(rest) = value.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(value)
    };
    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

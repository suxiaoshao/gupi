pub const CLEANUP_ARGUMENT: &str = "--gupi-cleanup-update";

// Passed over a private stdin pipe, never a globally writable command file.
#[derive(serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct InstallPlan {
    pub parent: u32,
    pub installer: std::path::PathBuf,
    pub executable: std::path::PathBuf,
    pub log: std::path::PathBuf,
    pub failure_message: String,
}
impl InstallPlan {
    pub fn new(
        parent: u32,
        installer: std::path::PathBuf,
        executable: std::path::PathBuf,
        log: std::path::PathBuf,
        failure_message: String,
    ) -> Self {
        Self {
            parent,
            installer,
            executable,
            log,
            failure_message,
        }
    }
}

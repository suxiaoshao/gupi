// Passed over a private stdin pipe, never a globally writable command file.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct InstallPlan {
    pub parent: u32,
    pub installer: std::path::PathBuf,
    pub executable: std::path::PathBuf,
    pub log: std::path::PathBuf,
    pub failure_message: String,
}

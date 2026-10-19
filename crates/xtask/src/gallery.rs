//! Disposable development fixtures; never reads the user's Pi/Gupi configuration.
use crate::error::Result;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
use crate::error::XtaskError;
use clap::{Args, Subcommand};

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod cleanup;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod launch;

#[derive(Subcommand)]
pub enum GalleryCommand {
    /// Launch the isolated UI extension; exit with Cmd+Q when finished.
    Ui(UiArgs),
    /// Launch the offline runtime provider; files are retained after exit.
    Runtime(RuntimeArgs),
}
#[derive(Args)]
pub struct UiArgs {
    #[arg(long, default_value = "pi")]
    pi: String,
    #[arg(long)]
    no_build: bool,
    #[arg(long)]
    prepare_only: bool,
    #[arg(long)]
    keep: bool,
    /// Remove only named, inactive owned fixtures; do not launch.
    #[arg(long, num_args = 1.., value_name = "ROOT_NAME")]
    cleanup: Vec<String>,
    /// Exclusion takes precedence over explicit cleanup selection.
    #[arg(long, value_name = "ROOT_NAME")]
    exclude: Vec<String>,
}
#[derive(Args)]
pub struct RuntimeArgs {
    #[arg(long)]
    no_build: bool,
    #[arg(long)]
    prepare_only: bool,
    #[arg(long)]
    changes: bool,
}
pub fn run(command: GalleryCommand) -> Result<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        launch::run(command)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = command;
        Err(XtaskError::msg(
            "Gallery supports macOS and Linux only; Windows is not supported",
        ))
    }
}

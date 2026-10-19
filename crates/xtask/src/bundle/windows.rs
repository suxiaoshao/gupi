use std::ffi::OsStr;
use std::path::PathBuf;

use tracing::info;

use crate::cmd::{run_cmd_os, run_cmd_program_os};
use crate::error::{Result, XtaskError};

pub(crate) fn install_windows_artifact(artifacts: &[PathBuf]) -> Result<()> {
    let installer = crate::bundle::preferred_windows_artifact(artifacts)
        .ok_or_else(|| XtaskError::msg("no installer artifact found"))?;
    info!(installer = %installer.display(), "installing artifact");
    if installer.extension().and_then(OsStr::to_str) == Some("msi") {
        let args: Vec<&OsStr> = vec![OsStr::new("/i"), installer.as_os_str()];
        run_cmd_os("msiexec.exe", &args, None)?;
    } else {
        run_cmd_program_os(installer.as_os_str(), &[], None)?;
    }
    Ok(())
}

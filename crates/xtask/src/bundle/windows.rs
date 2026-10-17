#[cfg(target_os = "windows")]
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::{env, process::Command};

#[cfg(target_os = "windows")]
use tracing::info;

#[cfg(target_os = "windows")]
use crate::cmd::{run_cmd_os, run_cmd_program_os};
use crate::error::{Result, XtaskError};

/// Bundle the official CRT family, including the companion DLLs needed by native
/// dependencies. Never copy a developer machine's System32 runtime into an MSI.
#[cfg(target_os = "windows")]
pub(crate) fn prepare_runtime_resources(
    staging: &Path,
    settings: &mut tauri_bundler::BundleSettings,
) -> Result<()> {
    let redist = runtime_redist_root()?;
    stage_runtime_resources(&redist, staging, settings)
}

fn stage_runtime_resources(
    redist: &Path,
    staging: &Path,
    settings: &mut tauri_bundler::BundleSettings,
) -> Result<()> {
    let files = runtime_files(redist)?;
    // tauri-utils normalizes resource paths by dropping Windows drive prefixes.
    // CI checks out on D: while Visual Studio is installed on C:, so resources
    // must be staged on the workspace drive before entering the bundler.
    fs::create_dir_all(staging)?;
    let resources = settings.resources_map.get_or_insert_default();
    for file in files {
        let name = file.file_name().expect("runtime file has a name");
        let staged = staging.join(name);
        fs::copy(&file, &staged)?;
        #[cfg(target_os = "windows")]
        info!(source = %file.display(), "bundling app-local Visual C++ runtime");
        resources.insert(
            staged.to_string_lossy().into_owned(),
            name.to_string_lossy().into_owned(),
        );
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn runtime_redist_root() -> Result<PathBuf> {
    if let Some(root) = env::var_os("VCToolsRedistDir") {
        return Ok(PathBuf::from(root));
    }
    let program_files = env::var_os("ProgramFiles(x86)")
        .ok_or_else(|| XtaskError::msg("missing ProgramFiles(x86); use an MSVC developer shell"))?;
    let vswhere =
        PathBuf::from(program_files).join("Microsoft Visual Studio/Installer/vswhere.exe");
    let output = Command::new(vswhere)
        .args([
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property",
            "installationPath",
            "-utf8",
        ])
        .output()?;
    let installation = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() || installation.trim().is_empty() {
        return Err(XtaskError::msg(
            "Visual C++ redistributable files not found; install the C++ build tools or set VCToolsRedistDir from the matching MSVC developer shell",
        ));
    }
    let vc = Path::new(installation.trim()).join("VC");
    let version =
        fs::read_to_string(vc.join("Auxiliary/Build/Microsoft.VCRedistVersion.default.txt"))?;
    Ok(vc.join("Redist/MSVC").join(version.trim()))
}

fn runtime_files(redist: &Path) -> Result<Vec<PathBuf>> {
    let mut directories = Vec::new();
    for entry in fs::read_dir(redist.join("x64"))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_dir() && name.starts_with("Microsoft.VC") && name.ends_with(".CRT")
        {
            directories.push(entry.path());
        }
    }
    if directories.len() != 1 {
        return Err(XtaskError::msg(format!(
            "expected one x64 Microsoft.VC*.CRT directory under {}, found {}",
            redist.display(),
            directories.len()
        )));
    }
    let directory = &directories[0];
    if !directory.join("vcruntime140.dll").is_file() {
        return Err(XtaskError::msg(format!(
            "{} is missing vcruntime140.dll",
            directory.display()
        )));
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
        {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(target_os = "windows")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_resources_resolve_from_the_workspace_drive() {
        let redist = tempfile::tempdir().unwrap();
        let crt = redist.path().join("x64/Microsoft.VC143.CRT");
        fs::create_dir_all(&crt).unwrap();
        fs::write(crt.join("vcruntime140.dll"), b"official runtime fixture").unwrap();
        // On CI the system temp directory is on C: and the workspace is on D:.
        let staging = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
        let mut settings = tauri_bundler::BundleSettings::default();
        stage_runtime_resources(redist.path(), staging.path(), &mut settings).unwrap();
        let resources = settings.resources_map.as_ref().unwrap();
        let mut resolved = tauri_utils::resources::ResourcePaths::from_map(resources, true).iter();
        let resource = resolved.next().unwrap().unwrap();
        assert_eq!(resource.target(), Path::new("vcruntime140.dll"));
        assert_eq!(
            fs::read(resource.path()).unwrap(),
            b"official runtime fixture"
        );
        assert!(resolved.next().is_none());
    }

    #[test]
    fn runtime_payload_uses_x64_crt_family_only() {
        let root = tempfile::tempdir().unwrap();
        let crt = root.path().join("x64/Microsoft.VC143.CRT");
        fs::create_dir_all(&crt).unwrap();
        for name in [
            "vcruntime140.dll",
            "msvcp140.dll",
            "vcruntime140_1.dll",
            "README.txt",
        ] {
            fs::write(crt.join(name), "fixture").unwrap();
        }
        fs::create_dir_all(root.path().join("x86/Microsoft.VC143.CRT")).unwrap();
        fs::create_dir_all(root.path().join("x64/Microsoft.VC143.OpenMP")).unwrap();
        let files = runtime_files(root.path()).unwrap();
        assert_eq!(files.len(), 3);
        assert!(
            files
                .iter()
                .all(|file| file.parent() == Some(crt.as_path()))
        );
    }

    #[test]
    fn incomplete_or_ambiguous_runtime_fails_packaging() {
        let root = tempfile::tempdir().unwrap();
        let crt = root.path().join("x64/Microsoft.VC143.CRT");
        fs::create_dir_all(&crt).unwrap();
        assert!(runtime_files(root.path()).is_err());
        fs::write(crt.join("vcruntime140.dll"), "fixture").unwrap();
        fs::create_dir_all(root.path().join("x64/Microsoft.VC145.CRT")).unwrap();
        assert!(runtime_files(root.path()).is_err());
    }
}

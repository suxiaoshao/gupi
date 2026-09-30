#[cfg(target_os = "linux")]
use std::{fs, path::Path, process::Command};

use crate::error::{Result, XtaskError};
#[cfg(target_os = "linux")]
use tauri_bundler::BundleSettings;

#[cfg(target_os = "linux")]
pub(crate) fn prepare_deb_dependencies(
    out_dir: &Path,
    binary_name: &str,
    settings: &mut BundleSettings,
) -> Result<()> {
    let binary = out_dir.join(binary_name);
    for args in [vec!["-l"], vec!["-d"]] {
        let output = Command::new("readelf").args(args).arg(&binary).output()?;
        if !output.status.success() {
            return Err(XtaskError::msg(
                "readelf failed to inspect the Linux binary",
            ));
        }
        if String::from_utf8_lossy(&output.stdout).contains("/nix/store/") {
            return Err(XtaskError::msg(
                "Linux release binaries must use native system libraries; build outside Nix to remove /nix/store interpreter/RPATH dependencies",
            ));
        }
    }

    // dpkg-shlibdeps needs a source control file, even for stdout-only output.
    let debian = out_dir.join("debian");
    fs::create_dir_all(&debian)?;
    fs::write(
        debian.join("control"),
        "Source: gupi\nSection: devel\nPriority: optional\nMaintainer: Sushao <48886207+suxiaoshao@users.noreply.github.com>\n\nPackage: gupi\nArchitecture: any\nDescription: A GPUI desktop host for Pi\n",
    )?;
    let output = Command::new("dpkg-shlibdeps")
        .args(["--ignore-missing-info", "-O"])
        .arg(&binary)
        .current_dir(out_dir)
        .output()?;
    if !output.status.success() {
        return Err(XtaskError::msg(format!(
            "dpkg-shlibdeps failed (install dpkg-dev and native library packages): {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let mut depends = parse_shlib_dependencies(&String::from_utf8_lossy(&output.stdout))?;
    // These graphics libraries are loaded dynamically and invisible to shlibdeps.
    depends.extend([
        "libvulkan1".to_owned(),
        "libegl1".to_owned(),
        "libwayland-client0".to_owned(),
        "libwayland-egl1".to_owned(),
    ]);
    depends.extend(settings.deb.depends.take().unwrap_or_default());
    depends.sort();
    depends.dedup();
    settings.deb.depends = Some(depends);
    Ok(())
}

fn parse_shlib_dependencies(output: &str) -> Result<Vec<String>> {
    let value = output
        .lines()
        .find_map(|line| line.strip_prefix("shlibs:Depends="))
        .ok_or_else(|| XtaskError::msg("dpkg-shlibdeps did not report shlibs:Depends"))?;
    let dependencies = value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if dependencies.is_empty() {
        return Err(XtaskError::msg(
            "dpkg-shlibdeps reported no runtime dependencies",
        ));
    }
    Ok(dependencies)
}

#[cfg(test)]
mod tests {
    use super::parse_shlib_dependencies;

    #[test]
    fn deb_dependencies_retain_symbol_version_constraints() {
        let deps = parse_shlib_dependencies(
            "shlibs:Depends=libc6 (>= 2.35), libfontconfig1 (>= 2.12.6)\n",
        )
        .unwrap();
        assert_eq!(deps, ["libc6 (>= 2.35)", "libfontconfig1 (>= 2.12.6)"]);
        assert!(parse_shlib_dependencies("shlibs:Depends=\n").is_err());
        assert!(parse_shlib_dependencies("other:value=libc6\n").is_err());
    }
}

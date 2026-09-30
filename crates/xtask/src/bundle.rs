use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub mod common;
#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod settings;
#[cfg(target_os = "windows")]
pub mod windows;

use crate::cli::{BundleArgs, MacosSigning};
use crate::cmd::run_cmd;
use crate::context::workspace_root;
use crate::error::{Result, XtaskError};
use crate::manifest::get_main_binary_name;
use tauri_bundler::{BundleBinary, PackageType, SettingsBuilder};
use tracing::info;

pub fn run(args: BundleArgs) -> Result<()> {
    args.validate_signing()?;
    #[cfg(not(target_os = "macos"))]
    if args.macos_signing != MacosSigning::Development {
        return Err(XtaskError::msg("macOS signing requires a macOS host"));
    }
    #[cfg(not(target_os = "windows"))]
    if args.install {
        return Err(XtaskError::msg("--install requires a Windows host"));
    }

    let project_dir = workspace_root()?;
    let target = args.target.as_deref().unwrap_or(native_target()?);
    validate_target(target)?;
    let target_root = resolve_target_root(&project_dir);
    let manifest_path = project_dir.join("Cargo.toml");
    let main_bin_name = get_main_binary_name(&manifest_path)?;
    let (package_settings, mut bundle_settings, localizations) =
        settings::read_bundle_settings(&manifest_path)?;
    let bundle_icons = common::prepare_bundle_icons(&project_dir)?;
    bundle_icons.apply_to_bundle_settings(&mut bundle_settings);
    let product_name = package_settings.product_name.clone();
    let version = package_settings.version.clone();
    let updater_key = crate::updater::public_key()?;

    #[cfg(target_os = "macos")]
    macos::prepare_bundle_settings(&mut bundle_settings, &localizations)?;
    #[cfg(not(target_os = "macos"))]
    let _ = localizations;

    run_cmd(
        "cargo",
        &[
            "build",
            "-p",
            "gupi",
            "--bin",
            "gupi",
            "--release",
            "--locked",
            "--features",
            "bundled",
            "--target",
            target,
        ],
        Some(&project_dir),
    )?;

    #[cfg(target_os = "windows")]
    if updater_key.is_some() {
        run_cmd(
            "cargo",
            &[
                "build",
                "-p",
                "gupi",
                "--bin",
                "gupi-update-helper",
                "--release",
                "--locked",
                "--features",
                "bundled",
                "--target",
                target,
            ],
            Some(&project_dir),
        )?;
        let sdk = crate::updater::sdk(&project_dir, "windows")?.join("WinSparkle-0.9.4");
        let resources = bundle_settings.resources_map.get_or_insert_default();
        resources.insert(
            sdk.join("x64/Release/WinSparkle.dll")
                .to_string_lossy()
                .into_owned(),
            "WinSparkle.dll".into(),
        );
        resources.insert(
            target_root
                .join(target)
                .join("release/gupi-update-helper.exe")
                .to_string_lossy()
                .into_owned(),
            "gupi-update-helper.exe".into(),
        );
        resources.insert(
            sdk.join("COPYING").to_string_lossy().into_owned(),
            "WinSparkle-COPYING.txt".into(),
        );
        resources.insert(
            sdk.join("COPYING.expat").to_string_lossy().into_owned(),
            "WinSparkle-LICENSE.txt".into(),
        );
    }
    #[cfg(target_os = "linux")]
    let _ = updater_key;

    let out_dir = prepare_bundle_staging(&target_root, target, &main_bin_name)?;
    #[cfg(target_os = "macos")]
    macos::verify_native_libraries(&out_dir.join(&main_bin_name))?;
    #[cfg(target_os = "linux")]
    linux::prepare_deb_dependencies(&out_dir, &main_bin_name, &mut bundle_settings)?;

    let mut settings_builder = SettingsBuilder::new()
        .project_out_directory(&out_dir)
        .target(target.to_owned())
        .package_types(default_package_types())
        .package_settings(package_settings)
        .bundle_settings(bundle_settings)
        .binaries(vec![BundleBinary::new(main_bin_name, true)])
        // Gupi does not embed Tauri's bundle-type marker.
        .binary_patching(false)
        // All macOS mutations must precede our final signature and notarization.
        .no_sign(cfg!(target_os = "macos"));

    if let Some(local_tools_dir) = env::var_os("TAURI_BUNDLER_TOOLS_DIR") {
        settings_builder = settings_builder.local_tools_directory(local_tools_dir);
    }
    let settings = settings_builder
        .build()
        .map_err(|err| XtaskError::msg(format!("failed to build bundle settings: {err}")))?;
    let bundles = tauri_bundler::bundle_project(&settings)
        .map_err(|err| XtaskError::msg(format!("failed to bundle Gupi: {err}")))?;
    let dist_dir = project_dir.join("dist").join(target);

    #[cfg(target_os = "macos")]
    {
        let app_path = macos::find_app_bundle(&out_dir.join("bundle"), &product_name)?
            .ok_or_else(|| XtaskError::msg("bundler did not produce a macOS .app"))?;
        if let Some(key) = updater_key {
            macos::embed_updater(&project_dir, &app_path, target, &key)?;
        }
        macos::inject_liquid_glass_icon(&project_dir, &app_path, &bundle_icons)?;
        macos::finalize_codesign(&project_dir, &app_path, &args)?;
        reset_directory(&dist_dir)?;
        let suffix = if args.macos_signing == MacosSigning::Development {
            "_development"
        } else {
            ""
        };
        let arch = target.split('-').next().unwrap_or(target);
        let archive = dist_dir.join(format!("{product_name}_{version}_{arch}_macos{suffix}.zip"));
        macos::archive_app(&app_path, &archive)?;
        info!(artifact = %archive.display());
        let dmg = archive.with_extension("dmg");
        macos::create_dmg(&app_path, &dmg, &product_name, &args)?;
        info!(artifact = %dmg.display());
        let _ = bundles;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (product_name, version);
        let artifacts: Vec<PathBuf> = bundles
            .into_iter()
            .flat_map(|bundle| bundle.bundle_paths)
            .filter(|path| path.is_file())
            .collect();
        if artifacts.is_empty() {
            return Err(XtaskError::msg("bundler did not produce an installer"));
        }
        reset_directory(&dist_dir)?;
        for artifact in &artifacts {
            let filename = artifact
                .file_name()
                .ok_or_else(|| XtaskError::msg("installer has no filename"))?;
            let destination = dist_dir.join(filename);
            fs::copy(artifact, &destination)?;
            info!(artifact = %destination.display());
        }
        #[cfg(target_os = "windows")]
        if args.install {
            windows::install_windows_artifact(&artifacts)?;
        }
    }

    info!(target, dist = %dist_dir.display(), "Gupi packaging complete");
    Ok(())
}

fn native_target() -> Result<&'static str> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        _ => Err(XtaskError::msg("unsupported packaging host architecture")),
    }
}

fn validate_target(target: &str) -> Result<()> {
    let supported = match env::consts::OS {
        "macos" => ["aarch64-apple-darwin", "x86_64-apple-darwin"].as_slice(),
        "windows" => ["x86_64-pc-windows-msvc"].as_slice(),
        "linux" => ["x86_64-unknown-linux-gnu"].as_slice(),
        _ => [].as_slice(),
    };
    if !supported.contains(&target) {
        return Err(XtaskError::msg(format!(
            "target {target} cannot be packaged on this host; supported: {}",
            supported.join(", ")
        )));
    }
    Ok(())
}

fn resolve_target_root(project_dir: &Path) -> PathBuf {
    target_root_from(project_dir, env::var_os("CARGO_TARGET_DIR").as_deref())
}

fn target_root_from(project_dir: &Path, configured: Option<&OsStr>) -> PathBuf {
    match configured.map(Path::new) {
        Some(path) if path.is_absolute() => path.to_owned(),
        Some(path) => project_dir.join(path),
        None => project_dir.join("target"),
    }
}

fn reset_directory(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn prepare_bundle_staging(
    target_root: &Path,
    target: &str,
    main_bin_name: &str,
) -> Result<PathBuf> {
    let build_out_dir = target_root.join(target).join("release");
    let staging_out_dir = target_root
        .join("xtask-bundle")
        .join(target)
        .join("release");
    let filename = format!("{main_bin_name}{}", env::consts::EXE_SUFFIX);
    let source = build_out_dir.join(&filename);
    if !source.is_file() {
        return Err(XtaskError::msg(format!(
            "failed to find built binary {}",
            source.display()
        )));
    }
    reset_directory(&staging_out_dir)?;
    fs::copy(&source, staging_out_dir.join(filename))?;
    let webview2_loader = build_out_dir.join("WebView2Loader.dll");
    if webview2_loader.is_file() {
        fs::copy(webview2_loader, staging_out_dir.join("WebView2Loader.dll"))?;
    }
    Ok(staging_out_dir)
}

#[cfg(any(target_os = "windows", test))]
pub(crate) fn preferred_windows_artifact(artifacts: &[PathBuf]) -> Option<&PathBuf> {
    artifacts
        .iter()
        .find(|path| {
            path.extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| extension.eq_ignore_ascii_case("msi"))
                && path
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .and_then(|stem| stem.rsplit('_').next())
                    .is_some_and(|locale| locale.eq_ignore_ascii_case("en-US"))
        })
        .or_else(|| {
            artifacts
                .iter()
                .find(|path| path.extension() == Some(OsStr::new("msi")))
        })
        .or_else(|| artifacts.first())
}

fn default_package_types() -> Vec<PackageType> {
    #[cfg(target_os = "macos")]
    return vec![PackageType::MacOsBundle];
    #[cfg(target_os = "linux")]
    return vec![PackageType::Deb];
    #[cfg(target_os = "windows")]
    return vec![PackageType::WindowsMsi];
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_directory_resolves_relative_and_absolute_configuration() {
        let root = if cfg!(windows) {
            Path::new(r"C:\project")
        } else {
            Path::new("/project")
        };
        assert_eq!(target_root_from(root, None), root.join("target"));
        assert_eq!(
            target_root_from(root, Some(OsStr::new("build"))),
            root.join("build")
        );
        let absolute = root.join("elsewhere");
        assert_eq!(target_root_from(root, Some(absolute.as_os_str())), absolute);
    }

    #[test]
    fn localized_install_prefers_english_and_falls_back_to_an_msi() {
        let localized = vec![
            PathBuf::from("Gupi_0.1.0_x64_de-DE.msi"),
            PathBuf::from("Gupi_0.1.0_x64_en-US.msi"),
            PathBuf::from("Gupi_0.1.0_x64.exe"),
        ];
        assert_eq!(preferred_windows_artifact(&localized), Some(&localized[1]));
        let fallback = vec![
            PathBuf::from("Gupi_0.1.0_x64.exe"),
            PathBuf::from("Gupi_0.1.0_x64_zh-CN.msi"),
        ];
        assert_eq!(preferred_windows_artifact(&fallback), Some(&fallback[1]));
    }
}

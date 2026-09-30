use std::ffi::OsStr;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

use crate::bundle::{common::BundleIconAssets, settings::BundleLocalization};
use crate::cli::{BundleArgs, MacosSigning};
use crate::cmd::{command_exists, run_cmd_os};
use crate::error::{Result, XtaskError};
use tauri_bundler::{BundleSettings, PlistKind};

pub(crate) fn verify_native_libraries(binary: &Path) -> Result<()> {
    for flag in ["-L", "-l"] {
        let output = Command::new("otool").arg(flag).arg(binary).output()?;
        if !output.status.success() {
            return Err(XtaskError::msg("otool failed to inspect the macOS binary"));
        }
        if String::from_utf8_lossy(&output.stdout).contains("/nix/store/") {
            return Err(XtaskError::msg(
                "macOS release binaries must use native system libraries; build with rustup and Xcode outside Nix to remove /nix/store dependencies",
            ));
        }
    }
    Ok(())
}

pub fn prepare_bundle_settings(
    bundle_settings: &mut BundleSettings,
    localizations: &[BundleLocalization],
) -> Result<()> {
    bundle_settings.macos.info_plist = Some(PlistKind::Plist(
        bundle_info_plist_overrides(localizations).into(),
    ));
    bundle_settings.macos.minimum_system_version =
        Some(std::env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "11.0".to_owned()));
    Ok(())
}

pub fn find_app_bundle(bundle_dir: &Path, product_name: &str) -> Result<Option<PathBuf>> {
    let app_name = format!("{product_name}.app");
    for bundle_subdir in ["macos", "osx"] {
        let app_bundle_dir = bundle_dir.join(bundle_subdir);
        let app_path = app_bundle_dir.join(&app_name);
        if app_path.is_dir() {
            return Ok(Some(app_path));
        }
    }

    Ok(None)
}

pub(crate) fn embed_updater(root: &Path, app: &Path, target: &str, key: &str) -> Result<()> {
    let sdk = crate::updater::sdk(root, "macos")?;
    let destination = app.join("Contents/Frameworks/Sparkle.framework");
    fs::create_dir_all(destination.parent().unwrap())?;
    run_cmd_os(
        "ditto",
        &[
            sdk.join("Sparkle.framework").as_os_str(),
            destination.as_os_str(),
        ],
        None,
    )?;
    fs::copy(
        sdk.join("LICENSE"),
        app.join("Contents/Resources/Sparkle-LICENSE.txt"),
    )?;
    let path = app.join("Contents/Info.plist");
    let mut value = plist::Value::from_file(&path)?;
    let dict = value
        .as_dictionary_mut()
        .ok_or_else(|| XtaskError::msg("invalid app plist"))?;
    dict.insert("SUPublicEDKey".into(), plist::Value::String(key.into()));
    let arch = target.split('-').next().unwrap();
    dict.insert(
        "SUFeedURL".into(),
        plist::Value::String(format!(
            "https://github.com/suxiaoshao/gupi/releases/latest/download/appcast-macos-{arch}.xml"
        )),
    );
    for name in [
        "SUEnableAutomaticChecks",
        "SUAutomaticallyUpdate",
        "SUAllowsAutomaticUpdates",
        "SUSendProfileInfo",
        "SUShowReleaseNotes",
    ] {
        dict.insert(name.into(), plist::Value::Boolean(false));
    }
    value.to_file_xml(path)?;
    Ok(())
}

pub fn inject_liquid_glass_icon(
    app_dir: &Path,
    app_path: &Path,
    bundle_icon_assets: &BundleIconAssets,
) -> Result<()> {
    let Some((icon_dir, icon_dirs)) = find_liquid_glass_icon_dirs(app_dir)? else {
        warn!(app_dir = %app_dir.display(), "未找到 .icon 目录，跳过 Liquid Glass 图标注入");
        return Ok(());
    };
    let icon_name = icon_dir
        .file_stem()
        .and_then(|x| x.to_str())
        .filter(|x| !x.is_empty())
        .unwrap_or("Icon");

    if !command_exists("xcrun") {
        warn!("未找到 xcrun，跳过 Liquid Glass 图标注入（保留普通图标）");
        return Ok(());
    }

    let staging = tempfile::Builder::new()
        .prefix("xtask-bundle-assets-")
        .tempdir()
        .map_err(|err| {
            XtaskError::msg(format!("failed to create icon compiler directory: {err}"))
        })?;
    let tmp_dir = staging.path();
    let actool_source_dir = tmp_dir.join("source");
    let actool_output_dir = tmp_dir.join("output");
    fs::create_dir_all(&actool_output_dir).map_err(|err| {
        XtaskError::msg(format!(
            "failed to create actool output dir {}: {err}",
            actool_output_dir.display()
        ))
    })?;
    let staged_icons = icon_dirs
        .iter()
        .map(|icon| {
            stage_liquid_glass_icon_dir(
                icon,
                &bundle_icon_assets.source_base_icon(),
                &actool_source_dir,
            )
        })
        .collect::<Result<Vec<_>>>()?;

    let actool_plist = actool_output_dir.join("assetcatalog_generated_info.plist");
    let mut actool_args: Vec<&OsStr> = vec![OsStr::new("actool")];
    actool_args.extend(staged_icons.iter().map(|icon| icon.as_os_str()));
    actool_args.extend([
        OsStr::new("--compile"),
        actool_output_dir.as_os_str(),
        OsStr::new("--output-format"),
        OsStr::new("human-readable-text"),
        OsStr::new("--notices"),
        OsStr::new("--warnings"),
        OsStr::new("--errors"),
        OsStr::new("--output-partial-info-plist"),
        actool_plist.as_os_str(),
        OsStr::new("--app-icon"),
        OsStr::new(icon_name),
        OsStr::new("--include-all-app-icons"),
        OsStr::new("--enable-on-demand-resources"),
        OsStr::new("NO"),
        OsStr::new("--development-region"),
        OsStr::new("en"),
        OsStr::new("--target-device"),
        OsStr::new("mac"),
        OsStr::new("--platform"),
        OsStr::new("macosx"),
        OsStr::new("--minimum-deployment-target"),
        OsStr::new("26.0"),
    ]);

    let actool_result = run_cmd_os("xcrun", &actool_args, None);

    if let Err(err) = actool_result {
        warn!(error = %err, "actool 编译失败，跳过 Liquid Glass 图标注入（保留普通图标）");
        return Ok(());
    }

    let assets_car = actool_output_dir.join("Assets.car");
    if !assets_car.exists() {
        warn!("未生成 Assets.car，跳过 Liquid Glass 图标注入（保留普通图标）");
        return Ok(());
    }

    let target_assets = app_path.join("Contents/Resources/Assets.car");
    fs::copy(&assets_car, &target_assets).map_err(|err| {
        XtaskError::msg(format!(
            "failed to copy {} to {}: {err}",
            assets_car.display(),
            target_assets.display()
        ))
    })?;

    let plist = app_path.join("Contents/Info.plist");
    update_bundle_icon_name(&plist, icon_name)?;

    info!(app_path = %app_path.display(), "已注入 Liquid Glass 图标");
    Ok(())
}

/// Finalize the signature after all bundle resources have been written.
pub(crate) fn finalize_codesign(
    project_dir: &Path,
    app_path: &Path,
    args: &BundleArgs,
) -> Result<()> {
    if !command_exists("codesign") {
        return Err(XtaskError::msg(
            "codesign is required to finalize a macOS application bundle",
        ));
    }

    match args.macos_signing {
        MacosSigning::Development => {
            run_cmd_os(
                "codesign",
                &[
                    OsStr::new("--force"),
                    OsStr::new("--deep"),
                    OsStr::new("--sign"),
                    OsStr::new("-"),
                    app_path.as_os_str(),
                ],
                None,
            )?;
            info!("applied development ad-hoc signature (not notarized)");
        }
        MacosSigning::DeveloperId => sign_developer_id(project_dir, app_path, args)?,
    }

    let verify_args: Vec<&OsStr> = vec![
        OsStr::new("--verify"),
        OsStr::new("--deep"),
        OsStr::new("--strict"),
        app_path.as_os_str(),
    ];
    run_cmd_os("codesign", &verify_args, None)?;
    if args.macos_signing == MacosSigning::DeveloperId {
        notarize_and_staple(app_path, args)?;
    }
    Ok(())
}

pub(crate) fn archive_app(app_path: &Path, archive: &Path) -> Result<()> {
    run_cmd_os(
        "ditto",
        &[
            OsStr::new("-c"),
            OsStr::new("-k"),
            OsStr::new("--sequesterRsrc"),
            OsStr::new("--keepParent"),
            app_path.as_os_str(),
            archive.as_os_str(),
        ],
        None,
    )
}

/// Package the finalized app, keeping all mutations before the outer DMG signature.
pub(crate) fn create_dmg(
    app_path: &Path,
    dmg: &Path,
    product_name: &str,
    args: &BundleArgs,
) -> Result<()> {
    let staging = tempfile::Builder::new().prefix("gupi-dmg-").tempdir()?;
    let staged_app = staging.path().join(format!("{product_name}.app"));
    run_cmd_os(
        "ditto",
        &[app_path.as_os_str(), staged_app.as_os_str()],
        None,
    )?;
    std::os::unix::fs::symlink("/Applications", staging.path().join("Applications"))?;
    run_cmd_os(
        "hdiutil",
        &[
            OsStr::new("create"),
            OsStr::new("-volname"),
            OsStr::new(product_name),
            OsStr::new("-srcfolder"),
            staging.path().as_os_str(),
            OsStr::new("-fs"),
            OsStr::new("HFS+"),
            OsStr::new("-format"),
            OsStr::new("UDZO"),
            dmg.as_os_str(),
        ],
        None,
    )?;
    run_cmd_os("hdiutil", &[OsStr::new("verify"), dmg.as_os_str()], None)?;
    if args.macos_signing == MacosSigning::DeveloperId {
        let mut sign_args = vec![
            OsStr::new("--force"),
            OsStr::new("--timestamp"),
            OsStr::new("--sign"),
            OsStr::new(
                args.signing_identity
                    .as_deref()
                    .expect("validated identity"),
            ),
        ];
        if let Some(keychain) = &args.keychain {
            sign_args.extend([OsStr::new("--keychain"), keychain.as_os_str()]);
        }
        sign_args.push(dmg.as_os_str());
        run_cmd_os("codesign", &sign_args, None)?;
        run_cmd_os(
            "codesign",
            &[
                OsStr::new("--verify"),
                OsStr::new("--strict"),
                dmg.as_os_str(),
            ],
            None,
        )?;
        notarize(dmg, args)?;
        staple(dmg)?;
        run_cmd_os(
            "spctl",
            &[
                OsStr::new("--assess"),
                OsStr::new("--type"),
                OsStr::new("open"),
                OsStr::new("--context"),
                OsStr::new("context:primary-signature"),
                dmg.as_os_str(),
            ],
            None,
        )?;
    }
    Ok(())
}

fn sign_developer_id(project_dir: &Path, app_path: &Path, args: &BundleArgs) -> Result<()> {
    run_cmd_os("xattr", &[OsStr::new("-cr"), app_path.as_os_str()], None)?;
    let entitlements = args
        .entitlements
        .as_ref()
        .map(|path| project_dir.join(path));
    if let Some(path) = &entitlements {
        if !path.is_file() {
            return Err(XtaskError::msg(format!(
                "missing entitlements {}",
                path.display()
            )));
        }
        plist::Value::from_file(path)?;
    }

    // Sign actual Mach-O files and nested code bundles from the inside out.
    // --deep is reserved for verification, not Developer ID signing.
    let mut targets = Vec::new();
    for entry in walkdir::WalkDir::new(app_path) {
        let entry =
            entry.map_err(|err| XtaskError::msg(format!("failed to inspect app: {err}")))?;
        let path = entry.path();
        if path == app_path || entry.file_type().is_symlink() {
            continue;
        }
        if entry.file_type().is_file() {
            let mut magic = [0; 4];
            let mut file = fs::File::open(path)?;
            if file.read(&mut magic)? == magic.len() && is_macho_magic(magic) {
                targets.push(path.to_owned());
            }
        } else if entry.file_type().is_dir()
            && path
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|ext| matches!(ext, "app" | "framework" | "xpc"))
        {
            targets.push(path.to_owned());
        }
    }
    targets.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    targets.push(app_path.to_owned());
    for target in targets {
        let mut sign_args = vec![
            OsStr::new("--force"),
            OsStr::new("--timestamp"),
            OsStr::new("--options"),
            OsStr::new("runtime"),
            OsStr::new("--sign"),
            OsStr::new(
                args.signing_identity
                    .as_deref()
                    .expect("validated identity"),
            ),
        ];
        if let Some(keychain) = &args.keychain {
            sign_args.extend([OsStr::new("--keychain"), keychain.as_os_str()]);
        }
        if target == app_path
            && let Some(path) = &entitlements
        {
            sign_args.extend([OsStr::new("--entitlements"), path.as_os_str()]);
        } else {
            // Keep native updater helpers' own entitlements; application
            // entitlements must not be applied to third-party nested code.
            sign_args.push(OsStr::new("--preserve-metadata=entitlements"));
        }
        sign_args.push(target.as_os_str());
        run_cmd_os("codesign", &sign_args, None)?;
    }
    Ok(())
}

fn is_macho_magic(magic: [u8; 4]) -> bool {
    matches!(
        u32::from_be_bytes(magic),
        0xfeedface
            | 0xcefaedfe
            | 0xfeedfacf
            | 0xcffaedfe
            | 0xcafebabe
            | 0xbebafeca
            | 0xcafebabf
            | 0xbfbafeca
    )
}

fn notarize_and_staple(app_path: &Path, args: &BundleArgs) -> Result<()> {
    let staging = tempfile::Builder::new()
        .prefix("gupi-notarization-")
        .tempdir()?;
    let archive = staging.path().join("Gupi.zip");
    archive_app(app_path, &archive)?;
    notarize(&archive, args)?;
    staple(app_path)?;
    run_cmd_os(
        "spctl",
        &[
            OsStr::new("--assess"),
            OsStr::new("--type"),
            OsStr::new("execute"),
            app_path.as_os_str(),
        ],
        None,
    )
}

fn notarize(archive: &Path, args: &BundleArgs) -> Result<()> {
    let mut notary_args = vec![
        OsStr::new("notarytool"),
        OsStr::new("submit"),
        archive.as_os_str(),
        OsStr::new("--keychain-profile"),
        OsStr::new(args.notary_profile.as_deref().expect("validated profile")),
        OsStr::new("--wait"),
        OsStr::new("--timeout"),
        OsStr::new("30m"),
        OsStr::new("--output-format"),
        OsStr::new("plist"),
    ];
    if let Some(keychain) = &args.keychain {
        notary_args.extend([OsStr::new("--keychain"), keychain.as_os_str()]);
    }
    let output = Command::new("xcrun").args(notary_args).output()?;
    if !output.status.success() {
        return Err(XtaskError::msg(format!(
            "notarytool submission failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let response = plist::Value::from_reader(Cursor::new(&output.stdout))?;
    require_accepted_notarization(&response)
}

fn staple(artifact: &Path) -> Result<()> {
    run_cmd_os(
        "xcrun",
        &[
            OsStr::new("stapler"),
            OsStr::new("staple"),
            artifact.as_os_str(),
        ],
        None,
    )?;
    run_cmd_os(
        "xcrun",
        &[
            OsStr::new("stapler"),
            OsStr::new("validate"),
            artifact.as_os_str(),
        ],
        None,
    )
}

fn require_accepted_notarization(response: &plist::Value) -> Result<()> {
    let dict = response.as_dictionary();
    let status = dict
        .and_then(|dict| dict.get("status"))
        .and_then(plist::Value::as_string);
    if status != Some("Accepted") {
        let id = dict
            .and_then(|dict| dict.get("id"))
            .and_then(plist::Value::as_string)
            .unwrap_or("unknown");
        return Err(XtaskError::msg(format!(
            "notarization {id} was not accepted (status: {}); inspect it with xcrun notarytool log",
            status.unwrap_or("missing")
        )));
    }
    Ok(())
}

fn stage_liquid_glass_icon_dir(
    source_icon_dir: &Path,
    source_base_icon: &Path,
    stage_root: &Path,
) -> Result<PathBuf> {
    let icon_dir_name = source_icon_dir.file_name().ok_or_else(|| {
        XtaskError::msg(format!(
            "failed to resolve .icon dir name for {}",
            source_icon_dir.display()
        ))
    })?;
    let staged_icon_dir = stage_root.join(icon_dir_name);
    copy_dir_all(source_icon_dir, &staged_icon_dir)?;

    let staged_assets_dir = staged_icon_dir.join("Assets");
    fs::create_dir_all(&staged_assets_dir).map_err(|err| {
        XtaskError::msg(format!(
            "failed to create Liquid Glass icon assets dir {}: {err}",
            staged_assets_dir.display()
        ))
    })?;

    let staged_layer = staged_assets_dir.join("app-icon-liquid-glass.png");
    fs::copy(source_base_icon, &staged_layer).map_err(|err| {
        XtaskError::msg(format!(
            "failed to stage Liquid Glass icon layer {} from {}: {err}",
            staged_layer.display(),
            source_base_icon.display()
        ))
    })?;

    Ok(staged_icon_dir)
}

fn copy_dir_all(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target).map_err(|err| {
        XtaskError::msg(format!(
            "failed to create directory {}: {err}",
            target.display()
        ))
    })?;

    for entry in fs::read_dir(source)
        .map_err(|err| XtaskError::msg(format!("failed to read {}: {err}", source.display())))?
    {
        let entry = entry.map_err(|err| {
            XtaskError::msg(format!(
                "failed to read entry under {}: {err}",
                source.display()
            ))
        })?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_all(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path).map_err(|err| {
                XtaskError::msg(format!(
                    "failed to copy {} to {}: {err}",
                    source_path.display(),
                    target_path.display()
                ))
            })?;
        }
    }

    Ok(())
}

fn find_liquid_glass_icon_dirs(app_dir: &Path) -> Result<Option<(PathBuf, Vec<PathBuf>)>> {
    let icon_root = app_dir.join("build-assets/icon");
    if !icon_root.exists() {
        return Ok(None);
    }

    let mut icon_dirs = Vec::new();
    for entry in fs::read_dir(&icon_root)
        .map_err(|err| XtaskError::msg(format!("failed to read {}: {err}", icon_root.display())))?
    {
        let path = entry
            .map_err(|err| XtaskError::msg(format!("failed to read icon dir entry: {err}")))?
            .path();
        if path.is_dir() && path.extension().and_then(OsStr::to_str) == Some("icon") {
            icon_dirs.push(path);
        }
    }

    icon_dirs.sort();
    if icon_dirs.is_empty() {
        return Ok(None);
    }
    let selection = icon_root.join("default-icon");
    let default = if selection.exists() {
        let name = fs::read_to_string(&selection)?;
        icon_dirs
            .iter()
            .find(|path| path.file_stem().and_then(OsStr::to_str) == Some(name.trim()))
            .cloned()
            .ok_or_else(|| XtaskError::msg("default-icon must name an existing .icon directory"))?
    } else if icon_dirs.len() == 1 {
        icon_dirs[0].clone()
    } else {
        return Err(XtaskError::msg(
            "multiple .icon directories require build-assets/icon/default-icon",
        ));
    };
    Ok(Some((default, icon_dirs)))
}

fn update_bundle_icon_name(plist_path: &Path, icon_name: &str) -> Result<()> {
    let mut value = plist::Value::from_file(plist_path)?;
    let dict = value.as_dictionary_mut().ok_or_else(|| {
        XtaskError::msg(format!(
            "unexpected plist root type for {}: expected dictionary",
            plist_path.display()
        ))
    })?;
    dict.insert(
        "CFBundleIconName".to_string(),
        plist::Value::String(icon_name.to_string()),
    );
    value.to_file_xml(plist_path)?;
    Ok(())
}

fn bundle_info_plist_overrides(localizations: &[BundleLocalization]) -> plist::Dictionary {
    let mut dict = plist::Dictionary::new();
    dict.insert(
        "CFBundleDevelopmentRegion".to_string(),
        plist::Value::String("en".to_string()),
    );
    dict.insert(
        "CFBundleAllowMixedLocalizations".to_string(),
        plist::Value::Boolean(true),
    );
    dict.insert(
        "CFBundleLocalizations".to_string(),
        plist::Value::Array(
            localizations
                .iter()
                .map(|localization| {
                    plist::Value::String(localization.bundle_locale_tag.to_string())
                })
                .collect(),
        ),
    );
    dict
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::settings::resolve_bundle_localizations;

    #[test]
    fn a_successful_submission_must_also_have_an_accepted_status() {
        for status in ["Invalid", "In Progress"] {
            let mut dict = plist::Dictionary::new();
            dict.insert("status".into(), plist::Value::String(status.into()));
            assert!(require_accepted_notarization(&dict.into()).is_err());
        }
        assert!(require_accepted_notarization(&plist::Dictionary::new().into()).is_err());
        let mut accepted = plist::Dictionary::new();
        accepted.insert("status".into(), plist::Value::String("Accepted".into()));
        assert!(require_accepted_notarization(&accepted.into()).is_ok());
    }

    #[test]
    fn plist_overrides_include_bundle_localizations() {
        let localizations = resolve_bundle_localizations(None).unwrap();
        let dict = bundle_info_plist_overrides(&localizations);

        assert_eq!(
            dict.get("CFBundleDevelopmentRegion"),
            Some(&plist::Value::String("en".to_string()))
        );
        assert_eq!(
            dict.get("CFBundleAllowMixedLocalizations"),
            Some(&plist::Value::Boolean(true))
        );
        assert_eq!(
            dict.get("CFBundleLocalizations"),
            Some(&plist::Value::Array(vec![
                plist::Value::String("en".to_string()),
                plist::Value::String("zh_CN".to_string()),
            ]))
        );
    }
}

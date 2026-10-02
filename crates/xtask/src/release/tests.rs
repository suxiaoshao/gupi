use super::*;

// LCIDs come from build-assets/locales/wix/*.wxl, including regional variants.
const LOCALES: [(&str, u16); 9] = [
    ("en-US", 1033),
    ("zh-CN", 2052),
    ("zh-TW", 1028),
    ("ja-JP", 1041),
    ("ko-KR", 1042),
    ("de-DE", 1031),
    ("fr-FR", 1036),
    ("es-ES", 3082),
    ("pt-BR", 1046),
];

fn release() -> Release {
    Release {
        version: "1.2.3".into(),
        tag: "v1.2.3".into(),
        repository: "suxiaoshao/gupi".into(),
        publisher: "CN=Sushao".into(),
        cultures: LOCALES.iter().map(|(culture, _)| *culture).collect(),
        signing: MacosSigning::DeveloperId,
    }
}

fn msi_fixture(path: &Path, version: &str, language: u16) {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .unwrap();
    let mut package = msi::Package::create(msi::PackageType::Installer, file).unwrap();
    package.summary_info_mut().set_arch("x64");
    package
        .create_table(
            "Property",
            vec![
                msi::Column::build("Property").primary_key().id_string(72),
                msi::Column::build("Value").nullable().formatted_string(255),
            ],
        )
        .unwrap();
    for (key, value) in [
        ("ProductName", "Gupi".to_owned()),
        ("ProductVersion", version.to_owned()),
        ("Manufacturer", "CN=Sushao".to_owned()),
        ("ALLUSERS", "1".to_owned()),
        ("ProductLanguage", language.to_string()),
        (
            "ProductCode",
            format!("{{00000000-0000-0000-0000-{language:012}}}"),
        ),
        (
            "UpgradeCode",
            "{11111111-1111-1111-1111-111111111111}".to_owned(),
        ),
    ] {
        package
            .insert_rows(
                msi::Insert::into("Property")
                    .row(vec![msi::Value::from(key), msi::Value::from(value)]),
            )
            .unwrap();
    }
    package.flush().unwrap();
}

fn packages(root: &Path, release: &Release) {
    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    for filename in release.expected_packages() {
        fs::write(dist.join(filename), b"package").unwrap();
    }
    for (culture, code) in LOCALES {
        msi_fixture(
            &dist.join(format!("Gupi_{}_x64_{culture}.msi", release.version)),
            &release.version,
            code,
        );
    }
}

fn archived_manifests(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let source = File::open(root.join("dist/Gupi_1.2.3_distribution.tar.gz")).unwrap();
    tar::Archive::new(flate2::read::GzDecoder::new(source))
        .entries()
        .unwrap()
        .map(|entry| {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().into_owned();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            (path, bytes)
        })
        .collect()
}

#[test]
fn collect_reads_msi_identity_and_removes_stale_cask_on_development_rebuild() {
    let root = tempfile::tempdir().unwrap();
    let mut release = release();
    packages(root.path(), &release);
    release.collect(root.path(), false).unwrap();
    let output = root.path().join("distribution");
    let installers = fs::read_to_string(
        output.join("winget/manifests/s/suxiaoshao/Gupi/1.2.3/suxiaoshao.Gupi.installer.yaml"),
    )
    .unwrap();
    assert!(installers.contains("{00000000-0000-0000-0000-000000001028}"));
    assert!(installers.contains("InstallerLocale: zh-TW"));
    assert!(installers.contains("InstallerLocale: es-ES"));
    assert!(installers.contains("ElevationRequirement: elevationRequired\n"));
    assert!(output.join("homebrew/Casks/g/gupi.rb").is_file());
    let archived = archived_manifests(root.path());
    assert_eq!(archived.len(), 5);
    assert_eq!(
        archived[Path::new("homebrew/Casks/g/gupi.rb")],
        fs::read(output.join("homebrew/Casks/g/gupi.rb")).unwrap()
    );
    assert_eq!(
        archived
            [Path::new("winget/manifests/s/suxiaoshao/Gupi/1.2.3/suxiaoshao.Gupi.installer.yaml")],
        installers.as_bytes()
    );

    for arch in ["aarch64", "x86_64"] {
        for ext in ["dmg", "zip"] {
            let stem = format!("Gupi_1.2.3_{arch}_macos");
            fs::rename(
                root.path().join(format!("dist/{stem}.{ext}")),
                root.path().join(format!("dist/{stem}_development.{ext}")),
            )
            .unwrap();
        }
    }
    release.signing = MacosSigning::Development;
    release.collect(root.path(), false).unwrap();
    assert!(!output.join("homebrew").exists());
    assert!(!archived_manifests(root.path()).contains_key(Path::new("homebrew/Casks/g/gupi.rb")));
    assert!(
        fs::read_to_string(root.path().join("dist/SHA256SUMS"))
            .unwrap()
            .contains("Gupi_1.2.3_amd64.deb")
    );
    fs::remove_file(
        root.path()
            .join("dist/Gupi_1.2.3_aarch64_macos_development.dmg"),
    )
    .unwrap();
    assert!(release.collect(root.path(), false).is_err());
}

#[test]
fn collect_refuses_an_msi_for_a_different_version() {
    let root = tempfile::tempdir().unwrap();
    let release = release();
    packages(root.path(), &release);
    msi_fixture(
        &root.path().join("dist/Gupi_1.2.3_x64_en-US.msi"),
        "1.2.2",
        1033,
    );
    let error = release.collect(root.path(), false).unwrap_err().to_string();
    assert!(error.contains("MSI identity, version, scope or language differs"));
    assert!(!root.path().join("dist/SHA256SUMS").exists());
}

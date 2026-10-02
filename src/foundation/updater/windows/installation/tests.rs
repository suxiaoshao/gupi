use super::*;
use windows_sys::Win32::System::ApplicationInstallationAndServicing::{
    MSIDBOPEN_CREATE, MsiDatabaseCommit,
};

const PRODUCT: &str = "{02786B0E-2F61-4372-B129-2E1D1717D4D3}";

fn fixture(path: &Path, product: &str, language: Option<&str>) {
    let path: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let mut database = MsiHandle::default();
    assert_eq!(
        unsafe { MsiOpenDatabaseW(path.as_ptr(), MSIDBOPEN_CREATE, &mut database.0) },
        0
    );
    let execute = |sql: &str| {
        let mut view = MsiHandle::default();
        assert_eq!(
            unsafe { MsiDatabaseOpenViewW(database.0, wide(sql).as_ptr(), &mut view.0) },
            0
        );
        assert_eq!(unsafe { MsiViewExecute(view.0, 0) }, 0);
    };
    execute(
        "CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` CHAR(0) LOCALIZABLE PRIMARY KEY `Property`)",
    );
    execute(&format!(
        "INSERT INTO `Property` (`Property`, `Value`) VALUES ('ProductCode', '{product}')"
    ));
    if let Some(language) = language {
        execute(&format!(
            "INSERT INTO `Property` (`Property`, `Value`) VALUES ('ProductLanguage', '{language}')"
        ));
    }
    assert_eq!(unsafe { MsiDatabaseCommit(database.0) }, 0);
}

fn neutral_culture(path: &Path) -> Option<String> {
    product_culture(PRODUCT, |name| match name {
        "Language" => Some("0".into()),
        "LocalPackage" => Some(path.to_str().unwrap().into()),
        _ => panic!("unexpected property {name}"),
    })
}

#[test]
fn neutral_registration_recovers_each_installer_language_without_modifying_cache() {
    let directory = tempfile::tempdir().unwrap();
    for (language, expected) in [
        ("1033", "en-US"),
        ("2052", "zh-CN"),
        ("1028", "zh-TW"),
        ("1041", "ja-JP"),
        ("1042", "ko-KR"),
        ("1031", "de-DE"),
        ("1036", "fr-FR"),
        ("3082", "es-ES"),
        ("1046", "pt-BR"),
    ] {
        let path = directory.path().join(format!("{language}.msi"));
        fixture(&path, PRODUCT, Some(language));
        let original = std::fs::read(&path).unwrap();
        assert_eq!(neutral_culture(&path).as_deref(), Some(expected));
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
}

#[test]
fn explicit_language_does_not_require_a_cached_package() {
    assert_eq!(
        product_culture(PRODUCT, |name| {
            assert_eq!(name, "Language");
            Some("2052".into())
        })
        .as_deref(),
        Some("zh-CN")
    );
}

#[test]
fn untrusted_or_unusable_cache_never_guesses_a_language() {
    let directory = tempfile::tempdir().unwrap();
    for (index, product, language) in [
        (0, "{BBA7C379-1114-4160-820A-6F0155591A17}", Some("2052")),
        (1, PRODUCT, None),
        (2, PRODUCT, Some("0")),
        (3, PRODUCT, Some("9999")),
    ] {
        let path = directory.path().join(format!("{index}.msi"));
        fixture(&path, product, language);
        assert_eq!(neutral_culture(&path), None);
    }
    assert_eq!(neutral_culture(&directory.path().join("missing.msi")), None);
    let corrupt = directory.path().join("corrupt.msi");
    std::fs::write(&corrupt, b"not an MSI").unwrap();
    assert_eq!(neutral_culture(&corrupt), None);
    assert_eq!(product_culture(PRODUCT, |_| None), None);
    assert_eq!(
        product_culture(PRODUCT, |name| (name == "Language").then(|| "0".into())),
        None
    );
}

#[test]
fn an_unregistered_directory_is_not_an_installed_application() {
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(installed_culture_at(directory.path()), None);
}

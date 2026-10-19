use super::*;
fn fixture(text: &str) -> (tempfile::TempDir, PathBuf, File) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth.json");
    fs::write(&path, text).unwrap();
    let file = File::load(&path).unwrap();
    (dir, path, file)
}
#[test]
fn edits_preserve_fields_order_and_other_providers() {
    let (_dir, path, file) = fixture(
        r#"{"before":{"future":"secret"},"p":{"type":"api_key","env":{"ID":"secret"},"key":"old","future":1},"after":{"type":"oauth","access":"secret"}}"#,
    );
    let file = file
        .commit("p", &file.entry("p").revision(), Some("!literal $KEY"))
        .unwrap();
    let root = read(&path).unwrap();
    assert_eq!(root.keys().collect::<Vec<_>>(), ["before", "p", "after"]);
    assert_eq!(root["p"]["env"]["ID"], "secret");
    assert_eq!(root["p"]["future"], 1);
    assert_eq!(root["p"]["key"], "!literal $KEY");
    assert_eq!(root["after"]["access"], "secret");
    file.commit("p", &file.entry("p").revision(), None).unwrap();
    let root = read(&path).unwrap();
    assert!(!root.contains_key("p"));
    assert_eq!(root.keys().collect::<Vec<_>>(), ["before", "after"]);
}
#[test]
fn target_changes_conflict_and_other_changes_survive() {
    for (initial, changed) in [
        (
            r#"{"p":{"type":"api_key","key":"a"}}"#,
            r#"{"p":{"type":"api_key","key":"b"}}"#,
        ),
        (
            r#"{"p":{"type":"api_key","env":{"ID":"a"}}}"#,
            r#"{"p":{"type":"api_key","env":{"ID":"b"}}}"#,
        ),
        (
            r#"{"p":{"type":"oauth","access":"a"}}"#,
            r#"{"p":{"type":"oauth","access":"b"}}"#,
        ),
        ("{}", r#"{"p":{"type":"api_key","key":"a"}}"#),
    ] {
        let (_dir, path, file) = fixture(initial);
        fs::write(&path, changed).unwrap();
        assert!(matches!(
            file.commit("p", &file.entry("p").revision(), Some("new")),
            Err(Error::Conflict)
        ));
        assert_eq!(fs::read_to_string(path).unwrap(), changed);
    }
    let (_dir, path, file) = fixture(r#"{"p":{"type":"api_key","key":"a"}}"#);
    fs::write(
        &path,
        r#"{"p":{"type":"api_key","key":"a"},"other":{"new":1}}"#,
    )
    .unwrap();
    file.commit("p", &file.entry("p").revision(), Some("b"))
        .unwrap();
    assert_eq!(read(&path).unwrap()["other"]["new"], 1);
}
#[test]
fn invalid_files_and_held_locks_never_overwrite() {
    let (_dir, path, file) = fixture("{}");
    for text in ["[]", "null", "{secret"] {
        fs::write(&path, text).unwrap();
        assert!(matches!(File::load(&path), Err(Error::Parse)));
        assert!(matches!(
            file.commit("p", &file.entry("p").revision(), Some("new")),
            Err(Error::Parse)
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }
    fs::write(&path, "{}").unwrap();
    let _lock = Lock::acquire(&path).unwrap();
    assert!(matches!(
        file.commit("p", &file.entry("p").revision(), Some("new")),
        Err(Error::Busy)
    ));
    assert_eq!(fs::read_to_string(path).unwrap(), "{}");
}
#[test]
fn metadata_and_errors_do_not_expose_secrets() {
    let (_dir, _path, file) = fixture(
        "\u{feff}{\"p\":{\"type\":\"oauth\",\"access\":\"secret\"},\"q\":{\"type\":\"oauth\",\"expires\":\"secret\"},\"r\":{\"type\":\"oauth\",\"expires\":123}}",
    );
    assert_eq!(file.entry("p").expires(), None);
    assert_eq!(file.entry("q").expires(), None);
    assert_eq!(file.entry("r").expires(), Some(123.));
    for error in [
        Error::Read,
        Error::Parse,
        Error::Write,
        Error::Busy,
        Error::Conflict,
        Error::Invalid,
    ] {
        assert!(!format!("{error} {error:?}").contains("secret"));
    }
}
#[test]
fn concurrent_file_creation_is_not_truncated() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth.json");
    let file = File::load(&path).unwrap();
    fs::write(&path, r#"{"other":{"unknown":"secret"}}"#).unwrap();
    ensure_file(&path).unwrap();
    file.commit("p", &file.entry("p").revision(), Some("new"))
        .unwrap();
    assert_eq!(read(&path).unwrap()["other"]["unknown"], "secret");
}
#[cfg(unix)]
#[test]
fn private_creation_preserves_permissions_inode_and_symlink() {
    use std::os::unix::{
        fs::symlink,
        fs::{MetadataExt as _, PermissionsExt as _},
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new/nested/auth.json");
    let file = File::load(&path).unwrap();
    let file = file
        .commit("p", &file.entry("p").revision(), Some("new"))
        .unwrap();
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
    for parent in [
        path.parent().unwrap(),
        path.parent().unwrap().parent().unwrap(),
    ] {
        assert_eq!(fs::metadata(parent).unwrap().mode() & 0o777, 0o700);
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let inode = fs::metadata(&path).unwrap().ino();
    file.commit("p", &file.entry("p").revision(), Some("next"))
        .unwrap();
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o640);
    assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
    let link = dir.path().join("link.json");
    symlink(&path, &link).unwrap();
    let file = File::load(&link).unwrap();
    file.commit("p", &file.entry("p").revision(), Some("linked"))
        .unwrap();
    assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
    assert_eq!(read(&path).unwrap()["p"]["key"], "linked");
}
#[test]
fn oauth_and_unknown_replacement_are_complete_and_empty_key_is_rejected() {
    for text in [
        r#"{"p":{"type":"oauth","refresh":"secret"}}"#,
        r#"{"p":{"type":"future","key":"secret"}}"#,
    ] {
        let (_dir, path, file) = fixture(text);
        assert!(matches!(
            file.commit("p", &file.entry("p").revision(), Some("")),
            Err(Error::Invalid)
        ));
        file.commit("p", &file.entry("p").revision(), Some("new"))
            .unwrap();
        assert_eq!(
            read(&path).unwrap()["p"],
            serde_json::json!({"type":"api_key","key":"new"})
        );
    }
}

#[test]
fn provider_identity_is_preserved_verbatim() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth.json");
    let mut file = File::load(&path).unwrap();
    for name in [
        "中文供应商",
        "provider with spaces",
        "--credentials",
        " odd provider ",
    ] {
        assert!(valid_provider(name));
        file = file
            .commit(name, &file.entry(name).revision(), Some("secret"))
            .unwrap();
        assert_eq!(file.entry(name).provider(), name);
        assert_eq!(file.entry(name).kind(), Kind::ApiKey);
    }
    for name in [
        "中文供应商",
        "provider with spaces",
        "--credentials",
        " odd provider ",
    ] {
        file = file
            .commit(name, &file.entry(name).revision(), None)
            .unwrap();
        assert_eq!(file.entry(name).kind(), Kind::Missing);
    }
}

#[test]
fn empty_ids_support_set_replace_remove_and_preserve_other_entries() {
    let (_dir, path, mut file) = fixture(r#"{"Empty ID":{"type":"api_key","key":"sibling"}}"#);
    assert!(!valid_provider(""));
    assert!(!addressable_provider(""));
    assert!(!checkable_provider(""));
    file = file
        .commit("", &file.entry("").revision(), Some("first"))
        .unwrap();
    file = file
        .commit("", &file.entry("").revision(), Some("second"))
        .unwrap();
    assert_eq!(read(&path).unwrap()[""]["key"], "second");
    assert!(matches!(
        file.commit("", &file.entry("").revision(), Some("")),
        Err(Error::Invalid)
    ));
    file.commit("", &file.entry("").revision(), None).unwrap();
    let root = read(&path).unwrap();
    assert!(!root.contains_key(""));
    assert_eq!(root["Empty ID"]["key"], "sibling");
    for kind in ["oauth", "future"] {
        fs::write(
            &path,
            serde_json::json!({"":{"type":kind,"private":"old"}}).to_string(),
        )
        .unwrap();
        let file = File::load(&path).unwrap();
        file.commit("", &file.entry("").revision(), Some("new"))
            .unwrap();
        assert_eq!(
            read(&path).unwrap()[""],
            serde_json::json!({"type":"api_key","key":"new"})
        );
    }
}
#[test]
fn empty_id_revisions_distinguish_missing_and_changed_entries() {
    for (before, after) in [
        (r#"{}"#, r#"{"":{"type":"api_key","key":"new"}}"#),
        (
            r#"{"":{"type":"api_key","key":"old"}}"#,
            r#"{"":{"type":"api_key","key":"new"}}"#,
        ),
        (r#"{"":{"type":"api_key","key":"old"}}"#, r#"{}"#),
    ] {
        let (_dir, path, file) = fixture(before);
        fs::write(&path, after).unwrap();
        for key in [Some("replacement"), None] {
            assert!(matches!(
                file.commit("", &file.entry("").revision(), key),
                Err(Error::Conflict)
            ));
            assert_eq!(fs::read_to_string(&path).unwrap(), after);
        }
    }
}

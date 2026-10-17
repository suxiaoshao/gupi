use super::*;
use serde_json::json;
fn fixture(kind: Kind, text: &str) -> (tempfile::TempDir, File) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(kind.filename()), text).unwrap();
    let file = File::load(kind, &dir.path().join(kind.filename())).unwrap();
    (dir, file)
}
#[test]
fn models_parser_used_on_both_reads_and_retains_empty_providers() {
    let (dir, file) = fixture(
        Kind::Models,
        "\u{feff}{// note\n\"providers\":{\"p\":{\"baseUrl\":\"https://x//y\",},},\"unknown\":7,}",
    );
    assert!(file.comments());
    let saved = file
        .commit(dir.path(), &Scope::Global, &[("p".into(), None)], None)
        .unwrap();
    assert_eq!(saved.names().unwrap(), Vec::<String>::new());
    let text = std::fs::read_to_string(dir.path().join("models.json")).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&text).unwrap(),
        json!({"providers":{},"unknown":7})
    );
    assert!(!saved.comments());
    assert!(File::load(Kind::Mcp, &dir.path().join("models.json")).is_ok());
}
#[test]
fn strict_mcp_rejects_bom_comments_trailing_and_models_rejects_blocks() {
    for text in ["\u{feff}{}", "{//x\n}", "{\"x\":1,}"] {
        let (dir, _) = fixture(Kind::Models, text);
        assert!(File::load(Kind::Mcp, &dir.path().join("models.json")).is_err());
    }
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("models.json"), "{/* x */}").unwrap();
    assert!(File::load(Kind::Models, &dir.path().join("models.json")).is_err());
}
#[test]
fn order_only_dirty_and_external_order_conflict() {
    let (dir, file) = fixture(
        Kind::Mcp,
        r#"{"mcpServers":{"s":{"command":"x","toolExposure":{"a*":"direct","b*":"hidden"}}}}"#,
    );
    let mut edit = file.entry("s").unwrap();
    edit.move_item(&["toolExposure".into()], 0, 1).unwrap();
    assert!(!edit.same(&file.entry("s").unwrap()));
    let saved = file
        .commit(
            dir.path(),
            &Scope::Global,
            &[("s".into(), Some(edit))],
            None,
        )
        .unwrap();
    assert!(file.changed(&saved, "s"));
    assert!(matches!(
        file.commit(dir.path(), &Scope::Global, &[("s".into(), None)], None),
        Err(Error::Conflict)
    ));
}
#[test]
fn independent_entries_unknown_fields_aliases_and_positions_survive() {
    let (dir, file) = fixture(
        Kind::Mcp,
        r#"{"first":1,"mcpServers":{"a":{"type":"streamable-http","url":"https://x","exposure":"codemode-deferred","unknown":{"x":2}},"b":{"command":"b"}},"last":2}"#,
    );
    let mut a = file.entry("a").unwrap();
    a.set(&["description".into()], Some(json!("new"))).unwrap();
    let mut b = file.entry("b").unwrap();
    b.set(&["command".into()], Some(json!("changed"))).unwrap();
    file.commit(dir.path(), &Scope::Global, &[("b".into(), Some(b))], None)
        .unwrap();
    let saved = file
        .commit(dir.path(), &Scope::Global, &[("a".into(), Some(a))], None)
        .unwrap();
    let a = saved.entry("a").unwrap();
    assert_eq!(a.visible(&["type".into()]), Some(json!("streamable-http")));
    assert_eq!(
        a.visible(&["exposure".into()]),
        Some(json!("codemode-deferred"))
    );
    assert_eq!(
        saved
            .document
            .root()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["first", "mcpServers", "last"]
    );
    assert_eq!(
        saved.entry("b").unwrap().visible(&["command".into()]),
        Some(json!("changed"))
    );
}
#[test]
fn credentials_are_opaque_and_keep_replace_empty_remove_are_distinct() {
    for (kind, text, paths) in [
        (
            Kind::Models,
            r#"{"providers":{"p":{"apiKey":"secret","headers":{"A":"secret"},"models":[{"id":"m","headers":{"A":"secret"}}],"modelOverrides":{"m":{"headers":{"A":"secret"}}}}}}"#,
            vec![
                vec!["apiKey"],
                vec!["headers", "A"],
                vec!["models", "0", "headers", "A"],
                vec!["modelOverrides", "m", "headers", "A"],
            ],
        ),
        (
            Kind::Mcp,
            r#"{"mcpServers":{"p":{"command":"x","env":{"A":"secret"},"headers":{"A":"secret"},"oauth":{"clientSecret":"secret"}}}}"#,
            vec![
                vec!["env", "A"],
                vec!["headers", "A"],
                vec!["oauth", "clientSecret"],
            ],
        ),
    ] {
        let (dir, file) = fixture(kind, text);
        let mut entry = file.entry("p").unwrap();
        let original = entry.clone();
        for path in paths {
            let p: Vec<_> = path.into_iter().map(str::to_owned).collect();
            assert!(entry.present(&p));
            assert!(entry.visible(&p).is_none());
            entry.credential(&p, Credential::Keep).unwrap();
            assert!(entry.same(&original));
            entry
                .credential(&p, Credential::Replace(String::new()))
                .unwrap();
            assert_eq!(at(&entry.value, &p), Some(&json!("")));
            entry.credential(&p, Credential::Remove).unwrap();
            assert!(!entry.present(&p));
            entry = original.clone();
        }
        file.commit(
            dir.path(),
            &Scope::Global,
            &[("p".into(), Some(entry))],
            None,
        )
        .unwrap();
        assert!(!Error::Parse.to_string().contains("secret"));
    }
}
#[test]
fn partial_override_and_minimal_validation() {
    let mut partial = Entry::new(Kind::Mcp);
    partial
        .set(&["enabled".into()], Some(json!(false)))
        .unwrap();
    assert!(partial.validate(true).is_ok());
    assert!(partial.validate(false).is_err());
    partial.set(&["command".into()], Some(json!("x"))).unwrap();
    partial
        .set(&["auth".into(), "provider".into()], Some(json!("p")))
        .unwrap();
    assert!(partial.validate(true).is_err());
    partial
        .set(&["auth".into(), "provider".into()], None)
        .unwrap();
    partial
        .set(&["oauth".into(), "callbackPort".into()], Some(json!(65536)))
        .unwrap();
    assert!(partial.validate(false).is_err());
    let mut model = Entry::new(Kind::Models);
    model.append(&["models".into()], json!({})).unwrap();
    assert!(model.validate(false).is_err());
    model
        .set(
            &["models".into(), "0".into(), "id".into()],
            Some(json!("m")),
        )
        .unwrap();
    assert!(model.validate(false).is_ok());
}
#[test]
fn name_collision_and_missing_project_rejected() {
    let (dir, file) = fixture(Kind::Mcp, r#"{"mcpServers":{"a-b":{"command":"x"}}}"#);
    assert_eq!(file.check_name("a_b", None), Err(Error::Duplicate));
    assert_eq!(file.check_name("a.b", None), Err(Error::Invalid));
    let empty = File::load(Kind::Mcp, &dir.path().join("missing")).unwrap();
    assert_eq!(empty.check_name("a_b", Some(&file)), Err(Error::Duplicate));
    assert!(empty.check_name("a-b", Some(&file)).is_ok());
    assert!(matches!(
        file.commit(
            dir.path(),
            &Scope::Project(dir.path().join("gone")),
            &[],
            None
        ),
        Err(Error::Project)
    ));
}

#[test]
fn project_partial_override_preserves_unknown_fields_and_absent_parents() {
    let (dir, global) = fixture(Kind::Mcp, r#"{"mcpServers":{"server":{"command":"x"}}}"#);
    let cwd = dir.path().join("project");
    std::fs::create_dir(&cwd).unwrap();
    std::fs::create_dir(cwd.join(".pi")).unwrap();
    let path = cwd.join(".pi/mcp.json");
    std::fs::write(
        &path,
        r#"{"mcpServers":{"server":{"enabled":false,"future":null}},"unknown":1}"#,
    )
    .unwrap();
    let file = File::load(Kind::Mcp, &path).unwrap();
    let mut entry = file.entry("server").unwrap();
    let original = entry.clone();
    entry
        .set(&["oauth".into(), "clientId".into()], None)
        .unwrap();
    assert!(entry.same(&original));
    entry.set(&["enabled".into()], Some(json!(true))).unwrap();
    assert!(entry.validate(true).is_ok());
    let next = file
        .commit(
            dir.path(),
            &Scope::Project(cwd.clone()),
            &[("server".into(), Some(entry))],
            Some(Some(json!(false))),
        )
        .unwrap();
    assert_eq!(
        next.entry("server").unwrap().visible(&["future".into()]),
        Some(Value::Null)
    );
    assert!(!path.with_extension("json.lock").exists());
    assert_eq!(
        global.entry("server").unwrap().visible(&["command".into()]),
        Some(json!("x"))
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap(),
        json!({"mcpServers":{"server":{"enabled":true,"future":null}},"unknown":1,"autoEnableCodemode":false})
    );
}

#[test]
fn credential_keep_restores_key_position_and_unsupported_unchanged_numbers_survive() {
    let (_, file) = fixture(
        Kind::Mcp,
        r#"{"mcpServers":{"s":{"command":"x","headers":{"first":"one","second":"two"},"timeout":"legacy"}}}"#,
    );
    let mut entry = file.entry("s").unwrap();
    let original = entry.clone();
    let path = vec!["headers".into(), "first".into()];
    entry.credential(&path, Credential::Remove).unwrap();
    entry.restore_credential(&original, &path).unwrap();
    assert!(entry.same(&original));
    entry
        .set(&["description".into()], Some(json!("new")))
        .unwrap();
    assert!(entry.validate_edit(&original, false).is_ok());
    entry.set(&["timeout".into()], Some(json!(-1))).unwrap();
    assert!(entry.validate_edit(&original, false).is_err());
}

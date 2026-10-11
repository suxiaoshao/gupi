use super::trust;
use super::*;
use serde_json::json;
use std::fs;

const ENABLED: Leaf = &["compaction", "enabled"];
const RESERVE: Leaf = &["compaction", "reserveTokens"];
const STEERING: Leaf = &["steeringMode"];

fn file(dir: &tempfile::TempDir, contents: &str) -> PathBuf {
    let path = dir.path().join("settings.json");
    fs::write(&path, contents).unwrap();
    path
}

fn text(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn missing_files_read_as_empty_and_invalid_files_are_errors() {
    let dir = tempfile::tempdir().unwrap();
    let missing = load(&dir.path().join("settings.json")).unwrap();
    assert!(!missing.exists());
    assert!(missing.root().is_empty());

    let bom = file(&dir, "\u{feff}{\"steeringMode\":\"all\"}");
    assert_eq!(load(&bom).unwrap().get(STEERING), Some(&json!("all")));

    for invalid in ["{", "[]", "{\"a\":1,}"] {
        let path = file(&dir, invalid);
        assert!(matches!(load(&path), Err(Error::Read(_))), "{invalid}");
        let change = Change::new(STEERING, None, Some(json!("all")));
        assert!(matches!(commit(&path, &[change]), Err(Error::Read(_))));
        assert_eq!(text(&path), invalid, "an unreadable file is never replaced");
    }
}

#[test]
fn commits_touch_only_requested_leaves_and_keep_unknown_fields_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(
        &dir,
        r#"{"zeta":{"x":1},"compaction":{"keepRecentTokens":5,"custom":true},"alpha":[1]}"#,
    );
    let changes = [
        Change::new(ENABLED, None, Some(json!(false))),
        Change::new(STEERING, None, Some(json!("all"))),
    ];
    let document = commit(&path, &changes).unwrap();
    assert_eq!(document.get(ENABLED), Some(&json!(false)));
    let written: Value = serde_json::from_str(&text(&path)).unwrap();
    assert_eq!(
        written,
        json!({"zeta":{"x":1},"compaction":{"keepRecentTokens":5,"custom":true,"enabled":false},"alpha":[1],"steeringMode":"all"})
    );
    let keys: Vec<_> = written.as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys, ["zeta", "compaction", "alpha", "steeringMode"]);
    assert!(text(&path).ends_with("}\n"));
}

#[test]
fn removing_a_leaf_restores_inheritance_and_prunes_emptied_parents() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(
        &dir,
        r#"{"compaction":{"enabled":false},"retry":{"enabled":true,"maxRetries":2}}"#,
    );
    commit(
        &path,
        &[
            Change::new(ENABLED, Some(json!(false)), None),
            Change::new(&["retry", "maxRetries"], Some(json!(2)), None),
            Change::new(RESERVE, None, None),
        ],
    )
    .unwrap();
    let written: Value = serde_json::from_str(&text(&path)).unwrap();
    assert_eq!(written, json!({"retry":{"enabled":true}}));

    // An object that was already empty is not a deletion target.
    let path = file(&dir, r#"{"compaction":{}}"#);
    commit(&path, &[Change::new(ENABLED, None, None)]).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&text(&path)).unwrap(),
        json!({"compaction":{}})
    );
}

#[test]
fn same_leaf_external_changes_conflict_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(
        &dir,
        r#"{"compaction":{"enabled":true},"steeringMode":"all"}"#,
    );
    let before = text(&path);
    // Changed since read.
    let changed = commit(
        &path,
        &[
            Change::new(ENABLED, Some(json!(false)), Some(json!(true))),
            Change::new(STEERING, Some(json!("all")), Some(json!("one-at-a-time"))),
        ],
    );
    assert_eq!(changed, Err(Error::Conflict(vec![ENABLED])));
    // Deleted since read.
    let deleted = commit(
        &path,
        &[Change::new(RESERVE, Some(json!(1)), Some(json!(2)))],
    );
    assert_eq!(deleted, Err(Error::Conflict(vec![RESERVE])));
    // Appeared since read.
    let appeared = commit(&path, &[Change::new(STEERING, None, None)]);
    assert_eq!(appeared, Err(Error::Conflict(vec![STEERING])));
    assert_eq!(text(&path), before);
}

#[test]
fn edits_to_other_leaves_made_since_reading_are_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, r#"{"compaction":{"enabled":true}}"#);
    let read = load(&path).unwrap();
    // Pi or another editor changes a different field.
    fs::write(
        &path,
        r#"{"compaction":{"enabled":true,"reserveTokens":7},"theme":"dark"}"#,
    )
    .unwrap();
    commit(
        &path,
        &[Change::new(
            ENABLED,
            read.get(ENABLED).cloned(),
            Some(json!(false)),
        )],
    )
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&text(&path)).unwrap(),
        json!({"compaction":{"enabled":false,"reserveTokens":7},"theme":"dark"})
    );
}

#[test]
fn a_held_lock_blocks_writes_and_is_never_removed() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "{}");
    let foreign = dir.path().join("settings.json.lock");
    fs::create_dir(&foreign).unwrap();
    let result = commit(&path, &[Change::new(STEERING, None, Some(json!("all")))]);
    assert!(matches!(result, Err(Error::Locked(_))));
    assert!(update(&path, |_| Ok(())).is_err());
    assert!(foreign.is_dir());
    assert_eq!(text(&path), "{}");

    fs::remove_dir(&foreign).unwrap();
    commit(&path, &[Change::new(STEERING, None, Some(json!("all")))]).unwrap();
    assert!(!foreign.exists(), "our own lock is released");
}

#[test]
fn first_project_save_creates_the_pi_directory() {
    let dir = tempfile::tempdir().unwrap();
    let path = Scope::Project(dir.path().to_owned()).file(Path::new("/unused"));
    assert_eq!(path, dir.path().join(".pi/settings.json"));
    let document = Scope::Project(dir.path().to_owned())
        .commit(
            Path::new("/unused"),
            &[Change::new(STEERING, None, Some(json!("all")))],
        )
        .unwrap();
    assert!(document.exists());
    assert_eq!(
        serde_json::from_str::<Value>(&text(&path)).unwrap(),
        json!({"steeringMode":"all"})
    );
}

#[test]
fn update_rewrites_the_latest_contents_under_the_lock() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, r#"{"skills":["a"],"other":1}"#);
    update(&path, |root| {
        root.insert("skills".into(), json!(["a", "b"]));
        Ok(())
    })
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&text(&path)).unwrap(),
        json!({"skills":["a","b"],"other":1})
    );
}

#[test]
fn trust_records_only_the_exact_directory() {
    let agent = tempfile::tempdir().unwrap();
    let projects = tempfile::tempdir().unwrap();
    let parent = trust::canonical(projects.path());
    let project = parent.join("app");
    fs::create_dir(&project).unwrap();
    let other = "/elsewhere/kept";
    fs::write(
        trust::file(agent.path()),
        json!({ other: false, parent.to_string_lossy(): false }).to_string(),
    )
    .unwrap();
    assert_eq!(
        trust::decision(agent.path(), &project).unwrap(),
        trust::Decision::Distrusted(parent.clone())
    );

    assert_eq!(trust::trust(agent.path(), &project).unwrap(), project);
    let records: Value = serde_json::from_str(&text(&trust::file(agent.path()))).unwrap();
    let mut expected = vec![
        (other.to_owned(), json!(false)),
        (parent.to_string_lossy().into_owned(), json!(false)),
        (project.to_string_lossy().into_owned(), json!(true)),
    ];
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    let actual: Vec<_> = records
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    assert_eq!(
        actual, expected,
        "sorted, with ancestors and others untouched"
    );
    assert_eq!(
        trust::decision(agent.path(), &project).unwrap(),
        trust::Decision::Trusted(project.clone())
    );
    assert_eq!(
        trust::decision(agent.path(), &parent).unwrap(),
        trust::Decision::Distrusted(parent.clone())
    );
}

#[test]
fn trust_lookup_uses_the_nearest_ancestor_and_rejects_invalid_records() {
    let agent = tempfile::tempdir().unwrap();
    let projects = tempfile::tempdir().unwrap();
    let parent = trust::canonical(projects.path());
    let nested = parent.join("a/b");
    fs::create_dir_all(&nested).unwrap();
    assert_eq!(
        trust::decision(agent.path(), &nested).unwrap(),
        trust::Decision::Unset
    );
    fs::write(
        trust::file(agent.path()),
        json!({ parent.join("a").to_string_lossy(): null, parent.to_string_lossy(): true })
            .to_string(),
    )
    .unwrap();
    assert_eq!(
        trust::decision(agent.path(), &nested).unwrap(),
        trust::Decision::Trusted(parent.clone())
    );

    fs::write(trust::file(agent.path()), r#"{"/x":"yes"}"#).unwrap();
    assert!(matches!(
        trust::decision(agent.path(), &nested),
        Err(Error::Read(_))
    ));
    assert!(trust::trust(agent.path(), &nested).is_err());
    assert_eq!(text(&trust::file(agent.path())), r#"{"/x":"yes"}"#);
}

#[test]
fn trust_requires_an_existing_absolute_folder() {
    let agent = tempfile::tempdir().unwrap();
    let projects = tempfile::tempdir().unwrap();
    let file_path = projects.path().join("file");
    fs::write(&file_path, "").unwrap();
    for cwd in [
        PathBuf::from("relative/project"),
        projects.path().join("missing"),
        file_path,
    ] {
        assert!(
            trust::trust(agent.path(), &cwd).is_err(),
            "{}",
            cwd.display()
        );
    }
    assert!(!trust::file(agent.path()).exists(), "nothing was recorded");
}

#[test]
fn project_scope_rejects_invalid_folders_without_creating_them() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("deleted-project");
    let file = dir.path().join("file");
    fs::write(&file, "not a directory").unwrap();
    for cwd in [
        PathBuf::new(),
        PathBuf::from("relative-project"),
        missing.clone(),
        file,
    ] {
        let scope = Scope::Project(cwd);
        assert!(scope.validated().is_err());
        assert!(
            scope
                .commit(
                    dir.path(),
                    &[Change::new(STEERING, None, Some(json!("all")))]
                )
                .is_err()
        );
    }
    assert!(!missing.exists());
    let project = dir.path().join("project");
    fs::create_dir(&project).unwrap();
    let scope = Scope::Project(project.clone()).validated().unwrap();
    fs::remove_dir(&project).unwrap();
    assert!(
        scope
            .commit(
                dir.path(),
                &[Change::new(STEERING, None, Some(json!("all")))]
            )
            .is_err()
    );
    assert!(!project.exists());
}

use super::*;
use std::fs;

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

const SKILL: &str = "---\nname: shared\ndescription: Does things\n---\nBody\n";

#[test]
fn project_paths_must_resolve_inside_the_projects_own_pi_folder() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().join("project");
    fs::create_dir_all(&cwd).unwrap();
    // Nothing exists yet: the nearest existing ancestor decides.
    assert!(project_owns(&cwd, &cwd.join(".pi/skills/a/SKILL.md")));
    assert!(project_owns(&cwd, &cwd.join(".pi/SYSTEM.md")));
    assert!(!project_owns(&cwd, &cwd.join(".pi")));
    assert!(!project_owns(&cwd, &cwd.join("SYSTEM.md")));
    assert!(!project_owns(&cwd, &cwd.join(".pi/../SYSTEM.md")));
    assert!(!project_owns(&cwd, &dir.path().join("other/.pi/SYSTEM.md")));

    write(&cwd.join(".pi/prompts/a.md"), "a");
    assert!(project_owns(&cwd, &cwd.join(".pi/prompts/a.md")));
}

#[cfg(unix)]
#[test]
fn links_leaving_the_project_are_not_owned() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().join("project");
    let global = dir.path().join("global");
    write(&global.join("prompts/g.md"), "g");
    fs::create_dir_all(cwd.join(".pi/prompts")).unwrap();
    symlink(global.join("prompts/g.md"), cwd.join(".pi/prompts/g.md")).unwrap();
    assert!(!project_owns(&cwd, &cwd.join(".pi/prompts/g.md")));

    // A `.pi` folder that links elsewhere owns nothing, even new files.
    let linked = dir.path().join("linked");
    fs::create_dir_all(&linked).unwrap();
    symlink(&global, linked.join(".pi")).unwrap();
    assert!(!project_owns(&linked, &linked.join(".pi/prompts/g.md")));
    assert!(!project_owns(&linked, &linked.join(".pi/prompts/new.md")));
}

#[test]
fn project_ownership_uses_the_filesystems_directory_spelling() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().join("project");
    write(&cwd.join(".PI/prompts/a.md"), "a");
    // Exercise case-insensitive volumes without changing case-sensitive ones'
    // semantics: `.PI` is not the project's `.pi` on those volumes.
    if cwd.join(".pi").exists() {
        assert!(project_owns(&cwd, &cwd.join(".pi/prompts/a.md")));
        assert!(project_owns(&cwd, &cwd.join(".PI/prompts/a.md")));
        assert!(project_owns(&cwd, &cwd.join(".pi/prompts/new.md")));
        let catalog = scan_project(&cwd).unwrap();
        let resource = catalog
            .resources
            .iter()
            .find(|r| r.kind == Kind::Prompt)
            .unwrap();
        assert!(resource.is_project_owned());
        assert!(resource.editable);
    } else {
        assert!(!project_owns(&cwd, &cwd.join(".PI/prompts/a.md")));
    }
}

#[test]
fn project_scan_reports_missing_and_invalid_folders() {
    let dir = tempfile::tempdir().unwrap();
    assert!(scan_project(&dir.path().join("gone")).is_err());

    let empty = scan_project(dir.path()).unwrap();
    assert!(empty.resources.is_empty());
    assert_eq!(empty.root, dir.path().join(".pi"));
    assert!(!dir.path().join(".pi").exists(), "a scan never creates .pi");

    write(&dir.path().join(".pi"), "not a folder");
    assert!(scan_project(dir.path()).is_err());
}

#[test]
fn project_scan_lists_own_skills_and_prompts_with_pi_names() {
    let dir = tempfile::tempdir().unwrap();
    let pi = dir.path().join(".pi");
    write(&pi.join("skills/folder/SKILL.md"), SKILL);
    write(
        &pi.join("skills/plain/SKILL.md"),
        "---\ndescription: d\n---\n",
    );
    write(
        &pi.join("skills/silent/SKILL.md"),
        "---\nname: silent\n---\n",
    );
    write(
        &pi.join("prompts/review.md"),
        "---\nname: other\n---\nReview\n",
    );
    write(&pi.join("prompts/nested/skip.md"), "nested");
    write(
        &pi.join("settings.json"),
        r#"{"prompts":["-prompts/review.md"]}"#,
    );

    let catalog = scan_project(dir.path()).unwrap();
    let find = |file: &str| {
        catalog
            .resources
            .iter()
            .find(|r| r.path == pi.join(file))
            .unwrap_or_else(|| panic!("{file}"))
    };
    let shared = find("skills/folder/SKILL.md");
    assert_eq!(shared.key.as_deref(), Some("shared"));
    assert!(shared.editable && shared.enabled);
    assert_eq!(
        find("skills/plain/SKILL.md").key.as_deref(),
        Some("plain"),
        "falls back to the folder name"
    );
    assert_eq!(
        find("skills/silent/SKILL.md").key,
        None,
        "Pi skips a skill without a description"
    );
    let review = find("prompts/review.md");
    assert_eq!(
        review.key.as_deref(),
        Some("review"),
        "prompts use file names"
    );
    assert!(!review.enabled, "project overrides apply");
    assert!(
        catalog
            .resources
            .iter()
            .all(|r| r.path != pi.join("prompts/nested/skip.md"))
    );
}

#[test]
fn saves_refuse_files_changed_since_opening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".pi/SYSTEM.md");
    let missing = Baseline::Missing;

    save_text(&path, "", false, &missing).unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "",
        "an empty file is kept"
    );
    let changed = save_text(&path, "x", false, &missing).unwrap_err();
    assert!(changed.is_changed(), "the file appeared after opening");

    let opened = Baseline::Text(String::new());
    save_text(&path, "first", false, &opened).unwrap();
    assert!(
        save_text(&path, "second", false, &opened)
            .unwrap_err()
            .is_changed()
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), "first");

    let created = dir.path().join(".pi/prompts/new.md");
    save_text(&created, "new", true, &missing).unwrap();
    assert!(
        save_text(&created, "again", true, &missing)
            .unwrap_err()
            .is_changed()
    );
    assert_eq!(fs::read_to_string(&created).unwrap(), "new");
}

#[test]
fn project_scan_lists_standalone_and_explicit_resources_without_writes() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().join("project");
    let pi = cwd.join(".pi");
    let external = dir.path().join("external");
    write(
        &pi.join("extensions/local.ts"),
        "throw new Error('must not execute');",
    );
    write(&pi.join("themes/local.json"), "{}");
    write(&pi.join("skills/local/SKILL.md"), SKILL);
    write(&external.join("custom.md"), "External prompt");
    write(
        &external.join("remote.ts"),
        "throw new Error('must not execute');",
    );
    write(&external.join("remote.json"), "{}");
    write(&external.join("skill/SKILL.md"), SKILL);
    let settings = serde_json::json!({
        "unknown": {"preserve": true},
        "extensions": [external.join("remote.ts"), "-extensions/local.ts"],
        "themes": [external.join("remote.json")],
        "prompts": [external.join("custom.md")],
        "skills": [external.join("skill"), "-skills/local/SKILL.md"]
    })
    .to_string();
    write(&pi.join("settings.json"), &settings);
    let catalog = scan_project(&cwd).unwrap();
    assert_eq!(catalog.resources.len(), 7);
    let extension = catalog
        .resources
        .iter()
        .find(|r| r.path == pi.join("extensions/local.ts"))
        .unwrap();
    assert!(!extension.enabled && !extension.editable && extension.is_project_owned());
    let skill = catalog
        .resources
        .iter()
        .find(|r| r.path == pi.join("skills/local/SKILL.md"))
        .unwrap();
    assert!(!skill.enabled && skill.editable);
    for resource in catalog
        .resources
        .iter()
        .filter(|r| r.path.starts_with(&external))
    {
        assert!(!resource.editable && resource.enabled && !resource.is_project_owned());
    }
    assert!(
        catalog.resources.iter().any(|r| r.kind == Kind::Theme
            && r.path == pi.join("themes/local.json")
            && !r.editable)
    );
    assert_eq!(
        fs::read_to_string(pi.join("settings.json")).unwrap(),
        settings
    );
}

#[test]
fn custom_project_paths_are_owned_but_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let pi = dir.path().join(".pi");
    write(&pi.join("custom/review.md"), "Review");
    write(
        &pi.join("settings.json"),
        r#"{"prompts":["custom/review.md"]}"#,
    );
    let catalog = scan_project(dir.path()).unwrap();
    assert_eq!(catalog.resources.len(), 1);
    assert!(catalog.resources[0].is_project_owned());
    assert!(!catalog.resources[0].editable);
}

#[test]
fn optimistic_reload_preserves_project_ownership() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    let agent = dir.path().join("agent");
    for root in [project.join(".pi"), agent.clone()] {
        write(&root.join("skills/x/SKILL.md"), SKILL);
        write(&root.join("prompts/p.md"), "Before");
    }
    for (mut catalog, owned) in [
        (scan_project(&project).unwrap(), true),
        (scan(agent, None, &[]).unwrap(), false),
    ] {
        let resources = catalog.resources.clone();
        assert_eq!(resources.len(), 2);
        for resource in &resources {
            assert_eq!(resource.is_project_owned(), owned);
            write(
                &resource.path,
                if resource.kind == Kind::Skill {
                    SKILL
                } else {
                    "After"
                },
            );
            reload_resource(&mut catalog, resource.clone());
        }
        assert_eq!(catalog.resources.len(), resources.len());
        for (reloaded, original) in catalog.resources.iter().zip(&resources) {
            assert_eq!(reloaded.path, original.path);
            assert_eq!(reloaded.is_project_owned(), owned);
            assert_eq!(reloaded.editable, original.editable);
            assert_eq!(reloaded.enabled, original.enabled);
        }
    }
}

#[test]
fn catalogs_accept_settings_with_a_utf8_bom() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(".pi");
    write(
        &root.join("settings.json"),
        "\u{feff}{\"prompts\":[\"custom/p.md\"]}",
    );
    write(&root.join("custom/p.md"), "Prompt body");
    let before = fs::read(root.join("settings.json")).unwrap();
    for catalog in [
        scan_project(dir.path()).unwrap(),
        scan(root.clone(), None, &[]).unwrap(),
    ] {
        assert_eq!(catalog.resources.len(), 1);
        assert_eq!(catalog.resources[0].path, root.join("custom/p.md"));
    }
    assert_eq!(fs::read(root.join("settings.json")).unwrap(), before);
}

#[test]
fn project_resource_save_does_not_recreate_a_missing_project() {
    let dir = tempfile::tempdir().unwrap();
    for (relative, create) in [
        ("skills/new/SKILL.md", true),
        ("prompts/new.md", true),
        ("SYSTEM.md", false),
    ] {
        let cwd = dir.path().join("project");
        fs::create_dir(&cwd).unwrap();
        let path = cwd.join(".pi").join(relative);
        fs::remove_dir(&cwd).unwrap();
        assert!(save_project_text(&cwd, &path, "draft", create, &Baseline::Missing).is_err());
        assert!(!cwd.exists());
        // A real project without .pi still supports its first resource save.
        fs::create_dir(&cwd).unwrap();
        save_project_text(&cwd, &path, "draft", create, &Baseline::Missing).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "draft");
        fs::remove_dir_all(&cwd).unwrap();
    }
}

#[test]
fn project_package_deltas_only_override_matching_inherited_resources() {
    let dir = tempfile::tempdir().unwrap();
    let agent = dir.path().join("agent");
    let cwd = dir.path().join("project");
    let package = dir.path().join("shared");
    for (path, text) in [
        ("skills/a/SKILL.md", SKILL),
        ("prompts/a.md", "A"),
        ("prompts/b.md", "B"),
        ("extensions/a.ts", ""),
        ("themes/a.json", "{}"),
    ] {
        write(&package.join(path), text);
    }
    write(&package.join("package.json"), "{}");
    write(
        &agent.join("settings.json"),
        &json!({"packages":[package]}).to_string(),
    );
    write(&cwd.join(".pi/settings.json"), &json!({"packages":[{"source":package,"autoload":false,
        "skills":["!skills/**"], "prompts":["!prompts/*", "+prompts/b.md"], "extensions":["-extensions/a.ts"]}]}).to_string());
    let global = scan(agent, None, &[]).unwrap();
    let project = scan_project(&cwd).unwrap();
    assert_eq!(global.resources.len(), 5);
    assert!(project.resources.is_empty());
    for resource in &global.resources {
        let enabled = project.inherited_enabled(&global.root, resource);
        assert_eq!(
            enabled,
            resource.kind == Kind::Theme || resource.path.ends_with("prompts/b.md")
        );
        assert!(resource.enabled, "global catalog remains unchanged");
    }
    // Empty and unmatched rules preserve disabled global entries too.
    let mut theme = global
        .resources
        .iter()
        .find(|r| r.kind == Kind::Theme)
        .unwrap()
        .clone();
    theme.enabled = false;
    assert!(!project.inherited_enabled(&global.root, &theme));
}

#[test]
fn project_toggles_write_the_project_array_and_package_only() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().join("project");
    let global = dir.path().join("global");
    write(&global.join("prompts/global.md"), "Global");
    write(&global.join("settings.json"), r#"{"unknown":1}"#);
    write(&cwd.join(".pi/prompts/local.md"), "Local");
    write(&cwd.join(".pi/pkg/prompts/pkg.md"), "Package");
    write(
        &cwd.join(".pi/pkg/package.json"),
        r#"{"pi":{"prompts":["prompts/pkg.md"]}}"#,
    );
    write(
        &cwd.join(".pi/settings.json"),
        r#"{"packages":["./pkg"],"unknown":2}"#,
    );
    let catalog = scan_project(&cwd).unwrap();
    assert_eq!(catalog.resources.len(), 2);
    for resource in &catalog.resources {
        set_project_enabled(&cwd, resource, false).unwrap();
    }
    let settings = read_json(&cwd.join(".pi/settings.json")).unwrap();
    assert_eq!(settings["prompts"], json!(["-prompts/local.md"]));
    assert_eq!(
        settings["packages"][0],
        json!({"source":"./pkg","prompts":["-prompts/pkg.md"]})
    );
    assert_eq!(settings["unknown"], 2);
    let inherited = scan(global.clone(), None, &[]).unwrap().resources.remove(0);
    assert!(set_project_enabled(&cwd, &inherited, false).is_err());
    assert_eq!(
        read_json(&global.join("settings.json")).unwrap(),
        json!({"unknown":1})
    );
}

#[test]
fn toggles_refuse_unsupported_arrays_and_autoload_false_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("prompts/p.md"), "p");
    let resource = Resource::new(
        Kind::Prompt,
        dir.path().join("prompts/p.md"),
        dir.path().to_owned(),
        "p".into(),
    );
    for value in [
        json!(null),
        json!(false),
        json!("p"),
        json!(["p", 7]),
        json!({"x":1}),
    ] {
        let original = json!({"prompts":value,"unknown":true}).to_string();
        write(&dir.path().join("settings.json"), &original);
        assert!(set_enabled(dir.path(), &resource, false).is_err());
        assert_eq!(
            fs::read_to_string(dir.path().join("settings.json")).unwrap(),
            original
        );
        for autoload in [true, false] {
            let original = json!({"packages":[{"source":"./pkg","autoload":autoload,"prompts":value}],"unknown":true}).to_string();
            write(&dir.path().join("settings.json"), &original);
            let mut packaged = resource.clone();
            packaged.package = Some("./pkg".into());
            assert!(set_enabled(dir.path(), &packaged, true).is_err());
            assert_eq!(
                fs::read_to_string(dir.path().join("settings.json")).unwrap(),
                original
            );
        }
    }
    write(
        &dir.path().join("settings.json"),
        r#"{"packages":[{"source":"./pkg","autoload":false,"prompts":[] }]}"#,
    );
    let before = fs::read_to_string(dir.path().join("settings.json")).unwrap();
    let mut packaged = resource;
    packaged.package = Some("./pkg".into());
    assert!(set_enabled(dir.path(), &packaged, true).is_err());
    assert_eq!(
        fs::read_to_string(dir.path().join("settings.json")).unwrap(),
        before
    );
}

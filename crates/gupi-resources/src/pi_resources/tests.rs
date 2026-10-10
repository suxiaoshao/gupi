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

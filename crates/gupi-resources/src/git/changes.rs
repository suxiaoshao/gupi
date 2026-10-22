//! Bounded, read-only worktree snapshots. Paths remain OS identities, never pathspecs.
use super::*;
use std::collections::HashMap;
use std::io::Read;
use std::time::SystemTime;

const MAX_FILES: usize = 5_000;
const MAX_TEXT: usize = 1024 * 1024;
const MAX_LINES: usize = 50_000;
const MAX_LINE: usize = 16_384;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChangeSource {
    #[default]
    Unstaged,
    Staged,
    Untracked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    Modified,
    Added,
    Deleted,
    Conflict,
    Special,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    path: PathBuf,
    kind: ChangeKind,
    stats: Option<(usize, usize)>,
    head: String,
    index: String,
    mode: String,
    stamp: Option<(u64, Option<SystemTime>)>,
    disk: bool,
}
impl Change {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn kind(&self) -> ChangeKind {
        self.kind
    }
    pub fn can_open_worktree(&self) -> bool {
        self.disk && self.path.to_str().is_some()
    }
    #[cfg(feature = "test-support")]
    pub fn for_test(path: PathBuf, kind: ChangeKind) -> Self {
        Self {
            path,
            kind,
            stats: Some((1, 1)),
            head: String::new(),
            index: String::new(),
            mode: String::new(),
            stamp: None,
            disk: false,
        }
    }
    pub fn stats(&self) -> Option<(usize, usize)> {
        self.stats
    }
}
#[derive(Debug)]
pub struct ChangeList {
    source: ChangeSource,
    files: Vec<Change>,
    stats_error: Option<Error>,
}
impl ChangeList {
    pub fn into_parts(self) -> (ChangeSource, Vec<Change>, Option<Error>) {
        (self.source, self.files, self.stats_error)
    }
    pub fn source(&self) -> ChangeSource {
        self.source
    }
    pub fn files(&self) -> &[Change] {
        &self.files
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentState {
    Text,
    Binary,
    Special,
    Conflict,
    TooLarge,
    InvalidPath,
    NoTextChanges,
}
#[derive(Clone, Debug)]
pub struct ChangeContent {
    state: ContentState,
    patch: Option<String>,
    original: Option<String>,
    modified: Option<String>,
    disk: Option<PathBuf>,
}
impl ChangeContent {
    #[cfg(feature = "test-support")]
    pub fn for_test(original: String, modified: String, patch: String) -> Self {
        Self {
            state: if original == modified {
                ContentState::NoTextChanges
            } else {
                ContentState::Text
            },
            original: Some(original),
            modified: Some(modified),
            patch: Some(patch),
            disk: None,
        }
    }
    #[cfg(feature = "test-support")]
    pub fn unavailable_for_test(state: ContentState) -> Self {
        Self {
            state,
            patch: None,
            original: None,
            modified: None,
            disk: None,
        }
    }
    pub fn state(&self) -> ContentState {
        self.state
    }
    pub fn patch(&self) -> Option<&str> {
        self.patch.as_deref()
    }
    pub fn original(&self) -> Option<&str> {
        self.original.as_deref()
    }
    pub fn modified(&self) -> Option<&str> {
        self.modified.as_deref()
    }
    pub fn disk_path(&self) -> Option<&Path> {
        self.disk.as_deref()
    }
}
impl Repository {
    /// All source memberships come from one status output. Statistics are best-effort
    /// independent reads; their failure must never remove entries from the snapshot.
    /// Returns staged, unstaged and untracked lists, in that order.
    pub async fn changes(&self) -> Result<[ChangeList; 3], Error> {
        self.changes_from_status(&self.status().await?).await
    }
    async fn status(&self) -> Result<Vec<u8>, Error> {
        Ok(checked(
            self.root(),
            &[
                "status",
                "--porcelain=v2",
                "--no-renames",
                "-z",
                "--untracked-files=all",
                "--ignore-submodules=none",
            ],
        )
        .await?
        .stdout)
    }
    async fn changes_from_status(&self, bytes: &[u8]) -> Result<[ChangeList; 3], Error> {
        let mut lists = [
            self.status_list(bytes, ChangeSource::Staged)?,
            self.status_list(bytes, ChangeSource::Unstaged)?,
            self.status_list(bytes, ChangeSource::Untracked)?,
        ];
        for list in &mut lists {
            list.stats_error = self.add_stats(list).await.err();
        }
        Ok(lists)
    }
    fn status_list(&self, bytes: &[u8], source: ChangeSource) -> Result<ChangeList, Error> {
        let mut files = parse_status(bytes, source)?;
        if files.len() > MAX_FILES {
            return Err(Error::TooLarge);
        }
        for file in &mut files {
            if file.path.to_str().is_none() {
                continue;
            }
            let metadata = std::fs::symlink_metadata(self.root.join(&file.path)).ok();
            file.disk = metadata.as_ref().is_some_and(|m| m.is_file());
            if source != ChangeSource::Staged
                && file.kind != ChangeKind::Deleted
                && metadata
                    .as_ref()
                    .is_some_and(|m| !m.is_file() && !m.file_type().is_symlink())
            {
                file.kind = ChangeKind::Special;
            }
            if source != ChangeSource::Staged {
                file.stamp = stamp(&self.root.join(&file.path))?;
            }
        }
        Ok(ChangeList {
            source,
            files,
            stats_error: None,
        })
    }
    async fn add_stats(&self, list: &mut ChangeList) -> Result<(), Error> {
        if list.source == ChangeSource::Untracked {
            return Ok(());
        }
        let mut args = vec![
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--numstat",
            "-z",
            "--no-renames",
        ];
        if list.source == ChangeSource::Staged {
            args.push("--cached");
        }
        let stats = numstats(&checked(self.root(), &args).await?.stdout)?;
        for file in &mut list.files {
            if !matches!(file.kind, ChangeKind::Conflict | ChangeKind::Special) {
                file.stats = stats.get(&file.path).copied().flatten();
            }
        }
        Ok(())
    }
    /// Metadata is checked before and after reading. Never mix a changed worktree
    /// body with a previously selected index/HEAD identity or count snapshot.
    pub async fn change_content(
        &self,
        source: ChangeSource,
        file: &Change,
    ) -> Result<ChangeContent, Error> {
        let Some(path) = file.path.to_str() else {
            return Ok(ChangeContent {
                state: ContentState::InvalidPath,
                patch: None,
                original: None,
                modified: None,
                disk: None,
            });
        };
        self.validate(source, file).await?;
        let full = self.root.join(&file.path);
        let disk = std::fs::symlink_metadata(&full)
            .ok()
            .filter(|m| m.is_file())
            .map(|_| full.clone());
        let mut content = ChangeContent {
            state: ContentState::Text,
            patch: None,
            original: None,
            modified: None,
            disk,
        };
        if file.kind == ChangeKind::Conflict {
            content.state = ContentState::Conflict;
        } else if file.kind == ChangeKind::Special {
            content.state = ContentState::Special;
        } else {
            let original = self
                .blob(if source == ChangeSource::Staged {
                    &file.head
                } else {
                    &file.index
                })
                .await;
            let modified = if source == ChangeSource::Staged {
                self.blob(&file.index).await
            } else if file.kind == ChangeKind::Deleted {
                Ok(Vec::new())
            } else {
                read_worktree(&full)
            };
            match (original, modified) {
                (Ok(old), Ok(new)) => {
                    if old.contains(&0) || new.contains(&0) {
                        content.state = ContentState::Binary;
                    } else if let (Ok(old), Ok(new)) =
                        (String::from_utf8(old), String::from_utf8(new))
                    {
                        if !text_within_limits(&old) || !text_within_limits(&new) {
                            content.state = ContentState::TooLarge;
                        } else {
                            let patch = if source == ChangeSource::Untracked {
                                Some(added_patch(
                                    path,
                                    &new,
                                    worktree_mode(self.root(), &full).await?,
                                ))
                            } else {
                                let mut args = vec![
                                    "--literal-pathspecs",
                                    "diff",
                                    "--text",
                                    "--no-ext-diff",
                                    "--no-textconv",
                                    "--no-color",
                                    "--unified=50000",
                                    "--no-renames",
                                ];
                                if source == ChangeSource::Staged {
                                    args.push("--cached");
                                }
                                args.push("--");
                                args.push(path);
                                match checked(self.root(), &args).await {
                                    Ok(out) => Some(
                                        String::from_utf8(out.stdout)
                                            .map_err(|e| Error::Failed(e.to_string()))?,
                                    ),
                                    Err(Error::TooLarge) => None,
                                    Err(e) => return Err(e),
                                }
                            };
                            // --text must not produce binary-only patch metadata. Treat a
                            // contradictory response as a query failure, not a content verdict.
                            if patch.as_ref().is_some_and(|patch| {
                                patch.lines().any(|line| {
                                    line.starts_with("Binary files ") || line == "GIT binary patch"
                                })
                            }) {
                                return Err(Error::Failed(
                                    "Git returned binary metadata despite --text".into(),
                                ));
                            }
                            content.state = if patch.is_none() {
                                ContentState::TooLarge
                            } else if file.kind == ChangeKind::Modified
                                && patch.as_ref().is_some_and(|patch| {
                                    !patch.lines().any(|line| line.starts_with("@@ "))
                                })
                            {
                                ContentState::NoTextChanges
                            } else {
                                ContentState::Text
                            };
                            content.original = Some(old);
                            content.modified = Some(new);
                            content.patch = patch;
                        }
                    } else {
                        content.state = ContentState::Binary;
                    }
                }
                (Err(Error::TooLarge), _) | (_, Err(Error::TooLarge)) => {
                    content.state = ContentState::TooLarge
                }
                (Err(e), _) | (_, Err(e)) => return Err(e),
            }
        }
        self.validate(source, file).await?;
        Ok(content)
    }
    async fn validate(&self, source: ChangeSource, file: &Change) -> Result<(), Error> {
        let mut current = self.status_list(&self.status().await?, source)?;
        self.add_stats(&mut current).await?;
        if current.files.iter().any(|entry| entry == file) {
            Ok(())
        } else {
            Err(Error::Changed)
        }
    }
    async fn blob(&self, object: &str) -> Result<Vec<u8>, Error> {
        if object.is_empty() || object.bytes().all(|c| c == b'0') {
            return Ok(Vec::new());
        }
        let out = run_bounded(self.root(), &["cat-file", "blob", object], "git", MAX_TEXT).await?;
        if out.status.success() {
            Ok(out.stdout)
        } else {
            Err(failure(out))
        }
    }
}
fn stamp(path: &Path) -> Result<Option<(u64, Option<SystemTime>)>, Error> {
    match std::fs::symlink_metadata(path) {
        Ok(m) => Ok(Some((m.len(), m.modified().ok()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
fn read_worktree(path: &Path) -> Result<Vec<u8>, Error> {
    let Some((size, _)) = stamp(path)? else {
        return Ok(Vec::new());
    };
    if size > MAX_TEXT as u64 {
        return Err(Error::TooLarge);
    }
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Ok(std::fs::read_link(path)?
            .as_os_str()
            .as_encoded_bytes()
            .to_vec());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_TEXT as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_TEXT {
        return Err(Error::TooLarge);
    }
    Ok(bytes)
}
pub fn text_within_limits(text: &str) -> bool {
    text.len() <= MAX_TEXT
        && text.lines().take(MAX_LINES + 1).count() <= MAX_LINES
        && text.lines().all(|l| l.len() <= MAX_LINE)
}
async fn worktree_mode(root: &Path, path: &Path) -> Result<&'static str, Error> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok("120000");
    }
    // Let Git parse its effective Boolean configuration, including inherited values.
    // Git defaults to true when unset. Fail closed on invalid/unreadable config:
    // guessing a mode could export a patch that changes executable semantics.
    let configured = checked(
        root,
        &[
            "config",
            "--type=bool",
            "--default=true",
            "--get",
            "core.fileMode",
        ],
    )
    .await?;
    let file_mode = match value(&configured.stdout)?.as_str() {
        "true" => true,
        "false" => false,
        _ => {
            return Err(Error::Failed(
                "Invalid core.fileMode response from Git".into(),
            ));
        }
    };
    #[cfg(not(unix))]
    let _ = file_mode;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if file_mode && metadata.permissions().mode() & 0o100 != 0 {
            return Ok("100755");
        }
    }
    Ok("100644")
}
/// Git's quoted paths use C-style byte escapes, not JSON Unicode escapes.
fn quote_path(prefix: &str, path: &str) -> String {
    let mut result = String::from("\"");
    for byte in prefix.bytes().chain(path.bytes()) {
        match byte {
            b'"' | b'\\' => {
                result.push('\\');
                result.push(char::from(byte));
            }
            0x20..=0x7e => result.push(char::from(byte)),
            _ => result.push_str(&format!("\\{byte:03o}")),
        }
    }
    result.push('"');
    result
}
fn added_patch(path: &str, text: &str, mode: &str) -> String {
    let old = quote_path("a/", path);
    let new = quote_path("b/", path);
    let mut patch =
        format!("diff --git {old} {new}\nnew file mode {mode}\n--- /dev/null\n+++ {new}\n");
    if text.is_empty() {
        return patch;
    }
    patch.push_str(&format!("@@ -0,0 +1,{} @@\n", text.lines().count()));
    for line in text.split_inclusive('\n') {
        patch.push('+');
        patch.push_str(line);
        if !line.ends_with('\n') {
            patch.push_str("\n\\ No newline at end of file\n");
        }
    }
    patch
}
fn parse_status(bytes: &[u8], source: ChangeSource) -> Result<Vec<Change>, Error> {
    let mut files = Vec::new();
    for record in bytes.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let parts = match record[0] {
            b'?' => {
                if source == ChangeSource::Untracked {
                    files.push(Change {
                        path: record_path(&record[2..])?,
                        kind: ChangeKind::Added,
                        stats: None,
                        head: String::new(),
                        index: String::new(),
                        mode: String::new(),
                        stamp: None,
                        disk: false,
                    });
                }
                continue;
            }
            b'1' => record.splitn(9, |b| *b == b' ').collect::<Vec<_>>(),
            b'u' => {
                let parts = record.splitn(11, |b| *b == b' ').collect::<Vec<_>>();
                if source != ChangeSource::Untracked && parts.len() == 11 {
                    files.push(Change {
                        path: record_path(parts[10])?,
                        kind: ChangeKind::Conflict,
                        stats: None,
                        head: String::new(),
                        index: String::new(),
                        mode: String::new(),
                        stamp: None,
                        disk: false,
                    });
                }
                continue;
            }
            _ => continue,
        };
        if parts.len() < 9 || parts[1].len() != 2 {
            return Err(Error::Failed("Invalid Git status".into()));
        }
        let code = parts[1][usize::from(source == ChangeSource::Unstaged)];
        if source == ChangeSource::Untracked || code == b'.' {
            continue;
        }
        let special =
            parts[2].first() == Some(&b'S') || parts[3] == b"160000" || parts[4] == b"160000";
        files.push(Change {
            path: record_path(parts[parts.len() - 1])?,
            kind: if special {
                ChangeKind::Special
            } else {
                match code {
                    b'A' => ChangeKind::Added,
                    b'D' => ChangeKind::Deleted,
                    _ => ChangeKind::Modified,
                }
            },
            stats: None,
            head: value(parts[6])?,
            index: value(parts[7])?,
            mode: value(parts[5])?,
            stamp: None,
            disk: false,
        });
        if files.len() > MAX_FILES {
            return Err(Error::TooLarge);
        }
    }
    Ok(files)
}
fn numstats(bytes: &[u8]) -> Result<HashMap<PathBuf, Option<(usize, usize)>>, Error> {
    let mut result = HashMap::new();
    for record in bytes.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let mut parts = record.splitn(3, |b| *b == b'\t');
        let added = parts.next().unwrap_or_default();
        let removed = parts.next().unwrap_or_default();
        let name = parts
            .next()
            .ok_or_else(|| Error::Failed("Invalid Git numstat".into()))?;
        let stats = std::str::from_utf8(added)
            .ok()
            .and_then(|a| a.parse().ok())
            .zip(
                std::str::from_utf8(removed)
                    .ok()
                    .and_then(|r| r.parse().ok()),
            );
        result.insert(record_path(name)?, stats);
    }
    Ok(result)
}

fn record_path(bytes: &[u8]) -> Result<PathBuf, Error> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(bytes.to_vec())
            .map(PathBuf::from)
            .map_err(|e| Error::Failed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn git(root: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .current_dir(root)
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.autocrlf=false",
            ])
            .arg("-c")
            .arg(format!(
                "core.hooksPath={}",
                root.join("no-hooks").display()
            ))
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    async fn read_changes(repo: &Repository, source: ChangeSource) -> ChangeList {
        let list = repo
            .changes()
            .await
            .unwrap()
            .into_iter()
            .find(|list| list.source() == source)
            .unwrap();
        assert!(list.stats_error.is_none());
        list
    }
    #[test]
    fn text_policy_bounds_bytes_lines_and_long_lines() {
        assert!(text_within_limits("let value = 1;\n"));
        assert!(!text_within_limits(&"x".repeat(MAX_LINE + 1)));
        assert!(!text_within_limits(&"\n".repeat(MAX_LINES + 1)));
        assert!(!text_within_limits(
            &"short line\n".repeat(MAX_TEXT / 10 + 1)
        ));
    }
    #[test]
    fn staged_unstaged_untracked_versions_and_stale_reads_are_distinct() {
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet"]);
            std::fs::write(root.join(".gitignore"), "ignored\n").unwrap();
            std::fs::write(root.join("[same] file.txt"), "base\n").unwrap();
            git(root, &["add", "."]);
            let repo = discover(root).await.unwrap().unwrap();
            // An unborn HEAD has an empty staged baseline.
            let staged = read_changes(&repo, ChangeSource::Staged).await;
            let added = staged
                .files()
                .iter()
                .find(|f| f.path() == Path::new("[same] file.txt"))
                .unwrap();
            assert_eq!(
                repo.change_content(ChangeSource::Staged, added)
                    .await
                    .unwrap()
                    .original(),
                Some("")
            );
            git(root, &["commit", "--quiet", "-m", "base"]);
            std::fs::write(root.join("[same] file.txt"), "index\n").unwrap();
            git(root, &["add", "."]);
            std::fs::write(root.join("[same] file.txt"), "disk\n").unwrap();
            std::fs::write(root.join("ignored"), "secret").unwrap();
            std::fs::write(root.join("new file.txt"), "new\n").unwrap();
            let index_before = std::fs::read(repo.git_dir().join("index")).unwrap();
            let [staged, unstaged, new] = repo.changes().await.unwrap();
            assert_eq!(new.files().len(), 1);
            let old = repo
                .change_content(ChangeSource::Staged, &staged.files()[0])
                .await
                .unwrap();
            assert_eq!(
                (old.original(), old.modified()),
                (Some("base\n"), Some("index\n"))
            );
            let current = repo
                .change_content(ChangeSource::Unstaged, &unstaged.files()[0])
                .await
                .unwrap();
            assert_eq!(
                (current.original(), current.modified()),
                (Some("index\n"), Some("disk\n"))
            );
            assert!(current.patch().unwrap().contains("+disk"));
            assert_eq!(
                repo.change_content(ChangeSource::Untracked, &new.files()[0])
                    .await
                    .unwrap()
                    .modified(),
                Some("new\n")
            );
            assert_eq!(
                std::fs::read(repo.git_dir().join("index")).unwrap(),
                index_before
            );
            std::fs::write(root.join("[same] file.txt"), "changed after list\n").unwrap();
            assert!(matches!(
                repo.change_content(ChangeSource::Unstaged, &unstaged.files()[0])
                    .await,
                Err(Error::Changed)
            ));
        });
    }
    #[test]
    fn rename_delete_binary_limits_and_no_external_drivers() {
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet"]);
            for name in ["old.txt", "delete.txt", "binary.bin"] {
                std::fs::write(root.join(name), "base\n").unwrap();
            }
            git(root, &["add", "."]);
            git(root, &["commit", "--quiet", "-m", "base"]);
            git(root, &["mv", "old.txt", "renamed file.txt"]);
            std::fs::remove_file(root.join("delete.txt")).unwrap();
            std::fs::write(root.join("binary.bin"), [0, 1, 2]).unwrap();
            // These commands would fail if either unsafe diff hook were executed.
            git(root, &["config", "diff.external", "gupi-must-never-run"]);
            git(
                root,
                &["config", "diff.fixture.textconv", "gupi-must-never-run"],
            );
            std::fs::write(root.join(".gitattributes"), "*.txt diff=fixture\n").unwrap();
            std::fs::write(root.join("huge"), vec![b'x'; MAX_TEXT + 1]).unwrap();
            let repo = discover(root).await.unwrap().unwrap();
            let staged = read_changes(&repo, ChangeSource::Staged).await;
            assert_eq!(staged.files().len(), 2);
            for (path, kind, old, new) in [
                ("old.txt", ChangeKind::Deleted, "base\n", ""),
                ("renamed file.txt", ChangeKind::Added, "", "base\n"),
            ] {
                let file = staged
                    .files()
                    .iter()
                    .find(|f| f.path() == Path::new(path))
                    .unwrap();
                assert_eq!(file.kind(), kind);
                let body = repo
                    .change_content(ChangeSource::Staged, file)
                    .await
                    .unwrap();
                assert_eq!((body.original(), body.modified()), (Some(old), Some(new)));
                let patch = body.patch().unwrap();
                assert!(!patch.contains("rename from"));
                assert!(!patch.contains("similarity index"));
            }
            let list = read_changes(&repo, ChangeSource::Unstaged).await;
            for file in list.files() {
                let body = repo
                    .change_content(ChangeSource::Unstaged, file)
                    .await
                    .unwrap();
                if file.path() == Path::new("binary.bin") {
                    assert_eq!(body.state(), ContentState::Binary);
                    assert!(file.stats().is_none());
                }
                if file.path() == Path::new("delete.txt") {
                    assert_eq!(body.original(), Some("base\n"));
                    assert!(body.disk_path().is_none());
                }
            }
            let list = read_changes(&repo, ChangeSource::Untracked).await;
            let huge = list
                .files()
                .iter()
                .find(|f| f.path() == Path::new("huge"))
                .unwrap();
            assert_eq!(
                repo.change_content(ChangeSource::Untracked, huge)
                    .await
                    .unwrap()
                    .state(),
                ContentState::TooLarge
            );
            assert!(matches!(
                run_bounded(root, &["status", "--porcelain=v2", "-z"], "git", 1).await,
                Err(Error::TooLarge)
            ));
        });
    }
    #[cfg(unix)]
    #[test]
    fn nul_paths_and_symlinks_do_not_read_external_targets() {
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet"]);
            let name = "[name]\n.txt\n";
            std::fs::write(root.join(name), "content").unwrap();
            std::os::unix::fs::symlink("/gupi-nonexistent-outside-target", root.join("link"))
                .unwrap();
            let repo = discover(root).await.unwrap().unwrap();
            let list = read_changes(&repo, ChangeSource::Untracked).await;
            let file = list
                .files()
                .iter()
                .find(|f| f.path() == Path::new(name))
                .unwrap();
            assert_eq!(
                repo.change_content(ChangeSource::Untracked, file)
                    .await
                    .unwrap()
                    .modified(),
                Some("content")
            );
            let link = list
                .files()
                .iter()
                .find(|f| f.path() == Path::new("link"))
                .unwrap();
            let body = repo
                .change_content(ChangeSource::Untracked, link)
                .await
                .unwrap();
            assert_eq!(body.modified(), Some("/gupi-nonexistent-outside-target"));
            assert!(body.disk_path().is_none());
        });
    }

    #[test]
    fn intent_to_add_move_is_two_unstaged_paths_without_rename_detection() {
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet"]);
            std::fs::write(root.join("old.txt"), "same\n").unwrap();
            git(root, &["add", "."]);
            git(root, &["commit", "--quiet", "-m", "base"]);
            std::fs::rename(root.join("old.txt"), root.join("new.txt")).unwrap();
            git(root, &["add", "-N", "new.txt"]);
            // Even an explicit user rename setting cannot override the reader's policy.
            git(root, &["config", "diff.renames", "copies"]);
            let repo = discover(root).await.unwrap().unwrap();
            let list = read_changes(&repo, ChangeSource::Unstaged).await;
            assert_eq!(list.files().len(), 2);
            for (path, kind, old, new) in [
                ("old.txt", ChangeKind::Deleted, "same\n", ""),
                ("new.txt", ChangeKind::Added, "", "same\n"),
            ] {
                let file = list
                    .files()
                    .iter()
                    .find(|f| f.path() == Path::new(path))
                    .unwrap();
                assert_eq!(file.kind(), kind);
                let content = repo
                    .change_content(ChangeSource::Unstaged, file)
                    .await
                    .unwrap();
                assert_eq!(
                    (content.original(), content.modified()),
                    (Some(old), Some(new))
                );
                let patch = content.patch().unwrap();
                assert!(!patch.contains("rename from") && !patch.contains("similarity index"));
                assert!(patch.contains(if kind == ChangeKind::Added {
                    "new file mode"
                } else {
                    "deleted file mode"
                }));
            }
        });
    }

    #[test]
    fn deleted_file_replaced_by_directory_keeps_deleted_body_and_untracked_child() {
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet"]);
            std::fs::write(root.join("replaced"), "deleted text\n").unwrap();
            git(root, &["add", "."]);
            git(root, &["commit", "--quiet", "-m", "base"]);
            std::fs::remove_file(root.join("replaced")).unwrap();
            std::fs::create_dir(root.join("replaced")).unwrap();
            std::fs::write(root.join("replaced/child.txt"), "child\n").unwrap();
            let repo = discover(root).await.unwrap().unwrap();
            let list = read_changes(&repo, ChangeSource::Unstaged).await;
            let file = list
                .files()
                .iter()
                .find(|f| f.path() == Path::new("replaced"))
                .unwrap();
            assert_eq!(file.kind(), ChangeKind::Deleted);
            let content = repo
                .change_content(ChangeSource::Unstaged, file)
                .await
                .unwrap();
            assert_eq!(content.state(), ContentState::Text);
            assert_eq!(
                (content.original(), content.modified()),
                (Some("deleted text\n"), Some(""))
            );
            assert!(content.patch().unwrap().contains("-deleted text"));
            assert!(content.disk_path().is_none() && !file.can_open_worktree());
            let untracked = read_changes(&repo, ChangeSource::Untracked).await;
            assert_eq!(untracked.files().len(), 1);
            assert_eq!(untracked.files()[0].path(), Path::new("replaced/child.txt"));
        });
    }

    #[cfg(unix)]
    #[test]
    fn untracked_modes_preserve_regular_executable_and_symlink_patches() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        smol::block_on(async {
            for setting in [Some("true"), Some("false"), None] {
                let fixture = tempfile::tempdir().unwrap();
                let root = fixture.path();
                git(root, &["init", "--quiet"]);
                if let Some(setting) = setting {
                    git(root, &["config", "core.fileMode", setting]);
                } else {
                    git(root, &["config", "--unset", "core.fileMode"]);
                }
                for (name, mode) in [
                    ("plain.txt", 0o644),
                    ("other-exec.txt", 0o645),
                    ("group-exec.txt", 0o654),
                    ("executable.sh", 0o744),
                ] {
                    std::fs::write(root.join(name), "text\n").unwrap();
                    std::fs::set_permissions(
                        root.join(name),
                        std::fs::Permissions::from_mode(mode),
                    )
                    .unwrap();
                }
                symlink("plain.txt", root.join("link")).unwrap();
                let repo = discover(root).await.unwrap().unwrap();
                let list = read_changes(&repo, ChangeSource::Untracked).await;
                for (path, mode) in [
                    ("plain.txt", "100644"),
                    ("other-exec.txt", "100644"),
                    ("group-exec.txt", "100644"),
                    (
                        "executable.sh",
                        if setting == Some("false") {
                            "100644"
                        } else {
                            "100755"
                        },
                    ),
                    ("link", "120000"),
                ] {
                    let file = list
                        .files()
                        .iter()
                        .find(|f| f.path() == Path::new(path))
                        .unwrap();
                    let content = repo
                        .change_content(ChangeSource::Untracked, file)
                        .await
                        .unwrap();
                    let patch = content.patch().unwrap();
                    git(root, &["add", "--", path]);
                    let indexed = std::process::Command::new("git")
                        .current_dir(root)
                        .args(["ls-files", "--stage", "--", path])
                        .output()
                        .unwrap();
                    assert!(indexed.status.success());
                    let indexed = String::from_utf8(indexed.stdout).unwrap();
                    let indexed_mode = indexed.split_whitespace().next().unwrap();
                    assert_eq!(indexed_mode, mode);
                    assert!(patch.contains(&format!("new file mode {indexed_mode}\n")));
                    let target = tempfile::tempdir().unwrap();
                    git(target.path(), &["init", "--quiet"]);
                    let input = target.path().join("fixture.patch");
                    std::fs::write(&input, patch).unwrap();
                    git(target.path(), &["apply", "fixture.patch"]);
                    let metadata = std::fs::symlink_metadata(target.path().join(path)).unwrap();
                    if mode == "120000" {
                        assert!(metadata.file_type().is_symlink());
                        assert_eq!(
                            std::fs::read_link(target.path().join(path)).unwrap(),
                            Path::new("plain.txt")
                        );
                    } else {
                        assert_eq!(metadata.permissions().mode() & 0o100 != 0, mode == "100755");
                    }
                }
            }
        });
    }

    #[cfg(unix)]
    #[test]
    fn mode_only_staged_and_unstaged_keep_raw_patch_without_text_changes() {
        use std::os::unix::fs::PermissionsExt;
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet"]);
            git(root, &["config", "core.fileMode", "true"]);
            std::fs::write(root.join("mode.sh"), "echo same\n").unwrap();
            std::fs::set_permissions(root.join("mode.sh"), std::fs::Permissions::from_mode(0o644))
                .unwrap();
            git(root, &["add", "."]);
            git(root, &["commit", "--quiet", "-m", "base"]);
            std::fs::set_permissions(root.join("mode.sh"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
            let repo = discover(root).await.unwrap().unwrap();
            for source in [ChangeSource::Unstaged, ChangeSource::Staged] {
                if source == ChangeSource::Staged {
                    git(root, &["add", "mode.sh"]);
                }
                let list = read_changes(&repo, source).await;
                assert_eq!(list.files()[0].kind(), ChangeKind::Modified);
                let content = repo.change_content(source, &list.files()[0]).await.unwrap();
                assert_eq!(content.state(), ContentState::NoTextChanges);
                assert_eq!(
                    (content.original(), content.modified()),
                    (Some("echo same\n"), Some("echo same\n"))
                );
                let patch = content.patch().unwrap();
                assert!(patch.contains("old mode 100644\nnew mode 100755\n"));
                assert!(!patch.contains("@@"));
            }
        });
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_status_paths_are_retained_but_rejected_before_any_git_or_body_read() {
        use std::os::unix::ffi::OsStrExt;
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            // No Git repository: unsupported names must be rejected before validation or I/O.
            let repo = Repository {
                root: fixture.path().into(),
                git_dir: fixture.path().join(".git"),
                common_dir: fixture.path().join(".git"),
            };
            for (source, bytes) in [
                (ChangeSource::Untracked, b"? bad-\xff.txt\0".as_slice()),
                (
                    ChangeSource::Unstaged,
                    b"1 .M N... 100644 100644 100644 1111 1111 bad-\xff.txt\0".as_slice(),
                ),
                (
                    ChangeSource::Staged,
                    b"1 M. N... 100644 100644 100644 1111 1111 bad-\xff.txt\0".as_slice(),
                ),
            ] {
                let mut files = parse_status(bytes, source).unwrap();
                assert_eq!(files.len(), 1);
                files[0].disk = true;
                assert_eq!(files[0].path().as_os_str().as_bytes(), b"bad-\xff.txt");
                assert!(!files[0].can_open_worktree());
                let content = repo.change_content(source, &files[0]).await.unwrap();
                assert_eq!(content.state(), ContentState::InvalidPath);
                assert!(
                    content.patch().is_none()
                        && content.original().is_none()
                        && content.modified().is_none()
                        && content.disk_path().is_none()
                );
            }
        });
    }
    #[cfg(unix)]
    #[test]
    fn unicode_control_names_generate_git_compatible_patch_headers() {
        smol::block_on(async {
            let source = tempfile::tempdir().unwrap();
            let target = tempfile::tempdir().unwrap();
            git(source.path(), &["init", "--quiet"]);
            git(target.path(), &["init", "--quiet"]);
            let name = "line-\u{1}-é\n.txt";
            std::fs::write(source.path().join(name), "contents\n").unwrap();
            let repo = discover(source.path()).await.unwrap().unwrap();
            let list = read_changes(&repo, ChangeSource::Untracked).await;
            let content = repo
                .change_content(ChangeSource::Untracked, &list.files()[0])
                .await
                .unwrap();
            std::fs::write(
                target.path().join("fixture.patch"),
                content.patch().unwrap(),
            )
            .unwrap();
            git(target.path(), &["apply", "fixture.patch"]);
            assert_eq!(
                std::fs::read(target.path().join(name)).unwrap(),
                b"contents\n"
            );
        });
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn filesystem_non_unicode_files_stay_listed_without_readable_bodies() {
        use std::os::unix::ffi::OsStringExt;
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet"]);
            let tracked = PathBuf::from(std::ffi::OsString::from_vec(b"tracked-\xff.txt".to_vec()));
            let untracked = PathBuf::from(std::ffi::OsString::from_vec(b"new-\xfe.txt".to_vec()));
            std::fs::write(root.join(&tracked), "base\n").unwrap();
            git(root, &["add", "."]);
            git(root, &["commit", "--quiet", "-m", "base"]);
            std::fs::write(root.join(&tracked), "modified\n").unwrap();
            std::fs::write(root.join(&untracked), "new\n").unwrap();
            let repo = discover(root).await.unwrap().unwrap();
            for (source, path) in [
                (ChangeSource::Unstaged, &tracked),
                (ChangeSource::Untracked, &untracked),
                (ChangeSource::Staged, &tracked),
            ] {
                if source == ChangeSource::Staged {
                    git(root, &["add", "."]);
                }
                let list = read_changes(&repo, source).await;
                let file = list.files().iter().find(|f| f.path() == path).unwrap();
                let content = repo.change_content(source, file).await.unwrap();
                assert_eq!(content.state(), ContentState::InvalidPath);
                assert!(content.patch().is_none() && content.modified().is_none());
                assert!(!file.can_open_worktree());
            }
        });
    }
    #[cfg(unix)]
    #[test]
    fn mode_only_with_git_line_ending_normalization_still_has_no_text_changes() {
        use std::os::unix::fs::PermissionsExt;
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet"]);
            git(root, &["config", "core.fileMode", "true"]);
            std::fs::write(root.join(".gitattributes"), "mode.sh text eol=crlf\n").unwrap();
            std::fs::write(root.join("mode.sh"), "same\n").unwrap();
            std::fs::set_permissions(root.join("mode.sh"), std::fs::Permissions::from_mode(0o644))
                .unwrap();
            git(root, &["add", "."]);
            git(root, &["commit", "--quiet", "-m", "base"]);
            std::fs::write(root.join("mode.sh"), "same\r\n").unwrap();
            std::fs::set_permissions(root.join("mode.sh"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
            let repo = discover(root).await.unwrap().unwrap();
            let files = read_changes(&repo, ChangeSource::Unstaged).await;
            let content = repo
                .change_content(ChangeSource::Unstaged, &files.files()[0])
                .await
                .unwrap();
            assert_ne!(content.original(), content.modified());
            assert_eq!(content.state(), ContentState::NoTextChanges);
            assert!(!content.patch().unwrap().contains("@@"));
        });
    }
    #[cfg(unix)]
    #[test]
    fn index_non_unicode_paths_stay_listed_on_hosts_that_cannot_create_the_name() {
        use std::io::Write;
        use std::os::unix::ffi::OsStrExt;
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet", "--object-format=sha1"]);
            std::fs::write(root.join("empty"), "").unwrap();
            git(root, &["hash-object", "-w", "empty"]);
            let mut child = std::process::Command::new("git")
                .current_dir(root)
                .args(["update-index", "-z", "--index-info"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"100644 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391\tbad-\xff.txt\0")
                .unwrap();
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let repo = discover(root).await.unwrap().unwrap();
            for source in [ChangeSource::Staged, ChangeSource::Unstaged] {
                let list = read_changes(&repo, source).await;
                let file = list
                    .files()
                    .iter()
                    .find(|f| f.path().as_os_str().as_bytes() == b"bad-\xff.txt")
                    .unwrap();
                assert!(!file.can_open_worktree());
                assert_eq!(
                    repo.change_content(source, file).await.unwrap().state(),
                    ContentState::InvalidPath
                );
            }
        });
    }
    #[test]
    fn staging_after_status_capture_cannot_drop_file_between_sources() {
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet"]);
            std::fs::write(root.join("file.txt"), "base\n").unwrap();
            git(root, &["add", "."]);
            git(root, &["commit", "--quiet", "-m", "base"]);
            std::fs::write(root.join("file.txt"), "changed\n").unwrap();
            let repo = discover(root).await.unwrap().unwrap();
            let status = repo.status().await.unwrap();
            // A real index change between the shared status read and the later
            // numstat queries must not decide whether the file exists in the list.
            git(root, &["add", "."]);
            let [staged, unstaged, untracked] = repo.changes_from_status(&status).await.unwrap();
            assert!(staged.files().is_empty());
            assert_eq!(unstaged.files().len(), 1);
            assert_eq!(unstaged.files()[0].path(), Path::new("file.txt"));
            assert!(untracked.files().is_empty());
            let [staged, unstaged, _] = repo.changes().await.unwrap();
            assert_eq!(staged.files().len(), 1);
            assert!(unstaged.files().is_empty());
        });
    }
    #[test]
    fn statistics_failure_keeps_all_memberships_from_the_shared_status() {
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet"]);
            std::fs::write(root.join("file.txt"), "base\n").unwrap();
            git(root, &["add", "."]);
            git(root, &["commit", "--quiet", "-m", "base"]);
            std::fs::write(root.join("file.txt"), "index\n").unwrap();
            git(root, &["add", "."]);
            std::fs::write(root.join("file.txt"), "worktree\n").unwrap();
            std::fs::write(root.join("new.txt"), "new\n").unwrap();
            let repo = discover(root).await.unwrap().unwrap();
            let status = repo.status().await.unwrap();
            // Losing the repository after the one successful status read makes
            // both real numstat commands fail, without invalidating its entries.
            std::fs::rename(root.join(".git"), root.join("saved-git")).unwrap();
            let [staged, unstaged, untracked] = repo.changes_from_status(&status).await.unwrap();
            for list in [&staged, &unstaged] {
                assert_eq!(list.files().len(), 1);
                assert_eq!(list.files()[0].path(), Path::new("file.txt"));
                assert!(list.files()[0].stats().is_none());
                assert!(list.stats_error.is_some());
            }
            assert_eq!(staged.files()[0].index, unstaged.files()[0].index);
            assert_eq!(untracked.files().len(), 1);
            assert_eq!(untracked.files()[0].path(), Path::new("new.txt"));
            assert!(untracked.stats_error.is_none());
            // Failure of the status command itself is instead a whole-snapshot error.
            assert!(repo.changes().await.is_err());
        });
    }
    #[cfg(unix)]
    #[test]
    fn untracked_mode_uses_git_boolean_parser_and_rejects_config_errors() {
        use std::os::unix::fs::PermissionsExt;
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet"]);
            let file = root.join("executable.sh");
            std::fs::write(&file, "echo test\n").unwrap();
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o744)).unwrap();
            for (value, expected) in [("yes", "100755"), ("0", "100644")] {
                git(root, &["config", "core.fileMode", value]);
                assert_eq!(worktree_mode(root, &file).await.unwrap(), expected);
            }
            git(root, &["config", "core.fileMode", "not-a-boolean"]);
            assert!(worktree_mode(root, &file).await.is_err());
            std::fs::write(root.join(".git/config"), "[broken config\n").unwrap();
            assert!(worktree_mode(root, &file).await.is_err());
        });
    }

    #[test]
    fn text_attributes_keep_unknown_stats_but_text_bodies_without_running_drivers() {
        smol::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let root = fixture.path();
            git(root, &["init", "--quiet"]);
            std::fs::write(
                root.join(".gitattributes"),
                "*.dat binary\n*.raw -diff\n*.cv diff=probe\n",
            )
            .unwrap();
            for name in [
                "text.dat",
                "text.raw",
                "text.cv",
                "plain.txt",
                "contains-nul.dat",
                "invalid.dat",
                "large.dat",
            ] {
                std::fs::write(root.join(name), "old\n").unwrap();
            }
            git(root, &["add", "."]);
            git(root, &["commit", "--quiet", "-m", "base"]);
            // Any accidental invocation fails the real query; no external programs run.
            for key in ["diff.external", "diff.probe.command", "diff.probe.textconv"] {
                git(root, &["config", key, "gupi-driver-must-not-run"]);
            }
            git(root, &["config", "diff.probe.binary", "true"]);
            for name in ["text.dat", "text.raw", "text.cv", "plain.txt"] {
                std::fs::write(root.join(name), "new\n").unwrap();
            }
            // NUL (even with an extension) is a Windows device name, so use an
            // ordinary filename and verify the actual binary fixture reached disk.
            std::fs::write(root.join("contains-nul.dat"), b"new\0\n").unwrap();
            assert_eq!(
                std::fs::read(root.join("contains-nul.dat")).unwrap(),
                b"new\0\n"
            );
            std::fs::write(root.join("invalid.dat"), b"new\xff\n").unwrap();
            std::fs::write(root.join("large.dat"), vec![b'x'; MAX_TEXT + 1]).unwrap();
            let repo = discover(root).await.unwrap().unwrap();
            for source in [ChangeSource::Unstaged, ChangeSource::Staged] {
                if source == ChangeSource::Staged {
                    git(root, &["add", "."]);
                }
                let list = read_changes(&repo, source).await;
                for name in ["text.dat", "text.raw", "text.cv", "plain.txt"] {
                    let file = list
                        .files()
                        .iter()
                        .find(|file| file.path() == Path::new(name))
                        .unwrap_or_else(|| {
                            panic!(
                                "missing {name:?} in {:?} list: {:?}",
                                list.source(),
                                list.files()
                            )
                        });
                    assert_eq!(
                        file.stats(),
                        if name == "plain.txt" {
                            Some((1, 1))
                        } else {
                            None
                        }
                    );
                    let body = repo.change_content(source, file).await.unwrap();
                    assert_eq!(body.state(), ContentState::Text);
                    assert_eq!(body.original(), Some("old\n"));
                    assert_eq!(body.modified(), Some("new\n"));
                    let patch = body.patch().unwrap();
                    assert!(patch.contains("@@ ") && patch.contains("-old\n+new\n"));
                    assert!(
                        !patch.contains("Binary files ") && !patch.contains("GIT binary patch")
                    );
                }
                for (name, state) in [
                    ("contains-nul.dat", ContentState::Binary),
                    ("invalid.dat", ContentState::Binary),
                    ("large.dat", ContentState::TooLarge),
                ] {
                    let file = list
                        .files()
                        .iter()
                        .find(|file| file.path() == Path::new(name))
                        .unwrap_or_else(|| {
                            panic!(
                                "missing {name:?} in {:?} list: {:?}",
                                list.source(),
                                list.files()
                            )
                        });
                    let body = repo.change_content(source, file).await.unwrap();
                    assert_eq!(body.state(), state);
                    assert!(body.patch().is_none());
                    assert!(body.original().is_none() && body.modified().is_none());
                }
            }
            std::fs::write(root.join("new.dat"), "untracked text\n").unwrap();
            let list = read_changes(&repo, ChangeSource::Untracked).await;
            let file = list
                .files()
                .iter()
                .find(|file| file.path() == Path::new("new.dat"))
                .unwrap_or_else(|| {
                    panic!(
                        "missing new.dat in {:?} list: {:?}",
                        list.source(),
                        list.files()
                    )
                });
            let body = repo
                .change_content(ChangeSource::Untracked, file)
                .await
                .unwrap();
            assert!(file.stats().is_none());
            assert_eq!(body.state(), ContentState::Text);
            assert!(body.patch().unwrap().contains("+untracked text"));
        });
    }
}

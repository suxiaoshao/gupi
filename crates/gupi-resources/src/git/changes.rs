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
    Renamed,
    Conflict,
    Special,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    path: PathBuf,
    previous: Option<PathBuf>,
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
    pub fn previous_path(&self) -> Option<&Path> {
        self.previous.as_deref()
    }
    pub fn kind(&self) -> ChangeKind {
        self.kind
    }
    pub fn can_open_worktree(&self) -> bool {
        self.disk
    }
    #[cfg(feature = "test-support")]
    pub fn for_test(path: PathBuf, kind: ChangeKind) -> Self {
        Self {
            path,
            previous: None,
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeList {
    source: ChangeSource,
    files: Vec<Change>,
}
impl ChangeList {
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
    Renamed,
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
            state: ContentState::Text,
            original: Some(original),
            modified: Some(modified),
            patch: Some(patch),
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
    pub async fn changes(&self, source: ChangeSource) -> Result<ChangeList, Error> {
        let out = checked(
            self.root(),
            &[
                "status",
                "--porcelain=v2",
                "-z",
                "--untracked-files=all",
                "--ignore-submodules=none",
            ],
        )
        .await?;
        let mut files = parse_status(&out.stdout, source)?;
        if files.len() > MAX_FILES {
            return Err(Error::TooLarge);
        }
        let stats = if source != ChangeSource::Untracked {
            let mut args = vec![
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "--no-color",
                "--numstat",
                "-z",
                "--find-renames",
            ];
            if source == ChangeSource::Staged {
                args.push("--cached");
            }
            numstats(&checked(self.root(), &args).await?.stdout)?
        } else {
            HashMap::new()
        };
        for file in &mut files {
            file.stats = if matches!(file.kind, ChangeKind::Conflict | ChangeKind::Special) {
                None
            } else {
                stats.get(&file.path).copied().flatten()
            };
            let metadata = std::fs::symlink_metadata(self.root.join(&file.path)).ok();
            file.disk = metadata.as_ref().is_some_and(|m| m.is_file());
            if source != ChangeSource::Staged
                && metadata
                    .as_ref()
                    .is_some_and(|m| !m.is_file() && !m.file_type().is_symlink())
            {
                file.kind = ChangeKind::Special;
                file.stats = None;
            }
            if source != ChangeSource::Staged {
                file.stamp = stamp(&self.root.join(&file.path))?;
            }
        }
        Ok(ChangeList { source, files })
    }
    /// Metadata is checked before and after reading. Never mix a changed worktree
    /// body with a previously selected index/HEAD identity or count snapshot.
    pub async fn change_content(
        &self,
        source: ChangeSource,
        file: &Change,
    ) -> Result<ChangeContent, Error> {
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
                                    &file.path,
                                    &new,
                                    if file.disk { "100644" } else { "120000" },
                                ))
                            } else {
                                let mut args = vec![
                                    "--literal-pathspecs",
                                    "diff",
                                    "--no-ext-diff",
                                    "--no-textconv",
                                    "--no-color",
                                    "--unified=50000",
                                    "--find-renames",
                                ];
                                if source == ChangeSource::Staged {
                                    args.push("--cached");
                                }
                                args.push("--");
                                let path = file
                                    .path
                                    .to_str()
                                    .ok_or_else(|| Error::Failed("Non-Unicode diff path".into()))?;
                                args.push(path);
                                let previous = file.previous.as_ref().and_then(|p| p.to_str());
                                if let Some(previous) = previous {
                                    args.push(previous);
                                }
                                match checked(self.root(), &args).await {
                                    Ok(out) => Some(
                                        String::from_utf8(out.stdout)
                                            .map_err(|e| Error::Failed(e.to_string()))?,
                                    ),
                                    Err(Error::TooLarge) => None,
                                    Err(e) => return Err(e),
                                }
                            };
                            content.state = if patch.is_none() {
                                ContentState::TooLarge
                            } else if old == new && file.kind == ChangeKind::Renamed {
                                ContentState::Renamed
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
        if self
            .changes(source)
            .await?
            .files
            .iter()
            .any(|current| current == file)
        {
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
fn added_patch(path: &Path, text: &str, mode: &str) -> String {
    let old = serde_json::to_string(&format!("a/{}", path.to_string_lossy())).unwrap();
    let new = serde_json::to_string(&format!("b/{}", path.to_string_lossy())).unwrap();
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
    let mut records = bytes.split(|b| *b == 0).filter(|r| !r.is_empty());
    let mut files = Vec::new();
    while let Some(record) = records.next() {
        let (parts, previous) = match record[0] {
            b'?' => {
                if source == ChangeSource::Untracked {
                    files.push(Change {
                        path: record_path(&record[2..])?,
                        previous: None,
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
            b'1' => (record.splitn(9, |b| *b == b' ').collect::<Vec<_>>(), None),
            b'2' => (
                record.splitn(10, |b| *b == b' ').collect::<Vec<_>>(),
                Some(record_path(records.next().ok_or_else(|| {
                    Error::Failed("Missing rename path".into())
                })?)?),
            ),
            b'u' => {
                let parts = record.splitn(11, |b| *b == b' ').collect::<Vec<_>>();
                if source != ChangeSource::Untracked && parts.len() == 11 {
                    files.push(Change {
                        path: record_path(parts[10])?,
                        previous: None,
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
            previous: if source == ChangeSource::Staged {
                previous
            } else {
                None
            },
            kind: if special {
                ChangeKind::Special
            } else {
                match code {
                    b'A' => ChangeKind::Added,
                    b'D' => ChangeKind::Deleted,
                    b'R' | b'C' => ChangeKind::Renamed,
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
    let mut records = bytes.split(|b| *b == 0).filter(|r| !r.is_empty());
    let mut result = HashMap::new();
    while let Some(record) = records.next() {
        let mut parts = record.splitn(3, |b| *b == b'\t');
        let added = parts.next().unwrap_or_default();
        let removed = parts.next().unwrap_or_default();
        let name = parts
            .next()
            .ok_or_else(|| Error::Failed("Invalid Git numstat".into()))?;
        let name = if name.is_empty() {
            records
                .next()
                .ok_or_else(|| Error::Failed("Missing rename source".into()))?;
            records
                .next()
                .ok_or_else(|| Error::Failed("Missing rename target".into()))?
        } else {
            name
        };
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
            let staged = repo.changes(ChangeSource::Staged).await.unwrap();
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
            let staged = repo.changes(ChangeSource::Staged).await.unwrap();
            let unstaged = repo.changes(ChangeSource::Unstaged).await.unwrap();
            let new = repo.changes(ChangeSource::Untracked).await.unwrap();
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
            let staged = repo.changes(ChangeSource::Staged).await.unwrap();
            assert_eq!(
                staged.files()[0].previous_path(),
                Some(Path::new("old.txt"))
            );
            assert_eq!(
                repo.change_content(ChangeSource::Staged, &staged.files()[0])
                    .await
                    .unwrap()
                    .state(),
                ContentState::Renamed
            );
            let list = repo.changes(ChangeSource::Unstaged).await.unwrap();
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
            let list = repo.changes(ChangeSource::Untracked).await.unwrap();
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
            let list = repo.changes(ChangeSource::Untracked).await.unwrap();
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
}

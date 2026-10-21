use super::*;
use regex::RegexBuilder;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

const POOL_LIMIT: usize = 100;
const DISPLAY_LIMIT: usize = 20;
const VISIT_LIMIT: usize = 100_000;
const SEARCH_BUDGET: Duration = Duration::from_secs(2);
const DEBOUNCE: Duration = Duration::from_millis(40);

pub(super) struct Files {
    query: String,
    cwd: PathBuf,
    pub(super) result: Option<Result<Vec<ProjectPath>, String>>,
    cancel: Arc<AtomicBool>,
    _task: Task<()>,
}
impl Drop for Files {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ProjectPath {
    path: PathBuf,
    directory: bool,
    display: String,
}

impl CommandPalette {
    #[cfg(test)]
    pub(crate) fn file_search_ready(&self) -> bool {
        self.files
            .as_ref()
            .is_some_and(|files| files.result.is_some())
    }
    pub(super) fn sync_files(&mut self, cx: &mut Context<Self>) {
        let query = self.input.read(cx).value().to_string();
        if !query.starts_with('@') {
            self.files = None;
            return;
        }
        let Some(cwd) = self
            .home
            .as_ref()
            .and_then(|(_, state)| state.read(cx).current().map(|s| s.info().cwd.clone()))
        else {
            return;
        };
        if self
            .files
            .as_ref()
            .is_some_and(|files| files.query == query && files.cwd == cwd)
        {
            return;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.files_generation += 1;
        let generation = self.files_generation;
        let flag = cancel.clone();
        let scan_query = query.clone();
        let scan_cwd = cwd.clone();
        self.files = Some(Files {
            query,
            cwd,
            result: None,
            cancel,
            _task: cx.spawn(async move |owner, cx| {
                cx.background_executor().timer(DEBOUNCE).await;
                let home = gupi_resources::paths::Discovery::environment()
                    .ok()
                    .map(|paths| paths.home);
                let query = scan_query.clone();
                let cwd = scan_cwd.clone();
                let files = cx
                    .background_spawn(async move { scan(&cwd, &query, home.as_deref(), &flag) })
                    .await;
                let _ = owner.update(cx, |owner, cx| {
                    if !owner.is_open
                        || !owner.target_valid(cx)
                        || owner.files_generation != generation
                    {
                        return;
                    }
                    if let Some(current) = &mut owner.files
                        && current.query == scan_query
                        && current.cwd == scan_cwd
                    {
                        current.result = Some(files);
                        cx.notify();
                    }
                });
            }),
        });
    }

    pub(super) fn file_candidates(&self, _: &str, cx: &App) -> Vec<Row> {
        let Some(files) = &self.files else {
            return vec![];
        };
        // Never expose results for a previous input, even before its Change event runs.
        if files.query != self.input.read(cx).value().as_ref() {
            return vec![];
        }
        let Some(Ok(files)) = &files.result else {
            return vec![];
        };
        let Some(session) = self
            .home
            .as_ref()
            .and_then(|(_, state)| state.read(cx).current())
        else {
            return vec![];
        };
        files
            .iter()
            .map(|entry| Row {
                id: Id::File(entry.path.clone()),
                group: 5,
                title: format!(
                    "{}{}",
                    entry.path.file_name().unwrap_or_default().to_string_lossy(),
                    if entry.directory { "/" } else { "" }
                ),
                description: entry.display.clone(),
                detail: String::new(),
                tooltip: entry.path.to_string_lossy().into_owned(),
                icon: if entry.directory {
                    IconName::Folder
                } else {
                    IconName::FileText
                },
                enabled: self.target_valid(cx)
                    && session.can_edit_draft()
                    && session.can_read_attachments(),
            })
            .collect()
    }

    pub(super) fn enter_directory(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(entry) = self
            .files
            .as_ref()
            .and_then(|files| files.result.as_ref())
            .and_then(|result| result.as_ref().ok())
            .and_then(|files| {
                files
                    .iter()
                    .find(|entry| entry.path == path && entry.directory)
            })
        else {
            return false;
        };
        let quoted = self.input.read(cx).value().starts_with("@\"");
        let display = format!("{}/", entry.display.trim_end_matches('/'));
        let text = if quoted || display.contains(char::is_whitespace) {
            format!("@\"{display}\"")
        } else {
            format!("@{display}")
        };
        let cursor = text.len() - usize::from(text.ends_with('"'));
        self.input.update(cx, |input, cx| {
            input.set_value(text, window, cx);
            input.set_selected_range(cursor..cursor, cx);
            input.focus(window, cx);
        });
        // Select the first row rather than opting into Command's persistent
        // no-selection state. The next model then keeps/reset-selects row 0.
        self.list.update(cx, |list, cx| {
            list.set_selected_index(Some(IndexPath::new(0).section(5)), window, cx)
        });
        self.sync_files(cx);
        cx.notify();
        true
    }
}

struct Scope {
    root: PathBuf,
    query: String,
    display_base: String,
}
fn scope(cwd: &Path, raw: &str, home: Option<&Path>) -> Scope {
    let raw = raw.strip_prefix('@').unwrap_or(raw);
    let raw = if let Some(quoted) = raw.strip_prefix('"') {
        quoted.strip_suffix('"').unwrap_or(quoted)
    } else {
        raw
    };
    let normalized = raw.replace('\\', "/");
    if let Some(slash) = normalized.rfind('/') {
        let display_base = &normalized[..=slash];
        let root = if display_base.starts_with("~/") {
            home.map(|home| home.join(&display_base[2..]))
        } else {
            let path = PathBuf::from(display_base);
            Some(if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            })
        };
        if let Some(root) = root.filter(|root| root.is_dir()) {
            return Scope {
                root,
                query: normalized[slash + 1..].into(),
                display_base: display_base.into(),
            };
        }
    }
    Scope {
        root: cwd.to_owned(),
        query: normalized,
        display_base: String::new(),
    }
}

fn pattern(query: &str) -> Result<regex::Regex, regex::Error> {
    // fd treats a path as escaped segments; plain names remain smart-case regexes.
    let pattern = if query.contains('/') {
        let trimmed = query.trim_matches('/');
        if trimmed.is_empty() {
            query.to_owned()
        } else {
            let mut pattern = trimmed
                .split('/')
                .filter(|part| !part.is_empty())
                .map(regex::escape)
                .collect::<Vec<_>>()
                .join("[/\\\\]");
            if query.ends_with('/') {
                pattern.push_str("[/\\\\]");
            }
            pattern
        }
    } else {
        query.to_owned()
    };
    RegexBuilder::new(&pattern)
        .case_insensitive(!query.chars().any(char::is_uppercase))
        .build()
}

fn score(path: &str, query: &str, directory: bool) -> u16 {
    if query.is_empty() {
        return 1;
    }
    let name = path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_lowercase();
    let query = query.to_lowercase();
    let score = if name == query {
        100
    } else if name.starts_with(&query) {
        80
    } else if name.contains(&query) {
        50
    } else if path.to_lowercase().contains(&query) {
        30
    } else {
        return 0;
    };
    score + if directory { 10 } else { 0 }
}

fn scan(
    cwd: &Path,
    query: &str,
    home: Option<&Path>,
    cancel: &AtomicBool,
) -> Result<Vec<ProjectPath>, String> {
    let scope = scope(cwd, query, home);
    if scope
        .root
        .components()
        .any(|part| part.as_os_str() == ".git")
    {
        return Ok(vec![]);
    }
    let Ok(pattern) = pattern(&scope.query) else {
        return Ok(vec![]);
    };
    let started = Instant::now();
    let mut visited = 0;
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    let mut error = None;
    'pools: for depth in [Some(1), None] {
        let mut matches = 0;
        let mut builder = ignore::WalkBuilder::new(&scope.root);
        builder
            .hidden(false)
            .follow_links(true)
            .max_depth(depth)
            .add_custom_ignore_filename(".fdignore")
            .filter_entry(|entry| entry.file_name() != ".git");
        for entry in builder.build() {
            if cancel.load(Ordering::Relaxed) {
                return Ok(vec![]);
            }
            if visited >= VISIT_LIMIT || started.elapsed() >= SEARCH_BUDGET {
                break 'pools;
            }
            visited += 1;
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    if err.is_io() {
                        error = Some(err.to_string());
                    }
                    continue;
                }
            };
            if entry.depth() == 0 {
                continue;
            }
            let Some(kind) = entry.file_type() else {
                continue;
            };
            if !kind.is_file() && !kind.is_dir() {
                continue;
            }
            let relative = entry
                .path()
                .strip_prefix(&scope.root)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            let match_path = if scope.query.contains('/') {
                relative.as_str()
            } else {
                relative.rsplit('/').next().unwrap_or(&relative)
            };
            if !pattern.is_match(match_path) {
                continue;
            }
            matches += 1;
            let directory = kind.is_dir();
            let ranked_path = format!("{relative}{}", if directory { "/" } else { "" });
            let score = score(&ranked_path, &scope.query, directory);
            if score > 0 && seen.insert(entry.path().to_owned()) {
                paths.push((
                    score,
                    ranked_path,
                    ProjectPath {
                        path: entry.into_path(),
                        directory,
                        display: format!("{}{relative}", scope.display_base),
                    },
                ));
            }
            if matches >= POOL_LIMIT {
                break;
            }
        }
    }
    if paths.is_empty()
        && let Some(error) = error
    {
        return Err(error);
    }
    paths.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| {
                a.1.split('/')
                    .filter(|s| !s.is_empty())
                    .count()
                    .cmp(&b.1.split('/').filter(|s| !s.is_empty()).count())
            })
            .then_with(|| a.1.encode_utf16().count().cmp(&b.1.encode_utf16().count()))
            .then_with(|| a.1.cmp(&b.1))
    });
    Ok(paths
        .into_iter()
        .take(DISPLAY_LIMIT)
        .map(|(_, _, path)| path)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for directory in [
            ".git",
            "src",
            "app/[slug]",
            "src/(group)",
            "foo.rs",
            "deep/nested",
            "path with spaces",
            ".hidden",
        ] {
            std::fs::create_dir_all(root.path().join(directory)).unwrap();
        }
        for file in [
            "foo",
            "foo.rs/x",
            "foobar.rs",
            "afoo.rs",
            "FOOCASE.rs",
            "fooXrs",
            "a_b_c.rs",
            "src/main.rs",
            "src/(group)/file",
            "app/[slug]/page",
            "deep/foo.rs",
            "deep/nested/foo.rs",
            ".hidden/config",
            ".secret",
            "ignored",
            "fdignored",
            ".git/secret",
            "path with spaces/文档.rs",
            "ä.rs",
            "z.rs",
            "中文.rs",
        ] {
            std::fs::write(root.path().join(file), "fixture").unwrap();
        }
        std::fs::write(root.path().join(".gitignore"), "ignored\n").unwrap();
        std::fs::write(root.path().join(".fdignore"), "fdignored\n").unwrap();
        root
    }

    fn search(root: &Path, query: &str) -> Vec<ProjectPath> {
        scan(root, query, Some(root), &AtomicBool::new(false)).unwrap()
    }

    #[test]
    fn ranking_smart_case_regex_hidden_ignore_and_scoped_paths() {
        let root = fixture();
        let files = search(root.path(), "@foo");
        assert_eq!(files[0].display, "foo");
        assert_eq!(files[1].display, "foo.rs");
        assert!(files.iter().any(|entry| entry.display == "FOOCASE.rs"));
        assert_eq!(
            search(root.path(), "@FOO")
                .iter()
                .map(|entry| entry.display.as_str())
                .collect::<Vec<_>>(),
            ["FOOCASE.rs"]
        );
        assert!(search(root.path(), "@abc").is_empty());
        assert!(search(root.path(), "@[").is_empty());
        // Regex may admit fooXrs, but only literal scoring can keep it.
        assert!(
            !search(root.path(), "@foo.rs")
                .iter()
                .any(|entry| entry.display == "fooXrs")
        );
        let empty = search(root.path(), "@");
        assert!(empty.iter().any(|entry| entry.display == ".secret"));
        for query in ["@ignored", "@fdignored", "@.git/"] {
            assert!(search(root.path(), query).is_empty(), "{query}");
        }
        assert!(
            search(root.path(), "@src/")
                .iter()
                .any(|entry| entry.display == "src/main.rs")
        );
        assert_eq!(search(root.path(), "@foo.rs/")[0].display, "foo.rs/x");
        assert_eq!(
            search(root.path(), "@\"path with spaces/\"")[0].display,
            "path with spaces/文档.rs"
        );
        assert!(
            search(root.path(), "@~/src/")
                .iter()
                .any(|entry| entry.display == "~/src/main.rs")
        );
        let absolute = format!("@{}/src/", root.path().display());
        assert!(
            search(root.path(), &absolute)
                .iter()
                .any(|entry| entry.path == root.path().join("src/main.rs"))
        );
        let relative = scope(&root.path().join("src"), "@../foo.rs/", None);
        assert_eq!(relative.query, "");
        assert!(relative.root.is_dir());
        let missing = scope(root.path(), "@missing.name/file", None);
        assert_eq!(missing.root, root.path());
        assert_eq!(missing.query, "missing.name/file");
        assert!(
            pattern("missing.name/file")
                .unwrap()
                .is_match("missing.name/file")
        );
        assert!(
            !pattern("missing.name/file")
                .unwrap()
                .is_match("missingXname/file")
        );
        assert_eq!(score("directory/", "", true), 1);
        assert_eq!(score("foo/", "foo", true), 110);
        assert_eq!(score("src/foo.rs", "src/", false), 30);
    }

    #[test]
    fn bounded_pools_keep_shallow_matches_and_cancel_without_results() {
        let root = fixture();
        for index in 0..300 {
            std::fs::write(root.path().join(format!("deep/item-{index}")), "").unwrap();
        }
        let files = search(root.path(), "@");
        assert_eq!(files.len(), DISPLAY_LIMIT);
        assert!(files.iter().any(|entry| entry.display == "foo"));
        assert!(
            scan(root.path(), "@", None, &AtomicBool::new(true))
                .unwrap()
                .is_empty()
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.path().join("src"), root.path().join("linked"))
                .unwrap();
            std::os::unix::fs::symlink(root.path(), root.path().join("src/loop")).unwrap();
            assert!(
                search(root.path(), "@linked/")
                    .iter()
                    .any(|entry| entry.display == "linked/main.rs")
            );
        }
    }

    #[gpui_kit::test]
    fn file_results_install_model_before_footer_on_first_render(cx: &mut TestAppContext) {
        use gpui_kit::component::Root;
        use gpui_kit::test::TestWindowExt as _;
        let fixture = fixture();
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::host::install_headless(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
            gupi_pi_runtime::init(cx);
            cx.set_global(gupi_settings::layout::LayoutState::default());
        });
        let state = cx.new(|cx| ConversationState::new("unused-pi".into(), cx));
        state.update(cx, |state, _| {
            let info = gupi_conversation::session_catalog::SessionInfo::new(Default::default(), "files".into(), fixture.path().to_owned());
            let mut session = gupi_conversation::conversation::Session::new(info, String::new());
            *session.state_for_test() = Some(serde_json::from_value(serde_json::json!({"sessionId":"fixture","isStreaming":false,"isCompacting":false})).unwrap());
            state.sessions_for_test().insert("files".into(), session);
            *state.selected_for_test() = Some("files".into());
        });
        let mut home = None;
        let (_, visual) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| HomeView::with_state(state.clone(), window, cx));
            home = Some(view.clone());
            Root::new(view, window, cx)
        });
        let home = home.unwrap();
        let panel = visual.update(|window, cx| {
            CommandPalette::open(
                Some((home.downgrade(), state)),
                false,
                "@src/main.rs".into(),
                window,
                cx,
            )
        });
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(50));
        visual.run_until_parked();
        assert!(visual.update(|_, cx| panel.read(cx).file_search_ready()));
        visual.update(|window, cx| {
            panel.update(cx, |panel, cx| {
                // Reproduce the loading -> results boundary: the list still
                // has the empty loading model when its parent starts rendering.
                let _ = RenderOnce::render(Command::new(&panel.list).searchable(false), window, cx);
                assert!(panel.selected(cx).is_none());
                let _ = panel.render(window, cx);
                assert_eq!(
                    panel.selected(cx),
                    Some(Id::File(fixture.path().join("src/main.rs")))
                );
            });
            // One results frame; no refresh/extra render_frame to repair it.
            window.draw(cx).clear(cx);
            let action = window.find("command-send");
            assert_eq!(
                action.label(),
                Some(t(cx, "command-select-resource").as_str())
            );
            assert_ne!(action.disabled(), Some(true));
            window.click("command-send", cx);
        });
        visual.run_until_parked();
        visual.update(|_, cx| {
            assert!(!panel.read(cx).is_open());
            assert_eq!(home.read(cx).input.read(cx).content().tokens().len(), 1);
        });
    }

    #[test]
    fn pi_13_provider_comparison() {
        // Opt in against the installed, exact Pi 1.3 provider. Fixtures remain
        // local and disposable; no agent, extension, auth, or model is started.
        let Ok(provider) = std::env::var("GUPI_PI_AUTOCOMPLETE") else {
            return;
        };
        let root = fixture();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.path().join("src"), root.path().join("linked")).unwrap();
        let mut queries = vec![
            "@",
            "@foo",
            "@FOO",
            "@foo.rs",
            "@foo.*",
            "@[",
            "@src/",
            "@app/[slug]/page",
            "@src/(group)/file",
            "@foo.rs/",
            "@ignored",
            "@fdignored",
            "@abc",
            "@.secret",
            "@.hidden/",
            "@中文",
            "@ä",
            "@.rs",
            "@~/src/",
            "@src/../foo",
            "@missing.name/file",
            "@\"path with spaces/",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        queries.push(format!("@{}/src/", root.path().display()));
        #[cfg(unix)]
        queries.push("@linked/".into());
        let output = std::process::Command::new("node").args(["--input-type=module", "-e", r#"
            import { pathToFileURL } from 'node:url';
            const { CombinedAutocompleteProvider } = await import(pathToFileURL(process.argv[1]).href);
            const p = new CombinedAutocompleteProvider([], process.argv[2], process.argv[3]);
            const out = [];
            for (const q of JSON.parse(process.argv[4])) {
                const result = await p.getSuggestions([q], 0, q.length, { signal: new AbortController().signal });
                out.push((result?.items ?? []).map(item => item.description));
            }
            console.log(JSON.stringify(out));
        "#]).arg(provider).arg(root.path()).arg("fd").arg(serde_json::to_string(&queries).unwrap()).env("HOME", root.path()).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let expected: Vec<Vec<String>> = serde_json::from_slice(&output.stdout).unwrap();
        for (query, expected) in queries.iter().zip(expected) {
            let actual: Vec<_> = search(root.path(), query)
                .into_iter()
                .map(|entry| entry.display)
                .collect();
            if query == "@" || query == "@.rs" {
                // localeCompare's Unicode tie order is deliberately not a
                // dependency of the app. Verify membership and ranking tiers.
                let mut actual_set = actual.clone();
                let mut expected_set = expected.clone();
                actual_set.sort();
                expected_set.sort();
                assert_eq!(actual_set, expected_set, "Pi membership: {query}");
                let tiers = |paths: &[String]| {
                    paths
                        .iter()
                        .map(|path| {
                            (
                                score(
                                    path,
                                    query.trim_start_matches('@'),
                                    root.path().join(path).is_dir(),
                                ),
                                path.split('/').filter(|s| !s.is_empty()).count(),
                                path.encode_utf16().count()
                                    + usize::from(root.path().join(path).is_dir()),
                            )
                        })
                        .collect::<Vec<_>>()
                };
                assert_eq!(tiers(&actual), tiers(&expected), "Pi rank tiers: {query}");
            } else {
                assert_eq!(actual, expected, "Pi query: {query}");
            }
        }
    }
}

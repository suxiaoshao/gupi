//! Temporary text-pair adapter until Kit exposes comparison construction.
use super::*;
use gpui_kit::component::diff::DiffFile;
use similar::TextDiff;

pub(super) struct Prepared {
    pub content: ChangeContent,
    pub normalized: (String, String),
    // None means unchanged: the owner keeps its current block without diff/parse work.
    pub files: Option<Vec<DiffFile>>,
}
impl Prepared {
    pub fn new(
        path: &std::path::Path,
        content: ChangeContent,
        previous: Option<&(String, String)>,
    ) -> Result<Self, String> {
        let normalized = (
            content.original().unwrap_or_default().replace("\r\n", "\n"),
            content.modified().unwrap_or_default().replace("\r\n", "\n"),
        );
        let files = if previous == Some(&normalized) {
            None
        } else if content.state() == ContentState::Text && normalized.0 != normalized.1 {
            let diff = TextDiff::configure()
                .timeout(Duration::from_secs(1))
                .diff_lines(&normalized.0, &normalized.1);
            // Actual names never enter patch syntax. Outer chrome owns the real path.
            let patch = diff
                .unified_diff()
                .context_radius(
                    normalized
                        .0
                        .lines()
                        .count()
                        .max(normalized.1.lines().count()),
                )
                .header("a/file", "b/file")
                .to_string();
            let language = crate::home::files::directory::language(path);
            Some(
                DiffFile::parse(&patch)
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(|file| file.with_language(language))
                    .collect(),
            )
        } else {
            Some(Vec::new())
        };
        Ok(Self {
            content,
            normalized,
            files,
        })
    }
}

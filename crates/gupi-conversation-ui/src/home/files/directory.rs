use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Directory,
    File,
    Other,
}

#[derive(Clone, Debug)]
pub(super) struct Entry {
    pub path: PathBuf,
    pub kind: Kind,
    pub link: bool,
    pub target: Option<PathBuf>,
    pub error: Option<String>,
}

pub(super) struct Directory {
    pub entries: Vec<Entry>,
    pub incomplete: bool,
}

pub(super) fn read(path: &Path) -> Result<Directory, String> {
    let mut entries = Vec::new();
    let mut incomplete = false;
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                incomplete = true;
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(kind) => kind,
            Err(_) => {
                incomplete = true;
                continue;
            }
        };
        let link = file_type.is_symlink();
        let target = link.then(|| fs::read_link(entry.path()).ok()).flatten();
        let (kind, error) = if link {
            match fs::metadata(entry.path()) {
                Ok(metadata) if metadata.is_dir() => (Kind::Directory, None),
                Ok(metadata) if metadata.is_file() => (Kind::File, None),
                Ok(_) => (Kind::Other, None),
                Err(error) => (Kind::Other, Some(error.to_string())),
            }
        } else if file_type.is_dir() {
            (Kind::Directory, None)
        } else if file_type.is_file() {
            (Kind::File, None)
        } else {
            (Kind::Other, None)
        };
        entries.push(Entry {
            path: entry.path(),
            kind,
            link,
            target,
            error,
        });
    }
    entries.sort_by_cached_key(|entry| {
        (
            entry.kind != Kind::Directory,
            entry
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase(),
            entry.path.clone(),
        )
    });
    Ok(Directory {
        entries,
        incomplete,
    })
}

pub(in crate::home) struct Source {
    pub text: String,
    pub long_line: bool,
}

pub(in crate::home) fn source(path: &Path) -> Result<Source, &'static str> {
    const LIMIT: u64 = 1024 * 1024;
    if matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_lowercase()
            .as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "bmp"
            | "ico"
            | "pdf"
            | "zip"
            | "gz"
            | "7z"
            | "mp3"
            | "wav"
            | "mp4"
            | "mov"
            | "exe"
            | "dll"
            | "dylib"
            | "so"
            | "wasm"
    ) {
        return Err("files-unsupported");
    }
    let metadata = fs::metadata(path).map_err(|_| "files-read-failed")?;
    if !metadata.is_file() {
        return Err("files-unsupported");
    }
    let file = fs::File::open(path).map_err(|_| "files-read-failed")?;
    // Recheck the opened object before reading; never read a special file.
    if !file.metadata().map_err(|_| "files-read-failed")?.is_file() {
        return Err("files-unsupported");
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "files-read-failed")?;
    if bytes.len() as u64 > LIMIT {
        return Err("files-too-large");
    }
    if bytes.contains(&0) {
        return Err("files-unsupported");
    }
    let text = String::from_utf8(bytes).map_err(|_| "files-unsupported")?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned();
    if text.lines().count() > 20_000 {
        return Err("files-too-large");
    }
    let long_line = text.lines().any(|line| line.len() > 16 * 1024);
    Ok(Source { text, long_line })
}

pub(in crate::home) fn language(path: &Path) -> &'static str {
    match path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
    {
        "Cargo.lock" => return "toml",
        "Makefile" | "GNUmakefile" | "makefile" => return "make",
        _ => {}
    }
    match path
        .extension()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "rs" => "rust",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "md" | "markdown" => "markdown",
        "sh" | "bash" | "zsh" => "bash",
        "js" | "mjs" | "cjs" | "jsx" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "html" | "htm" => "html",
        "css" => "css",
        "py" | "pyi" => "python",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
        "sql" => "sql",
        "diff" | "patch" => "diff",
        "rb" => "ruby",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "lua" => "lua",
        "php" => "php",
        "scala" => "scala",
        "zig" => "zig",
        "svelte" => "svelte",
        "astro" => "astro",
        "ex" | "exs" => "elixir",
        "erb" => "erb",
        "ejs" => "ejs",
        _ => "text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_and_preview_obey_file_boundaries() {
        let root = std::env::temp_dir().join(format!(
            "gupi-files-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("z-directory")).unwrap();
        fs::write(root.join("a.rs"), "\u{feff}fn main() {}\n").unwrap();
        fs::write(root.join(".hidden"), "hidden").unwrap();
        fs::write(root.join("binary"), [0, 1, 2]).unwrap();
        fs::write(root.join("invalid"), [0xff]).unwrap();
        fs::write(root.join("huge"), vec![b'x'; 1024 * 1024 + 1]).unwrap();
        fs::write(root.join("lines"), "x\n".repeat(20_001)).unwrap();
        fs::write(root.join("long"), "x".repeat(16 * 1024 + 1)).unwrap();
        let dir = read(&root).unwrap();
        assert_eq!(dir.entries[0].kind, Kind::Directory);
        assert!(dir.entries.iter().any(|e| e.path.ends_with(".hidden")));
        assert!(read(&root.join("z-directory")).unwrap().entries.is_empty());
        assert_eq!(source(&root.join("a.rs")).unwrap().text, "fn main() {}\n");
        for name in ["binary", "invalid"] {
            assert_eq!(source(&root.join(name)).err(), Some("files-unsupported"));
        }
        for name in ["huge", "lines"] {
            assert_eq!(source(&root.join(name)).err(), Some("files-too-large"));
        }
        assert!(source(&root.join("long")).unwrap().long_line);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("a.rs"), root.join("linked")).unwrap();
            assert_eq!(source(&root.join("linked")).unwrap().text, "fn main() {}\n");
            assert_eq!(
                read(&root)
                    .unwrap()
                    .entries
                    .iter()
                    .find(|e| e.path.ends_with("linked"))
                    .unwrap()
                    .kind,
                Kind::File
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn links_follow_pi_targets_and_keep_lexical_identity() {
        let root = Path::new("/tmp").join(format!(
            "gupi-links-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let project = root.join("project");
        let outside = root.join("outside");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("source.rs"), "outside fixture").unwrap();
        fs::write(outside.join("huge"), vec![b'x'; 1024 * 1024 + 1]).unwrap();
        let link = |target: &Path, name: &str| {
            std::os::unix::fs::symlink(target, project.join(name)).unwrap()
        };
        link(&outside, "folder");
        link(&outside.join("source.rs"), "alias.rs");
        link(&outside.join("missing"), "broken");
        link(&outside.join("huge"), "huge-link");
        link(&project, "cycle");
        let socket = std::os::unix::net::UnixListener::bind(outside.join("socket")).unwrap();
        link(&outside.join("socket"), "socket-link");
        assert_eq!(
            source(&project.join("socket-link")).err(),
            Some("files-unsupported")
        );
        let entries = read(&project).unwrap().entries;
        let special = entries
            .iter()
            .find(|entry| entry.path.ends_with("socket-link"))
            .unwrap();
        assert_eq!(special.kind, Kind::Other);
        assert!(special.link && special.error.is_none());
        let folder = entries
            .iter()
            .find(|entry| entry.path.ends_with("folder"))
            .unwrap();
        assert_eq!(folder.kind, Kind::Directory);
        assert!(folder.link);
        assert_eq!(folder.target.as_ref(), Some(&outside));
        assert!(
            entries
                .iter()
                .take(2)
                .all(|entry| entry.kind == Kind::Directory)
        );
        let file = entries
            .iter()
            .find(|entry| entry.path.ends_with("alias.rs"))
            .unwrap();
        assert_eq!(file.kind, Kind::File);
        assert!(file.link);
        assert_eq!(source(&file.path).unwrap().text, "outside fixture");
        let child = read(&folder.path)
            .unwrap()
            .entries
            .into_iter()
            .find(|entry| entry.path.ends_with("source.rs"))
            .unwrap();
        assert!(child.path.starts_with(&project));
        assert_eq!(source(&child.path).unwrap().text, "outside fixture");
        let broken = entries
            .iter()
            .find(|entry| entry.path.ends_with("broken"))
            .unwrap();
        assert_eq!(broken.kind, Kind::Other);
        assert!(broken.link && broken.error.is_some());
        assert_eq!(
            source(&project.join("huge-link")).err(),
            Some("files-too-large")
        );
        // Enumeration is one level, even if a link points back to this directory.
        assert_eq!(
            read(&project.join("cycle")).unwrap().entries.len(),
            entries.len()
        );
        // A directory cached as ordinary may be replaced by a link before expansion.
        let replaced = project.join("replaced");
        fs::create_dir(&replaced).unwrap();
        assert_eq!(
            read(&project)
                .unwrap()
                .entries
                .iter()
                .find(|entry| entry.path == replaced)
                .unwrap()
                .kind,
            Kind::Directory
        );
        fs::remove_dir(&replaced).unwrap();
        std::os::unix::fs::symlink(&outside, &replaced).unwrap();
        assert_eq!(
            source(&replaced.join("source.rs")).unwrap().text,
            "outside fixture"
        );
        assert!(
            read(&replaced)
                .unwrap()
                .entries
                .iter()
                .any(|entry| entry.path.ends_with("source.rs"))
        );
        drop(socket);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn filenames_and_ambiguous_extensions_have_deliberate_languages() {
        for (path, expected) in [
            ("Cargo.lock", "toml"),
            ("Makefile", "make"),
            ("view.tsx", "tsx"),
            ("module.mts", "typescript"),
            ("x.h", "c"),
            ("config.nix", "text"),
            ("x.swift", "text"),
            ("README.MD", "markdown"),
        ] {
            assert_eq!(language(Path::new(path)), expected);
        }
    }
}

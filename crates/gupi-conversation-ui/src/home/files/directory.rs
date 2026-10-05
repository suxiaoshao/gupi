use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Directory,
    File,
    Link,
    Other,
}

#[derive(Clone, Debug)]
pub(super) struct Entry {
    pub path: PathBuf,
    pub kind: Kind,
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
        let kind = match entry.file_type() {
            Ok(kind) if kind.is_symlink() => Kind::Link,
            Ok(kind) if kind.is_dir() => Kind::Directory,
            Ok(kind) if kind.is_file() => Kind::File,
            Ok(_) => Kind::Other,
            Err(_) => {
                incomplete = true;
                continue;
            }
        };
        entries.push(Entry {
            path: entry.path(),
            kind,
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
    let metadata = fs::symlink_metadata(path).map_err(|_| "files-read-failed")?;
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
            assert_eq!(
                source(&root.join("linked")).err(),
                Some("files-unsupported")
            );
            assert_eq!(
                read(&root)
                    .unwrap()
                    .entries
                    .iter()
                    .find(|e| e.path.ends_with("linked"))
                    .unwrap()
                    .kind,
                Kind::Link
            );
        }
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

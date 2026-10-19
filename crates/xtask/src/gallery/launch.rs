use super::{GalleryCommand, RuntimeArgs, UiArgs, cleanup};
use crate::{
    context,
    error::{Result, XtaskError},
};
use std::{
    collections::BTreeMap,
    env, fs,
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(target_os = "macos")]
use std::{os::unix::ffi::OsStrExt as _, process::Stdio};

type Environment = BTreeMap<String, String>;
const DIRECTORIES: &[&str] = &["config", "logs", "data", "agent", "sessions"];

pub(super) fn run(command: GalleryCommand) -> Result<()> {
    match command {
        GalleryCommand::Ui(args) => ui(args),
        GalleryCommand::Runtime(args) => runtime(args),
    }
}
fn ui(args: UiArgs) -> Result<()> {
    let parent = env::temp_dir().canonicalize()?;
    if !args.cleanup.is_empty() {
        return Ok(cleanup::cleanup(&parent, &args.cleanup, &args.exclude)?);
    }
    if !args.prepare_only
        && cleanup::candidates(&parent)?
            .iter()
            .any(|item| item.root().join("Gupi UI Gallery.app").is_dir())
    {
        return Err(XtaskError::msg(
            "Existing gallery bundles must be stopped and explicitly removed with --cleanup ROOT_NAME... before launching another",
        ));
    }
    let repo = context::workspace_root()?;
    let pi = which::which(&args.pi)
        .map_err(|_| XtaskError::msg("Pi executable not found; pass --pi /absolute/path/to/pi"))?;
    let binary = binary(&repo, args.no_build, args.prepare_only)?;
    let root = fixture(cleanup::PREFIX, &["project-a", "project-b"])?;
    let overrides = overrides(&root)?;
    let mut environment: Environment = [
        "PATH",
        "TMPDIR",
        "TMP",
        "TEMP",
        "LANG",
        "LC_ALL",
        "USER",
        "LOGNAME",
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XDG_RUNTIME_DIR",
        "SystemRoot",
    ]
    .into_iter()
    .filter_map(|key| {
        env::var_os(key).map(|value| {
            value
                .into_string()
                .map(|value| (key.to_owned(), value))
                .map_err(|_| XtaskError::msg("Non-UTF-8 gallery environment"))
        })
    })
    .collect::<Result<_>>()?;
    environment.extend(overrides.clone());
    let wrapper = wrapper(&root, "pi-gallery", &pi, &repo, "ui-gallery.mjs", &[])?;
    config(&root, &wrapper, true)?;
    fs::write(
        root.join("environment.json"),
        serde_json::to_vec_pretty(&environment)
            .map_err(|_| XtaskError::msg("Cannot encode gallery environment"))?,
    )?;
    println!("Gallery directory: {}", root.display());
    println!(
        "Type /gupi-ui in the new Gupi window. Use project-a / project-b for isolation checks."
    );
    if args.prepare_only {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    let binary = bundle(&root, &binary, &environment)?;
    #[cfg(target_os = "linux")]
    let binary = {
        // Keep the running UI executable inside its fixture so explicit
        // cleanup can recognize it on Linux as well as in the macOS bundle.
        let executable = root.join("gupi-bin");
        fs::copy(binary, &executable)?;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))?;
        executable
    };
    let log = fs::File::create(root.join("logs/process.log"))?;
    let result = (|| {
        let mut process = Command::new(binary)
            .env_clear()
            .envs(&environment)
            .current_dir(root.join("project-a"))
            .stdout(log.try_clone()?)
            .stderr(log)
            .spawn()?;
        fs::write(root.join("gupi.pid"), process.id().to_string())?;
        println!("Gupi PID: {}; exit with Cmd+Q when finished.", process.id());
        process.wait()?;
        Ok(())
    })();
    if args.keep || result.is_err() {
        println!("Kept gallery files: {}", root.display());
    } else {
        cleanup::cleanup_own(&root);
    }
    result
}
fn runtime(args: RuntimeArgs) -> Result<()> {
    let repo = context::workspace_root()?;
    let pi = which::which("pi").map_err(|_| XtaskError::msg("Pi executable not found in PATH"))?;
    let binary = binary(&repo, args.no_build, args.prepare_only)?;
    let root = fixture("gupi-runtime-gallery-", &["project"])?;
    let mut overrides = overrides(&root)?;
    overrides.insert(
        "GUPI_RUNTIME_PROJECT".into(),
        text(&root.join("project"))?.into(),
    );
    overrides.insert(
        "GUPI_RUNTIME_GALLERY_CASE".into(),
        if args.changes { "changes" } else { "runtime" }.into(),
    );
    let wrapper = wrapper(
        &root,
        "pi-runtime",
        &pi,
        &repo,
        "runtime-gallery.mjs",
        &[
            "--provider",
            "gupi-runtime",
            "--model",
            "gallery",
            "--tools",
            "read,write,edit,bash,find,grep,ls,runtime_check",
        ],
    )?;
    config(&root, &wrapper, false)?;
    // Only explicit fixture overrides, never inherited potentially sensitive env.
    fs::write(
        root.join("environment.json"),
        serde_json::to_vec_pretty(&overrides)
            .map_err(|_| XtaskError::msg("Cannot encode gallery environment"))?,
    )?;
    println!(
        "测试目录：{}\n请选择项目：{}\n模型：Runtime Gallery · 慢速运行\n{}退出后保留测试目录，便于查看 session。",
        root.display(),
        root.join("project").display(),
        if args.changes {
            "净差异场景：net.txt 合成净补丁、report.md 写入后编辑、fallback.txt 回退、cancel.txt 抵消。请提前展开 net.txt 观察更新。"
        } else {
            "发送任意文字开始，两轮思考和工具调用约需几分钟。"
        }
    );
    if !args.prepare_only {
        // Runtime intentionally retains inherited variables, with isolated data
        // paths overriding them; it does not create a relaunchable bundle.
        let status = Command::new(binary)
            .envs(&overrides)
            .current_dir(root.join("project"))
            .status()?;
        if !status.success() {
            return Err(XtaskError::msg(format!("Runtime gallery exited: {status}")));
        }
    }
    Ok(())
}
fn fixture(prefix: &str, projects: &[&str]) -> Result<PathBuf> {
    let root = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()?
        .keep()
        .canonicalize()?;
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    for name in DIRECTORIES.iter().chain(projects.iter()) {
        fs::create_dir(root.join(name))?;
    }
    Ok(root)
}
fn text(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| XtaskError::msg("Gallery path must be UTF-8"))
}
fn overrides(root: &Path) -> Result<Environment> {
    [
        ("HOME", root.to_owned()),
        ("GUPI_CONFIG_DIR", root.join("config")),
        ("GUPI_LOG_DIR", root.join("logs")),
        ("GUPI_DATA_DIR", root.join("data")),
        ("PI_CODING_AGENT_DIR", root.join("agent")),
        ("PI_CODING_AGENT_SESSION_DIR", root.join("sessions")),
    ]
    .into_iter()
    .map(|(key, value)| Ok((key.into(), text(&value)?.to_owned())))
    .collect()
}

fn binary(repo: &Path, no_build: bool, prepare_only: bool) -> Result<PathBuf> {
    if !no_build && !prepare_only {
        let status = Command::new("cargo")
            .args(["build", "-p", "gupi", "--locked"])
            .current_dir(repo)
            .status()?;
        if !status.success() {
            return Err(XtaskError::msg("Gupi build failed"));
        }
    }
    let binary = repo.join("target/debug/gupi");
    if !prepare_only && !binary.is_file() {
        return Err(XtaskError::msg("Build Gupi first"));
    }
    Ok(binary)
}
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
fn wrapper(
    root: &Path,
    name: &str,
    pi: &Path,
    repo: &Path,
    extension: &str,
    extra: &[&str],
) -> Result<PathBuf> {
    let extension = repo.join("tests/fixtures").join(extension);
    let mut flags = vec![
        "--offline",
        "--no-extensions",
        "--no-skills",
        "--no-prompt-templates",
        "--no-themes",
        "--no-context-files",
        "--no-approve",
        "--extension",
        text(&extension)?,
    ];
    flags.extend(extra);
    let path = root.join(name);
    let pi = shell_quote(text(pi)?);
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nif [ \"$#\" -eq 1 ] && [ \"$1\" = \"--version\" ]; then exec {pi} \"$@\"; fi\nexec {pi} \"$@\" {}\n",
            flags
                .into_iter()
                .map(shell_quote)
                .collect::<Vec<_>>()
                .join(" ")
        ),
    )?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
    Ok(path)
}
fn config(root: &Path, wrapper: &Path, ui: bool) -> Result<()> {
    let mut config = toml::Table::new();
    config.insert(
        "pi_command".into(),
        toml::Value::String(text(wrapper)?.into()),
    );
    config.insert("language".into(), toml::Value::String("chinese".into()));
    if ui {
        config.insert("theme".into(), toml::Value::String("light".into()));
    }
    fs::write(root.join("config/config.toml"), config.to_string())?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn c_string(value: &[u8]) -> Result<String> {
    if value.contains(&0) {
        return Err(XtaskError::msg(
            "Gallery paths/environment cannot contain NUL",
        ));
    }
    Ok(format!(
        "\"{}\"",
        value
            .iter()
            .map(|byte| format!("\\{byte:03o}"))
            .collect::<String>()
    ))
}
#[cfg(target_os = "macos")]
fn bundle(root: &Path, binary: &Path, environment: &Environment) -> Result<PathBuf> {
    let contents = root.join("Gupi UI Gallery.app/Contents");
    fs::create_dir_all(contents.join("MacOS"))?;
    let executable = contents.join("MacOS/gupi-bin");
    fs::copy(binary, &executable)?;
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))?;
    let directories = DIRECTORIES
        .iter()
        .chain(["project-a"].iter())
        .map(|name| c_string(root.join(name).as_os_str().as_bytes()))
        .collect::<Result<Vec<_>>>()?
        .join(",\n");
    let env = environment
        .iter()
        .map(|(key, value)| c_string(format!("{key}={value}").as_bytes()))
        .collect::<Result<Vec<_>>>()?
        .join(",\n");
    let project = c_string(root.join("project-a").as_os_str().as_bytes())?;
    let executable = c_string(executable.as_os_str().as_bytes())?;
    let source = format!(
        r#"#include <stdio.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <unistd.h>
static const char *directories[] = {{{directories}, NULL}};
static char *const environment[] = {{{env}, NULL}};
int main(int argc, char **argv) {{
    struct stat info;
    for (const char **path = directories; *path; ++path) {{
        if (stat(*path, &info) != 0 || !S_ISDIR(info.st_mode)) {{
            fputs("gallery: required fixture directory unavailable\n", stderr); return 1;
        }}
    }}
    if (chdir({project}) != 0) {{ perror("gallery: chdir"); return 1; }}
    char **arguments = calloc((size_t)argc + 1, sizeof(char *));
    if (!arguments) {{ perror("gallery: arguments"); return 1; }}
    arguments[0] = {executable};
    for (int index = 1; index < argc; ++index) arguments[index] = argv[index];
    execve(arguments[0], arguments, environment);
    perror("gallery: execve"); return 127;
}}
"#
    );
    let source_path = root.join("gallery-bootstrap.c");
    fs::write(&source_path, source)?;
    fs::set_permissions(&source_path, fs::Permissions::from_mode(0o600))?;
    let bootstrap = contents.join("MacOS/gupi");
    let status = Command::new("/usr/bin/cc")
        .args(["-Os", "-Wall", "-Wextra", "-Werror"])
        .arg(source_path)
        .arg("-o")
        .arg(&bootstrap)
        .stdin(Stdio::null())
        .status()?;
    if !status.success() {
        return Err(XtaskError::msg("Gallery bootstrap compilation failed"));
    }
    fs::set_permissions(&bootstrap, fs::Permissions::from_mode(0o700))?;
    let mut plist = plist::Dictionary::new();
    for (key, value) in [
        ("CFBundleExecutable", "gupi"),
        ("CFBundleIdentifier", "top.sushao.gupi.ui-gallery"),
        ("CFBundleName", "Gupi UI Gallery"),
        ("CFBundlePackageType", "APPL"),
        ("CFBundleVersion", "1"),
    ] {
        plist.insert(key.into(), value.into());
    }
    plist.insert("NSHighResolutionCapable".into(), true.into());
    plist.insert(
        "LSEnvironment".into(),
        plist::Value::Dictionary(
            environment
                .iter()
                .map(|(key, value)| (key.clone(), plist::Value::String(value.clone())))
                .collect(),
        ),
    );
    plist::Value::Dictionary(plist).to_file_xml(contents.join("Info.plist"))?;
    Ok(bootstrap)
}

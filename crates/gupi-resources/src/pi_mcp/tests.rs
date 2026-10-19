use super::*;
use crate::pi_config_files::{File, Kind};
use serde_json::json;

fn server(state: &str) -> Value {
    json!({"name":"fixture","scope":"global","enabled":true,"state":state,"tools":["private-tool-name"],"resources":true,"transport":"https://private.invalid?credential=fixture","error":"private-error","source":"private-source","override":"private-override","toolExposure":{"private-tool-name":"direct"}})
}
#[test]
fn projection_validates_types_and_never_retains_nonwhitelisted_fields() {
    for (raw, expected) in [
        ("connecting", State::Connecting),
        ("connected", State::Connected),
        ("disconnected", State::Disconnected),
        ("needs-auth", State::NeedsAuth),
        ("failed", State::Failed),
        ("closed", State::Closed),
        ("disabled", State::Disabled),
    ] {
        for code in [0, 1] {
            let text =
                json!({"servers":[server(raw)],"errors":["private-error"],"note":"private-note"})
                    .to_string();
            let report = parse(Some(code), text.as_bytes()).unwrap();
            assert!(report.has_configuration_errors() && report.has_untrusted_project());
            assert_eq!(report.servers()[0].state(), expected);
            assert_eq!(report.servers()[0].tools(), 1);
            assert!(report.servers()[0].has_resources());
            assert!(report.server(ServerScope::Global, "fixture").is_some());
            assert!(report.server(ServerScope::Project, "fixture").is_none());
            assert!(!format!("{report:?}").contains("private"));
        }
    }
    let mut valid = json!({"servers":[server("connected")],"errors":[]});
    for key in ["name", "scope", "enabled", "state", "tools"] {
        let mut missing = valid.clone();
        missing["servers"][0].as_object_mut().unwrap().remove(key);
        assert!(parse(Some(0), missing.to_string().as_bytes()).is_none());
        let mut wrong = valid.clone();
        wrong["servers"][0][key] = json!(12);
        assert!(parse(Some(0), wrong.to_string().as_bytes()).is_none());
    }
    for (key, value) in [
        ("state", json!("future")),
        ("scope", json!("extension")),
        ("tools", json!([1])),
        ("resources", json!("yes")),
        ("name", json!("https://private.invalid")),
    ] {
        let mut wrong = valid.clone();
        wrong["servers"][0][key] = value;
        assert!(parse(Some(0), wrong.to_string().as_bytes()).is_none());
    }
    assert!(parse(Some(2), valid.to_string().as_bytes()).is_none());
    assert!(parse(None, valid.to_string().as_bytes()).is_none());
    valid["servers"] = json!([]);
    valid["errors"] = json!(["private-error"]);
    assert!(
        parse(Some(1), valid.to_string().as_bytes())
            .unwrap()
            .has_configuration_errors()
    );
}
#[test]
fn oauth_uses_saved_http_definition_trust_and_exact_override_identity() {
    let dir = tempfile::tempdir().unwrap();
    let global = dir.path().join("mcp.json");
    std::fs::write(&global,json!({"mcpServers":{"oauth":{"url":"https://fixture.invalid"},"provider":{"url":"https://fixture.invalid","auth":{"provider":"p"}},"auth-null":{"url":"https://fixture.invalid","auth":null},"header":{"url":"https://fixture.invalid","headers":{"aUtHoRiZaTiOn":"private"}},"stdio":{"command":"private-command"},"bad-headers":{"url":"https://fixture.invalid","headers":{"X-Key":1}},"--help":{"url":"https://fixture.invalid"}}}).to_string()).unwrap();
    let global = File::load(Kind::Mcp, &global).unwrap();
    assert_eq!(
        global.mcp_oauth_eligibility("oauth", None, &Scope::Global, false),
        Eligibility::Eligible
    );
    for name in ["provider", "auth-null", "header", "stdio", "bad-headers"] {
        assert_eq!(
            global.mcp_oauth_eligibility(name, None, &Scope::Global, false),
            Eligibility::Ineligible
        );
    }
    assert_eq!(
        global.mcp_oauth_eligibility("--help", None, &Scope::Global, false),
        Eligibility::UnsupportedName
    );
    let project = dir.path().join("project.json");
    std::fs::write(&project,r#"{"mcpServers":{"oauth":{"enabled":false},"missing":{"enabled":true},"invalid":{"headers":{}},"full":{"url":"http://fixture.invalid"}}}"#).unwrap();
    let project = File::load(Kind::Mcp, &project).unwrap();
    let scope = Scope::Project(dir.path().into());
    assert_eq!(
        project.mcp_oauth_eligibility("oauth", Some(&global), &scope, true),
        Eligibility::Eligible
    );
    assert_eq!(
        project.mcp_oauth_eligibility("oauth", Some(&global), &scope, false),
        Eligibility::UntrustedProject
    );
    assert_eq!(
        project.mcp_oauth_eligibility("full", Some(&global), &scope, true),
        Eligibility::Eligible
    );
    for name in ["missing", "invalid"] {
        assert_eq!(
            project.mcp_oauth_eligibility(name, Some(&global), &scope, true),
            Eligibility::Ineligible
        );
    }
}
#[test]
fn cli_help_names_are_blocked_and_other_options_are_safely_positional() {
    for name in ["--help", "-h", "", "bad\nname"] {
        assert!(Action::Login(name.into()).arguments().is_none());
        assert!(Action::Logout(name.into()).arguments().is_none());
    }
    assert_eq!(
        Action::Login("--timeout".into()).arguments().unwrap(),
        ["mcp", "login", "--timeout", "300", "--", "--timeout"]
    );
    assert_eq!(
        Action::Logout("--json".into()).arguments().unwrap(),
        ["mcp", "logout", "--", "--json"]
    );
}

#[cfg(unix)]
mod subprocess {
    use super::*;
    use std::{os::unix::fs::PermissionsExt as _, path::Path, process::Stdio};
    fn script(dir: &Path, body: &str) -> OsString {
        let path = dir.join("fake-pi");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        path.into_os_string()
    }
    fn env(dir: &Path) -> Variables {
        let variables: Vec<_> = ["HOME", "PI_CODING_AGENT_DIR", "PI_CODING_AGENT_SESSION_DIR"]
            .map(|name| (name.into(), dir.as_os_str().to_owned()))
            .into();
        Box::pin(std::future::ready(variables))
    }
    async fn until(path: &Path) {
        tokio::time::timeout(Duration::from_secs(3), async {
            while !path.is_file() {
                tokio::time::sleep(Duration::from_millis(5)).await
            }
        })
        .await
        .unwrap();
    }
    async fn gone(pid: i32) {
        tokio::time::timeout(Duration::from_secs(3), async {
            while unsafe { libc::kill(pid, 0) } == 0 {
                tokio::time::sleep(Duration::from_millis(10)).await
            }
        })
        .await
        .expect("owned fixture child must be stopped/reaped");
    }
    #[tokio::test]
    async fn projects_cwd_exit_mapping_limits_and_private_output() {
        let dir = tempfile::tempdir().unwrap();
        let record = dir.path().join("cwd");
        let reply = json!({"servers":[server("needs-auth")],"errors":[]}).to_string();
        let command = script(
            dir.path(),
            &format!(
                "/bin/pwd > '{}'\nprintf '%s' '{}'\nprintf 'private-stderr' >&2\nexit 1",
                record.display(),
                reply
            ),
        );
        let out = run(
            command,
            Action::Check,
            Scope::Project(dir.path().into()),
            env(dir.path()),
            Cancellation::new(),
        )
        .await;
        assert!(matches!(out, Outcome::Checked(_)));
        assert!(!format!("{out:?}").contains("private"));
        assert_eq!(
            std::fs::read_to_string(&record).unwrap().trim(),
            dir.path().canonicalize().unwrap().to_str().unwrap()
        );
        for (code, expected) in [(0, Outcome::Completed), (1, Outcome::Failed)] {
            let command = script(
                dir.path(),
                &format!(
                    "/bin/pwd > '{}'\nprintf 'private-stdout'\nexit {code}",
                    record.display()
                ),
            );
            assert_eq!(
                run(
                    command,
                    Action::Logout("fixture".into()),
                    Scope::Global,
                    env(dir.path()),
                    Cancellation::new()
                )
                .await,
                expected
            );
            let cwd = std::fs::read_to_string(&record).unwrap();
            assert_ne!(Path::new(cwd.trim()), dir.path());
            assert!(!Path::new(cwd.trim()).exists());
        }
        let huge = dir.path().join("huge");
        std::fs::write(&huge, vec![b'x'; Action::Check.output_limit() + 1]).unwrap();
        let command = script(
            dir.path(),
            &format!("/bin/cat '{}'\nexec /bin/sleep 30", huge.display()),
        );
        assert_eq!(
            run(
                command,
                Action::Check,
                Scope::Global,
                env(dir.path()),
                Cancellation::new()
            )
            .await,
            Outcome::Failed
        );
    }
    #[tokio::test]
    async fn cancellation_timeout_and_dropped_awaiter_keep_cleanup_alive() {
        for mode in ["cancel", "timeout", "drop"] {
            let dir = tempfile::tempdir().unwrap();
            let parent = dir.path().join("parent");
            let descendant = dir.path().join("descendant");
            let command = script(
                dir.path(),
                &format!(
                    "printf '%s' \"$$\" > '{}'\n/bin/sleep 30 &\nprintf '%s' \"$!\" > '{}'\nwait",
                    parent.display(),
                    descendant.display()
                ),
            );
            let cancel = Cancellation::new();
            let token = cancel.clone();
            let vars = env(dir.path());
            let task = tokio::spawn(run_with(
                command,
                Action::Check,
                Scope::Global,
                vars,
                token,
                if mode == "timeout" {
                    Duration::from_secs(2)
                } else {
                    Duration::from_secs(10)
                },
            ));
            until(&descendant).await;
            let parent_pid = std::fs::read_to_string(&parent).unwrap().parse().unwrap();
            let child_pid: i32 = std::fs::read_to_string(&descendant)
                .unwrap()
                .parse()
                .unwrap();
            if mode == "drop" {
                task.abort();
                let _ = task.await;
            } else {
                if mode == "cancel" {
                    cancel.cancel();
                }
                assert_eq!(
                    task.await.unwrap(),
                    if mode == "timeout" {
                        Outcome::TimedOut
                    } else {
                        Outcome::Cancelled
                    }
                );
            }
            gone(parent_pid).await;
            // A killed orphan can briefly remain a zombie until the OS reaps it.
            let status = std::process::Command::new("/bin/ps")
                .args(["-p", &child_pid.to_string(), "-o", "stat="])
                .stdout(Stdio::piped())
                .output()
                .unwrap();
            assert!(
                !status.status.success()
                    || String::from_utf8_lossy(&status.stdout)
                        .trim()
                        .starts_with('Z')
            );
        }
    }
}
#[tokio::test]
async fn environment_loading_counts_toward_cancellation_and_deadline() {
    let stalled = || -> Variables { Box::pin(std::future::pending()) };
    let cancel = Cancellation::new();
    let task = tokio::spawn(run_with(
        "pi".into(),
        Action::Check,
        Scope::Global,
        stalled(),
        cancel.clone(),
        Duration::from_secs(30),
    ));
    cancel.cancel();
    let out = tokio::time::timeout(Duration::from_secs(3), task).await;
    assert_eq!(out.unwrap().unwrap(), Outcome::Cancelled);
    let out = tokio::time::timeout(
        Duration::from_secs(3),
        run_with(
            "pi".into(),
            Action::Check,
            Scope::Global,
            stalled(),
            Cancellation::new(),
            Duration::from_millis(50),
        ),
    )
    .await;
    assert_eq!(out.unwrap(), Outcome::TimedOut);
}

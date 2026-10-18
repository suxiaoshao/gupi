//! Whitelisted protocol only: subprocess output and stderr never reach callers.
use super::addressable_provider;
use serde_json::Value;
use std::{ffi::OsString, process::Stdio, time::Duration};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckStatus {
    Ready,
    NotReady,
    Invalid,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckReason {
    ProviderNotFound,
    NotConfigured,
    NotAvailable,
    InvalidState,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Check {
    status: CheckStatus,
    reason: Option<CheckReason>,
    kind: Option<super::Kind>,
}
impl Check {
    pub fn status(&self) -> CheckStatus {
        self.status
    }
    pub fn reason(&self) -> Option<CheckReason> {
        self.reason
    }
    pub fn kind(&self) -> Option<super::Kind> {
        self.kind
    }
}
fn parse(provider: &str, code: Option<i32>, bytes: &[u8]) -> Option<Check> {
    if !matches!(code, Some(0..=2)) {
        return None;
    }
    let root: Value = serde_json::from_slice(bytes).ok()?;
    if root.get("provider")?.as_str()? != provider {
        return None;
    }
    let status = match root.get("status")?.as_str()? {
        "ready" => CheckStatus::Ready,
        "not_ready" => CheckStatus::NotReady,
        "invalid" => CheckStatus::Invalid,
        _ => return None,
    };
    let reason = match root.get("reason").and_then(Value::as_str) {
        Some("provider_not_found") => Some(CheckReason::ProviderNotFound),
        Some("credentials_not_configured") => Some(CheckReason::NotConfigured),
        Some("credential_not_available") => Some(CheckReason::NotAvailable),
        Some("invalid_state") => Some(CheckReason::InvalidState),
        _ => None,
    };
    let kind = match root.get("authType").and_then(Value::as_str) {
        Some("api_key") => Some(super::Kind::ApiKey),
        Some("oauth") => Some(super::Kind::OAuth),
        _ => None,
    };
    Some(Check {
        status,
        reason,
        kind,
    })
}
/// No shell, refresh, credentials flag or stderr capture. A timeout kills the child.
pub async fn check(
    command: &str,
    provider: &str,
    variables: Vec<(OsString, OsString)>,
) -> Option<Check> {
    // Pi scans these flags before consuming --provider's following value.
    // In particular, passing --credentials would opt into credential output.
    if !addressable_provider(provider)
        || matches!(
            provider,
            "--credentials" | "--json" | "--no-refresh" | "--min-expiry"
        )
    {
        return None;
    }
    let command = command.to_owned();
    let resolve_variables = variables.clone();
    let attempt = async move {
        let executable = tokio::task::spawn_blocking(move || {
            pi_rpc::probe::resolve_command_with_env(&command, &resolve_variables)
        })
        .await
        .ok()?
        .ok()?;
        let mut cmd = tokio::process::Command::new(executable);
        cmd.args([
            "auth",
            "check",
            "--provider",
            provider,
            "--json",
            "--no-refresh",
        ])
        .envs(variables)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);
        let output = cmd.output().await.ok()?;
        parse(provider, output.status.code(), &output.stdout)
    };
    tokio::time::timeout(Duration::from_secs(10), attempt)
        .await
        .ok()?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_is_whitelisted() {
        for code in 0..=2 {
            for status in ["ready", "not_ready", "invalid"] {
                let text = format!(
                    r#"{{"provider":"p","status":"{status}","reason":"secret","authType":"secret","credentials":"secret"}}"#
                );
                let result = parse("p", Some(code), text.as_bytes()).unwrap();
                assert_eq!(result.reason(), None);
                assert_eq!(result.kind(), None);
                assert!(!format!("{result:?}").contains("secret"));
            }
        }
        for text in [
            "",
            "secret",
            r#"{"provider":"other","status":"ready"}"#,
            r#"{"provider":"p","status":"secret"}"#,
        ] {
            assert!(parse("p", Some(2), text.as_bytes()).is_none());
        }
        assert!(parse("p", Some(3), br#"{"provider":"p","status":"ready"}"#).is_none());
        for (reason, expected) in [
            ("provider_not_found", CheckReason::ProviderNotFound),
            ("credentials_not_configured", CheckReason::NotConfigured),
            ("credential_not_available", CheckReason::NotAvailable),
            ("invalid_state", CheckReason::InvalidState),
        ] {
            let text = format!(
                r#"{{"provider":"p","status":"not_ready","reason":"{reason}","authType":"oauth"}}"#
            );
            let result = parse("p", Some(1), text.as_bytes()).unwrap();
            assert_eq!(result.reason(), Some(expected));
            assert_eq!(result.kind(), Some(super::super::Kind::OAuth));
        }
    }
}

#[cfg(test)]
mod transport_tests {
    #[tokio::test]
    async fn unsafe_provider_ids_never_start_a_process() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("started");
        let script = dir.path().join("fake-pi");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::write(
                &script,
                format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
            )
            .unwrap();
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
            for provider in [
                "--credentials",
                "--json",
                "--no-refresh",
                "--min-expiry",
                " leading",
                "trailing ",
                "line\nfeed",
                "nul\0id",
                "line\u{2028}separator",
            ] {
                assert!(
                    super::check(script.to_str().unwrap(), provider, vec![])
                        .await
                        .is_none()
                );
                assert!(!marker.exists());
            }
        }
        #[cfg(not(unix))]
        let _ = (marker, script);
    }
}

#[cfg(all(test, unix))]
mod process_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt as _;
    #[tokio::test]
    async fn structured_nonzero_replies_and_unicode_arguments_are_supported() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("fake-pi");
        for (code, status) in [(0, "ready"), (1, "not_ready"), (2, "invalid")] {
            for provider in ["中文供应商", "provider with spaces"] {
                let reply=serde_json::json!({"provider":provider,"status":status,"authType":"api_key","extra":"fixture-secret"}).to_string();
                let record = dir.path().join("argv");
                std::fs::write(&script,format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nprintf '%s' '{}'\nprintf '%s' 'fixture-stderr-secret' >&2\nexit {code}\n",record.display(),reply)).unwrap();
                std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
                let result = check(script.to_str().unwrap(), provider, vec![])
                    .await
                    .unwrap();
                assert_eq!(
                    result.status(),
                    match code {
                        0 => CheckStatus::Ready,
                        1 => CheckStatus::NotReady,
                        _ => CheckStatus::Invalid,
                    }
                );
                assert!(!format!("{result:?}").contains("secret"));
                assert_eq!(
                    std::fs::read_to_string(record)
                        .unwrap()
                        .lines()
                        .collect::<Vec<_>>(),
                    [
                        "auth",
                        "check",
                        "--provider",
                        provider,
                        "--json",
                        "--no-refresh"
                    ]
                );
            }
        }
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf '%s' 'fixture-stderr-secret' >&2\nexit 2\n",
        )
        .unwrap();
        assert!(check(script.to_str().unwrap(), "p", vec![]).await.is_none());
    }
}

#[cfg(all(test, unix))]
mod resolution_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt as _;
    #[tokio::test]
    async fn command_lookup_uses_snapshot_path_and_missing_commands_do_not_spawn() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("pi-auth-snapshot-fixture");
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf '%s' '{\"provider\":\"p\",\"status\":\"ready\"}'\n",
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
        let env = vec![(OsString::from("PATH"), dir.path().as_os_str().to_owned())];
        assert_eq!(
            check("pi-auth-snapshot-fixture", "p", env.clone())
                .await
                .unwrap()
                .status(),
            CheckStatus::Ready
        );
        assert!(
            check("pi-auth-does-not-exist-fixture", "p", env)
                .await
                .is_none()
        );
    }
}

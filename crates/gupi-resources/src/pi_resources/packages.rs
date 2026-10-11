//! Project package commands preserve Pi's extension trust decision.
use super::*;

/// Pi package identity: npm name, git repository without ref, or resolved local path.
pub fn package_identity(root: &Path, source: &str) -> Option<String> {
    if let Some(name) = crate::catalog::installed_name(source) {
        return Some(format!("npm:{name}"));
    }
    let path = discovery::package_path(root, source, &Value::Null, &[], false)?;
    let mut normalized = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            part => normalized.push(part.as_os_str()),
        }
    }
    if source.starts_with("git:") || source.contains("://") {
        Some(format!(
            "git:{}",
            normalized.strip_prefix(root.join("git")).ok()?.display()
        ))
    } else {
        Some(format!("local:{}", normalized.display()))
    }
}

/// Re-read declarations before mutation; a delta must never be removed or overwritten.
pub fn project_package_allowed(cwd: &Path, action: &str, source: &str) -> Result<(), Error> {
    if !matches!(action, "install" | "remove") {
        return Err(Error("Project package updates are not supported".into()));
    }
    let root = cwd.join(".pi");
    let settings = read_json(&root.join("settings.json"))?;
    if !settings.is_object() {
        return Err(Error("Pi settings must be an object".into()));
    }
    let requested_base = if action == "install" {
        cwd
    } else {
        root.as_path()
    };
    let identity = package_identity(requested_base, source)
        .ok_or_else(|| Error("Invalid package source".into()))?;
    let mut found = false;
    for entry in settings["packages"].as_array().into_iter().flatten() {
        let Some(saved) = entry.as_str().or_else(|| entry["source"].as_str()) else {
            continue;
        };
        if package_identity(&root, saved).as_ref() == Some(&identity) {
            found = true;
            if entry["autoload"] == false {
                return Err(Error(
                    "This project already has read-only filters for this package".into(),
                ));
            }
        }
    }
    if action == "remove" && !found {
        return Err(Error("Project package is no longer declared".into()));
    }
    Ok(())
}

pub async fn project_package_action(
    command: PathBuf,
    agent: PathBuf,
    cwd: PathBuf,
    action: &'static str,
    source: String,
    env: Vec<(OsString, OsString)>,
) -> Result<(), Error> {
    let source = source.trim().to_owned();
    if source.is_empty() || source.starts_with('-') {
        return Err(Error("Enter a package source".into()));
    }
    let check_agent = agent.clone();
    let check_cwd = cwd.clone();
    let check_source = source.clone();
    smol::unblock(move || {
        use crate::pi_settings::trust::{self, Decision};
        if !check_cwd.is_absolute()
            || !check_cwd.is_dir()
            || trust::canonical(&check_cwd) != check_cwd
        {
            return Err(Error(
                "Project folder is no longer available at its original path".into(),
            ));
        }
        let trusted = match trust::decision(&check_agent, &check_cwd)
            .map_err(|e| Error(e.to_string()))?
        {
            Decision::Trusted(_) => true,
            Decision::Distrusted(_) => false,
            Decision::Unset => {
                read_json(&check_agent.join("settings.json"))?["defaultProjectTrust"] == "always"
            }
        };
        if !trusted {
            return Err(Error("Trust this project before changing packages".into()));
        }
        project_package_allowed(&check_cwd, action, &check_source)
    })
    .await?;
    // Pi resolves command-line local sources against cwd, while declarations
    // resolve against .pi. Preserve that distinction for remove.
    let command_source = if action == "remove"
        && package_identity(&cwd.join(".pi"), &source)
            .is_some_and(|identity| identity.starts_with("local:"))
    {
        discovery::package_path(&cwd.join(".pi"), &source, &Value::Null, &[], false)
            .ok_or_else(|| Error("Invalid local package source".into()))?
            .to_string_lossy()
            .into_owned()
    } else {
        source.clone()
    };
    let output = tokio::process::Command::new(command)
        .envs(env)
        .env("PI_CODING_AGENT_DIR", &agent)
        .current_dir(&cwd)
        .arg(action)
        .arg(&command_source)
        .arg("--local")
        // No approval override: Pi extensions may still deny or remember trust.
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output()
        .await?;
    if output.status.success() {
        Ok(())
    } else {
        let message = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
        Err(Error(format!(
            "Pi {action} ({}): {}",
            output.status,
            message.chars().take(4000).collect::<String>()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_catalog_and_deltas_use_project_storage_and_identity() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = dir.path();
        std::fs::create_dir(cwd.join(".pi")).unwrap();
        std::fs::write(cwd.join(".pi/settings.json"), r#"{"packages":["npm:@scope/pkg@2",{"source":"npm:filtered@1","autoload":false},"git:github.com/u/repo@main","../local"]}"#).unwrap();
        let catalog = scan_project(cwd).unwrap();
        assert_eq!(catalog.packages.len(), 4);
        assert_eq!(
            catalog.packages[0].path,
            cwd.join(".pi/npm/node_modules/@scope/pkg")
        );
        assert!(catalog.packages[1].is_delta());
        assert!(project_package_allowed(cwd, "install", "npm:filtered@3").is_err());
        assert!(project_package_allowed(cwd, "remove", "npm:filtered").is_err());
        assert!(project_package_allowed(cwd, "update", "npm:@scope/pkg").is_err());
        assert!(project_package_allowed(cwd, "remove", "npm:@scope/pkg").is_ok());
        assert_eq!(
            package_identity(cwd, "git:github.com/u/repo@main"),
            package_identity(cwd, "https://github.com/u/repo.git@dev")
        );
        assert_eq!(
            package_identity(&cwd.join(".pi"), "../local"),
            package_identity(cwd, "./local")
        );
        assert!(!cwd.join(".pi/npm").exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn project_command_preserves_extension_trust_and_scope() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let agent = dir.path().join("agent");
        let cwd = dir.path().join("project");
        std::fs::create_dir(&agent).unwrap();
        std::fs::create_dir(&cwd).unwrap();
        let cwd = crate::pi_settings::trust::canonical(&cwd);
        let command = dir.path().join("pi-fixture");
        let capture = dir.path().join("capture");
        std::fs::write(
            &command,
            "#!/bin/sh\nprintf '%s\\n' \"$PWD\" \"$PI_CODING_AGENT_DIR\" \"$@\" > \"$CAPTURE\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o700)).unwrap();
        let env = vec![(OsString::from("CAPTURE"), capture.as_os_str().to_owned())];
        assert!(
            project_package_action(
                command.clone(),
                agent.clone(),
                cwd.clone(),
                "install",
                "npm:example".into(),
                env.clone()
            )
            .await
            .is_err()
        );
        assert!(!capture.exists(), "untrusted project never starts Pi");
        crate::pi_settings::trust::trust(&agent, &cwd).unwrap();
        let before = std::fs::read(agent.join("trust.json")).unwrap();
        project_package_action(
            command.clone(),
            agent.clone(),
            cwd.clone(),
            "install",
            "npm:example".into(),
            env.clone(),
        )
        .await
        .unwrap();
        let args = std::fs::read_to_string(&capture).unwrap();
        let lines: Vec<_> = args.lines().collect();
        assert_eq!(Path::new(lines[0]), cwd);
        assert_eq!(Path::new(lines[1]), agent);
        assert_eq!(&lines[2..], &["install", "npm:example", "--local"]);
        assert_eq!(std::fs::read(agent.join("trust.json")).unwrap(), before);
        std::fs::write(
            &command,
            "#!/bin/sh\necho 'extension denied project' >&2\nexit 1\n",
        )
        .unwrap();
        let result =
            project_package_action(command, agent, cwd, "install", "npm:example".into(), env).await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("extension denied project")
        );
    }
}

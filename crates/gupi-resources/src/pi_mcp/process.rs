use super::{Action, Cancellation, Outcome, Variables, parse};
use crate::pi_settings::Scope;
use std::{ffi::OsString, process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt as _, process::Command};
mod containment;

pub(super) async fn execute(
    command: OsString,
    action: Action,
    scope: Scope,
    variables: Variables,
    cancellation: Cancellation,
    duration: Duration,
) -> Outcome {
    let Some(arguments) = action.arguments() else {
        return Outcome::Failed;
    };
    let deadline = tokio::time::Instant::now() + duration;
    let preparation = async {
        let variables = variables.await;
        let (directory, temporary) = tokio::task::spawn_blocking(move || -> Option<_> {
            match scope {
                Scope::Global => {
                    let temporary = tempfile::tempdir().ok()?;
                    Some((temporary.path().to_owned(), Some(temporary)))
                }
                Scope::Project(directory) if directory.is_dir() => Some((directory, None)),
                _ => None,
            }
        })
        .await
        .ok()??;
        let env = variables.clone();
        let executable = tokio::task::spawn_blocking(move || {
            pi_rpc::probe::resolve_command_with_env(&command, &env)
        })
        .await
        .ok()?
        .ok()?;
        Some((directory, temporary, executable, variables))
    };
    let prepared = tokio::select! {
        biased;
        _ = cancellation.cancelled() => return Outcome::Cancelled,
        _ = tokio::time::sleep_until(deadline) => return Outcome::TimedOut,
        value = preparation => value,
    };
    let Some((directory, _temporary, executable, variables)) = prepared else {
        return Outcome::Failed;
    };
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .envs(variables)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .kill_on_drop(true);
    let Ok(mut containment) = containment::Containment::configure(&mut command, &action) else {
        return Outcome::Failed;
    };
    let Ok(mut child) = command.spawn() else {
        return Outcome::Failed;
    };
    if containment.attach(&child).is_err() {
        containment.stop();
        let _ = child.start_kill();
        let _ = child.wait().await;
        return Outcome::Failed;
    }
    let stdout = child.stdout.take();
    let completion = async {
        let mut stdout = stdout.ok_or(())?;
        let read = async {
            let mut retained = Vec::new();
            let mut buffer = [0; 8192];
            let mut count = 0usize;
            loop {
                let size = stdout.read(&mut buffer).await.map_err(|_| ())?;
                if size == 0 {
                    break;
                }
                count += size;
                if count > action.output_limit() {
                    return Err(());
                }
                if matches!(action, Action::Check) {
                    retained.extend_from_slice(&buffer[..size]);
                }
            }
            Ok(retained)
        };
        let wait = async { child.wait().await.map_err(|_| ()) };
        let (status, bytes) = tokio::try_join!(wait, read)?;
        Ok::<_, ()>(match action {
            Action::Check => parse(status.code(), &bytes)
                .map(Outcome::Checked)
                .unwrap_or(Outcome::Failed),
            _ if status.success() => Outcome::Completed,
            _ => Outcome::Failed,
        })
    };
    let outcome = tokio::select! {
        biased;
        _ = cancellation.cancelled() => Outcome::Cancelled,
        _ = tokio::time::sleep_until(deadline) => Outcome::TimedOut,
        value = completion => value.unwrap_or(Outcome::Failed),
    };
    containment.stop();
    let _ = child.start_kill();
    let _ = child.wait().await;
    outcome
}

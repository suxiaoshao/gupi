//! Login-shell exports shared by every Pi-related subprocess.
use gpui_kit::{App, Global};
use gpui_tokio::Tokio;
use std::{ffi::OsString, fmt::Display};
#[cfg(any(target_os = "macos", test))]
use std::{future::Future, sync::Arc};
#[cfg(any(target_os = "macos", test))]
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Snapshot {
    variables: Vec<(OsString, OsString)>,
    warning: Option<String>,
}
impl Snapshot {
    pub fn variables(&self) -> Vec<(OsString, OsString)> {
        self.variables.clone()
    }
    pub fn explain(&self, error: impl Display) -> String {
        match &self.warning {
            Some(warning) => format!("{error}\nShell environment: {warning}"),
            None => error.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct Environment {
    inherited: Vec<(OsString, OsString)>,
    capture: bool,
    #[cfg(any(target_os = "macos", test))]
    loaded: Arc<Mutex<Option<Snapshot>>>,
}
struct GlobalEnvironment(Environment);
impl Global for GlobalEnvironment {}

pub fn init(cx: &mut App) {
    let environment = Environment::new(should_capture());
    cx.set_global(GlobalEnvironment(environment.clone()));
    Tokio::spawn(cx, async move { environment.load(false).await }).detach();
}

pub fn current(cx: &App) -> Environment {
    cx.try_global::<GlobalEnvironment>()
        .map(|environment| environment.0.clone())
        .unwrap_or_else(|| Environment::new(false))
}

fn should_capture() -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::io::IsTerminal;
        // Terminal launches already have the user's deliberately selected environment.
        !(std::io::stdin().is_terminal()
            || std::io::stdout().is_terminal()
            || std::io::stderr().is_terminal())
    }
    #[cfg(not(target_os = "macos"))]
    false
}

impl Environment {
    fn new(capture: bool) -> Self {
        Self {
            inherited: std::env::vars_os().collect(),
            capture,
            #[cfg(any(target_os = "macos", test))]
            loaded: Arc::new(Mutex::new(None)),
        }
    }
    pub async fn load(&self, refresh: bool) -> Snapshot {
        #[cfg(target_os = "macos")]
        if self.capture {
            return self
                .load_with(refresh, crate::shell_environment::capture())
                .await;
        }
        let _ = (refresh, self.capture);
        Snapshot {
            variables: self.inherited.clone(),
            warning: None,
        }
    }
    #[cfg(any(target_os = "macos", test))]
    async fn load_with(
        &self,
        refresh: bool,
        capture: impl Future<Output = Result<Vec<(OsString, OsString)>, String>>,
    ) -> Snapshot {
        let mut loaded = self.loaded.lock().await;
        if !refresh && let Some(snapshot) = &*loaded {
            return snapshot.clone();
        }
        let snapshot = match capture.await {
            Ok(variables) => Snapshot {
                variables,
                warning: None,
            },
            Err(warning) => {
                // Keep absolute/system-installed commands usable when shell setup fails.
                tracing::warn!("Could not load login shell environment; using inherited exports");
                Snapshot {
                    variables: self.inherited.clone(),
                    warning: Some(warning),
                }
            }
        };
        *loaded = Some(snapshot.clone());
        snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn cached_failure_uses_inherited_environment_until_explicit_retry() {
        let environment = Environment {
            inherited: vec![("PATH".into(), "/original/bin".into())],
            capture: true,
            loaded: Arc::new(Mutex::new(None)),
        };
        let failed = environment
            .load_with(false, async { Err("shell timed out".into()) })
            .await;
        assert_eq!(
            failed.variables(),
            [("PATH".into(), "/original/bin".into())]
        );
        assert!(failed.explain("pi unavailable").contains("shell timed out"));
        let cached = environment
            .clone()
            .load_with(false, async { panic!("must share cached result") })
            .await;
        assert_eq!(cached.variables, failed.variables);
        let retried = environment
            .load_with(true, async {
                Ok(vec![
                    ("PATH".into(), "/node/bin:/pi/bin".into()),
                    ("HTTPS_PROXY".into(), "http://proxy.example:8080".into()),
                ])
            })
            .await;
        assert_eq!(retried.variables().len(), 2);
        assert!(retried.warning.is_none());
        assert_eq!(
            environment
                .load_with(false, async { panic!("must reuse successful result") })
                .await
                .variables,
            retried.variables
        );
    }
}
